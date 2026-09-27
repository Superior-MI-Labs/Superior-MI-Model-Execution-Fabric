use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use smi_ir::{FieldName, GraphRevision, InstanceId, QualifiedName, Sha256Digest};
use smi_mef_air::{AirClient, AirQualification};
use smi_mef_core::{
    text_generate_v1, EnvironmentSnapshot, ProviderQualification, TextGenerateRequest,
    SNAPSHOT_SCHEMA_VERSION,
};
use smi_mef_llamacpp::{LlamaCppClient, LlamaCppQualification};
use smi_mef_observe::{observe_environment, ObservationConfig};
use smi_mef_plan::{DeploymentPlan, DeploymentPlanner, DeploymentPolicy, RealizationTarget};

const DEFAULT_TIMEOUT_SECONDS: u64 = 30;

fn main() -> ExitCode {
    match run(env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let arguments: Vec<_> = arguments.collect();
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(usage());
    };
    match command {
        "snapshot" => run_snapshot(&arguments[1..]),
        "llama-qualify" => run_llama_qualify(&arguments[1..]),
        "llama-generate" => run_llama_generate(&arguments[1..]),
        "air-qualify" => run_air_qualify(&arguments[1..]),
        "air-generate" => run_air_generate(&arguments[1..]),
        "plan" => run_plan(&arguments[1..]),
        "execute-plan" => run_execute_plan(&arguments[1..]),
        _ => Err(format!("unknown command '{command}'\n{}", usage())),
    }
}

fn run_snapshot(arguments: &[String]) -> Result<(), String> {
    let mut config = ObservationConfig::default();
    let mut output = None;
    let mut index = 0;
    while index < arguments.len() {
        let flag = &arguments[index];
        let value = option_value(arguments, index, flag)?;
        match flag.as_str() {
            "--llama-endpoint" => config = config.with_llama_endpoint(value),
            "--air-endpoint" => config = config.with_air_endpoint(value),
            "--model-root" => config = config.with_model_root(value),
            "--output" => output = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown snapshot option '{flag}'\n{}", usage())),
        }
        index += 2;
    }

    let snapshot = observe_environment(&config)
        .map_err(|error| format!("cannot construct built-in adapter identity: {error}"))?;
    let bytes = snapshot
        .to_canonical_bytes()
        .map_err(|error| format!("cannot serialize environment snapshot: {error}"))?;
    let identity = snapshot
        .identity()
        .map_err(|error| format!("cannot identify environment snapshot: {error}"))?;

    if let Some(path) = output {
        write_bytes(&path, &bytes)?;
        eprintln!("snapshot_path={}", path.display());
    } else {
        println!("{}", String::from_utf8_lossy(&bytes));
    }
    eprintln!("snapshot_sha256={identity}");
    Ok(())
}

fn run_llama_qualify(arguments: &[String]) -> Result<(), String> {
    let mut endpoint = None;
    let mut requested_model = None;
    let mut environment = None;
    let mut output = None;
    let mut timeout_seconds = DEFAULT_TIMEOUT_SECONDS;
    let mut index = 0;
    while index < arguments.len() {
        let flag = &arguments[index];
        let value = option_value(arguments, index, flag)?;
        match flag.as_str() {
            "--endpoint" => endpoint = Some(value.to_owned()),
            "--model" => requested_model = Some(value.to_owned()),
            "--environment" => environment = Some(PathBuf::from(value)),
            "--output" => output = Some(PathBuf::from(value)),
            "--timeout-seconds" => timeout_seconds = parse_positive_u64(value, flag)?,
            _ => {
                return Err(format!(
                    "unknown llama-qualify option '{flag}'\n{}",
                    usage()
                ))
            }
        }
        index += 2;
    }

    let endpoint = endpoint.ok_or_else(|| "llama-qualify requires --endpoint".to_owned())?;
    let environment =
        environment.ok_or_else(|| "llama-qualify requires --environment".to_owned())?;
    let output = output.ok_or_else(|| "llama-qualify requires --output".to_owned())?;
    let snapshot = read_environment_snapshot(&environment)?;
    ensure_llama_endpoint_was_observed(&snapshot, &endpoint)?;
    let environment_sha256 = snapshot
        .identity()
        .map_err(|error| format!("cannot identify environment snapshot: {error}"))?;
    let client = LlamaCppClient::new(&endpoint, Duration::from_secs(timeout_seconds))
        .map_err(|error| error.to_string())?;
    let qualification = client
        .qualify(requested_model.as_deref(), environment_sha256)
        .map_err(|error| error.to_string())?;
    let bytes = qualification
        .to_canonical_bytes()
        .map_err(|error| format!("cannot serialize llama.cpp qualification: {error}"))?;
    write_bytes(&output, &bytes)?;
    let identity = qualification
        .qualification()
        .identity()
        .map_err(|error| format!("cannot identify llama.cpp qualification: {error}"))?;
    eprintln!("qualification_path={}", output.display());
    eprintln!(
        "qualified_model={}",
        qualification.qualification().selected_model()
    );
    eprintln!("qualification_sha256={identity}");
    Ok(())
}

