/// YIN pitch detection wrapper using the pitch-detection crate.
/// Manages an internal ring buffer and provides a simple push/detect interface.

use pitch_detection::detector::yin::YINDetector;
use pitch_detection::detector::PitchDetector as PitchDetectorTrait;

const MIN_FREQ: f32 = 80.0;
const MAX_FREQ: f32 = 1000.0;
const POWER_THRESHOLD: f64 = 5.0;
const CLARITY_THRESHOLD: f64 = 0.70;

pub struct PitchDetector {
    sample_rate: f32,
    buffer: Vec<f32>,
    buf_size: usize,
    write_pos: usize,
    filled: bool,
    analysis_buffer: Vec<f64>,
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
            analysis_buffer: vec![0.0; buf_size],
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
        if !self.filled {
            return None;
        }

        // Fill analysis buffer from ring buffer without allocating
        let tail = &self.buffer[self.write_pos..];
        let head = &self.buffer[..self.write_pos];
        for (i, &s) in tail.iter().chain(head.iter()).enumerate() {
            self.analysis_buffer[i] = s as f64;
        }

        let size = self.buf_size;
        let padding = size / 2;

        // YINDetector uses Rc internally (not Send), so it must be created per-call.
        // The analysis_buffer reuse still eliminates the main per-call allocation.
        let mut detector = YINDetector::new(size, padding);
        let pitch = detector.get_pitch(
            &self.analysis_buffer[..size],
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
        self.analysis_buffer = vec![0.0; buf_size];
        self.write_pos = 0;
        self.filled = false;
    }
}
