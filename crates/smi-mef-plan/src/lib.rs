#![doc = "Deterministic evidence-bound deployment planning for Superior MI MEF R0."]

use core::fmt;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use smi_ir::{
    CapabilityBinding, CapabilityRef, FieldName, GraphDelta, GraphOperation, GraphRevision,
    InstanceId, QualifiedName, Sha256Digest,
};
use smi_mef_core::{EnvironmentSnapshot, ProviderQualification};

/// Current canonical deployment-plan schema.
pub const DEPLOYMENT_PLAN_SCHEMA_VERSION: u16 = 1;

/// Explicit R0 provider-selection policy.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentPolicy {
    provider: QualifiedName,
}

impl DeploymentPolicy {
    #[must_use]
    pub const fn new(provider: QualifiedName) -> Self {
        Self { provider }
    }

    #[must_use]
    pub const fn provider(&self) -> &QualifiedName {
        &self.provider
    }
}

/// Builder realization target supplied to MEF by the structural owner.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RealizationTarget {
    base_revision: GraphRevision,
    consumer: InstanceId,
    slot: FieldName,
    provider_instance: InstanceId,
    current_provider_instance: Option<InstanceId>,
}

impl RealizationTarget {
    #[must_use]
    pub const fn new(
        base_revision: GraphRevision,
        consumer: InstanceId,
        slot: FieldName,
        provider_instance: InstanceId,
        current_provider_instance: Option<InstanceId>,
    ) -> Self {
        Self {
            base_revision,
            consumer,
            slot,
            provider_instance,
            current_provider_instance,
        }
    }

    #[must_use]
    pub const fn base_revision(&self) -> &GraphRevision {
        &self.base_revision
    }

    #[must_use]
    pub const fn consumer(&self) -> &InstanceId {
        &self.consumer
    }

    #[must_use]
    pub const fn slot(&self) -> &FieldName {
        &self.slot
    }

    #[must_use]
    pub const fn provider_instance(&self) -> &InstanceId {
        &self.provider_instance
    }

    #[must_use]
    pub const fn current_provider_instance(&self) -> Option<&InstanceId> {
        self.current_provider_instance.as_ref()
    }
}

/// Deterministic MEF realization proposal. It is evidence, not committed Builder state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentPlan {
    schema: u16,
    policy: DeploymentPolicy,
    capability: CapabilityRef,
    environment_sha256: Sha256Digest,
    qualification_sha256: Sha256Digest,
    provider: QualifiedName,
    endpoint: String,
    model: String,
    target: RealizationTarget,
    graph_delta: Option<GraphDelta>,
}

impl DeploymentPlan {
    #[must_use]
    pub const fn schema(&self) -> u16 {
        self.schema
    }

    #[must_use]
    pub const fn policy(&self) -> &DeploymentPolicy {
        &self.policy
    }

    #[must_use]
    pub const fn capability(&self) -> &CapabilityRef {
        &self.capability
    }

    #[must_use]
    pub const fn environment_sha256(&self) -> &Sha256Digest {
        &self.environment_sha256
    }

    #[must_use]
    pub const fn qualification_sha256(&self) -> &Sha256Digest {
        &self.qualification_sha256
    }

    #[must_use]
    pub const fn provider(&self) -> &QualifiedName {
        &self.provider
    }

    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    #[must_use]
    pub const fn target(&self) -> &RealizationTarget {
        &self.target
    }

    #[must_use]
    pub const fn graph_delta(&self) -> Option<&GraphDelta> {
        self.graph_delta.as_ref()
    }

    /// Serializes the plan deterministically.
    ///
    /// # Errors
    /// Returns a JSON serialization error when canonical bytes cannot be produced.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Computes the plan identity over canonical bytes.
    ///
    /// # Errors
    /// Returns a JSON serialization error when canonical bytes cannot be produced.
    pub fn identity(&self) -> Result<Sha256Digest, serde_json::Error> {
        self.to_canonical_bytes()
            .map(|bytes| Sha256Digest::of_bytes(&bytes))
    }

