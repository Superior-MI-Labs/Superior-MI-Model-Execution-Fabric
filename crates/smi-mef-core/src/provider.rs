use core::fmt;

use serde::{Deserialize, Deserializer, Serialize};
use smi_ir::{CapabilityRef, QualifiedName, Sha256Digest};

/// Current canonical provider-qualification schema.
pub const PROVIDER_QUALIFICATION_SCHEMA_VERSION: u16 = 1;
/// Current canonical text-generation receipt schema.
pub const TEXT_GENERATION_RECEIPT_SCHEMA_VERSION: u16 = 1;

/// Semantic error in provider qualification evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderEvidenceError {
    EmptyEndpoint,
    EmptyModel,
    SelectedModelNotAdvertised,
    UnsupportedSchema(u16),
}

impl fmt::Display for ProviderEvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEndpoint => formatter.write_str("provider endpoint must not be empty"),
            Self::EmptyModel => formatter.write_str("selected model must not be empty"),
            Self::SelectedModelNotAdvertised => {
                formatter.write_str("selected model was not present in advertised models")
            }
            Self::UnsupportedSchema(schema) => {
                write!(
                    formatter,
                    "unsupported provider qualification schema {schema}"
                )
            }
        }
    }
}

impl std::error::Error for ProviderEvidenceError {}

/// Provider-independent evidence that one concrete runtime/model combination qualified a capability.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProviderQualification {
    schema: u16,
    provider: QualifiedName,
    endpoint: String,
    capability: CapabilityRef,
    environment_sha256: Sha256Digest,
    selected_model: String,
    advertised_models: Vec<String>,
    provider_evidence_sha256: Sha256Digest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderQualificationWire {
    schema: u16,
    provider: QualifiedName,
    endpoint: String,
    capability: CapabilityRef,
    environment_sha256: Sha256Digest,
    selected_model: String,
    advertised_models: Vec<String>,
    provider_evidence_sha256: Sha256Digest,
}

impl<'de> Deserialize<'de> for ProviderQualification {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ProviderQualificationWire::deserialize(deserializer)?;
        if wire.schema != PROVIDER_QUALIFICATION_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(
                ProviderEvidenceError::UnsupportedSchema(wire.schema),
            ));
        }
        Self::new(
            wire.provider,
            wire.endpoint,
            wire.capability,
            wire.environment_sha256,
            wire.selected_model,
            wire.advertised_models,
            wire.provider_evidence_sha256,
        )
        .map_err(<D::Error as serde::de::Error>::custom)
    }
}

impl ProviderQualification {
    /// Creates canonical provider qualification evidence.
    ///
    /// # Errors
    /// Returns [`ProviderEvidenceError`] when endpoint/model evidence is incomplete or inconsistent.
    pub fn new(
        provider: QualifiedName,
        endpoint: impl Into<String>,
        capability: CapabilityRef,
        environment_sha256: Sha256Digest,
        selected_model: impl Into<String>,
        mut advertised_models: Vec<String>,
        provider_evidence_sha256: Sha256Digest,
    ) -> Result<Self, ProviderEvidenceError> {
        let endpoint = endpoint.into();
        if endpoint.is_empty() {
            return Err(ProviderEvidenceError::EmptyEndpoint);
        }
        let selected_model = selected_model.into();
        if selected_model.is_empty() {
            return Err(ProviderEvidenceError::EmptyModel);
        }
        advertised_models.sort();
        advertised_models.dedup();
        if !advertised_models
            .iter()
            .any(|model| model == &selected_model)
        {
            return Err(ProviderEvidenceError::SelectedModelNotAdvertised);
        }
        Ok(Self {
            schema: PROVIDER_QUALIFICATION_SCHEMA_VERSION,
            provider,
            endpoint,
            capability,
            environment_sha256,
            selected_model,
            advertised_models,
            provider_evidence_sha256,
        })
    }

    /// Returns the evidence schema version.
    #[must_use]
    pub const fn schema(&self) -> u16 {
        self.schema
    }

    /// Returns the provider adapter identity.
    #[must_use]
    pub const fn provider(&self) -> &QualifiedName {
        &self.provider
    }

    /// Returns the qualified runtime endpoint.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Returns the qualified capability.
    #[must_use]
    pub const fn capability(&self) -> &CapabilityRef {
        &self.capability
    }

    /// Returns the environment snapshot identity this qualification is bound to.
    #[must_use]
    pub const fn environment_sha256(&self) -> &Sha256Digest {
        &self.environment_sha256
    }

    /// Returns the explicitly selected model.
    #[must_use]
    pub fn selected_model(&self) -> &str {
        &self.selected_model
    }

    /// Returns the canonical advertised-model set seen during qualification.
    #[must_use]
    pub fn advertised_models(&self) -> &[String] {
        &self.advertised_models
    }

    /// Returns the digest of provider-specific probe evidence.
    #[must_use]
    pub const fn provider_evidence_sha256(&self) -> &Sha256Digest {
        &self.provider_evidence_sha256
    }

