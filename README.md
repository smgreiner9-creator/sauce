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

## Notes

- **Stereo handling:** Pitch correction is mono internally. The dry/wet knob blends between the original stereo signal and the mono-processed wet signal. At 100% wet the output is dual-mono. Best used on mono vocal tracks or with dry/wet blending to preserve stereo width.
- **Formant shift:** The formant knob is bypassed at 0 (default). At non-zero values it applies cepstral envelope shifting — works best on longer buffer sizes (512+).

## Build from Source

Requires Rust toolchain.

```bash
cargo xtask bundle sauce --release
```

Output: `target/bundled/Sauce.vst3`
