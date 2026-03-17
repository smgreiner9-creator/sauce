/// Cepstral formant shifting.
/// When formant shift is 0, bypassed entirely.
/// When non-zero: FFT → log magnitude → IFFT → lifter → shift envelope → resynthesize.

use realfft::{RealFftPlanner, RealToComplex, ComplexToReal};
use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

pub struct FormantShifter {
    sample_rate: f32,
    fft_size: usize,
    hop_size: usize,
    lifter_order: usize,
    // Pre-allocated FFT plans
    r2c: Arc<dyn RealToComplex<f64>>,
    c2r: Arc<dyn ComplexToReal<f64>>,
    complex_fft: Arc<dyn Fft<f64>>,
    complex_ifft: Arc<dyn Fft<f64>>,
    // Pre-allocated working buffers
    windowed: Vec<f64>,
    spectrum: Vec<Complex<f64>>,
    magnitudes: Vec<f64>,
    phases: Vec<f64>,
    envelope: Vec<f64>,
    shifted_envelope: Vec<f64>,
    new_spectrum: Vec<Complex<f64>>,
    frame_output: Vec<f64>,
    cepstrum: Vec<Complex<f64>>,
}

impl FormantShifter {
    pub fn new(sample_rate: f32) -> Self {
        let fft_size = 2048;
        Self::build(sample_rate, fft_size)
    }

