#!/usr/bin/env bash
# Installs the pinned deterministic quality-gate tools.
# Versions are pinned; upgrade deliberately by editing this file.
# Requires: rustup/cargo on PATH. llvm-cov also needs the llvm-tools component.
set -euo pipefail

tools=(
    "cargo-crap@0.6.1"
    "cargo-deny@0.20.2"
    "cargo-mutants@27.1.0"
    "cargo-llvm-cov@0.9.1"
)

rustup component add llvm-tools-preview

for tool in "${tools[@]}"; do
    echo "Installing $tool"
    cargo install --locked "$tool"
done

echo "Verifying installed tools:"
cargo crap --version
cargo deny --version
cargo mutants --version
cargo llvm-cov --version