fn run_llama_generate(arguments: &[String]) -> Result<(), String> {
    let mut qualification_path = None;
    let mut prompt = None;
    let mut max_output_tokens = None;
    let mut output = None;
    let mut timeout_seconds = DEFAULT_TIMEOUT_SECONDS;
    let mut index = 0;
    while index < arguments.len() {
        let flag = &arguments[index];
        let value = option_value(arguments, index, flag)?;
        match flag.as_str() {
            "--qualification" => qualification_path = Some(PathBuf::from(value)),
            "--prompt" => prompt = Some(value.to_owned()),
            "--max-output-tokens" => max_output_tokens = Some(parse_positive_u32(value, flag)?),
            "--output" => output = Some(PathBuf::from(value)),
            "--timeout-seconds" => timeout_seconds = parse_positive_u64(value, flag)?,
            _ => {
                return Err(format!(
                    "unknown llama-generate option '{flag}'\n{}",
                    usage()
                ))
            }
        }
        index += 2;
    }

    let qualification_path =
        qualification_path.ok_or_else(|| "llama-generate requires --qualification".to_owned())?;
    let prompt = prompt.ok_or_else(|| "llama-generate requires --prompt".to_owned())?;
    let max_output_tokens = max_output_tokens
        .ok_or_else(|| "llama-generate requires --max-output-tokens".to_owned())?;
    let output = output.ok_or_else(|| "llama-generate requires --output".to_owned())?;

    let qualification_bytes = fs::read(&qualification_path).map_err(|error| {
        format!(
            "cannot read qualification '{}': {error}",
            qualification_path.display()
        )
    })?;
    let qualification = LlamaCppQualification::from_bytes(&qualification_bytes)
        .map_err(|error| format!("invalid llama.cpp qualification: {error}"))?;
    let request = TextGenerateRequest::new(prompt, max_output_tokens)
        .map_err(|error| format!("invalid text generation request: {error}"))?;
    let client = LlamaCppClient::new(
        qualification.qualification().endpoint(),
        Duration::from_secs(timeout_seconds),
    )
    .map_err(|error| error.to_string())?;
    let execution = client
        .generate(&qualification, &request)
        .map_err(|error| error.to_string())?;
    let bytes = execution
        .to_canonical_bytes()
        .map_err(|error| format!("cannot serialize llama.cpp execution: {error}"))?;
    write_bytes(&output, &bytes)?;
    eprintln!("execution_path={}", output.display());
    eprintln!("model={}", execution.receipt().model());
    println!("{}", execution.response().text());
    Ok(())
}

fn run_air_qualify(arguments: &[String]) -> Result<(), String> {
    let mut endpoint = None;
    let mut requested_model = None;
    let mut environment = None;
    let mut output = None;
    let mut timeout_seconds = DEFAULT_TIMEOUT_SECONDS;
    let mut index = 0;
    while index < arguments.len() {
        let flag = &arguments[index];
        let value = option_value(arguments, index, flag)?;
        match flag.as_str() {
            "--endpoint" => endpoint = Some(value.to_owned()),
            "--model" => requested_model = Some(value.to_owned()),
            "--environment" => environment = Some(PathBuf::from(value)),
            "--output" => output = Some(PathBuf::from(value)),
            "--timeout-seconds" => timeout_seconds = parse_positive_u64(value, flag)?,
            _ => return Err(format!("unknown air-qualify option '{flag}'\n{}", usage())),
        }
        index += 2;
    }

    let endpoint = endpoint.ok_or_else(|| "air-qualify requires --endpoint".to_owned())?;
    let environment = environment.ok_or_else(|| "air-qualify requires --environment".to_owned())?;
    let output = output.ok_or_else(|| "air-qualify requires --output".to_owned())?;
    let snapshot = read_environment_snapshot(&environment)?;
    ensure_air_endpoint_was_observed(&snapshot, &endpoint)?;
    let environment_sha256 = snapshot
        .identity()
        .map_err(|error| format!("cannot identify environment snapshot: {error}"))?;
    let client = AirClient::new(&endpoint, Duration::from_secs(timeout_seconds))
        .map_err(|error| error.to_string())?;
    let qualification = client
        .qualify(requested_model.as_deref(), environment_sha256)
        .map_err(|error| error.to_string())?;
    let bytes = qualification
        .to_canonical_bytes()
        .map_err(|error| format!("cannot serialize AIR qualification: {error}"))?;
    write_bytes(&output, &bytes)?;
    let identity = qualification
        .qualification()
        .identity()
        .map_err(|error| format!("cannot identify AIR qualification: {error}"))?;
    eprintln!("qualification_path={}", output.display());
    eprintln!(
        "qualified_model={}",
        qualification.qualification().selected_model()
    );
    eprintln!("qualification_sha256={identity}");
    Ok(())
}

