use nih_plug::prelude::*;
use std::sync::Arc;
use atomic_float::AtomicF32;

pub mod dsp;
pub mod editor;

pub struct Sauce {
    params: Arc<SauceParams>,
    sample_rate: f32,
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
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        true
    }

    fn reset(&mut self) {}

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            let input_gain = self.params.input_gain.smoothed.next();
            let output_gain = self.params.output_gain.smoothed.next();
            for sample in channel_samples {
                *sample *= input_gain * output_gain;
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
