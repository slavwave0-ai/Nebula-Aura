# Nebula Aura v1.0

Nebula Aura is a 64-bit open-source MIT-licensed CLAP and VST3 exciter plugin made by Nebula Audio.

## Highlights
- Sci-fi synthwave egui UI with neon cyan/magenta/yellow palette and animated scanlines.
- Freely resizable/scalable UI with proportional control layout.
- DSP controls: input, output, harmonics, frequency, slope, odd/even emphasis, mix, phase invert, oversampling selector.
- Spectrum analyzer and gain-reduction readout.
- Preset model, A/B state switching model, MIDI mapping framework, and soft FX bypass.
- Cross-platform build scripts for Linux/macOS/Windows.

## Build
- Linux: `./scripts/build-linux.sh`
- macOS: `./scripts/build-macos.sh`
- Windows (PowerShell): `./scripts/build-windows.ps1`

## CI
GitHub Actions compiles Linux, macOS, and Windows artifacts for CLAP and VST3 on every push.
