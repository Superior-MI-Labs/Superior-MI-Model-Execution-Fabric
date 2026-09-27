use serde::{Deserialize, Serialize};
use smi_ir::{QualifiedName, Sha256Digest};

/// Current canonical environment snapshot schema.
pub const SNAPSHOT_SCHEMA_VERSION: u16 = 1;

/// Read-only host observation.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HostObservation {
    os: String,
    architecture: String,
    logical_cpus: usize,
    memory_total_bytes: Option<u64>,
    gpus: Vec<GpuObservation>,
}

impl HostObservation {
    /// Creates a host observation and canonicalizes GPU ordering.
    #[must_use]
    pub fn new(
        os: impl Into<String>,
        architecture: impl Into<String>,
        logical_cpus: usize,
        memory_total_bytes: Option<u64>,
        mut gpus: Vec<GpuObservation>,
    ) -> Self {
        gpus.sort();
        gpus.dedup();
        Self {
            os: os.into(),
            architecture: architecture.into(),
            logical_cpus,
            memory_total_bytes,
            gpus,
        }
    }

    /// Returns the observed OS identifier.
    #[must_use]
    pub fn os(&self) -> &str {
        &self.os
    }

    /// Returns the observed architecture.
    #[must_use]
    pub fn architecture(&self) -> &str {
        &self.architecture
    }

    /// Returns the logical CPU count.
    #[must_use]
    pub const fn logical_cpus(&self) -> usize {
        self.logical_cpus
    }

    /// Returns total memory when available.
    #[must_use]
    pub const fn memory_total_bytes(&self) -> Option<u64> {
        self.memory_total_bytes
    }

    /// Returns canonical GPU observations.
    #[must_use]
    pub fn gpus(&self) -> &[GpuObservation] {
        &self.gpus
    }
}

/// Read-only GPU metadata.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GpuObservation {
    backend: String,
    name: String,
    uuid: String,
    memory_total_bytes: u64,
    driver_version: String,
}

impl GpuObservation {
    /// Creates a GPU observation.
    #[must_use]
    pub fn new(
        backend: impl Into<String>,
        name: impl Into<String>,
        uuid: impl Into<String>,
        memory_total_bytes: u64,
        driver_version: impl Into<String>,
    ) -> Self {
        Self {
            backend: backend.into(),
            name: name.into(),
            uuid: uuid.into(),
            memory_total_bytes,
            driver_version: driver_version.into(),
        }
    }
}

/// Runtime installation/configuration observation. Endpoint candidates are not reachability claims.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeObservation {
    adapter: QualifiedName,
    executable_path: Option<String>,
    endpoint_candidates: Vec<String>,
}

impl RuntimeObservation {
    /// Creates a runtime observation and canonicalizes endpoint candidates.
    #[must_use]
    pub fn new(
        adapter: QualifiedName,
        executable_path: Option<String>,
        mut endpoint_candidates: Vec<String>,
    ) -> Self {
        endpoint_candidates.sort();
        endpoint_candidates.dedup();
        Self {
            adapter,
            executable_path,
            endpoint_candidates,
        }
    }

    /// Returns the adapter observation identity.
    #[must_use]
    pub const fn adapter(&self) -> &QualifiedName {
        &self.adapter
    }

    /// Returns the executable path when discovered.
    #[must_use]
    pub fn executable_path(&self) -> Option<&str> {
        self.executable_path.as_deref()
    }

    /// Returns configured endpoint candidates. No reachability is implied.
    #[must_use]
    pub fn endpoint_candidates(&self) -> &[String] {
        &self.endpoint_candidates
    }
}

/// Observation of a configured model-search root.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRootObservation {
    path: String,
    exists: bool,
    is_directory: bool,
}

impl ModelRootObservation {
    /// Creates a model-root observation.
    #[must_use]
    pub fn new(path: impl Into<String>, exists: bool, is_directory: bool) -> Self {
        Self {
            path: path.into(),
            exists,
            is_directory,
        }
    }
}

/// Canonical read-only evidence describing one observed execution environment.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentSnapshot {
    schema: u16,
    host: HostObservation,
    runtimes: Vec<RuntimeObservation>,
    model_roots: Vec<ModelRootObservation>,
    warnings: Vec<String>,
}

impl EnvironmentSnapshot {
    /// Creates and canonicalizes an environment snapshot.
    #[must_use]
    pub fn new(
        host: HostObservation,
        mut runtimes: Vec<RuntimeObservation>,
        mut model_roots: Vec<ModelRootObservation>,
        mut warnings: Vec<String>,
    ) -> Self {
        runtimes.sort();
        runtimes.dedup();
        model_roots.sort();
        model_roots.dedup();
        warnings.sort();
        warnings.dedup();
        Self {
            schema: SNAPSHOT_SCHEMA_VERSION,
            host,
            runtimes,
            model_roots,
            warnings,
        }
    }

    /// Returns the snapshot schema version.
    #[must_use]
    pub const fn schema(&self) -> u16 {
        self.schema
    }