    /// Serializes qualification evidence deterministically.
    ///
    /// # Errors
    /// Returns a JSON serialization error if serialization fails.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Computes qualification identity over canonical bytes.
    ///
    /// # Errors
    /// Returns a JSON serialization error if canonical bytes cannot be produced.
    pub fn identity(&self) -> Result<Sha256Digest, serde_json::Error> {
        self.to_canonical_bytes()
            .map(|bytes| Sha256Digest::of_bytes(&bytes))
    }
}

/// Provider-independent receipt for one bounded text-generation execution.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TextGenerationReceipt {
    schema: u16,
    qualification_sha256: Sha256Digest,
    provider: QualifiedName,
    endpoint: String,
    model: String,
    capability: CapabilityRef,
    request_sha256: Sha256Digest,
    provider_response_sha256: Sha256Digest,
}

impl TextGenerationReceipt {
    /// Creates a canonical execution receipt.
    #[must_use]
    pub fn new(
        qualification_sha256: Sha256Digest,
        provider: QualifiedName,
        endpoint: impl Into<String>,
        model: impl Into<String>,
        capability: CapabilityRef,
        request_sha256: Sha256Digest,
        provider_response_sha256: Sha256Digest,
    ) -> Self {
        Self {
            schema: TEXT_GENERATION_RECEIPT_SCHEMA_VERSION,
            qualification_sha256,
            provider,
            endpoint: endpoint.into(),
            model: model.into(),
            capability,
            request_sha256,
            provider_response_sha256,
        }
    }

    /// Returns the receipt schema version.
    #[must_use]
    pub const fn schema(&self) -> u16 {
        self.schema
    }

    /// Returns the qualification identity this execution used.
    #[must_use]
    pub const fn qualification_sha256(&self) -> &Sha256Digest {
        &self.qualification_sha256
    }

    /// Returns the provider adapter identity.
    #[must_use]
    pub const fn provider(&self) -> &QualifiedName {
        &self.provider
    }

    /// Returns the endpoint used for execution.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Returns the model used for execution.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Returns the capability executed.
    #[must_use]
    pub const fn capability(&self) -> &CapabilityRef {
        &self.capability
    }

    /// Returns the canonical request digest.
    #[must_use]
    pub const fn request_sha256(&self) -> &Sha256Digest {
        &self.request_sha256
    }

    /// Returns the digest of canonical provider-specific response evidence.
    #[must_use]
    pub const fn provider_response_sha256(&self) -> &Sha256Digest {
        &self.provider_response_sha256
    }
}

#[cfg(test)]
mod tests {
    use smi_ir::{CapabilityId, CapabilityRef, QualifiedName, SemanticVersion, Sha256Digest};

    use super::{ProviderEvidenceError, ProviderQualification};

    fn capability() -> CapabilityRef {
        CapabilityRef::new(
            CapabilityId::new("text.generate").expect("capability id"),
            SemanticVersion::new(1, 0, 0),
        )
    }

    #[test]
    fn qualification_canonicalizes_advertised_models() {
        let qualification = ProviderQualification::new(
            QualifiedName::new("provider.llamacpp.http").expect("provider"),
            "http://127.0.0.1:1920",
            capability(),
            Sha256Digest::of_bytes(b"environment"),
            "model-a",
            vec![
                "model-b".to_owned(),
                "model-a".to_owned(),
                "model-a".to_owned(),
            ],
            Sha256Digest::of_bytes(b"evidence"),
        )
        .expect("qualification");
        assert_eq!(
            qualification.advertised_models(),
            &["model-a".to_owned(), "model-b".to_owned()]
        );
    }

    #[test]
    fn deserialization_rejects_wrong_schema_and_invalid_selection() {
        let valid = ProviderQualification::new(
            QualifiedName::new("provider.llamacpp.http").expect("provider"),
            "http://127.0.0.1:1920",
            capability(),
            Sha256Digest::of_bytes(b"environment"),
            "model-a",
            vec!["model-a".to_owned()],
            Sha256Digest::of_bytes(b"evidence"),
        )
        .expect("qualification");
        let mut value = serde_json::to_value(valid).expect("json");
        value["schema"] = serde_json::json!(99);
        assert!(serde_json::from_value::<ProviderQualification>(value).is_err());

        let invalid = serde_json::json!({
            "schema": 1,
            "provider": "provider.llamacpp.http",
            "endpoint": "http://127.0.0.1:1920",
            "capability": {"id": "text.generate", "version": {"major": 1, "minor": 0, "patch": 0}},
            "environment_sha256": Sha256Digest::of_bytes(b"environment"),
            "selected_model": "missing",
            "advertised_models": ["model-a"],
            "provider_evidence_sha256": Sha256Digest::of_bytes(b"evidence")
        });
        assert!(serde_json::from_value::<ProviderQualification>(invalid).is_err());
    }

    #[test]
    fn qualification_rejects_unadvertised_selection() {
        let result = ProviderQualification::new(
            QualifiedName::new("provider.llamacpp.http").expect("provider"),
            "http://127.0.0.1:1920",
            capability(),
            Sha256Digest::of_bytes(b"environment"),
            "missing",
            vec!["model-a".to_owned()],
            Sha256Digest::of_bytes(b"evidence"),
        );
        assert_eq!(
            result,
            Err(ProviderEvidenceError::SelectedModelNotAdvertised)
        );
    }
}
