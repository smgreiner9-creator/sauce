# Sauce — Architecture

## Signal Flow

```
Input → Input Gain → [Mono Sum] → YIN Pitch Detection → Note Snap (Key/Scale)
    → TD-PSOLA Pitch Shift → Formant Shift (if non-zero) → Dry/Wet Mix → Output Gain → Output
```

## Module Map

| Module | File | Purpose |
|--------|------|---------|
| Plugin core | `src/lib.rs` | nih-plug Plugin trait, params, process() |
| Note snap | `src/dsp/note_snap.rs` | Freq↔MIDI conversion, scale quantization |
| YIN detector | `src/dsp/yin.rs` | Ring-buffered pitch detection (80Hz–1kHz) |
| PSOLA shifter | `src/dsp/psola.rs` | Time-domain pitch shifting with OLA |
| Formant shift | `src/dsp/formant.rs` | Cepstral envelope extraction + shifting |
| GUI | `src/editor/mod.rs` | egui editor layout and title rendering |
| Widgets | `src/editor/widgets.rs` | Neon knobs, dropdowns, pitch meter |

## Key Design Decisions

- **T-Pain mode only**: Retune speed hardcoded to 0ms. No graphical editing.
- **Formant bypass at 0**: When formant shift = 0, the module is skipped entirely.
- **Mono processing**: Stereo input summed to mono for DSP. Output duplicated to all channels.
- **pitch-detection crate**: Uses battle-tested YIN implementation.
- **Block processing**: PSOLA and formant operate on the full buffer block.

## Dependencies

| Crate | Purpose |
|-------|---------|
| nih_plug | VST3/CLAP plugin framework |
| nih_plug_egui | egui integration |
| pitch-detection | YIN algorithm |
| realfft + rustfft | FFT for formant cepstral analysis |
| atomic_float | Lock-free float for audio↔GUI communication |
