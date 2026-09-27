#![doc = "Bounded AIR HTTP provider for Model Execution Fabric R0."]

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
const PROVIDER_NAME: &str = "provider.air.http";

/// Error returned by the bounded AIR adapter.
#[derive(Debug)]
pub enum AirError {
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
    ModelIdentityMismatch {
        selected: String,
        loaded: String,
    },
    Qualification(ProviderEvidenceError),
    QualificationMismatch(String),
    Serialization(serde_json::Error),
}

impl fmt::Display for AirError {
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
            Self::NotReady(status) => write!(formatter, "AIR server is not ready: {status}"),
            Self::NoModels => formatter.write_str("AIR advertised no loaded models"),
            Self::AmbiguousModels(models) => write!(
                formatter,
                "AIR advertised multiple models; select one explicitly: {}",
                models.join(", ")
            ),
            Self::RequestedModelMissing(model) => {
                write!(formatter, "requested model '{model}' was not advertised")
            }
            Self::ModelIdentityMismatch { selected, loaded } => write!(
                formatter,
                "selected AIR model '{selected}' does not match loaded model '{loaded}'"
            ),
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

impl std::error::Error for AirError {}

impl From<NameError> for AirError {
    fn from(error: NameError) -> Self {
        Self::Identity(error)
    }
}

impl From<ProviderEvidenceError> for AirError {
    fn from(error: ProviderEvidenceError) -> Self {
        Self::Qualification(error)
    }
}

impl From<serde_json::Error> for AirError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

/// Provider-specific AIR qualification document retaining raw probe responses as evidence.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AirQualification {
    schema: u16,
    qualification: ProviderQualification,
    health_response: Value,
    models_response: Value,
    model_response: Value,
    runtime_response: Value,
    completion_probe_response: Value,
}

