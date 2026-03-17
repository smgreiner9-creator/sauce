use sauce::dsp::yin::PitchDetector;

fn generate_sine(freq: f32, sample_rate: f32, num_samples: usize) -> Vec<f32> {
    (0..num_samples)
        .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate).sin())
        .collect()
}

#[test]
fn test_detect_a4() {
    let sr = 44100.0;
    let mut detector = PitchDetector::new(sr);
    let signal = generate_sine(440.0, sr, 4096);
    for &s in &signal { detector.push_sample(s); }
    let result = detector.detect();
    assert!(result.is_some(), "Should detect pitch");
    let freq = result.unwrap();
    assert!((freq - 440.0).abs() < 5.0, "Expected ~440 Hz, got {freq}");
}

#[test]
fn test_detect_low_e() {
    let sr = 44100.0;
    let mut detector = PitchDetector::new(sr);
    let signal = generate_sine(82.0, sr, 4096);
    for &s in &signal { detector.push_sample(s); }
    let result = detector.detect();
    assert!(result.is_some(), "Should detect low pitch");
    let freq = result.unwrap();
    assert!((freq - 82.0).abs() < 3.0, "Expected ~82 Hz, got {freq}");
}

#[test]
fn test_silence_returns_none() {
    let sr = 44100.0;
    let mut detector = PitchDetector::new(sr);
    for _ in 0..4096 { detector.push_sample(0.0); }
    let result = detector.detect();
    assert!(result.is_none(), "Silence should return None");
}
