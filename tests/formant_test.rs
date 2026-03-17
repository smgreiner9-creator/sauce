use sauce::dsp::formant::FormantShifter;

fn generate_sine(freq: f32, sample_rate: f32, num_samples: usize) -> Vec<f32> {
    (0..num_samples)
        .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate).sin())
        .collect()
}

#[test]
fn test_zero_shift_is_passthrough() {
    let sr = 44100.0;
    let input = generate_sine(440.0, sr, 2048);
    let mut shifter = FormantShifter::new(sr);
    let output = shifter.process(&input, 0.0);
    let rms_diff: f32 = input
        .iter()
        .zip(output.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f32>()
        / input.len() as f32;
    assert!(
        rms_diff.sqrt() < 0.1,
        "Zero shift should be near-passthrough, RMS diff = {}",
        rms_diff.sqrt()
    );
}

#[test]
fn test_nonzero_shift_changes_signal() {
    let sr = 44100.0;
    let input = generate_sine(440.0, sr, 2048);
    let mut shifter = FormantShifter::new(sr);
    let output = shifter.process(&input, 6.0);
    let rms_diff: f32 = input
        .iter()
        .zip(output.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f32>()
        / input.len() as f32;
    assert!(
        rms_diff.sqrt() > 0.01,
        "Non-zero shift should change the signal"
    );
}

#[test]
fn test_output_length_matches() {
    let sr = 44100.0;
    let input = generate_sine(440.0, sr, 2048);
    let mut shifter = FormantShifter::new(sr);
    let output = shifter.process(&input, 3.0);
    assert_eq!(output.len(), input.len());
}
