#!/usr/bin/env python3
from pathlib import Path
import re, sys
ROOT=Path(__file__).resolve().parents[1]
errors=[]
text=(ROOT/"rust-toolchain.toml").read_text()
if 'channel = "1.98.1"' not in text:
    errors.append("rust-toolchain.toml must pin Rust 1.98.1")
for path in ROOT.rglob("*"):
    if any(part in {"target", ".git"} for part in path.parts):
        continue
    if path.name.endswith((".rej", ".orig", ".pyc")) or path.name == "__pycache__":
        errors.append(f"generated/patch debris: {path.relative_to(ROOT)}")
for path in ROOT.rglob("*.rs"):
    source=path.read_text()
    if "#[allow(clippy::" in source:
        errors.append(f"unapproved Clippy suppression: {path.relative_to(ROOT)}")
    if re.search(r"\bif\s+let\b[^\n]+&&\s+let\b", source):
        errors.append(f"Rust 2024 let-chain in edition 2021 source: {path.relative_to(ROOT)}")
    production_source = source.split("#[cfg(test)]", 1)[0]
    if re.search(r"\.(?:expect|unwrap)\s*\(", production_source):
        errors.append(f"production expect/unwrap is forbidden: {path.relative_to(ROOT)}")

provider_path = ROOT / "crates/smi-mef-llamacpp/src/lib.rs"
if provider_path.is_file():
    provider = provider_path.read_text()
    if "http://127.0.0.1:1920" not in provider:
        errors.append("Wave 2 provider tests must retain the canonical loopback endpoint fixture")
    if "temperature: 0.0" not in provider or "stream: false" not in provider:
        errors.append("Wave 2 completion request lost bounded deterministic sampling defaults")

if errors:
    print("PRE-SUBMIT STATIC: FAIL")
    for error in errors: print(f"- {error}")
    sys.exit(1)
print("PRE-SUBMIT STATIC: PASS")
