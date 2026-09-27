#![doc = "Stable Model Execution Fabric R0 contracts and observation evidence types."]

mod capability;
mod provider;
mod snapshot;

pub use capability::{
    text_generate_v1, TextGenerateRequest, TextGenerateResponse, MAX_TEXT_GENERATE_OUTPUT_TOKENS,
    MAX_TEXT_GENERATE_PROMPT_BYTES,
};
pub use provider::{
    ProviderEvidenceError, ProviderQualification, TextGenerationReceipt,
    PROVIDER_QUALIFICATION_SCHEMA_VERSION, TEXT_GENERATION_RECEIPT_SCHEMA_VERSION,
};
pub use snapshot::{
    EnvironmentSnapshot, GpuObservation, HostObservation, ModelRootObservation, RuntimeObservation,
    SNAPSHOT_SCHEMA_VERSION,
};
