#!/usr/bin/env python3
from pathlib import Path
import os, re, sys

ROOT = Path(__file__).resolve().parents[1]
required = [
    "AGENTS.md", "ARCHITECTURE.md", "docs/status/CURRENT.md",
    "docs/roadmap/R0_WAVES.md", "docs/contracts/ENVIRONMENT_SNAPSHOT_V1.md",
    "docs/contracts/TEXT_GENERATE_V1.md", "docs/contracts/LLAMACPP_PROVIDER_V1.md",
    "docs/contracts/AIR_PROVIDER_V1.md",
    "docs/decisions/ADR-0001-authority-boundary.md", "docs/decisions/ADR-0002-builder-pin.md",
    "docs/decisions/ADR-0003-llamacpp-first-provider.md", "scripts/pre-submit.sh",
    "scripts/qualify-wave2.sh", "scripts/qualify-wave3.sh",
]
errors=[]
for rel in required:
    if not (ROOT/rel).is_file(): errors.append(f"missing required file: {rel}")

cargo=(ROOT/"Cargo.toml").read_text()
expected='rev = "82a5ed814a42dc9ca4e8c1540227f18d4f97e11e"'
if expected not in cargo:
    errors.append("Builder R1 dependency is not pinned to the qualified commit")
if any(crate not in cargo for crate in ["smi-mef-core", "smi-mef-observe", "smi-mef-llamacpp", "smi-mef-air", "smi-mef-cli"]):
    errors.append("workspace crate set is incomplete")

arch=(ROOT/"ARCHITECTURE.md").read_text()
for phrase in ["SystemGraph", "GraphDelta", "EnvironmentSnapshot", "second graph or registry"]:
    if phrase not in arch:
        errors.append(f"architecture doctrine missing phrase: {phrase}")

observe=(ROOT/"crates/smi-mef-observe/src/lib.rs").read_text()
for forbidden in ["reqwest", "ureq", "TcpStream", "/v1/chat/completions", "/generate"]:
    if forbidden in observe:
        errors.append(f"Wave 1 observer contains execution/network primitive: {forbidden}")


provider=(ROOT/"crates/smi-mef-llamacpp/src/lib.rs").read_text()
for required_provider_token in ["/health", "/v1/models", "/v1/completions", "ProviderQualification"]:
    if required_provider_token not in provider:
        errors.append(f"llama.cpp provider missing qualification/execution token: {required_provider_token}")
for forbidden_authority in ["SystemGraph", "GraphDelta", "DefinitionRegistry"]:
    if forbidden_authority in provider:
        errors.append(f"llama.cpp provider imports/mentions Builder structural authority: {forbidden_authority}")
if "/v1/chat/completions" in provider:
    errors.append("Wave 2 provider must not widen text.generate into chat semantics")

air_provider=(ROOT/"crates/smi-mef-air/src/lib.rs").read_text()
for required_provider_token in ["/health", "/v1/models", "/model", "/runtime", "/v1/completions", "ProviderQualification"]:
    if required_provider_token not in air_provider:
        errors.append(f"AIR provider missing qualification/execution token: {required_provider_token}")
for forbidden_authority in ["SystemGraph", "GraphDelta", "DefinitionRegistry"]:
    if forbidden_authority in air_provider:
        errors.append(f"AIR provider imports/mentions Builder structural authority: {forbidden_authority}")
if "/v1/chat/completions" in air_provider:
    errors.append("Wave 3 provider must not widen text.generate into chat semantics")

for script in (ROOT/"scripts").glob("*.sh"):
    if not os.access(script, os.X_OK):
        errors.append(f"shell entrypoint is not executable: {script.relative_to(ROOT)}")

if errors:
    print("ARCHITECTURE CHECK: FAIL")
    for error in errors: print(f"- {error}")
    sys.exit(1)
print("ARCHITECTURE CHECK: PASS")