fn run_air_generate(arguments: &[String]) -> Result<(), String> {
    let mut qualification_path = None;
    let mut prompt = None;
    let mut max_output_tokens = None;
    let mut output = None;
    let mut timeout_seconds = DEFAULT_TIMEOUT_SECONDS;
    let mut index = 0;
    while index < arguments.len() {
        let flag = &arguments[index];
        let value = option_value(arguments, index, flag)?;
        match flag.as_str() {
            "--qualification" => qualification_path = Some(PathBuf::from(value)),
            "--prompt" => prompt = Some(value.to_owned()),
            "--max-output-tokens" => max_output_tokens = Some(parse_positive_u32(value, flag)?),
            "--output" => output = Some(PathBuf::from(value)),
            "--timeout-seconds" => timeout_seconds = parse_positive_u64(value, flag)?,
            _ => return Err(format!("unknown air-generate option '{flag}'\n{}", usage())),
        }
        index += 2;
    }

    let qualification_path =
        qualification_path.ok_or_else(|| "air-generate requires --qualification".to_owned())?;
    let prompt = prompt.ok_or_else(|| "air-generate requires --prompt".to_owned())?;
    let max_output_tokens =
        max_output_tokens.ok_or_else(|| "air-generate requires --max-output-tokens".to_owned())?;
    let output = output.ok_or_else(|| "air-generate requires --output".to_owned())?;

    let qualification_bytes = fs::read(&qualification_path).map_err(|error| {
        format!(
            "cannot read qualification '{}': {error}",
            qualification_path.display()
        )
    })?;
    let qualification = AirQualification::from_bytes(&qualification_bytes)
        .map_err(|error| format!("invalid AIR qualification: {error}"))?;
    let request = TextGenerateRequest::new(prompt, max_output_tokens)
        .map_err(|error| format!("invalid text generation request: {error}"))?;
    let client = AirClient::new(
        qualification.qualification().endpoint(),
        Duration::from_secs(timeout_seconds),
    )
    .map_err(|error| error.to_string())?;
    let execution = client
        .generate(&qualification, &request)
        .map_err(|error| error.to_string())?;
    let bytes = execution
        .to_canonical_bytes()
        .map_err(|error| format!("cannot serialize AIR execution: {error}"))?;
    write_bytes(&output, &bytes)?;
    eprintln!("execution_path={}", output.display());
    eprintln!("model={}", execution.receipt().model());
    println!("{}", execution.response().text());
    Ok(())
}

