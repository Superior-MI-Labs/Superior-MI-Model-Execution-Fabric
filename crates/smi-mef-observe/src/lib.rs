#![doc = "Read-only environment observation for Model Execution Fabric R0."]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use smi_ir::{NameError, QualifiedName};
use smi_mef_core::{
    EnvironmentSnapshot, GpuObservation, HostObservation, ModelRootObservation, RuntimeObservation,
};

const MIB_BYTES: u64 = 1024 * 1024;

/// User-supplied observation inputs. Endpoint URLs are candidates only in Wave 1.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ObservationConfig {
    llama_endpoints: Vec<String>,
    air_endpoints: Vec<String>,
    model_roots: Vec<PathBuf>,
}

impl ObservationConfig {
    /// Adds a configured llama.cpp endpoint candidate.
    #[must_use]
    pub fn with_llama_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.llama_endpoints.push(endpoint.into());
        self
    }

    /// Adds a configured AIR endpoint candidate.
    #[must_use]
    pub fn with_air_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.air_endpoints.push(endpoint.into());
        self
    }

    /// Adds a configured model root.
    #[must_use]
    pub fn with_model_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.model_roots.push(root.into());
        self
    }
}

/// Observes the local environment without contacting inference endpoints or invoking models.
///
/// # Errors
/// Returns [`NameError`] if Builder rejects a built-in provider adapter identifier.
pub fn observe_environment(config: &ObservationConfig) -> Result<EnvironmentSnapshot, NameError> {
    let mut warnings = Vec::new();
    let host = observe_host(&mut warnings);
    let runtimes = vec![
        observe_runtime(
            "provider.llamacpp.http",
            "llama-server",
            &config.llama_endpoints,
        )?,
        observe_runtime("provider.air.http", "air-server", &config.air_endpoints)?,
    ];
    let model_roots = config
        .model_roots
        .iter()
        .map(|path| {
            ModelRootObservation::new(path.display().to_string(), path.exists(), path.is_dir())
        })
        .collect();
    Ok(EnvironmentSnapshot::new(
        host,
        runtimes,
        model_roots,
        warnings,
    ))
}

fn observe_host(warnings: &mut Vec<String>) -> HostObservation {
    let logical_cpus = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let memory_total_bytes = read_linux_memory_total();
    let gpus = observe_nvidia_gpus(warnings);
    HostObservation::new(
        env::consts::OS,
        env::consts::ARCH,
        logical_cpus,
        memory_total_bytes,
        gpus,
    )
}

fn observe_runtime(
    adapter: &str,
    executable: &str,
    endpoints: &[String],
) -> Result<RuntimeObservation, NameError> {
    Ok(RuntimeObservation::new(
        QualifiedName::new(adapter)?,
        find_in_path(executable).map(|path| path.display().to_string()),
        endpoints.to_vec(),
    ))
}

fn find_in_path(executable: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|directory| directory.join(executable))
        .find(|candidate| candidate.is_file())
}

fn read_linux_memory_total() -> Option<u64> {
    let contents = fs::read_to_string("/proc/meminfo").ok()?;
    parse_mem_total_bytes(&contents)
}

fn parse_mem_total_bytes(contents: &str) -> Option<u64> {
    contents.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        if fields.next()? != "MemTotal:" {
            return None;
        }
        let kib = fields.next()?.parse::<u64>().ok()?;
        kib.checked_mul(1024)
    })
}

fn observe_nvidia_gpus(warnings: &mut Vec<String>) -> Vec<GpuObservation> {
    if find_in_path("nvidia-smi").is_none() {
        return Vec::new();
    }
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,uuid,memory.total,driver_version",
            "--format=csv,noheader,nounits",
        ])
        .output();
    let Ok(output) = output else {
        warnings.push("nvidia-smi exists but could not be executed".to_owned());
        return Vec::new();
    };
    if !output.status.success() {
        warnings.push(format!(
            "nvidia-smi observation failed with status {}",
            output.status
        ));
        return Vec::new();
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_nvidia_csv(&stdout, warnings)
}

fn parse_nvidia_csv(contents: &str, warnings: &mut Vec<String>) -> Vec<GpuObservation> {
    let mut observations = Vec::new();
    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<_> = line.split(',').map(str::trim).collect();
        if fields.len() != 4 {
            warnings.push(format!("ignored malformed nvidia-smi row: {line}"));
            continue;
        }
        let Ok(memory_mib) = fields[2].parse::<u64>() else {
            warnings.push(format!(
                "ignored nvidia-smi row with invalid memory: {line}"
            ));
            continue;
        };
        let Some(memory_total_bytes) = memory_mib.checked_mul(MIB_BYTES) else {
            warnings.push(format!(
                "ignored nvidia-smi row with overflowing memory: {line}"
            ));
            continue;
        };
        observations.push(GpuObservation::new(
            "nvidia",
            fields[0],
            fields[1],
            memory_total_bytes,
            fields[3],
        ));
    }
    observations
}

/// Returns whether a configured model-root path exists and is a directory.
#[must_use]
pub fn observe_model_root(path: &Path) -> ModelRootObservation {
    ModelRootObservation::new(path.display().to_string(), path.exists(), path.is_dir())
}

#[cfg(test)]
mod tests {
    use super::{parse_mem_total_bytes, parse_nvidia_csv};

    #[test]
    fn parses_linux_mem_total() {
        assert_eq!(
            parse_mem_total_bytes("MemTotal:       16384 kB\nMemFree: 42 kB\n"),
            Some(16_777_216)
        );
    }

    #[test]
    fn parses_nvidia_observations_and_reports_malformed_rows() {
        let mut warnings = Vec::new();
        let gpus = parse_nvidia_csv(
            "NVIDIA RTX, GPU-123, 16384, 595.84\nbad,row\n",
            &mut warnings,
        );
        assert_eq!(gpus.len(), 1);
        assert_eq!(warnings.len(), 1);
    }
}
