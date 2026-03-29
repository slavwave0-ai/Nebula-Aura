#!/usr/bin/env bash
set -euo pipefail
cargo build --release --features clap,vst3