fn run_plan(arguments: &[String]) -> Result<(), String> {
    let mut environment = None;
    let mut qualification_paths = Vec::new();
    let mut provider = None;
    let mut base_revision = None;
    let mut consumer = None;
    let mut slot = None;
    let mut provider_instance = None;
    let mut current_provider_instance = None;
    let mut output = None;
    let mut delta_output = None;
    let mut index = 0;
    while index < arguments.len() {
        let flag = &arguments[index];
        let value = option_value(arguments, index, flag)?;
        match flag.as_str() {
            "--environment" => environment = Some(PathBuf::from(value)),
            "--qualification" => qualification_paths.push(PathBuf::from(value)),
            "--provider" => provider = Some(value.to_owned()),
            "--base-revision" => base_revision = Some(value.to_owned()),
            "--consumer" => consumer = Some(value.to_owned()),
            "--slot" => slot = Some(value.to_owned()),
            "--provider-instance" => provider_instance = Some(value.to_owned()),
            "--current-provider-instance" => current_provider_instance = Some(value.to_owned()),
            "--output" => output = Some(PathBuf::from(value)),
            "--delta-output" => delta_output = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown plan option '{flag}'\n{}", usage())),
        }
        index += 2;
    }

    let environment = environment.ok_or_else(|| "plan requires --environment".to_owned())?;
    if qualification_paths.is_empty() {
        return Err("plan requires at least one --qualification".to_owned());
    }
    let provider = provider.ok_or_else(|| "plan requires explicit --provider policy".to_owned())?;
    let base_revision = base_revision.ok_or_else(|| "plan requires --base-revision".to_owned())?;
    let consumer = consumer.ok_or_else(|| "plan requires --consumer".to_owned())?;
    let slot = slot.ok_or_else(|| "plan requires --slot".to_owned())?;
    let provider_instance =
        provider_instance.ok_or_else(|| "plan requires --provider-instance".to_owned())?;
    let output = output.ok_or_else(|| "plan requires --output".to_owned())?;

    let snapshot = read_environment_snapshot(&environment)?;
    let qualifications = qualification_paths
        .iter()
        .map(|path| read_provider_qualification(path))
        .collect::<Result<Vec<_>, _>>()?;
    let policy = DeploymentPolicy::new(
        QualifiedName::new(provider)
            .map_err(|error| format!("invalid provider policy: {error}"))?,
    );
    let target = RealizationTarget::new(
        GraphRevision::new(
            Sha256Digest::new(base_revision)
                .map_err(|error| format!("invalid base revision: {error}"))?,
        ),
        InstanceId::new(consumer).map_err(|error| format!("invalid consumer instance: {error}"))?,
        FieldName::new(slot).map_err(|error| format!("invalid capability slot: {error}"))?,
        InstanceId::new(provider_instance)
            .map_err(|error| format!("invalid provider instance: {error}"))?,
        current_provider_instance
            .map(InstanceId::new)
            .transpose()
            .map_err(|error| format!("invalid current provider instance: {error}"))?,
    );
    let capability = text_generate_v1().map_err(|error| error.to_string())?;
    let plan = DeploymentPlanner::plan(&snapshot, &capability, &qualifications, policy, target)
        .map_err(|error| error.to_string())?;
    let bytes = plan
        .to_canonical_bytes()
        .map_err(|error| format!("cannot serialize deployment plan: {error}"))?;
    write_bytes(&output, &bytes)?;

    if let Some(delta_path) = delta_output {
        if let Some(delta) = plan.graph_delta() {
            let delta_bytes = delta
                .to_canonical_bytes()
                .map_err(|error| format!("cannot serialize Builder GraphDelta: {error}"))?;
            write_bytes(&delta_path, &delta_bytes)?;
            eprintln!("graph_delta_path={}", delta_path.display());
        } else {
            eprintln!("graph_delta=none");
        }
    }

    let identity = plan
        .identity()
        .map_err(|error| format!("cannot identify deployment plan: {error}"))?;
    eprintln!("plan_path={}", output.display());
    eprintln!("plan_sha256={identity}");
    eprintln!("provider={}", plan.provider());
    eprintln!("qualification_sha256={}", plan.qualification_sha256());
    Ok(())
}

