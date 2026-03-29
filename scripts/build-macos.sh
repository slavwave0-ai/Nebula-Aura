#!/usr/bin/env bash
set -euo pipefail
export CARGO_BUILD_TARGET="x86_64-apple-darwin"
cargo build --release --features clap,vst3 --target x86_64-apple-darwin
export CARGO_BUILD_TARGET="aarch64-apple-darwin"
cargo build --release --features clap,vst3 --target aarch64-apple-darwin