    /// Parses and revalidates one deployment-plan document.
    ///
    /// # Errors
    /// Returns [`PlanError`] for malformed or internally inconsistent plans.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PlanError> {
        let plan: Self = serde_json::from_slice(bytes)
            .map_err(|error| PlanError::Serialization(error.to_string()))?;
        plan.validate()?;
        Ok(plan)
    }

    /// Verifies that independently validated qualification evidence is exactly the evidence selected
    /// by this plan.
    ///
    /// # Errors
    /// Returns [`PlanError::QualificationMismatch`] when any evidence identity differs.
    pub fn verify_qualification(
        &self,
        qualification: &ProviderQualification,
    ) -> Result<(), PlanError> {
        let identity = qualification
            .identity()
            .map_err(|error| PlanError::Serialization(error.to_string()))?;
        if qualification.provider() != &self.provider
            || qualification.capability() != &self.capability
            || qualification.environment_sha256() != &self.environment_sha256
            || identity != self.qualification_sha256
        {
            return Err(PlanError::QualificationMismatch);
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), PlanError> {
        if self.schema != DEPLOYMENT_PLAN_SCHEMA_VERSION {
            return Err(PlanError::UnsupportedSchema(self.schema));
        }
        if self.policy.provider() != &self.provider {
            return Err(PlanError::InvalidPlan(
                "policy provider differs from selected provider".to_owned(),
            ));
        }
        let expected = build_graph_delta(&self.target)?;
        if self.graph_delta != expected {
            return Err(PlanError::InvalidPlan(
                "graph_delta does not match realization target".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Structured planning failure. Missing or ambiguous evidence never triggers a guess.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanError {
    UnsupportedSchema(u16),
    NoProviderEvidence,
    CapabilityNotQualified,
    StaleQualificationEvidence,
    AmbiguousQualification { count: usize },
    QualificationMismatch,
    InvalidPlan(String),
    Serialization(String),
    GraphDelta(String),
}

impl fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema(schema) => {
                write!(formatter, "unsupported deployment-plan schema {schema}")
            }
            Self::NoProviderEvidence => {
                formatter.write_str("explicitly selected provider has no qualification evidence")
            }
            Self::CapabilityNotQualified => formatter.write_str(
                "explicitly selected provider is not qualified for the requested capability",
            ),
            Self::StaleQualificationEvidence => formatter.write_str(
                "selected provider qualification is stale for the supplied environment snapshot",
            ),
            Self::AmbiguousQualification { count } => write!(
                formatter,
                "selected provider has {count} distinct current qualifications; policy is ambiguous"
            ),
            Self::QualificationMismatch => {
                formatter.write_str("qualification does not match the deployment plan")
            }
            Self::InvalidPlan(message) => write!(formatter, "invalid deployment plan: {message}"),
            Self::Serialization(message) => {
                write!(formatter, "deployment-plan serialization error: {message}")
            }
            Self::GraphDelta(message) => {
                write!(formatter, "cannot construct GraphDelta: {message}")
            }
        }
    }
}

impl std::error::Error for PlanError {}

/// Stateless deterministic planner. It proposes a Builder mutation but never applies one.
pub struct DeploymentPlanner;

impl DeploymentPlanner {
    /// Selects exactly one current qualification under an explicit provider policy.
    ///
    /// Candidate input ordering is irrelevant. Exact duplicate qualifications are deduplicated by
    /// semantic qualification identity. Multiple distinct current qualifications for the selected
    /// provider are rejected instead of guessed between.
    ///
    /// # Errors
    /// Returns [`PlanError`] when evidence is absent, stale, capability-incompatible, ambiguous, or
    /// cannot be canonically identified.
    pub fn plan(
        snapshot: &EnvironmentSnapshot,
        capability: &CapabilityRef,
        qualifications: &[ProviderQualification],
        policy: DeploymentPolicy,
        target: RealizationTarget,
    ) -> Result<DeploymentPlan, PlanError> {
        let environment_sha256 = snapshot
            .identity()
            .map_err(|error| PlanError::Serialization(error.to_string()))?;

        let selected_provider: Vec<_> = qualifications
            .iter()
            .filter(|qualification| qualification.provider() == policy.provider())
            .collect();
        if selected_provider.is_empty() {
            return Err(PlanError::NoProviderEvidence);
        }

        let capability_matches: Vec<_> = selected_provider
            .into_iter()
            .filter(|qualification| qualification.capability() == capability)
            .collect();
        if capability_matches.is_empty() {
            return Err(PlanError::CapabilityNotQualified);
        }

        let current_matches: Vec<_> = capability_matches
            .into_iter()
            .filter(|qualification| qualification.environment_sha256() == &environment_sha256)
            .collect();
        if current_matches.is_empty() {
            return Err(PlanError::StaleQualificationEvidence);
        }

        let mut unique = BTreeMap::new();
        for qualification in current_matches {
            let identity = qualification
                .identity()
                .map_err(|error| PlanError::Serialization(error.to_string()))?;
            unique.entry(identity).or_insert(qualification);
        }
        if unique.len() != 1 {
            return Err(PlanError::AmbiguousQualification {
                count: unique.len(),
            });
        }
        let (qualification_sha256, qualification) = unique
            .into_iter()
            .next()
            .ok_or(PlanError::NoProviderEvidence)?;

        let plan = DeploymentPlan {
            schema: DEPLOYMENT_PLAN_SCHEMA_VERSION,
            provider: qualification.provider().clone(),
            endpoint: qualification.endpoint().to_owned(),
            model: qualification.selected_model().to_owned(),
            capability: capability.clone(),
            environment_sha256,
            qualification_sha256,
            graph_delta: build_graph_delta(&target)?,
            policy,
            target,
        };
        plan.validate()?;
        Ok(plan)
    }
}

fn build_graph_delta(target: &RealizationTarget) -> Result<Option<GraphDelta>, PlanError> {
    if target.current_provider_instance() == Some(target.provider_instance()) {
        return Ok(None);
    }

    let mut operations = Vec::with_capacity(2);
    if target.current_provider_instance().is_some() {
        operations.push(GraphOperation::UnbindCapability {
            consumer: target.consumer().clone(),
            slot: target.slot().clone(),
        });
    }
    operations.push(GraphOperation::BindCapability(CapabilityBinding::new(
        target.consumer().clone(),
        target.slot().clone(),
        target.provider_instance().clone(),
    )));
    GraphDelta::new(target.base_revision().clone(), operations)
        .map(Some)
        .map_err(|error| PlanError::GraphDelta(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{DeploymentPlanner, DeploymentPolicy, PlanError, RealizationTarget};
    use smi_ir::{
        ArtifactBinding, ArtifactRef, CapabilityRequirement, DefinitionRegistry, FieldName,
        GraphDelta, GraphOperation, InstanceId, ModuleId, ModuleInstance, ModuleManifest,
        QualifiedName, SemanticVersion, Sha256Digest, SystemGraph, SystemId,
    };
    use smi_mef_core::{
        text_generate_v1, EnvironmentSnapshot, HostObservation, ProviderQualification,
    };

    fn snapshot() -> EnvironmentSnapshot {
        EnvironmentSnapshot::new(
            HostObservation::new("linux", "x86_64", 8, Some(16), Vec::new()),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }

    fn qualification(
        provider: &str,
        endpoint: &str,
        model: &str,
        environment_sha256: Sha256Digest,
        evidence: &[u8],
    ) -> ProviderQualification {
        ProviderQualification::new(
            QualifiedName::new(provider).expect("provider"),
            endpoint,
            text_generate_v1().expect("capability"),
            environment_sha256,
            model,
            vec![model.to_owned()],
            Sha256Digest::of_bytes(evidence),
        )
        .expect("qualification")
    }

    fn target(
        base_revision: smi_ir::GraphRevision,
        selected: &str,
        current: Option<&str>,
    ) -> RealizationTarget {
        RealizationTarget::new(
            base_revision,
            InstanceId::new("app.generator").expect("consumer"),
            FieldName::new("generator").expect("slot"),
            InstanceId::new(selected).expect("provider instance"),
            current.map(|value| InstanceId::new(value).expect("current provider")),
        )
    }

    #[test]
    fn provider_input_order_does_not_change_plan() {
        let environment = snapshot();
        let digest = environment.identity().expect("environment identity");
        let llama = qualification(
            "provider.llamacpp.http",
            "http://127.0.0.1:1920/",
            "llama-model",
            digest.clone(),
            b"llama",
        );
        let air = qualification(
            "provider.air.http",
            "http://127.0.0.1:8181/",
            "air-model",
            digest,
            b"air",
        );
        let revision = smi_ir::GraphRevision::new(Sha256Digest::of_bytes(b"graph"));
        let policy = DeploymentPolicy::new(
            QualifiedName::new("provider.air.http").expect("policy provider"),
        );
        let forward = DeploymentPlanner::plan(
            &environment,
            &text_generate_v1().expect("capability"),
            &[llama.clone(), air.clone()],
            policy.clone(),
            target(revision.clone(), "runtime.air", Some("runtime.llama")),
        )
        .expect("forward plan");
        let reverse = DeploymentPlanner::plan(
            &environment,
            &text_generate_v1().expect("capability"),
            &[air, llama],
            policy,
            target(revision, "runtime.air", Some("runtime.llama")),
        )
        .expect("reverse plan");
        assert_eq!(forward, reverse);
    }

    #[test]
    fn stale_qualification_is_rejected() {
        let environment = snapshot();
        let stale = qualification(
            "provider.air.http",
            "http://127.0.0.1:8181/",
            "air-model",
            Sha256Digest::of_bytes(b"different environment"),
            b"air",
        );
        let result = DeploymentPlanner::plan(
            &environment,
            &text_generate_v1().expect("capability"),
            &[stale],
            DeploymentPolicy::new(QualifiedName::new("provider.air.http").expect("provider")),
            target(
                smi_ir::GraphRevision::new(Sha256Digest::of_bytes(b"graph")),
                "runtime.air",
                Some("runtime.llama"),
            ),
        );
        assert_eq!(result, Err(PlanError::StaleQualificationEvidence));
    }

    #[test]
    fn distinct_current_qualifications_are_ambiguous() {
        let environment = snapshot();
        let digest = environment.identity().expect("environment identity");
        let first = qualification(
            "provider.air.http",
            "http://127.0.0.1:8181/",
            "air-model-a",
            digest.clone(),
            b"air-a",
        );
        let second = qualification(
            "provider.air.http",
            "http://127.0.0.1:8282/",
            "air-model-b",
            digest,
            b"air-b",
        );
        let result = DeploymentPlanner::plan(
            &environment,
            &text_generate_v1().expect("capability"),
            &[second, first],
            DeploymentPolicy::new(QualifiedName::new("provider.air.http").expect("provider")),
            target(
                smi_ir::GraphRevision::new(Sha256Digest::of_bytes(b"graph")),
                "runtime.air",
                Some("runtime.llama"),
            ),
        );
        assert_eq!(result, Err(PlanError::AmbiguousQualification { count: 2 }));
    }

    fn manifest(
        id: &str,
        provides: Vec<smi_ir::CapabilityRef>,
        requires: Vec<CapabilityRequirement>,
    ) -> ModuleManifest {
        ModuleManifest::new(
            ModuleId::new(id).expect("module id"),
            SemanticVersion::new(1, 0, 0),
            vec![ArtifactBinding::new(
                FieldName::new("implementation").expect("artifact name"),
                ArtifactRef::new(
                    Sha256Digest::of_bytes(id.as_bytes()),
                    u64::try_from(id.len()).expect("artifact size"),
                ),
            )],
            Vec::new(),
            Vec::new(),
            provides,
            requires,
            Vec::new(),
            Vec::new(),
        )
        .expect("manifest")
    }

    #[test]
    fn planner_graph_delta_applies_through_builder_authority() {
        let capability = text_generate_v1().expect("capability");
        let slot = FieldName::new("generator").expect("slot");
        let consumer_manifest = manifest(
            "mef.test.consumer",
            Vec::new(),
            vec![CapabilityRequirement::new(slot.clone(), capability.clone())],
        );
        let llama_manifest = manifest("mef.test.llama", vec![capability.clone()], Vec::new());
        let air_manifest = manifest("mef.test.air", vec![capability.clone()], Vec::new());

        let mut registry = DefinitionRegistry::new();
        registry
            .register_module(consumer_manifest.clone())
            .expect("register consumer");
        registry
            .register_module(llama_manifest.clone())
            .expect("register llama");
        registry
            .register_module(air_manifest.clone())
            .expect("register air");

        let consumer = InstanceId::new("app.generator").expect("consumer instance");
        let llama_instance = InstanceId::new("runtime.llama").expect("llama instance");
        let air_instance = InstanceId::new("runtime.air").expect("air instance");
        let empty = SystemGraph::empty(SystemId::new("mef.r0.fixture").expect("system id"));
        let setup = GraphDelta::new(
            empty.revision().expect("empty revision"),
            vec![
                GraphOperation::AddInstance(ModuleInstance::new(
                    consumer.clone(),
                    consumer_manifest.key().expect("consumer key"),
                )),
                GraphOperation::AddInstance(ModuleInstance::new(
                    llama_instance.clone(),
                    llama_manifest.key().expect("llama key"),
                )),
                GraphOperation::AddInstance(ModuleInstance::new(
                    air_instance.clone(),
                    air_manifest.key().expect("air key"),
                )),
                GraphOperation::BindCapability(smi_ir::CapabilityBinding::new(
                    consumer.clone(),
                    slot.clone(),
                    llama_instance.clone(),
                )),
            ],
        )
        .expect("setup delta");
        let graph = setup.apply(&empty, &registry).expect("setup graph");

        let environment = snapshot();
        let environment_sha256 = environment.identity().expect("environment identity");
        let air_qualification = qualification(
            "provider.air.http",
            "http://127.0.0.1:8181/",
            "air-model",
            environment_sha256,
            b"air",
        );
        let plan = DeploymentPlanner::plan(
            &environment,
            &capability,
            &[air_qualification],
            DeploymentPolicy::new(QualifiedName::new("provider.air.http").expect("provider")),
            RealizationTarget::new(
                graph.revision().expect("graph revision"),
                consumer.clone(),
                slot.clone(),
                air_instance.clone(),
                Some(llama_instance),
            ),
        )
        .expect("deployment plan");
        let delta = plan.graph_delta().expect("substitution delta");
        let rebound = delta
            .apply(&graph, &registry)
            .expect("Builder applies MEF proposal");
        let binding = rebound
            .capability_binding(&consumer, &slot)
            .expect("resulting binding");
        assert_eq!(binding.provider(), &air_instance);
    }
}
