use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use smi_mef_air::{AirClient, AirQualification};
use smi_mef_core::{EnvironmentSnapshot, TextGenerateRequest, SNAPSHOT_SCHEMA_VERSION};
use smi_mef_llamacpp::{LlamaCppClient, LlamaCppQualification};
use smi_mef_observe::{observe_environment, ObservationConfig};

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
}
