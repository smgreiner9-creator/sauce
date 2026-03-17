/// Integration tests for the PhaseTrackingShifter.
///
/// Uses sine wave generation and autocorrelation-based pitch measurement
/// to verify that the shifter produces the expected output frequencies.

// The shifter module is pub inside src/dsp/shifter.rs but we need to access
// it through the crate. Since mod.rs doesn't export it yet, we include directly.
// This file tests the shifter as a standalone module.

// We'll use a path attribute to include the module directly for testing,
// since we're told not to modify mod.rs yet.
#[path = "../src/dsp/shifter.rs"]
mod shifter;

use shifter::PhaseTrackingShifter;

const SR: f32 = 44100.0;

/// Generate a sine wave at the given frequency.
fn generate_sine(freq: f32, sample_rate: f32, num_samples: usize) -> Vec<f32> {
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate;
            (2.0 * std::f32::consts::PI * freq * t).sin()
        })
        .collect()
}

/// Measure the fundamental frequency of a signal using autocorrelation.
///
/// Uses the "first peak" strategy: finds the first autocorrelation peak above
/// a threshold of the global maximum. This avoids locking onto subharmonics.
///
/// Returns None if no clear pitch is found.
fn measure_pitch(signal: &[f32], sample_rate: f32) -> Option<f32> {
    let len = signal.len();
    if len < 256 {
        return None;
    }

    let max_lag = ((sample_rate / 80.0) as usize).min(len / 2); // min freq 80 Hz
    let min_lag = (sample_rate / 1000.0) as usize; // max freq 1000 Hz

    if min_lag >= max_lag {
        return None;
    }

    // Compute normalized autocorrelation
    let count = (len / 2).min(8192);
    let mut corrs = vec![0.0f64; max_lag + 1];

    // Zero-lag energy
    let mut energy = 0.0f64;
    for i in 0..count {
        energy += signal[i] as f64 * signal[i] as f64;
    }
    if energy < 1e-8 {
        return None;
    }

    for lag in min_lag..=max_lag {
        let mut sum = 0.0f64;
        let n = count.min(len - lag);
        for i in 0..n {
            sum += signal[i] as f64 * signal[i + lag] as f64;
        }
        corrs[lag] = sum / energy;
    }

    // Find global max correlation
    let global_max = corrs[min_lag..=max_lag]
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    if global_max < 0.2 {
        return None;
    }

    // Find the FIRST peak above 0.8 * global_max (avoids subharmonics)
    let threshold = 0.8 * global_max;
    let mut in_rise = false;
    let mut prev = f64::NEG_INFINITY;

    for lag in min_lag..max_lag {
        let c = corrs[lag];
        if c > prev {
            in_rise = true;
        } else if in_rise && c < prev {
            // prev was a local peak
            if prev >= threshold {
                return Some(sample_rate / (lag - 1) as f32);
            }
            in_rise = false;
        }
        prev = c;
    }

    None
}

#[test]
fn test_silence_before_set_pitch() {
    let mut shifter = PhaseTrackingShifter::new(SR);
    let input = generate_sine(220.0, SR, 4096);
    let mut all_zero = true;
    for &s in &input {
        let out = shifter.process_sample(s);
        if out != 0.0 {
            all_zero = false;
            break;
        }
    }
    assert!(all_zero, "Output should be silence before set_pitch is called");
}

#[test]
fn test_output_not_all_zeros() {
    let mut shifter = PhaseTrackingShifter::new(SR);
    shifter.set_pitch(220.0, 220.0);

    let input = generate_sine(220.0, SR, SR as usize); // 1 second
    let mut output = Vec::with_capacity(input.len());
    for &s in &input {
        output.push(shifter.process_sample(s));
    }

    // Skip latency samples
    let latency = shifter.latency_samples() as usize;
    let active = &output[latency..];

    let max_val = active.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
    assert!(
        max_val > 0.01,
        "Output should not be all zeros after pitch is set (max={max_val})"
    );
}

#[test]
fn test_no_shift_preserves_pitch() {
    let freq = 220.0;
    let mut shifter = PhaseTrackingShifter::new(SR);
    shifter.set_pitch(freq, freq); // no shift

    let num_samples = SR as usize; // 1 second
    let input = generate_sine(freq, SR, num_samples);
    let mut output = Vec::with_capacity(num_samples);
    for &s in &input {
        output.push(shifter.process_sample(s));
    }

    // Skip latency + some settling time
    let skip = shifter.latency_samples() as usize + 4096;
    if skip >= output.len() {
        panic!("Not enough samples after latency");
    }
    let analysis = &output[skip..];

    let detected = measure_pitch(analysis, SR);
    assert!(
        detected.is_some(),
        "Should detect a pitch in the output"
    );
    let detected = detected.unwrap();
    assert!(
        (detected - freq).abs() < 15.0,
        "No-shift should preserve pitch: expected ~{freq}Hz, got {detected}Hz"
    );
}

#[test]
fn test_octave_up_shift() {
    let input_freq = 220.0;
    let target_freq = 440.0;
    let mut shifter = PhaseTrackingShifter::new(SR);
    shifter.set_pitch(input_freq, target_freq);

    let num_samples = SR as usize; // 1 second
    let input = generate_sine(input_freq, SR, num_samples);
    let mut output = Vec::with_capacity(num_samples);
    for &s in &input {
        output.push(shifter.process_sample(s));
    }

    // Skip latency + settling
    let skip = shifter.latency_samples() as usize + 4096;
    let analysis = &output[skip..];

    let detected = measure_pitch(analysis, SR);
    assert!(
        detected.is_some(),
        "Should detect a pitch in octave-up output"
    );
    let detected = detected.unwrap();
    assert!(
        (detected - target_freq).abs() < 30.0,
        "Octave up should produce ~{target_freq}Hz, got {detected}Hz"
    );
}
