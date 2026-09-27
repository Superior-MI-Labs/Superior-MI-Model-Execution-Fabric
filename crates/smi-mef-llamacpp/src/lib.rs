#![doc = "Bounded `llama.cpp` HTTP provider for Model Execution Fabric R0."]

use core::fmt;
use std::io::Read;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use smi_ir::{NameError, QualifiedName, Sha256Digest};
use smi_mef_core::{
    text_generate_v1, ProviderEvidenceError, ProviderQualification, TextGenerateRequest,
    TextGenerateResponse, TextGenerationReceipt, PROVIDER_QUALIFICATION_SCHEMA_VERSION,
};

const QUALIFICATION_SCHEMA_VERSION: u16 = 1;
const EXECUTION_SCHEMA_VERSION: u16 = 1;
const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
const MAX_HTTP_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_HTTP_RESPONSE_BYTES_U64: u64 = 4 * 1024 * 1024;
const PROVIDER_NAME: &str = "provider.llamacpp.http";

/// Error returned by the bounded `llama.cpp` adapter.
#[derive(Debug)]
pub enum LlamaCppError {
    InvalidEndpoint(String),
    Identity(NameError),
    Transport(String),
    HttpStatus {
        route: &'static str,
        status: u16,
    },
    ResponseTooLarge {
        route: &'static str,
        limit_bytes: usize,
    },
    MalformedResponse {
        route: &'static str,
        message: String,
    },
    NotReady(String),
    NoModels,
    AmbiguousModels(Vec<String>),
    RequestedModelMissing(String),
    Qualification(ProviderEvidenceError),
    QualificationMismatch(String),
    Serialization(serde_json::Error),
}

impl fmt::Display for LlamaCppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEndpoint(message) => write!(formatter, "invalid endpoint: {message}"),
            Self::Identity(error) => write!(formatter, "provider identity error: {error}"),
            Self::Transport(message) => write!(formatter, "HTTP transport error: {message}"),
            Self::HttpStatus { route, status } => {
                write!(formatter, "{route} returned HTTP status {status}")
            }
            Self::ResponseTooLarge { route, limit_bytes } => {
                write!(
                    formatter,
                    "{route} response exceeded the {limit_bytes}-byte limit"
                )
            }
            Self::MalformedResponse { route, message } => {
                write!(formatter, "malformed {route} response: {message}")
            }
            Self::NotReady(status) => write!(formatter, "llama.cpp server is not ready: {status}"),
            Self::NoModels => formatter.write_str("llama.cpp advertised no loaded models"),
            Self::AmbiguousModels(models) => write!(
                formatter,
                "llama.cpp advertised multiple models; select one explicitly: {}",
                models.join(", ")
            ),
            Self::RequestedModelMissing(model) => {
                write!(formatter, "requested model '{model}' was not advertised")
            }
            Self::Qualification(error) => {
                write!(formatter, "qualification evidence error: {error}")
            }
            Self::QualificationMismatch(message) => {
                write!(formatter, "qualification mismatch: {message}")
            }
            Self::Serialization(error) => write!(formatter, "JSON serialization error: {error}"),
        }
    }
}

impl std::error::Error for LlamaCppError {}

impl From<NameError> for LlamaCppError {
    fn from(error: NameError) -> Self {
        Self::Identity(error)
    }
}

impl From<ProviderEvidenceError> for LlamaCppError {
    fn from(error: ProviderEvidenceError) -> Self {
        Self::Qualification(error)
    }
}

impl From<serde_json::Error> for LlamaCppError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

/// Provider-specific qualification document retaining raw probe responses as evidence.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LlamaCppQualification {
    schema: u16,
    qualification: ProviderQualification,
    health_response: Value,
    models_response: Value,
    completion_probe_response: Value,
}

impl LlamaCppQualification {
    /// Returns provider-independent qualification evidence.
    #[must_use]
    pub const fn qualification(&self) -> &ProviderQualification {
        &self.qualification
    }

