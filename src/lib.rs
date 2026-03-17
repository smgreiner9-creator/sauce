use nih_plug::prelude::*;
use std::sync::Arc;
use atomic_float::AtomicF32;

pub mod dsp;
pub mod editor;

use dsp::yin::PitchDetector;
use dsp::psola::PsolaShifter;
use dsp::formant::FormantShifter;
use dsp::note_snap::{self, ScaleType};

pub struct Sauce {
    params: Arc<SauceParams>,
    sample_rate: f32,
    pitch_detector: PitchDetector,
    psola_shifter: PsolaShifter,
    formant_shifter: FormantShifter,
    detected_pitch: Arc<AtomicF32>,
    target_pitch: Arc<AtomicF32>,
    mono_buffer: Vec<f32>,
    processed_buffer: Vec<f32>,
    dry_buffer: Vec<f32>,
    max_channels: usize,
}

#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum MusicalKey {
    #[id = "c"]
    C,
    #[id = "cs"]
    #[name = "C#"]
    CSharp,
    #[id = "d"]
    D,
    #[id = "ds"]
    #[name = "D#"]
    DSharp,
    #[id = "e"]
    E,
    #[id = "f"]
    F,
    #[id = "fs"]
    #[name = "F#"]
    FSharp,
    #[id = "g"]
    G,
    #[id = "gs"]
    #[name = "G#"]
    GSharp,
    #[id = "a"]
    A,
    #[id = "as"]
    #[name = "A#"]
    ASharp,
    #[id = "b"]
    B,
}

impl MusicalKey {
    pub fn semitone_offset(&self) -> u8 {
        match self {
            Self::C => 0, Self::CSharp => 1, Self::D => 2, Self::DSharp => 3,
            Self::E => 4, Self::F => 5, Self::FSharp => 6, Self::G => 7,
            Self::GSharp => 8, Self::A => 9, Self::ASharp => 10, Self::B => 11,
        }
    }
}

#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Scale {
    #[id = "chromatic"]
    Chromatic,
    #[id = "major"]
    Major,
    #[id = "minor"]
    Minor,
}

#[derive(Params)]
pub struct SauceParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<nih_plug_egui::EguiState>,

    #[id = "key"]
    pub key: EnumParam<MusicalKey>,

    #[id = "scale"]
    pub scale: EnumParam<Scale>,

    #[id = "formant"]
    pub formant_shift: FloatParam,

    #[id = "drywet"]
    pub dry_wet: FloatParam,

    #[id = "input_gain"]
    pub input_gain: FloatParam,

    #[id = "output_gain"]
    pub output_gain: FloatParam,
}

impl Default for Sauce {
    fn default() -> Self {
        Self {
            params: Arc::new(SauceParams::default()),
            sample_rate: 44100.0,
            pitch_detector: PitchDetector::new(44100.0),
            psola_shifter: PsolaShifter::new(44100.0),
            formant_shifter: FormantShifter::new(44100.0),
            detected_pitch: Arc::new(AtomicF32::new(0.0)),
            target_pitch: Arc::new(AtomicF32::new(0.0)),
            mono_buffer: Vec::new(),
            processed_buffer: Vec::new(),
            dry_buffer: Vec::new(),
            max_channels: 2,
        }
    }
}