impl AirQualification {
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
    /// Returns [`AirError`] if JSON is invalid or the qualification is not for this adapter.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AirError> {
        let document: Self = serde_json::from_slice(bytes)?;
        document.validate()?;
        Ok(document)
    }

    fn validate(&self) -> Result<(), AirError> {
        if self.schema != QUALIFICATION_SCHEMA_VERSION {
            return Err(AirError::QualificationMismatch(format!(
                "unsupported schema {}",
                self.schema
            )));
        }
        if self.qualification.schema() != PROVIDER_QUALIFICATION_SCHEMA_VERSION {
            return Err(AirError::QualificationMismatch(format!(
                "unsupported provider qualification schema {}",
                self.qualification.schema()
            )));
        }
        if self.qualification.provider().as_str() != PROVIDER_NAME {
            return Err(AirError::QualificationMismatch(
                "provider identity is not provider.air.http".to_owned(),
            ));
        }
        let capability = text_generate_v1()?;
        if self.qualification.capability() != &capability {
            return Err(AirError::QualificationMismatch(
                "capability is not text.generate@1.0.0".to_owned(),
            ));
        }
        let health = parse_health(&self.health_response)?;
        if health != "ok" {
            return Err(AirError::NotReady(health.to_owned()));
        }
        let models = parse_models(&self.models_response)?;
        if !models
            .iter()
            .any(|model| model == self.qualification.selected_model())
        {
            return Err(AirError::QualificationMismatch(
                "selected model is absent from retained model evidence".to_owned(),
            ));
        }
        let loaded_model = parse_model_id(&self.model_response)?;
        if loaded_model != self.qualification.selected_model() {
            return Err(AirError::QualificationMismatch(
                "selected model does not match retained /model evidence".to_owned(),
            ));
        }
        parse_runtime_backend(&self.runtime_response)?;
        parse_completion(&self.completion_probe_response)?;
        let evidence_digest = digest_probe_evidence(
            &self.health_response,
            &self.models_response,
            &self.model_response,
            &self.runtime_response,
            &self.completion_probe_response,
        )?;
        if self.qualification.provider_evidence_sha256() != &evidence_digest {
            return Err(AirError::QualificationMismatch(
                "retained probe evidence digest does not match qualification".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Provider-specific execution evidence plus the normalized response contract.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AirExecution {
    schema: u16,
    receipt: TextGenerationReceipt,
    response: TextGenerateResponse,
    provider_response: Value,
}

impl AirExecution {
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

/// Synchronous bounded client for one explicitly configured local AIR endpoint.
pub struct AirClient {
    endpoint: Url,
    client: Client,
}

impl AirClient {
    /// Constructs a local-only client with a bounded request timeout.
    ///
    /// # Errors
    /// Returns [`AirError`] when the endpoint is not a loopback HTTP base URL or the HTTP client
    /// cannot be constructed.
    pub fn new(endpoint: &str, timeout: Duration) -> Result<Self, AirError> {
        let endpoint = validate_endpoint(endpoint)?;
        let client = Client::builder()
            .timeout(timeout)
            .no_proxy()
            .build()
            .map_err(|error| AirError::Transport(error.to_string()))?;
        Ok(Self { endpoint, client })
    }

    /// Constructs a client using the R0 default timeout.
    ///
    /// # Errors
    /// Returns [`AirError`] when the endpoint is invalid or the HTTP client cannot be built.
    pub fn with_default_timeout(endpoint: &str) -> Result<Self, AirError> {
        Self::new(endpoint, Duration::from_secs(DEFAULT_TIMEOUT_SECONDS))
    }

    /// Probes AIR health, advertised model identity, loaded model identity, runtime state, and
    /// bounded completion compatibility before allowing a capability claim.
    ///
    /// When exactly one model is advertised, `requested_model` may be omitted. Multiple models
    /// require explicit selection.
    ///
    /// # Errors
    /// Returns [`AirError`] for transport/protocol failures, unhealthy servers, absent models,
    /// ambiguous/missing model selection, or disagreement between `/v1/models` and `/model`.
    pub fn qualify(
        &self,
        requested_model: Option<&str>,
        environment_sha256: Sha256Digest,
    ) -> Result<AirQualification, AirError> {
        let health_json = self.get_json("/health")?;
        let health = parse_health(&health_json)?;
        if health != "ok" {
            return Err(AirError::NotReady(health.to_owned()));
        }

        let catalog_json = self.get_json("/v1/models")?;
        let models = parse_models(&catalog_json)?;
        let selected_model = select_model(&models, requested_model)?;

        let loaded_model_json = self.get_json("/model")?;
        let loaded_model = parse_model_id(&loaded_model_json)?;
        if loaded_model != selected_model {
            return Err(AirError::ModelIdentityMismatch {
                selected: selected_model,
                loaded: loaded_model.to_owned(),
            });
        }

        let runtime_json = self.get_json("/runtime")?;
        parse_runtime_backend(&runtime_json)?;

        let completion_probe_request = CompletionRequest {
            model: &selected_model,
            prompt: "MEF provider qualification probe.",
            max_tokens: 1,
            temperature: 0.0,
            stream: false,
        };
        let completion_probe_json = self.post_completion(&completion_probe_request)?;
        parse_completion(&completion_probe_json)?;

        let evidence_digest = digest_probe_evidence(
            &health_json,
            &catalog_json,
            &loaded_model_json,
            &runtime_json,
            &completion_probe_json,
        )?;
        let qualification = ProviderQualification::new(
            QualifiedName::new(PROVIDER_NAME)?,
            self.endpoint.as_str(),
            text_generate_v1()?,
            environment_sha256,
            selected_model,
            models,
            evidence_digest,
        )?;
        Ok(AirQualification {
            schema: QUALIFICATION_SCHEMA_VERSION,
            qualification,
            health_response: health_json,
            models_response: catalog_json,
            model_response: loaded_model_json,
            runtime_response: runtime_json,
            completion_probe_response: completion_probe_json,
        })
    }

    /// Executes one bounded text-generation request using prior qualification evidence.
    ///
    /// # Errors
    /// Returns [`AirError`] if the qualification does not match this endpoint, the request fails,
    /// or the provider response cannot satisfy the R0 response contract.
    pub fn generate(
        &self,
        qualification: &AirQualification,
        request: &TextGenerateRequest,
    ) -> Result<AirExecution, AirError> {
        qualification.validate()?;
        if qualification.qualification.endpoint() != self.endpoint.as_str() {
            return Err(AirError::QualificationMismatch(
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
        Ok(AirExecution {
            schema: EXECUTION_SCHEMA_VERSION,
            receipt,
            response: normalized,
            provider_response,
        })
    }

    fn get_json(&self, route: &'static str) -> Result<Value, AirError> {
        let response = self
            .client
            .get(self.route(route)?)
            .send()
            .map_err(|error| AirError::Transport(error.to_string()))?;
        parse_bounded_json_response(response, route)
    }

    fn post_completion(&self, request: &CompletionRequest<'_>) -> Result<Value, AirError> {
        let response = self
            .client
            .post(self.route("/v1/completions")?)
            .json(request)
            .send()
            .map_err(|error| AirError::Transport(error.to_string()))?;
        parse_bounded_json_response(response, "/v1/completions")
    }

    fn route(&self, route: &str) -> Result<Url, AirError> {
        self.endpoint
            .join(route.trim_start_matches('/'))
            .map_err(|error| AirError::InvalidEndpoint(error.to_string()))
    }
}

fn parse_bounded_json_response(
    response: reqwest::blocking::Response,
    route: &'static str,
) -> Result<Value, AirError> {
    let status = response.status();
    if !status.is_success() {
        return Err(AirError::HttpStatus {
            route,
            status: status.as_u16(),
        });
    }
    let body = read_bounded_body(response, route)?;
    serde_json::from_slice(&body).map_err(|error| AirError::MalformedResponse {
        route,
        message: error.to_string(),
    })
}

fn read_bounded_body(
    response: reqwest::blocking::Response,
    route: &'static str,
) -> Result<Vec<u8>, AirError> {
    let mut body = Vec::new();
    let mut bounded = response.take(MAX_HTTP_RESPONSE_BYTES_U64 + 1);
    bounded
        .read_to_end(&mut body)
        .map_err(|error| AirError::Transport(error.to_string()))?;
    if body.len() > MAX_HTTP_RESPONSE_BYTES {
        return Err(AirError::ResponseTooLarge {
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

fn validate_endpoint(endpoint: &str) -> Result<Url, AirError> {
    let mut url =
        Url::parse(endpoint).map_err(|error| AirError::InvalidEndpoint(error.to_string()))?;
    if url.scheme() != "http" {
        return Err(AirError::InvalidEndpoint(
            "Wave 3 accepts local HTTP endpoints only".to_owned(),
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| AirError::InvalidEndpoint("endpoint must include a host".to_owned()))?;
    if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
        return Err(AirError::InvalidEndpoint(
            "Wave 3 endpoint must resolve syntactically to loopback".to_owned(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AirError::InvalidEndpoint(
            "credentials are not accepted in the endpoint URL".to_owned(),
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(AirError::InvalidEndpoint(
            "query and fragment are not accepted in the endpoint URL".to_owned(),
        ));
    }
    if url.path() != "/" && !url.path().is_empty() {
        return Err(AirError::InvalidEndpoint(
            "endpoint must be a base URL without a path".to_owned(),
        ));
    }
    url.set_path("/");
    Ok(url)
}

fn parse_health(value: &Value) -> Result<&str, AirError> {
    value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| AirError::MalformedResponse {
            route: "/health",
            message: "missing string field 'status'".to_owned(),
        })
}

fn parse_models(value: &Value) -> Result<Vec<String>, AirError> {
    let data =
        value
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| AirError::MalformedResponse {
                route: "/v1/models",
                message: "missing array field 'data'".to_owned(),
            })?;
    let mut models = Vec::with_capacity(data.len());
    for model in data {
        let id =
            model
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| AirError::MalformedResponse {
                    route: "/v1/models",
                    message: "model entry missing string field 'id'".to_owned(),
                })?;
        if !id.is_empty() {
            models.push(id.to_owned());
        }
    }
    models.sort();
    models.dedup();
    if models.is_empty() {
        return Err(AirError::NoModels);
    }
    Ok(models)
}

fn select_model(models: &[String], requested: Option<&str>) -> Result<String, AirError> {
    if let Some(requested) = requested {
        return models
            .iter()
            .find(|model| model.as_str() == requested)
            .cloned()
            .ok_or_else(|| AirError::RequestedModelMissing(requested.to_owned()));
    }
    if models.len() == 1 {
        return Ok(models[0].clone());
    }
    Err(AirError::AmbiguousModels(models.to_vec()))
}

fn parse_model_id(value: &Value) -> Result<&str, AirError> {
    value
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| AirError::MalformedResponse {
            route: "/model",
            message: "missing non-empty string field 'id'".to_owned(),
        })
}

fn parse_runtime_backend(value: &Value) -> Result<&str, AirError> {
    value
        .get("backend")
        .and_then(Value::as_str)
        .filter(|backend| !backend.is_empty())
        .ok_or_else(|| AirError::MalformedResponse {
            route: "/runtime",
            message: "missing non-empty string field 'backend'".to_owned(),
        })
}

fn parse_completion(value: &Value) -> Result<TextGenerateResponse, AirError> {
    let choices = value
        .get("choices")
        .and_then(Value::as_array)
        .ok_or_else(|| AirError::MalformedResponse {
            route: "/v1/completions",
            message: "missing array field 'choices'".to_owned(),
        })?;
    if choices.len() != 1 {
        let choice_count = choices.len();
        return Err(AirError::MalformedResponse {
            route: "/v1/completions",
            message: format!("expected exactly one choice, got {choice_count}"),
        });
    }
    let choice = &choices[0];
    let text =
        choice
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| AirError::MalformedResponse {
                route: "/v1/completions",
                message: "choice missing string field 'text'".to_owned(),
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
    model: &Value,
    runtime: &Value,
    completion: &Value,
) -> Result<Sha256Digest, serde_json::Error> {
    let documents = [health, models, model, runtime, completion];
    let mut evidence = Vec::new();
    for (index, document) in documents.iter().enumerate() {
        if index != 0 {
            evidence.push(0);
        }
        evidence.extend_from_slice(&serde_json::to_vec(document)?);
    }
    Ok(Sha256Digest::of_bytes(&evidence))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        parse_completion, parse_health, parse_model_id, parse_models, parse_runtime_backend,
        select_model, validate_endpoint,
    };

    #[test]
    fn endpoint_is_restricted_to_loopback_http() {
        assert!(validate_endpoint("http://127.0.0.1:8181").is_ok());
        assert!(validate_endpoint("http://localhost:8181/").is_ok());
        assert!(validate_endpoint("https://127.0.0.1:8181").is_err());
        assert!(validate_endpoint("http://192.168.1.50:8181").is_err());
        assert!(validate_endpoint("http://127.0.0.1:8181/path").is_err());
    }

    #[test]
    fn health_requires_status() {
        assert_eq!(
            parse_health(&json!({"status": "ok", "backend": "cuda"})).expect("health"),
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
    fn model_and_runtime_evidence_require_identity() {
        assert_eq!(
            parse_model_id(&json!({"id": "Qwen2.5 1.5B Instruct"})).expect("model"),
            "Qwen2.5 1.5B Instruct"
        );
        assert_eq!(
            parse_runtime_backend(&json!({"backend": "cuda"})).expect("runtime"),
            "cuda"
        );
        assert!(parse_model_id(&json!({"id": ""})).is_err());
        assert!(parse_runtime_backend(&json!({"backend": ""})).is_err());
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