    fn build(sample_rate: f32, fft_size: usize) -> Self {
        let hop_size = fft_size / 4;
        let lifter_order = ((fft_size as f32 * 1400.0 / sample_rate).round() as usize).max(1);

        let mut real_planner = RealFftPlanner::<f64>::new();
        let r2c = real_planner.plan_fft_forward(fft_size);
        let c2r = real_planner.plan_fft_inverse(fft_size);

        let mut complex_planner = FftPlanner::<f64>::new();
        let complex_fft = complex_planner.plan_fft_forward(fft_size);
        let complex_ifft = complex_planner.plan_fft_inverse(fft_size);

        let spec_len = fft_size / 2 + 1;

        Self {
            sample_rate,
            fft_size,
            hop_size,
            lifter_order,
            r2c,
            c2r,
            complex_fft,
            complex_ifft,
            windowed: vec![0.0; fft_size],
            spectrum: vec![Complex::new(0.0, 0.0); spec_len],
            magnitudes: vec![0.0; spec_len],
            phases: vec![0.0; spec_len],
            envelope: vec![0.0; spec_len],
            shifted_envelope: vec![0.0; spec_len],
            new_spectrum: vec![Complex::new(0.0, 0.0); spec_len],
            frame_output: vec![0.0; fft_size],
            cepstrum: vec![Complex::new(0.0, 0.0); fft_size],
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        *self = Self::build(sample_rate, self.fft_size);
    }

    pub fn process(&mut self, input: &[f32], shift_semitones: f32) -> Vec<f32> {
        if shift_semitones.abs() < 0.01 {
            return input.to_vec();
        }

        let len = input.len();
        if len == 0 {
            return vec![];
        }

        let fft_size = self.fft_size;
        let hop_size = self.hop_size;
        let shift_ratio = 2.0f64.powf(shift_semitones as f64 / 12.0);

        // For small blocks (< fft_size): zero-pad to fft_size and process as single frame
        if len < fft_size {
            return self.process_padded(input, shift_ratio);
        }

        // Standard OLA processing for blocks >= fft_size
        let num_frames = (len - fft_size) / hop_size + 1;
        let mut output = vec![0.0f32; len];
        let mut window_sum = vec![0.0f32; len];

        for frame_idx in 0..num_frames {
            let start = frame_idx * hop_size;
            let end = start + fft_size;
            if end > len {
                break;
            }

            self.process_frame(&input[start..end], shift_ratio);

            for i in 0..fft_size {
                let w = hann_window(i, fft_size);
                output[start + i] += (self.frame_output[i] * w) as f32;
                window_sum[start + i] += w as f32;
            }
        }

        for i in 0..len {
            if window_sum[i] > 1e-6 {
                output[i] /= window_sum[i];
            } else {
                output[i] = input[i];
            }
        }

        output
    }

    /// Process a block smaller than fft_size by zero-padding to fft_size.
    fn process_padded(&mut self, input: &[f32], shift_ratio: f64) -> Vec<f32> {
        let len = input.len();
        let fft_size = self.fft_size;

        // Build a zero-padded frame
        let mut padded = vec![0.0f32; fft_size];
        padded[..len].copy_from_slice(input);

        self.process_frame_from_f32(&padded, shift_ratio);

        // Extract just the original-length portion
        let mut output = vec![0.0f32; len];
        for i in 0..len {
            output[i] = self.frame_output[i] as f32;
        }
        output
    }

    /// Process a single frame (already fft_size length) from f32 input.
    fn process_frame_from_f32(&mut self, input: &[f32], shift_ratio: f64) {
        let fft_size = self.fft_size;
        for i in 0..fft_size {
            let w = hann_window(i, fft_size);
            self.windowed[i] = input[i] as f64 * w;
        }
        self.process_frame_inner(shift_ratio);
    }

    /// Process a single frame (already fft_size length) from f32 slice.
    fn process_frame(&mut self, input: &[f32], shift_ratio: f64) {
        let fft_size = self.fft_size;
        debug_assert_eq!(input.len(), fft_size);
        for i in 0..fft_size {
            let w = hann_window(i, fft_size);
            self.windowed[i] = input[i] as f64 * w;
        }
        self.process_frame_inner(shift_ratio);
    }

    /// Core frame processing: FFT → envelope → shift → IFFT.
    /// Expects self.windowed to be filled. Writes result to self.frame_output.
    fn process_frame_inner(&mut self, shift_ratio: f64) {
        let fft_size = self.fft_size;
        let spec_len = fft_size / 2 + 1;

        // Forward FFT (real → complex)
        self.r2c.process(&mut self.windowed, &mut self.spectrum).unwrap();

        // Extract magnitudes and phases
        for i in 0..spec_len {
            self.magnitudes[i] = self.spectrum[i].norm();
            self.phases[i] = self.spectrum[i].arg();
        }

        // Compute spectral envelope via cepstrum
        self.compute_spectral_envelope();

        // Shift envelope
        self.compute_shifted_envelope(shift_ratio);

        // Apply shifted envelope: new_mag = original_mag / original_envelope * shifted_envelope
        for i in 0..spec_len {
            let env = if self.envelope[i] > 1e-10 {
                self.envelope[i]
            } else {
                1e-10
            };
            let new_mag = self.magnitudes[i] / env * self.shifted_envelope[i];
            self.new_spectrum[i] = Complex::new(
                new_mag * self.phases[i].cos(),
                new_mag * self.phases[i].sin(),
            );
        }

        // C2R requires DC and Nyquist to be purely real
        self.new_spectrum[0].im = 0.0;
        self.new_spectrum[spec_len - 1].im = 0.0;

        // Inverse FFT (complex → real)
        self.c2r.process(&mut self.new_spectrum, &mut self.frame_output).unwrap();

        // Normalize IFFT output (realfft C2R does not normalize)
        let norm = 1.0 / fft_size as f64;
        for sample in self.frame_output.iter_mut() {
            *sample *= norm;
        }
    }

    /// Compute spectral envelope via real cepstrum with liftering.
    fn compute_spectral_envelope(&mut self) {
        let fft_size = self.fft_size;
        let spec_len = fft_size / 2 + 1;
        let lifter_order = self.lifter_order;

        // Log magnitude spectrum → build Hermitian-symmetric full-length for complex IFFT
        for i in 0..spec_len {
            let log_mag = (self.magnitudes[i].max(1e-10)).ln();
            self.cepstrum[i] = Complex::new(log_mag, 0.0);
        }
        // Mirror for Hermitian symmetry (indices spec_len..fft_size)
        for i in spec_len..fft_size {
            self.cepstrum[i] = Complex::new(self.cepstrum[fft_size - i].re, 0.0);
        }

        // IFFT to get cepstrum
        self.complex_ifft.process(&mut self.cepstrum);

        // Normalize IFFT: rustfft does not normalize
        let inv_n = 1.0 / fft_size as f64;
        for c in self.cepstrum.iter_mut() {
            c.re *= inv_n;
            c.im *= inv_n;
        }

        // Lifter: zero out high quefrency components
        // Keep [0..lifter_order] and [fft_size-lifter_order..fft_size], zero the middle
        for i in lifter_order..fft_size.saturating_sub(lifter_order) {
            self.cepstrum[i] = Complex::new(0.0, 0.0);
        }

        // FFT back to get smoothed log-magnitude envelope
        self.complex_fft.process(&mut self.cepstrum);

        // Extract envelope: exp(real part) — no division by fft_size needed here
        // because forward FFT is not normalized and we already normalized after IFFT
        for i in 0..spec_len {
            self.envelope[i] = self.cepstrum[i].re.exp();
        }
    }

    /// Shift envelope by frequency ratio using linear interpolation.
    fn compute_shifted_envelope(&mut self, ratio: f64) {
        let len = self.envelope.len();
        self.shifted_envelope[0] = self.envelope[0];
        for i in 1..len {
            let source_idx = i as f64 / ratio;
            let idx_low = source_idx.floor() as usize;
            let idx_high = idx_low + 1;
            let frac = source_idx - idx_low as f64;
            if idx_high < len {
                self.shifted_envelope[i] =
                    self.envelope[idx_low] * (1.0 - frac) + self.envelope[idx_high] * frac;
            } else if idx_low < len {
                self.shifted_envelope[i] = self.envelope[idx_low];
            } else {
                self.shifted_envelope[i] = self.envelope[0];
            }
        }
    }

    pub fn reset(&mut self) {
        // No cross-call state to clear
    }
}

#[inline]
fn hann_window(i: usize, size: usize) -> f64 {
    0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / size as f64).cos())
}