impl Default for SauceParams {
    fn default() -> Self {
        Self {
            editor_state: nih_plug_egui::EguiState::from_size(600, 400),

            key: EnumParam::new("Key", MusicalKey::C),

            scale: EnumParam::new("Scale", Scale::Chromatic),

            formant_shift: FloatParam::new(
                "Formant Shift",
                0.0,
                FloatRange::Linear { min: -12.0, max: 12.0 },
            )
            .with_unit(" st")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            dry_wet: FloatParam::new(
                "Dry/Wet",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),

            input_gain: FloatParam::new(
                "Input Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-24.0),
                    max: util::db_to_gain(24.0),
                    factor: FloatRange::gain_skew_factor(-24.0, 24.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            output_gain: FloatParam::new(
                "Output Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-24.0),
                    max: util::db_to_gain(24.0),
                    factor: FloatRange::gain_skew_factor(-24.0, 24.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

impl Plugin for Sauce {
    const NAME: &'static str = "Sauce";
    const VENDOR: &'static str = "Jen";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames::const_default(),
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.pitch_detector.set_sample_rate(buffer_config.sample_rate);
        self.psola_shifter.set_sample_rate(buffer_config.sample_rate);
        self.formant_shifter.set_sample_rate(buffer_config.sample_rate);

        let max_samples = buffer_config.max_buffer_size as usize;
        let num_channels = audio_io_layout
            .main_input_channels
            .map(|c| c.get() as usize)
            .unwrap_or(2);
        self.max_channels = num_channels;
        self.mono_buffer = vec![0.0; max_samples];
        self.processed_buffer = vec![0.0; max_samples];
        self.dry_buffer = vec![0.0; max_samples * num_channels];
        true
    }

    fn reset(&mut self) {
        self.pitch_detector.reset();
        self.psola_shifter.reset();
        self.formant_shifter.reset();
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(
            self.params.clone(),
            self.detected_pitch.clone(),
            self.target_pitch.clone(),
        )
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let num_samples = buffer.samples();
        let num_channels = buffer.channels();

        let key_root = self.params.key.value().semitone_offset();
        let scale = match self.params.scale.value() {
            Scale::Chromatic => ScaleType::Chromatic,
            Scale::Major => ScaleType::Major,
            Scale::Minor => ScaleType::Minor,
        };
        let formant_shift = self.params.formant_shift.value();

        // Save original per-channel samples and extract mono signal
        for (i, channel_samples) in buffer.iter_samples().enumerate() {
            let input_gain = self.params.input_gain.smoothed.next();
            let mut sum = 0.0f32;
            for (ch, sample) in channel_samples.into_iter().enumerate() {
                self.dry_buffer[i * num_channels + ch] = *sample;
                sum += *sample;
            }
            self.mono_buffer[i] = (sum / num_channels as f32) * input_gain;
        }
        let mono_len = num_samples;

        // Pitch detection
        for i in 0..mono_len {
            self.pitch_detector.push_sample(self.mono_buffer[i]);
        }
        let detected_freq = self.pitch_detector.detect();

        // Pitch correction
        if let Some(freq) = detected_freq {
            self.detected_pitch.store(freq, std::sync::atomic::Ordering::Relaxed);

            if let Some(target_freq) = note_snap::snap_frequency(freq, key_root, scale) {
                self.target_pitch.store(target_freq, std::sync::atomic::Ordering::Relaxed);

                // PSOLA writes into processed_buffer, formant reads from there
                self.psola_shifter.process_into(
                    &self.mono_buffer[..mono_len], freq, target_freq,
                    &mut self.processed_buffer[..mono_len],
                );
                // Formant shift: use mono_buffer as scratch to avoid allocation
                if formant_shift.abs() >= 0.01 {
                    // Copy PSOLA output to mono_buffer (scratch), then formant writes back to processed_buffer
                    self.mono_buffer[..mono_len].copy_from_slice(&self.processed_buffer[..mono_len]);
                    self.formant_shifter.process_into(
                        &self.mono_buffer[..mono_len], formant_shift,
                        &mut self.processed_buffer[..mono_len],
                    );
                }
            } else {
                self.processed_buffer[..mono_len].copy_from_slice(&self.mono_buffer[..mono_len]);
                self.psola_shifter.reset();
            }
        } else {
            self.detected_pitch.store(0.0, std::sync::atomic::Ordering::Relaxed);
            self.target_pitch.store(0.0, std::sync::atomic::Ordering::Relaxed);
            self.processed_buffer[..mono_len].copy_from_slice(&self.mono_buffer[..mono_len]);
            self.psola_shifter.reset();
        }

        // Dry/wet mix and write back, preserving stereo image for dry path
        for (i, mut channel_samples) in buffer.iter_samples().enumerate() {
            let dry_wet = self.params.dry_wet.smoothed.next();
            let output_gain = self.params.output_gain.smoothed.next();

            let wet = self.processed_buffer[i];
            for (ch, sample) in channel_samples.iter_mut().enumerate() {
                let dry = self.dry_buffer[i * num_channels + ch];
                let mixed = dry * (1.0 - dry_wet) + wet * dry_wet;
                *sample = mixed * output_gain;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Sauce {
    const CLAP_ID: &'static str = "com.jen.sauce";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("T-Pain style auto-tune");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::PitchShifter,
        ClapFeature::Stereo,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for Sauce {
    const VST3_CLASS_ID: [u8; 16] = *b"SauceAutotuneJen";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(Sauce);
nih_export_vst3!(Sauce);
