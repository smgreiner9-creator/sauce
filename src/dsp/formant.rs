/// Cepstral formant shifting.
/// When formant shift is 0, bypassed entirely.
/// When non-zero: FFT → log magnitude → IFFT → lifter → shift envelope → resynthesize.

use realfft::RealFftPlanner;
use rustfft::num_complex::Complex;

pub struct FormantShifter {
    sample_rate: f32,
    fft_size: usize,
}

impl FormantShifter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            fft_size: 2048,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub fn process(&self, input: &[f32], shift_semitones: f32) -> Vec<f32> {
        if shift_semitones.abs() < 0.01 {
            return input.to_vec();
        }

        let len = input.len();
        let fft_size = self.fft_size.min(len);
        let hop_size = fft_size / 4;
        let num_frames = (len.saturating_sub(fft_size)) / hop_size + 1;

        let mut output = vec![0.0f32; len];
        let mut window_sum = vec![0.0f32; len];

        let mut planner = RealFftPlanner::<f64>::new();
        let r2c = planner.plan_fft_forward(fft_size);
        let c2r = planner.plan_fft_inverse(fft_size);

        let shift_ratio = 2.0f64.powf(shift_semitones as f64 / 12.0);

        for frame_idx in 0..num_frames {
            let start = frame_idx * hop_size;
            let end = (start + fft_size).min(len);
            if end - start < fft_size {
                break;
            }

            let mut windowed: Vec<f64> = (0..fft_size)
                .map(|i| {
                    let w = 0.5
                        * (1.0
                            - (2.0 * std::f64::consts::PI * i as f64 / fft_size as f64).cos());
                    input[start + i] as f64 * w
                })
                .collect();

            let mut spectrum = r2c.make_output_vec();
            r2c.process(&mut windowed, &mut spectrum).unwrap();

            let magnitudes: Vec<f64> = spectrum.iter().map(|c| c.norm()).collect();
            let phases: Vec<f64> = spectrum.iter().map(|c| c.arg()).collect();
            let envelope = self.spectral_envelope(&magnitudes, fft_size);
            let shifted_envelope = self.shift_envelope(&envelope, shift_ratio);

            let new_spectrum: Vec<Complex<f64>> = spectrum
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let env = if envelope[i] > 1e-10 {
                        envelope[i]
                    } else {
                        1e-10
                    };
                    let new_mag = magnitudes[i] / env * shifted_envelope[i];
                    Complex::new(new_mag * phases[i].cos(), new_mag * phases[i].sin())
                })
                .collect();

            let mut inv_input = new_spectrum;
            // realfft C2R requires DC and Nyquist bins to be purely real
            inv_input[0].im = 0.0;
            let last = inv_input.len() - 1;
            inv_input[last].im = 0.0;
            let mut frame_output = c2r.make_output_vec();
            c2r.process(&mut inv_input, &mut frame_output).unwrap();

            for i in 0..fft_size {
                let w = 0.5
                    * (1.0
                        - (2.0 * std::f64::consts::PI * i as f64 / fft_size as f64).cos());
                output[start + i] += (frame_output[i] / fft_size as f64 * w) as f32;
                window_sum[start + i] += (w * w) as f32;
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

    fn spectral_envelope(&self, magnitudes: &[f64], fft_size: usize) -> Vec<f64> {
        let spec_len = magnitudes.len();
        let log_mag: Vec<f64> = magnitudes.iter().map(|&m| (m.max(1e-10)).ln()).collect();

        let mut full_log: Vec<Complex<f64>> = vec![Complex::new(0.0, 0.0); fft_size];
        for i in 0..spec_len {
            full_log[i] = Complex::new(log_mag[i], 0.0);
        }
        for i in spec_len..fft_size {
            full_log[i] = full_log[fft_size - i];
        }

        let mut planner = rustfft::FftPlanner::new();
        let ifft = planner.plan_fft_inverse(fft_size);
        ifft.process(&mut full_log);

        let lifter_order = 30;
        for i in lifter_order..fft_size - lifter_order {
            full_log[i] = Complex::new(0.0, 0.0);
        }

        let fft = planner.plan_fft_forward(fft_size);
        fft.process(&mut full_log);

        (0..spec_len)
            .map(|i| (full_log[i].re / fft_size as f64).exp())
            .collect()
    }

    fn shift_envelope(&self, envelope: &[f64], ratio: f64) -> Vec<f64> {
        let len = envelope.len();
        let mut shifted = vec![envelope[0]; len];
        for i in 1..len {
            let source_idx = i as f64 / ratio;
            let idx_low = source_idx.floor() as usize;
            let idx_high = idx_low + 1;
            let frac = source_idx - idx_low as f64;
            if idx_high < len {
                shifted[i] = envelope[idx_low] * (1.0 - frac) + envelope[idx_high] * frac;
            } else if idx_low < len {
                shifted[i] = envelope[idx_low];
            }
        }
        shifted
    }

    pub fn reset(&mut self) {}
}