fn run_execute_plan(arguments: &[String]) -> Result<(), String> {
    let mut plan_path = None;
    let mut qualification_path = None;
    let mut prompt = None;
    let mut max_output_tokens = None;
    let mut output = None;
    let mut timeout_seconds = DEFAULT_TIMEOUT_SECONDS;
    let mut index = 0;
    while index < arguments.len() {
        let flag = &arguments[index];
        let value = option_value(arguments, index, flag)?;
        match flag.as_str() {
            "--plan" => plan_path = Some(PathBuf::from(value)),
            "--qualification" => qualification_path = Some(PathBuf::from(value)),
            "--prompt" => prompt = Some(value.to_owned()),
            "--max-output-tokens" => max_output_tokens = Some(parse_positive_u32(value, flag)?),
            "--output" => output = Some(PathBuf::from(value)),
            "--timeout-seconds" => timeout_seconds = parse_positive_u64(value, flag)?,
            _ => return Err(format!("unknown execute-plan option '{flag}'\n{}", usage())),
        }
        index += 2;
    }

    let plan_path = plan_path.ok_or_else(|| "execute-plan requires --plan".to_owned())?;
    let qualification_path =
        qualification_path.ok_or_else(|| "execute-plan requires --qualification".to_owned())?;
    let prompt = prompt.ok_or_else(|| "execute-plan requires --prompt".to_owned())?;
    let max_output_tokens =
        max_output_tokens.ok_or_else(|| "execute-plan requires --max-output-tokens".to_owned())?;
    let output = output.ok_or_else(|| "execute-plan requires --output".to_owned())?;

    let plan_bytes = fs::read(&plan_path)
        .map_err(|error| format!("cannot read plan '{}': {error}", plan_path.display()))?;
    let plan = DeploymentPlan::from_bytes(&plan_bytes)
        .map_err(|error| format!("invalid deployment plan: {error}"))?;
    let qualification_bytes = fs::read(&qualification_path).map_err(|error| {
        format!(
            "cannot read qualification '{}': {error}",
            qualification_path.display()
        )
    })?;
    let request = TextGenerateRequest::new(prompt, max_output_tokens)
        .map_err(|error| format!("invalid text generation request: {error}"))?;

    match plan.provider().as_str() {
        "provider.llamacpp.http" => {
            let qualification = LlamaCppQualification::from_bytes(&qualification_bytes)
                .map_err(|error| format!("invalid llama.cpp qualification: {error}"))?;
            plan.verify_qualification(qualification.qualification())
                .map_err(|error| error.to_string())?;
            let client = LlamaCppClient::new(
                qualification.qualification().endpoint(),
                Duration::from_secs(timeout_seconds),
            )
            .map_err(|error| error.to_string())?;
            let execution = client
                .generate(&qualification, &request)
                .map_err(|error| error.to_string())?;
            let bytes = execution
                .to_canonical_bytes()
                .map_err(|error| format!("cannot serialize llama.cpp execution: {error}"))?;
            write_bytes(&output, &bytes)?;
            println!("{}", execution.response().text());
        }
        "provider.air.http" => {
            let qualification = AirQualification::from_bytes(&qualification_bytes)
                .map_err(|error| format!("invalid AIR qualification: {error}"))?;
            plan.verify_qualification(qualification.qualification())
                .map_err(|error| error.to_string())?;
            let client = AirClient::new(
                qualification.qualification().endpoint(),
                Duration::from_secs(timeout_seconds),
            )
            .map_err(|error| error.to_string())?;
            let execution = client
                .generate(&qualification, &request)
                .map_err(|error| error.to_string())?;
            let bytes = execution
                .to_canonical_bytes()
                .map_err(|error| format!("cannot serialize AIR execution: {error}"))?;
            write_bytes(&output, &bytes)?;
            println!("{}", execution.response().text());
        }
        provider => return Err(format!("unsupported planned provider '{provider}'")),
    }
    eprintln!("execution_path={}", output.display());
    eprintln!("provider={}", plan.provider());
    Ok(())
}

fn read_provider_qualification(path: &Path) -> Result<ProviderQualification, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read qualification '{}': {error}", path.display()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid qualification JSON '{}': {error}", path.display()))?;
    let provider = value
        .get("qualification")
        .and_then(|qualification| qualification.get("provider"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            format!(
                "qualification '{}' has no provider identity",
                path.display()
            )
        })?;
    match provider {
        "provider.llamacpp.http" => LlamaCppQualification::from_bytes(&bytes)
            .map(|document| document.qualification().clone())
            .map_err(|error| format!("invalid llama.cpp qualification: {error}")),
        "provider.air.http" => AirQualification::from_bytes(&bytes)
            .map(|document| document.qualification().clone())
            .map_err(|error| format!("invalid AIR qualification: {error}")),
        _ => Err(format!("unsupported qualification provider '{provider}'")),
    }
}

fn read_environment_snapshot(path: &Path) -> Result<EnvironmentSnapshot, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read environment '{}': {error}", path.display()))?;
    let snapshot: EnvironmentSnapshot = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid environment snapshot '{}': {error}", path.display()))?;
    if snapshot.schema() != SNAPSHOT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported environment snapshot schema {}",
            snapshot.schema()
        ));
    }
    Ok(snapshot)
}

fn ensure_llama_endpoint_was_observed(
    snapshot: &EnvironmentSnapshot,
    endpoint: &str,
) -> Result<(), String> {
    let configured = snapshot.runtimes().iter().any(|runtime| {
        runtime.adapter().as_str() == "provider.llamacpp.http"
            && runtime
                .endpoint_candidates()
                .iter()
                .any(|candidate| candidate == endpoint)
    });
    if !configured {
        return Err(format!(
            "llama.cpp endpoint '{endpoint}' was not configured in the supplied environment snapshot"
        ));
    }
    Ok(())
}

