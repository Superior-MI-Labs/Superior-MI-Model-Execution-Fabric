use serde::{Deserialize, Deserializer, Serialize};
use smi_ir::{CapabilityId, CapabilityRef, NameError, SemanticVersion};

/// R0 prompt-byte ceiling for one `text.generate` request.
pub const MAX_TEXT_GENERATE_PROMPT_BYTES: usize = 262_144;
/// R0 output-token ceiling for one `text.generate` request.
pub const MAX_TEXT_GENERATE_OUTPUT_TOKENS: u32 = 4_096;

/// Returns the canonical Builder capability reference for R0 text generation.
///
/// # Errors
/// Returns [`NameError`] if Builder rejects the canonical capability identifier.
pub fn text_generate_v1() -> Result<CapabilityRef, NameError> {
    CapabilityId::new("text.generate")
        .map(|id| CapabilityRef::new(id, SemanticVersion::new(1, 0, 0)))
}

/// Provider-independent R0 text generation request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TextGenerateRequest {
    prompt: String,
    max_output_tokens: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TextGenerateRequestWire {
    prompt: String,
    max_output_tokens: u32,
}

impl<'de> Deserialize<'de> for TextGenerateRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = TextGenerateRequestWire::deserialize(deserializer)?;
        Self::new(wire.prompt, wire.max_output_tokens)
            .map_err(<D::Error as serde::de::Error>::custom)
    }
}

impl TextGenerateRequest {
    /// Creates a bounded request.
    ///
    /// # Errors
    /// Returns an error when the prompt is empty or `max_output_tokens` is outside the R0 bound.
    pub fn new(prompt: impl Into<String>, max_output_tokens: u32) -> Result<Self, &'static str> {
        let prompt = prompt.into();
        if prompt.is_empty() {
            return Err("prompt must not be empty");
        }
        if prompt.len() > MAX_TEXT_GENERATE_PROMPT_BYTES {
            return Err("prompt exceeds the 262144-byte R0 limit");
        }
        if max_output_tokens == 0 || max_output_tokens > MAX_TEXT_GENERATE_OUTPUT_TOKENS {
            return Err("max_output_tokens must be between 1 and 4096");
        }
        Ok(Self {
            prompt,
            max_output_tokens,
        })
    }

    /// Returns the request prompt.
    #[must_use]
    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    /// Returns the maximum requested output token count.
    #[must_use]
    pub const fn max_output_tokens(&self) -> u32 {
        self.max_output_tokens
    }
}

/// Provider-independent R0 text generation result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TextGenerateResponse {
    text: String,
    finish_reason: Option<String>,
    output_tokens: Option<u64>,
}

impl TextGenerateResponse {
    /// Creates a normalized text generation response.
    #[must_use]
    pub fn new(
        text: impl Into<String>,
        finish_reason: Option<String>,
        output_tokens: Option<u64>,
    ) -> Self {
        Self {
            text: text.into(),
            finish_reason,
            output_tokens,
        }
    }

    /// Returns generated text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the provider-independent finish reason when known.
    #[must_use]
    pub fn finish_reason(&self) -> Option<&str> {
        self.finish_reason.as_deref()
    }

    /// Returns provider-reported output tokens when available.
    #[must_use]
    pub const fn output_tokens(&self) -> Option<u64> {
        self.output_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::{
        text_generate_v1, TextGenerateRequest, MAX_TEXT_GENERATE_OUTPUT_TOKENS,
        MAX_TEXT_GENERATE_PROMPT_BYTES,
    };

    #[test]
    fn capability_identity_reuses_builder_namespace() {
        assert_eq!(
            text_generate_v1()
                .expect("canonical capability")
                .to_string(),
            "text.generate@1.0.0"
        );
    }

    #[test]
    fn request_rejects_empty_or_unbounded_output() {
        assert!(TextGenerateRequest::new("", 1).is_err());
        assert!(TextGenerateRequest::new("hello", 0).is_err());
        assert!(TextGenerateRequest::new("hello", 32).is_ok());
        assert!(TextGenerateRequest::new("hello", MAX_TEXT_GENERATE_OUTPUT_TOKENS + 1).is_err());
        let oversized = "x".repeat(MAX_TEXT_GENERATE_PROMPT_BYTES + 1);
        assert!(TextGenerateRequest::new(oversized, 1).is_err());
    }

    #[test]
    fn deserialization_cannot_bypass_request_bounds() {
        let oversized = "x".repeat(MAX_TEXT_GENERATE_PROMPT_BYTES + 1);
        let json = serde_json::json!({
            "prompt": oversized,
            "max_output_tokens": 1
        });
        assert!(serde_json::from_value::<TextGenerateRequest>(json).is_err());
        assert!(serde_json::from_str::<TextGenerateRequest>(
            r#"{"prompt":"hello","max_output_tokens":0}"#
        )
        .is_err());
    }
}
