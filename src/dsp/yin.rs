/// YIN pitch detection with periodic analysis every N/4 samples.
///
/// Detection fires automatically every N/4 input samples.
/// Holds last valid pitch through unvoiced segments.

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

    // Periodic detection
    samples_since_detection: usize,
    detection_interval: usize, // N/4

    // Held pitch state
    current_pitch: Option<f32>,
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
            samples_since_detection: 0,
            detection_interval: buf_size / 4,
            current_pitch: None,
        }
    }

    /// Push a sample. Returns Some(freq) when detection fires and finds a pitch.
    /// Returns None most of the time (detection only runs every N/4 samples).
    /// When detection runs but finds no pitch, current_pitch is held (not cleared).
    pub fn push_sample(&mut self, sample: f32) -> Option<f32> {
        self.buffer[self.write_pos] = sample;
        self.write_pos += 1;
        if self.write_pos >= self.buf_size {
            self.write_pos = 0;
            self.filled = true;
        }

        self.samples_since_detection += 1;

        if self.filled && self.samples_since_detection >= self.detection_interval {
            self.samples_since_detection = 0;
            let detected = self.run_detection();
            if let Some(freq) = detected {
                self.current_pitch = Some(freq);
            }
            // If detection fails, current_pitch stays (hold behavior)
            return self.current_pitch;
        }

        None
    }

    /// Get the most recently detected pitch.
    pub fn current_pitch(&self) -> Option<f32> {
        self.current_pitch
    }

    /// Get the analysis window size N (for latency calculation).
    pub fn window_size(&self) -> usize {
        self.buf_size
    }

    fn run_detection(&mut self) -> Option<f32> {
        let tail = &self.buffer[self.write_pos..];
        let head = &self.buffer[..self.write_pos];
        for (i, &s) in tail.iter().chain(head.iter()).enumerate() {
            self.analysis_buffer[i] = s as f64;
        }

        let size = self.buf_size;
        let padding = size / 2;

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
        self.samples_since_detection = 0;
        self.current_pitch = None;
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let min_period_samples = (sample_rate / MIN_FREQ) as usize;
        let buf_size = (min_period_samples * 4).next_power_of_two().max(2048);
        self.buf_size = buf_size;
        self.buffer = vec![0.0; buf_size];
        self.analysis_buffer = vec![0.0; buf_size];
        self.detection_interval = buf_size / 4;
        self.write_pos = 0;
        self.filled = false;
        self.samples_since_detection = 0;
        self.current_pitch = None;
    }
}
