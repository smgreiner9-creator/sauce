use sauce::dsp::psola::PsolaShifter;

fn generate_sine(freq: f32, sample_rate: f32, num_samples: usize) -> Vec<f32> {
    (0..num_samples)
        .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate).sin())
        .collect()
}

fn measure_pitch(signal: &[f32], sample_rate: f32) -> Option<f32> {
    let max_period = (sample_rate / 80.0) as usize;
    let min_period = (sample_rate / 1000.0) as usize;
    let len = signal.len();
    if len < max_period * 2 { return None; }

    let analysis_len = len.min(8192);
    let sig = &signal[..analysis_len];

    let mut autocorr = Vec::new();
    for lag in min_period..=max_period.min(analysis_len / 2) {
        let mut corr = 0.0f32;
        let mut e1 = 0.0f32;
        let mut e2 = 0.0f32;
        let n = analysis_len - lag;
        for i in 0..n {
            corr += sig[i] * sig[i + lag];
            e1 += sig[i] * sig[i];
            e2 += sig[i + lag] * sig[i + lag];
        }
        let norm = (e1 * e2).sqrt();
        let nc = if norm > 0.0 { corr / norm } else { 0.0 };
        autocorr.push((lag, nc));
    }

    let threshold = 0.5;
    for i in 1..autocorr.len() - 1 {
        let (lag, nc) = autocorr[i];
        if nc > threshold && nc >= autocorr[i - 1].1 && nc >= autocorr[i + 1].1 {
            return Some(sample_rate / lag as f32);
        }
    }
    None
}

#[test]
fn test_no_shift_preserves_pitch() {
    let sr = 44100.0;
    let input = generate_sine(220.0, sr, 8192);
    let mut shifter = PsolaShifter::new(sr);
    let mut output = vec![0.0; input.len()];
    shifter.process_into(&input, 220.0, 220.0, &mut output);
    let detected = measure_pitch(&output, sr);
    assert!(detected.is_some());
    let freq = detected.unwrap();
    assert!((freq - 220.0).abs() < 10.0, "No-shift should preserve pitch, got {freq}");
}

#[test]
fn test_shift_up_octave() {
    let sr = 44100.0;
    let input = generate_sine(220.0, sr, 8192);
    let mut shifter = PsolaShifter::new(sr);
    let mut output = vec![0.0; input.len()];
    shifter.process_into(&input, 220.0, 440.0, &mut output);
    let detected = measure_pitch(&output, sr);
    assert!(detected.is_some());
    let freq = detected.unwrap();
    assert!((freq - 440.0).abs() < 20.0, "Expected ~440 Hz, got {freq}");
}

#[test]
fn test_output_length_matches_input() {
    let sr = 44100.0;
    let input = generate_sine(300.0, sr, 4096);
    let mut shifter = PsolaShifter::new(sr);
    let mut output = vec![0.0; input.len()];
    shifter.process_into(&input, 300.0, 350.0, &mut output);
    // Output was written into pre-allocated slice — length matches by construction
    assert_eq!(output.len(), input.len());
}
