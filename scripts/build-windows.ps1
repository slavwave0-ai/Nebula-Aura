$ErrorActionPreference = "Stop"
cargo build --release --features clap,vst3 --target x86_64-pc-windows-msvc
