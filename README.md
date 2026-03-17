# Sauce

T-Pain style auto-tune VST3 plugin.

## Download

Get the latest Windows installer from [Releases](../../releases).

Run `SauceInstaller.exe` → installs to your VST3 folder → open your DAW → Sauce appears.

## Features

- Hard-tune pitch correction (instant snap, robotic effect)
- Key and scale selection
- Formant shifting
- Dry/wet mix
- Neon cyberpunk UI

## Build from Source

Requires Rust toolchain.

```bash
cargo xtask bundle sauce --release
```

Output: `target/bundled/Sauce.vst3`
