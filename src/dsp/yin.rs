/// YIN pitch detection wrapper using the pitch-detection crate.
/// Manages an internal ring buffer and provides a simple push/detect interface.

use pitch_detection::detector::yin::YINDetector;
use pitch_detection::detector::PitchDetector as PitchDetectorTrait;

const MIN_FREQ: f32 = 80.0;
const MAX_FREQ: f32 = 1000.0;
const POWER_THRESHOLD: f64 = 5.0;
const CLARITY_THRESHOLD: f64 = 0.88;

pub struct PitchDetector {
    sample_rate: f32,
    buffer: Vec<f32>,
    buf_size: usize,
    write_pos: usize,
    filled: bool,
}

impl PitchDetector {
    pub fn new(sample_rate: f32) -> Self {
        let min_period_samples = (sample_rate / MIN_FREQ) as usize;
        let buf_size = (min_period_samples * 4).next_power_of_two().max(2048);
        Self {
            sample_rate,
            buffer: vec![0.0; buf_size],
            buf_size,
            write_pos: 0,
            filled: false,
        }
    }

    pub fn push_sample(&mut self, sample: f32) {
        self.buffer[self.write_pos] = sample;
        self.write_pos += 1;
        if self.write_pos >= self.buf_size {
            self.write_pos = 0;
            self.filled = true;
        }
    }

    pub fn detect(&mut self) -> Option<f32> {
        if !self.filled && self.write_pos < self.buf_size / 2 {
            return None;
        }

        let analysis: Vec<f64> = if self.filled {
            self.buffer[self.write_pos..]
                .iter()
                .chain(self.buffer[..self.write_pos].iter())
                .map(|&s| s as f64)
                .collect()
        } else {
            self.buffer[..self.write_pos]
                .iter()
                .map(|&s| s as f64)
                .collect()
        };

        let size = analysis.len();
        let padding = size / 2;

        let mut detector = YINDetector::new(size, padding);
        let pitch = detector.get_pitch(
            &analysis,
            self.sample_rate as usize,
            POWER_THRESHOLD,
            CLARITY_THRESHOLD,
        );

        match pitch {
            Some(p) if p.frequency >= MIN_FREQ as f64 && p.frequency <= MAX_FREQ as f64 => {
                Some(p.frequency as f32)
            }
            _ => None,
        }
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
        self.filled = false;
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let min_period_samples = (sample_rate / MIN_FREQ) as usize;
        let buf_size = (min_period_samples * 4).next_power_of_two().max(2048);
        self.buf_size = buf_size;
        self.buffer = vec![0.0; buf_size];
        self.write_pos = 0;
        self.filled = false;
    }
}