fn ensure_air_endpoint_was_observed(
    snapshot: &EnvironmentSnapshot,
    endpoint: &str,
) -> Result<(), String> {
    let configured = snapshot.runtimes().iter().any(|runtime| {
        runtime.adapter().as_str() == "provider.air.http"
            && runtime
                .endpoint_candidates()
                .iter()
                .any(|candidate| candidate == endpoint)
    });
    if !configured {
        return Err(format!(
            "AIR endpoint '{endpoint}' was not configured in the supplied environment snapshot"
        ));
    }
    Ok(())
}

fn option_value<'a>(arguments: &'a [String], index: usize, flag: &str) -> Result<&'a str, String> {
    arguments
        .get(index + 1)
        .map(String::as_str)
        .ok_or_else(|| format!("missing value for '{flag}'\n{}", usage()))
}

fn parse_positive_u64(value: &str, flag: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("'{flag}' requires a positive integer"))?;
    if parsed == 0 {
        return Err(format!("'{flag}' requires a positive integer"));
    }
    Ok(parsed)
}

fn parse_positive_u32(value: &str, flag: &str) -> Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("'{flag}' requires a positive integer"))?;
    if parsed == 0 {
        return Err(format!("'{flag}' requires a positive integer"));
    }
    Ok(parsed)
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create '{}': {error}", parent.display()))?;
    }
    fs::write(path, bytes).map_err(|error| format!("cannot write '{}': {error}", path.display()))
}

fn usage() -> String {
    concat!(
        "usage:\n",
        "  smi-mef-cli snapshot [--llama-endpoint URL] [--air-endpoint URL] ",
        "[--model-root PATH] [--output PATH]\n",
        "  smi-mef-cli llama-qualify --endpoint URL --environment PATH [--model ID] --output PATH ",
        "[--timeout-seconds N]\n",
        "  smi-mef-cli llama-generate --qualification PATH --prompt TEXT ",
        "--max-output-tokens N --output PATH [--timeout-seconds N]\n",
        "  smi-mef-cli air-qualify --endpoint URL --environment PATH [--model ID] --output PATH ",
        "[--timeout-seconds N]\n",
        "  smi-mef-cli air-generate --qualification PATH --prompt TEXT ",
        "--max-output-tokens N --output PATH [--timeout-seconds N]\n",
        "  smi-mef-cli plan --environment PATH --qualification PATH [--qualification PATH ...] ",
        "--provider NAME --base-revision SHA256 --consumer INSTANCE --slot NAME ",
        "--provider-instance INSTANCE [--current-provider-instance INSTANCE] --output PATH ",
        "[--delta-output PATH]\n",
        "  smi-mef-cli execute-plan --plan PATH --qualification PATH --prompt TEXT ",
        "--max-output-tokens N --output PATH [--timeout-seconds N]"
    )
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn unknown_command_is_rejected() {
        assert!(run(["unknown".to_owned()].into_iter()).is_err());
    }

    #[test]
    fn option_without_value_is_rejected() {
        assert!(run(["snapshot".to_owned(), "--model-root".to_owned()].into_iter()).is_err());
    }

    #[test]
    fn generation_requires_prior_qualification_path() {
        let result = run([
            "llama-generate",
            "--prompt",
            "hello",
            "--max-output-tokens",
            "8",
            "--output",
            "ignored.json",
        ]
        .map(str::to_owned)
        .into_iter());
        assert!(result.is_err());
    }
    #[test]
    fn air_generation_requires_prior_qualification_path() {
        let result = run([
            "air-generate",
            "--prompt",
            "hello",
            "--max-output-tokens",
            "8",
            "--output",
            "ignored.json",
        ]
        .map(str::to_owned)
        .into_iter());
        assert!(result.is_err());
    }

    #[test]
    fn planning_requires_explicit_provider_policy() {
        let result = run([
            "plan",
            "--environment",
            "ignored.json",
            "--qualification",
            "ignored.json",
            "--base-revision",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "--consumer",
            "app.generator",
            "--slot",
            "generator",
            "--provider-instance",
            "runtime.air",
            "--output",
            "ignored.json",
        ]
        .map(str::to_owned)
        .into_iter());
        assert!(result.is_err());
    }

    #[test]
    fn planned_execution_requires_plan_path() {
        let result = run([
            "execute-plan",
            "--qualification",
            "ignored.json",
            "--prompt",
            "hello",
            "--max-output-tokens",
            "8",
            "--output",
            "ignored.json",
        ]
        .map(str::to_owned)
        .into_iter());
        assert!(result.is_err());
    }
}