    /// Returns the host observation.
    #[must_use]
    pub const fn host(&self) -> &HostObservation {
        &self.host
    }

    /// Returns canonical runtime observations.
    #[must_use]
    pub fn runtimes(&self) -> &[RuntimeObservation] {
        &self.runtimes
    }

    /// Returns canonical model-root observations.
    #[must_use]
    pub fn model_roots(&self) -> &[ModelRootObservation] {
        &self.model_roots
    }

    /// Returns canonical warnings.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Serializes this snapshot into deterministic canonical JSON bytes.
    ///
    /// # Errors
    /// Returns a JSON serialization error if the document cannot be serialized.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&self.canonicalized())
    }

    /// Computes SHA-256 identity over canonical snapshot bytes.
    ///
    /// # Errors
    /// Returns a JSON serialization error if canonical bytes cannot be produced.
    pub fn identity(&self) -> Result<Sha256Digest, serde_json::Error> {
        self.to_canonical_bytes()
            .map(|bytes| Sha256Digest::of_bytes(&bytes))
    }

    fn canonicalized(&self) -> Self {
        let mut canonical = self.clone();
        canonical.host.gpus.sort();
        canonical.host.gpus.dedup();
        for runtime in &mut canonical.runtimes {
            runtime.endpoint_candidates.sort();
            runtime.endpoint_candidates.dedup();
        }
        canonical.runtimes.sort();
        canonical.runtimes.dedup();
        canonical.model_roots.sort();
        canonical.model_roots.dedup();
        canonical.warnings.sort();
        canonical.warnings.dedup();
        canonical
    }
}

#[cfg(test)]
mod tests {
    use smi_ir::QualifiedName;

    use super::{EnvironmentSnapshot, HostObservation, ModelRootObservation, RuntimeObservation};

    fn runtime(adapter: &str, endpoint: &str) -> RuntimeObservation {
        RuntimeObservation::new(
            QualifiedName::new(adapter).expect("adapter name"),
            None,
            vec![endpoint.to_owned()],
        )
    }

    #[test]
    fn snapshot_identity_is_insertion_order_independent() {
        let host = HostObservation::new("linux", "x86_64", 8, Some(16), Vec::new());
        let forward = EnvironmentSnapshot::new(
            host.clone(),
            vec![
                runtime("provider.llamacpp.http", "http://127.0.0.1:1920"),
                runtime("provider.air.http", "http://127.0.0.1:8181"),
            ],
            vec![
                ModelRootObservation::new("/models/b", true, true),
                ModelRootObservation::new("/models/a", true, true),
            ],
            vec!["z".to_owned(), "a".to_owned()],
        );
        let reverse = EnvironmentSnapshot::new(
            host,
            vec![
                runtime("provider.air.http", "http://127.0.0.1:8181"),
                runtime("provider.llamacpp.http", "http://127.0.0.1:1920"),
            ],
            vec![
                ModelRootObservation::new("/models/a", true, true),
                ModelRootObservation::new("/models/b", true, true),
            ],
            vec!["a".to_owned(), "z".to_owned()],
        );
        assert_eq!(
            forward.to_canonical_bytes().expect("canonical bytes"),
            reverse.to_canonical_bytes().expect("canonical bytes")
        );
        assert_eq!(
            forward.identity().expect("identity"),
            reverse.identity().expect("identity")
        );
    }

    #[test]
    fn deserialized_snapshot_is_canonicalized_before_identity() {
        let json = r#"{"schema":1,"host":{"os":"linux","architecture":"x86_64","logical_cpus":8,"memory_total_bytes":null,"gpus":[]},"runtimes":[{"adapter":"provider.llamacpp.http","executable_path":null,"endpoint_candidates":["http://b","http://a"]}],"model_roots":[],"warnings":["z","a"]}"#;
        let parsed: EnvironmentSnapshot = serde_json::from_str(json).expect("snapshot");
        let canonical = EnvironmentSnapshot::new(
            HostObservation::new("linux", "x86_64", 8, None, Vec::new()),
            vec![RuntimeObservation::new(
                QualifiedName::new("provider.llamacpp.http").expect("adapter name"),
                None,
                vec!["http://a".to_owned(), "http://b".to_owned()],
            )],
            Vec::new(),
            vec!["a".to_owned(), "z".to_owned()],
        );
        assert_eq!(
            parsed.to_canonical_bytes().expect("canonical bytes"),
            canonical.to_canonical_bytes().expect("canonical bytes")
        );
    }

    #[test]
    fn runtime_endpoint_candidates_are_sorted_and_deduplicated() {
        let observation = RuntimeObservation::new(
            QualifiedName::new("provider.llamacpp.http").expect("adapter name"),
            None,
            vec![
                "http://b".to_owned(),
                "http://a".to_owned(),
                "http://b".to_owned(),
            ],
        );
        assert_eq!(
            observation.endpoint_candidates(),
            &["http://a".to_owned(), "http://b".to_owned()]
        );
    }
}
