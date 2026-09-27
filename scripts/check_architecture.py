#!/usr/bin/env python3
from pathlib import Path
import os
import sys

ROOT = Path(__file__).resolve().parents[1]
required = [
    "AGENTS.md",
    "ARCHITECTURE.md",
    "docs/status/CURRENT.md",
    "docs/roadmap/R0_WAVES.md",
    "docs/contracts/ENVIRONMENT_SNAPSHOT_V1.md",
    "docs/contracts/TEXT_GENERATE_V1.md",
    "docs/contracts/LLAMACPP_PROVIDER_V1.md",
    "docs/contracts/AIR_PROVIDER_V1.md",
    "docs/contracts/DEPLOYMENT_PLAN_V1.md",
    "docs/decisions/ADR-0001-authority-boundary.md",
    "docs/decisions/ADR-0002-builder-pin.md",
    "docs/decisions/ADR-0003-llamacpp-first-provider.md",
    "docs/decisions/ADR-0004-explicit-evidence-bound-planning.md",
    "docs/verification/R0-ENDPOINT-PLAN.md",
    "docs/verification/WAVE-3-BASELINE.txt",
    "scripts/pre-submit.sh",
    "scripts/qualify-wave2.sh",
    "scripts/qualify-wave3.sh",
    "scripts/qualify-r0.sh",
    "scripts/collect-r0-evidence.sh",
]
errors = []
for rel in required:
    if not (ROOT / rel).is_file():
        errors.append(f"missing required file: {rel}")

cargo = (ROOT / "Cargo.toml").read_text()
expected = 'rev = "82a5ed814a42dc9ca4e8c1540227f18d4f97e11e"'
if expected not in cargo:
    errors.append("Builder R1 dependency is not pinned to the qualified commit")
for crate in [
    "smi-mef-core",
    "smi-mef-observe",
    "smi-mef-llamacpp",
    "smi-mef-air",
    "smi-mef-plan",
    "smi-mef-cli",
]:
    if crate not in cargo:
        errors.append(f"workspace missing crate: {crate}")

arch = (ROOT / "ARCHITECTURE.md").read_text()
for phrase in [
    "SystemGraph",
    "GraphDelta",
    "EnvironmentSnapshot",
    "DeploymentPlan",
    "explicit provider policy",
    "structural authority",
]:
    if phrase not in arch:
        errors.append(f"architecture doctrine missing phrase: {phrase}")

observe = (ROOT / "crates/smi-mef-observe/src/lib.rs").read_text()
for forbidden in ["reqwest", "ureq", "TcpStream", "/v1/chat/completions", "/generate"]:
    if forbidden in observe:
        errors.append(f"observer contains execution/network primitive: {forbidden}")

for crate, routes in {
    "smi-mef-llamacpp": ["/health", "/v1/models", "/v1/completions", "ProviderQualification"],
    "smi-mef-air": [
        "/health",
        "/v1/models",
        "/model",
        "/runtime",
        "/v1/completions",
        "ProviderQualification",
    ],
}.items():
    source = (ROOT / f"crates/{crate}/src/lib.rs").read_text()
    for token in routes:
        if token not in source:
            errors.append(f"{crate} missing qualification/execution token: {token}")
    for forbidden in ["SystemGraph", "GraphDelta", "DefinitionRegistry", "DeploymentPolicy"]:
        if forbidden in source:
            errors.append(f"{crate} crossed authority boundary with token: {forbidden}")
    if "/v1/chat/completions" in source:
        errors.append(f"{crate} must not widen text.generate into chat semantics")

planner = (ROOT / "crates/smi-mef-plan/src/lib.rs").read_text()
for token in [
    "EnvironmentSnapshot",
    "ProviderQualification",
    "DeploymentPolicy",
    "DeploymentPlan",
    "GraphDelta",
    "StaleQualificationEvidence",
    "AmbiguousQualification",
]:
    if token not in planner:
        errors.append(f"planner missing required token: {token}")
for forbidden in ["reqwest", "TcpStream", "LlamaCppClient", "AirClient", "SystemGraph"]:
    production = planner.split("#[cfg(test)]", 1)[0]
    if forbidden in production:
        errors.append(f"planner owns forbidden execution/structural primitive: {forbidden}")

cli = (ROOT / "crates/smi-mef-cli/src/main.rs").read_text()
for token in ['"plan"', '"execute-plan"', "DeploymentPlanner", "verify_qualification"]:
    if token not in cli:
        errors.append(f"CLI missing explicit plan/execute surface token: {token}")

for script in (ROOT / "scripts").glob("*.sh"):
    if not os.access(script, os.X_OK):
        errors.append(f"shell entrypoint is not executable: {script.relative_to(ROOT)}")

if errors:
    print("ARCHITECTURE CHECK: FAIL")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)
print("ARCHITECTURE CHECK: PASS")
