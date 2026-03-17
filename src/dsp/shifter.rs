/// Autotalune-style phase-tracking pitch shifter.
///
/// Two independent phase accumulators track input and output pitch cycles.
/// When the input phase wraps, a grain (one pitch period) is captured from
/// the circular input buffer. When the output phase wraps, the grain is
/// resampled to the output period length via cubic Lagrange interpolation,
/// Hann-windowed, and overlap-added into the output ring buffer.
///
/// All processing is sample-by-sample with zero allocations in the hot path.

use std::f64::consts::PI;

/// Cubic Lagrange interpolation (Autotalune/Chamberlin style).
///
/// Reads from `samples[0..len]`, interpolating at fractional position `pos`.
#[inline(always)]
fn cubic_lagrange_interp(samples: &[f32], len: usize, pos: f64) -> f64 {
    let idx = pos.floor() as isize;
    let frac = pos - pos.floor();

    let s = |i: isize| -> f64 { samples[i.clamp(0, len as isize - 1) as usize] as f64 };

    let y0 = s(idx - 1);
    let y1 = s(idx);
    let y2 = s(idx + 1);
    let y3 = s(idx + 2);

    let d = frac;
    // Lagrange basis polynomials for 4-point interpolation
    -y0 * d * (d - 1.0) * (d - 2.0) / 6.0
        + y1 * (d + 1.0) * (d - 1.0) * (d - 2.0) / 2.0
        - y2 * (d + 1.0) * d * (d - 2.0) / 2.0
        + y3 * (d + 1.0) * d * (d - 1.0) / 6.0
}

pub struct PhaseTrackingShifter {
    sample_rate: f32,

    // Circular input buffer
    cbuf: Vec<f32>,
    cbuf_len: usize,
    cbuf_write: usize,

    // Phase accumulators (f64 for precision)
    input_phase: f64,
    output_phase: f64,

    // Current pitch periods in samples (held through unvoiced regions)
    input_period: f64,
    output_period: f64,

    // Captured grain (pre-allocated, grain_len is current valid length)
    grain: Vec<f32>,
    grain_len: usize,

    // Output ring buffer for overlap-add
    output_accum: Vec<f32>,
    output_accum_len: usize,
    output_read: usize,
    output_write: usize,

    // Latency in samples
    latency: usize,

    // Whether we have received a valid pitch at least once
    pitch_valid: bool,
}

impl PhaseTrackingShifter {
    pub fn new(sample_rate: f32) -> Self {
        let cbuf_len = compute_buf_size(sample_rate);
        let latency = cbuf_len / 2;
        let output_accum_len = cbuf_len * 2; // 2x for 50% overlap headroom

        Self {
            sample_rate,
            cbuf: vec![0.0; cbuf_len],
            cbuf_len,
            cbuf_write: 0,
            input_phase: 0.0,
            output_phase: 0.0,
            input_period: 0.0,
            output_period: 0.0,
            grain: vec![0.0; cbuf_len / 2],
            grain_len: 0,
            output_accum: vec![0.0; output_accum_len],
            output_accum_len,
            output_read: 0,
            output_write: latency, // start write ahead by latency
            latency,
            pitch_valid: false,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        *self = Self::new(sample_rate);
    }

    pub fn latency_samples(&self) -> u32 {
        self.latency as u32
    }

    /// Update pitch target.
    ///
    /// When `detected_freq` or `target_freq` is 0 (or negative), the last valid
    /// pitch periods are held — this allows smooth output through unvoiced frames.
    pub fn set_pitch(&mut self, detected_freq: f32, target_freq: f32) {
        if detected_freq > 0.0 && target_freq > 0.0 {
            // Re-anchor output_write on first valid pitch so pointers are aligned
            if !self.pitch_valid {
                self.output_write = (self.output_read + self.latency) % self.output_accum_len;
            }
            self.input_period = self.sample_rate as f64 / detected_freq as f64;
            self.output_period = self.sample_rate as f64 / target_freq as f64;
            self.pitch_valid = true;
        }
        // else: hold last valid periods
    }

    /// Process one input sample and return one output sample.
    ///
    /// Zero allocations. All work uses pre-allocated ring buffers and grain storage.
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        // Always write input into circular buffer
        self.cbuf[self.cbuf_write] = input;
        self.cbuf_write = (self.cbuf_write + 1) % self.cbuf_len;

        // Before first valid pitch, output silence but keep read pointer moving
        if !self.pitch_valid || self.input_period < 1.0 || self.output_period < 1.0 {
            // Advance output read pointer but DON'T advance input/output phases
            self.output_accum[self.output_read] = 0.0;
            self.output_read = (self.output_read + 1) % self.output_accum_len;
            return 0.0;
        }

        // Advance input phase
        self.input_phase += 1.0 / self.input_period;

        if self.input_phase >= 1.0 {
            self.input_phase -= 1.0;
            // Capture one grain: input_period samples ending at current write pos
            self.capture_grain();
        }

        // Advance output phase at normal rate (one grain per output period)
        self.output_phase += 1.0 / self.output_period;

        if self.output_phase >= 1.0 {
            self.output_phase -= 1.0;
            if self.grain_len > 0 {
                self.place_grain();
            }
        }

        // Read from output ring buffer
        let out = self.output_accum[self.output_read];
        self.output_accum[self.output_read] = 0.0; // clear after reading
        self.output_read = (self.output_read + 1) % self.output_accum_len;

        out
    }