    /// Serializes this qualification document.
    ///
    /// # Errors
    /// Returns a JSON serialization error if serialization fails.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Parses a qualification document and revalidates its provider/capability identity.
    ///
    /// # Errors
    /// Returns [`LlamaCppError`] if JSON is invalid or the qualification is not for this adapter.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, LlamaCppError> {
        let document: Self = serde_json::from_slice(bytes)?;
        document.validate()?;
        Ok(document)
    }

    fn validate(&self) -> Result<(), LlamaCppError> {
        if self.schema != QUALIFICATION_SCHEMA_VERSION {
            return Err(LlamaCppError::QualificationMismatch(format!(
                "unsupported schema {}",
                self.schema
            )));
        }
        if self.qualification.schema() != PROVIDER_QUALIFICATION_SCHEMA_VERSION {
            return Err(LlamaCppError::QualificationMismatch(format!(
                "unsupported provider qualification schema {}",
                self.qualification.schema()
            )));
        }
        if self.qualification.provider().as_str() != PROVIDER_NAME {
            return Err(LlamaCppError::QualificationMismatch(
                "provider identity is not provider.llamacpp.http".to_owned(),
            ));
        }
        let capability = text_generate_v1()?;
        if self.qualification.capability() != &capability {
            return Err(LlamaCppError::QualificationMismatch(
                "capability is not text.generate@1.0.0".to_owned(),
            ));
        }
        let health = parse_health(&self.health_response)?;
        if health != "ok" {
            return Err(LlamaCppError::NotReady(health.to_owned()));
        }
        let models = parse_models(&self.models_response)?;
        if !models
            .iter()
            .any(|model| model == self.qualification.selected_model())
        {
            return Err(LlamaCppError::QualificationMismatch(
                "selected model is absent from retained model evidence".to_owned(),
            ));
        }
        parse_completion(&self.completion_probe_response)?;
        let evidence_digest = digest_probe_evidence(
            &self.health_response,
            &self.models_response,
            &self.completion_probe_response,
        )?;
        if self.qualification.provider_evidence_sha256() != &evidence_digest {
            return Err(LlamaCppError::QualificationMismatch(
                "retained probe evidence digest does not match qualification".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Provider-specific execution evidence plus the normalized response contract.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LlamaCppExecution {
    schema: u16,
    receipt: TextGenerationReceipt,
    response: TextGenerateResponse,
    provider_response: Value,
}

impl LlamaCppExecution {
    /// Returns the provider-independent receipt.
    #[must_use]
    pub const fn receipt(&self) -> &TextGenerationReceipt {
        &self.receipt
    }

    /// Returns the normalized provider-independent response.
    #[must_use]
    pub const fn response(&self) -> &TextGenerateResponse {
        &self.response
    }

    /// Serializes this execution evidence.
    ///
    /// # Errors
    /// Returns a JSON serialization error if serialization fails.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

/// Synchronous bounded client for one explicitly configured local `llama.cpp` endpoint.
pub struct LlamaCppClient {
    endpoint: Url,
    client: Client,
}

impl LlamaCppClient {
    /// Constructs a local-only client with a bounded request timeout.
    ///
    /// # Errors
    /// Returns [`LlamaCppError`] when the endpoint is not a loopback HTTP base URL or the HTTP
    /// client cannot be constructed.
    pub fn new(endpoint: &str, timeout: Duration) -> Result<Self, LlamaCppError> {
        let endpoint = validate_endpoint(endpoint)?;
        let client = Client::builder()
            .timeout(timeout)
            .no_proxy()
            .build()
            .map_err(|error| LlamaCppError::Transport(error.to_string()))?;
        Ok(Self { endpoint, client })
    }

    /// Constructs a client using the R0 default timeout.
    ///
    /// # Errors
    /// Returns [`LlamaCppError`] when the endpoint is invalid or the HTTP client cannot be built.
    pub fn with_default_timeout(endpoint: &str) -> Result<Self, LlamaCppError> {
        Self::new(endpoint, Duration::from_secs(DEFAULT_TIMEOUT_SECONDS))
    }

    /// Probes health and model identity before allowing a capability claim.
    ///
    /// When exactly one model is advertised, `requested_model` may be omitted. Multiple models
    /// require explicit selection.
    ///
    /// # Errors
    /// Returns [`LlamaCppError`] for transport/protocol failures, unhealthy servers, absent models,
    /// or ambiguous/missing model selection.
    pub fn qualify(
        &self,
        requested_model: Option<&str>,
        environment_sha256: Sha256Digest,
    ) -> Result<LlamaCppQualification, LlamaCppError> {
        let health_json = self.get_json("/health")?;
        let health = parse_health(&health_json)?;
        if health != "ok" {
            return Err(LlamaCppError::NotReady(health.to_owned()));
        }

        let models_json = self.get_json("/v1/models")?;
        let models = parse_models(&models_json)?;
        let selected_model = select_model(&models, requested_model)?;
        let completion_probe_request = CompletionRequest {
            model: &selected_model,
            prompt: "MEF provider qualification probe.",
            max_tokens: 1,
            temperature: 0.0,
            stream: false,
        };
        let completion_probe_json = self.post_completion(&completion_probe_request)?;
        parse_completion(&completion_probe_json)?;
        let evidence_digest =
            digest_probe_evidence(&health_json, &models_json, &completion_probe_json)?;
        let qualification = ProviderQualification::new(
            QualifiedName::new(PROVIDER_NAME)?,
            self.endpoint.as_str(),
            text_generate_v1()?,
            environment_sha256,
            selected_model,
            models,
            evidence_digest,
        )?;
        Ok(LlamaCppQualification {
            schema: QUALIFICATION_SCHEMA_VERSION,
            qualification,
            health_response: health_json,
            models_response: models_json,
            completion_probe_response: completion_probe_json,
        })
    }

    /// Executes one bounded text-generation request using prior qualification evidence.
    ///
    /// # Errors
    /// Returns [`LlamaCppError`] if the qualification does not match this endpoint, the request
    /// fails, or the provider response cannot satisfy the R0 response contract.
    pub fn generate(
        &self,
        qualification: &LlamaCppQualification,
        request: &TextGenerateRequest,
    ) -> Result<LlamaCppExecution, LlamaCppError> {
        qualification.validate()?;
        if qualification.qualification.endpoint() != self.endpoint.as_str() {
            return Err(LlamaCppError::QualificationMismatch(
                "qualification endpoint does not match client endpoint".to_owned(),
            ));
        }

        let provider_request = CompletionRequest {
            model: qualification.qualification.selected_model(),
            prompt: request.prompt(),
            max_tokens: request.max_output_tokens(),
            temperature: 0.0,
            stream: false,
        };
        let request_bytes = serde_json::to_vec(request)?;
        let provider_response = self.post_completion(&provider_request)?;
        let normalized = parse_completion(&provider_response)?;
        let provider_response_bytes = serde_json::to_vec(&provider_response)?;
        let qualification_sha256 = qualification.qualification.identity()?;
        let receipt = TextGenerationReceipt::new(
            qualification_sha256,
            QualifiedName::new(PROVIDER_NAME)?,
            self.endpoint.as_str(),
            qualification.qualification.selected_model(),
            text_generate_v1()?,
            Sha256Digest::of_bytes(&request_bytes),
            Sha256Digest::of_bytes(&provider_response_bytes),
        );
        Ok(LlamaCppExecution {
            schema: EXECUTION_SCHEMA_VERSION,
            receipt,
            response: normalized,
            provider_response,
        })
    }

    fn get_json(&self, route: &'static str) -> Result<Value, LlamaCppError> {
        let response = self
            .client
            .get(self.route(route)?)
            .send()
            .map_err(|error| LlamaCppError::Transport(error.to_string()))?;
        parse_bounded_json_response(response, route)
    }

    fn post_completion(&self, request: &CompletionRequest<'_>) -> Result<Value, LlamaCppError> {
        let response = self
            .client
            .post(self.route("/v1/completions")?)
            .json(request)
            .send()
            .map_err(|error| LlamaCppError::Transport(error.to_string()))?;
        parse_bounded_json_response(response, "/v1/completions")
    }

    fn route(&self, route: &str) -> Result<Url, LlamaCppError> {
        self.endpoint
            .join(route.trim_start_matches('/'))
            .map_err(|error| LlamaCppError::InvalidEndpoint(error.to_string()))
    }
}

fn parse_bounded_json_response(
    response: reqwest::blocking::Response,
    route: &'static str,
) -> Result<Value, LlamaCppError> {
    let status = response.status();
    if !status.is_success() {
        return Err(LlamaCppError::HttpStatus {
            route,
            status: status.as_u16(),
        });
    }
    let body = read_bounded_body(response, route)?;
    serde_json::from_slice(&body).map_err(|error| LlamaCppError::MalformedResponse {
        route,
        message: error.to_string(),
    })
}

fn read_bounded_body(
    response: reqwest::blocking::Response,
    route: &'static str,
) -> Result<Vec<u8>, LlamaCppError> {
    let mut body = Vec::new();
    let mut bounded = response.take(MAX_HTTP_RESPONSE_BYTES_U64 + 1);
    bounded
        .read_to_end(&mut body)
        .map_err(|error| LlamaCppError::Transport(error.to_string()))?;
    if body.len() > MAX_HTTP_RESPONSE_BYTES {
        return Err(LlamaCppError::ResponseTooLarge {
            route,
            limit_bytes: MAX_HTTP_RESPONSE_BYTES,
        });
    }
    Ok(body)
}

#[derive(Serialize)]
struct CompletionRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

fn validate_endpoint(endpoint: &str) -> Result<Url, LlamaCppError> {
    let mut url =
        Url::parse(endpoint).map_err(|error| LlamaCppError::InvalidEndpoint(error.to_string()))?;
    if url.scheme() != "http" {
        return Err(LlamaCppError::InvalidEndpoint(
            "Wave 2 accepts local HTTP endpoints only".to_owned(),
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| LlamaCppError::InvalidEndpoint("endpoint must include a host".to_owned()))?;
    if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
        return Err(LlamaCppError::InvalidEndpoint(
            "Wave 2 endpoint must resolve syntactically to loopback".to_owned(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(LlamaCppError::InvalidEndpoint(
            "credentials are not accepted in the endpoint URL".to_owned(),
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(LlamaCppError::InvalidEndpoint(
            "query and fragment are not accepted in the endpoint URL".to_owned(),
        ));
    }
    if url.path() != "/" && !url.path().is_empty() {
        return Err(LlamaCppError::InvalidEndpoint(
            "endpoint must be a base URL without a path".to_owned(),
        ));
    }
    url.set_path("/");
    Ok(url)
}

fn parse_health(value: &Value) -> Result<&str, LlamaCppError> {
    value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| LlamaCppError::MalformedResponse {
            route: "/health",
            message: "missing string field 'status'".to_owned(),
        })
}

fn parse_models(value: &Value) -> Result<Vec<String>, LlamaCppError> {
    let data = value.get("data").and_then(Value::as_array).ok_or_else(|| {
        LlamaCppError::MalformedResponse {
            route: "/v1/models",
            message: "missing array field 'data'".to_owned(),
        }
    })?;
    let mut models = Vec::with_capacity(data.len());
    for model in data {
        let id = model.get("id").and_then(Value::as_str).ok_or_else(|| {
            LlamaCppError::MalformedResponse {
                route: "/v1/models",
                message: "model entry missing string field 'id'".to_owned(),
            }
        })?;
        if !id.is_empty() {
            models.push(id.to_owned());
        }
    }
    models.sort();
    models.dedup();
    if models.is_empty() {
        return Err(LlamaCppError::NoModels);
    }
    Ok(models)
}

fn select_model(models: &[String], requested: Option<&str>) -> Result<String, LlamaCppError> {
    if let Some(requested) = requested {
        return models
            .iter()
            .find(|model| model.as_str() == requested)
            .cloned()
            .ok_or_else(|| LlamaCppError::RequestedModelMissing(requested.to_owned()));
    }
    if models.len() == 1 {
        return Ok(models[0].clone());
    }
    Err(LlamaCppError::AmbiguousModels(models.to_vec()))
}

fn parse_completion(value: &Value) -> Result<TextGenerateResponse, LlamaCppError> {
    let choices = value
        .get("choices")
        .and_then(Value::as_array)
        .ok_or_else(|| LlamaCppError::MalformedResponse {
            route: "/v1/completions",
            message: "missing array field 'choices'".to_owned(),
        })?;
    if choices.len() != 1 {
        let choice_count = choices.len();
        return Err(LlamaCppError::MalformedResponse {
            route: "/v1/completions",
            message: format!("expected exactly one choice, got {choice_count}"),
        });
    }
    let choice = &choices[0];
    let text = choice.get("text").and_then(Value::as_str).ok_or_else(|| {
        LlamaCppError::MalformedResponse {
            route: "/v1/completions",
            message: "choice missing string field 'text'".to_owned(),
        }
    })?;
    let finish_reason = choice
        .get("finish_reason")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let output_tokens = value
        .get("usage")
        .and_then(|usage| usage.get("completion_tokens"))
        .and_then(Value::as_u64);
    Ok(TextGenerateResponse::new(
        text,
        finish_reason,
        output_tokens,
    ))
}

fn digest_probe_evidence(
    health: &Value,
    models: &Value,
    completion: &Value,
) -> Result<Sha256Digest, serde_json::Error> {
    let health = serde_json::to_vec(health)?;
    let models = serde_json::to_vec(models)?;
    let completion = serde_json::to_vec(completion)?;
    let mut evidence = Vec::with_capacity(health.len() + models.len() + completion.len() + 2);
    evidence.extend_from_slice(&health);
    evidence.push(0);
    evidence.extend_from_slice(&models);
    evidence.push(0);
    evidence.extend_from_slice(&completion);
    Ok(Sha256Digest::of_bytes(&evidence))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{parse_completion, parse_health, parse_models, select_model, validate_endpoint};

    #[test]
    fn endpoint_is_restricted_to_loopback_http() {
        assert!(validate_endpoint("http://127.0.0.1:1920").is_ok());
        assert!(validate_endpoint("http://localhost:1920/").is_ok());
        assert!(validate_endpoint("https://127.0.0.1:1920").is_err());
        assert!(validate_endpoint("http://192.168.1.50:1920").is_err());
        assert!(validate_endpoint("http://127.0.0.1:1920/path").is_err());
    }

    #[test]
    fn health_requires_ok_status() {
        assert_eq!(
            parse_health(&json!({"status": "ok"})).expect("health"),
            "ok"
        );
        assert!(parse_health(&json!({"unexpected": true})).is_err());
    }

    #[test]
    fn model_selection_never_guesses_when_multiple_are_advertised() {
        let models = parse_models(&json!({
            "object": "list",
            "data": [{"id": "b"}, {"id": "a"}]
        }))
        .expect("models");
        assert_eq!(models, vec!["a".to_owned(), "b".to_owned()]);
        assert!(select_model(&models, None).is_err());
        assert_eq!(select_model(&models, Some("b")).expect("selection"), "b");
    }

    #[test]
    fn completion_normalizes_provider_response() {
        let response = parse_completion(&json!({
            "choices": [{"text": "hello", "finish_reason": "stop"}],
            "usage": {"completion_tokens": 1}
        }))
        .expect("completion");
        assert_eq!(response.text(), "hello");
        assert_eq!(response.finish_reason(), Some("stop"));
        assert_eq!(response.output_tokens(), Some(1));
    }
}