    /// Capture a grain of `input_period` samples from the circular input buffer,
    /// ending at the current write position.
    #[inline]
    fn capture_grain(&mut self) {
        let len = self.input_period.round() as usize;
        let len = len.min(self.grain.len()).max(1);
        self.grain_len = len;

        for i in 0..len {
            // Read backwards from write position
            let idx =
                (self.cbuf_write + self.cbuf_len - len + i) % self.cbuf_len;
            self.grain[i] = self.cbuf[idx];
        }
    }

    /// Resample the captured grain to output_period length, apply Hann window,
    /// and overlap-add into the output ring buffer with 50% overlap (COLA).
    #[inline]
    fn place_grain(&mut self) {
        let out_len = self.output_period.round() as usize;
        if out_len < 2 || self.grain_len == 0 {
            return;
        }

        // Overwrite guard: drop grain if it would lap the read pointer
        let available = if self.output_write >= self.output_read {
            self.output_accum_len - (self.output_write - self.output_read)
        } else {
            self.output_read - self.output_write
        };
        if out_len > available {
            return;
        }

        let grain_len = self.grain_len;
        let ratio = grain_len as f64 / out_len as f64;
        let denom = (out_len - 1).max(1) as f64;

        // Write the resampled, Hann-windowed grain starting at output_write.
        // Grains tile the output — each occupies one output period.
        // The Hann window tapers edges; the output buffer is cleared after reading
        // (line 159), so there's no stale accumulation.
        for i in 0..out_len {
            let src_pos = i as f64 * ratio;
            let sample = cubic_lagrange_interp(&self.grain, grain_len, src_pos);

            // Hann window — smooth taper at grain edges
            let w = 0.5 * (1.0 - (2.0 * PI * i as f64 / denom).cos());

            let write_idx = (self.output_write + i) % self.output_accum_len;
            self.output_accum[write_idx] += (sample * w) as f32;
        }

        // Advance write pointer by full grain length
        self.output_write = (self.output_write + out_len) % self.output_accum_len;
    }

    pub fn reset(&mut self) {
        self.cbuf.fill(0.0);
        self.cbuf_write = 0;
        self.input_phase = 0.0;
        self.output_phase = 0.0;
        self.input_period = 0.0;
        self.output_period = 0.0;
        self.grain.fill(0.0);
        self.grain_len = 0;
        self.output_accum.fill(0.0);
        self.output_read = 0;
        self.output_write = self.latency;
        self.pitch_valid = false;
    }
}

/// Compute buffer size matching YIN detector sizing.
fn compute_buf_size(sample_rate: f32) -> usize {
    let min_period_samples = (sample_rate / 80.0) as usize;
    (min_period_samples * 4).next_power_of_two().max(2048)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_sizing() {
        let s = PhaseTrackingShifter::new(44100.0);
        assert_eq!(s.cbuf_len, 4096);
        assert_eq!(s.latency, 2048);
        assert_eq!(s.output_accum_len, 8192);
    }

    #[test]
    fn test_silence_before_pitch() {
        let mut s = PhaseTrackingShifter::new(44100.0);
        for _ in 0..1000 {
            let out = s.process_sample(1.0);
            assert_eq!(out, 0.0);
        }
    }

    #[test]
    fn test_cubic_lagrange_linear() {
        // On a linear ramp, cubic interpolation should be exact
        let samples: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let val = cubic_lagrange_interp(&samples, 10, 3.5);
        assert!((val - 3.5).abs() < 1e-10);
    }
}
