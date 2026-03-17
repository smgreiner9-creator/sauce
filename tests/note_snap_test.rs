use sauce::dsp::note_snap::{freq_to_midi, midi_to_freq, snap_to_scale, correct_pitch, ScaleType};

#[test]
fn test_a4_is_midi_69() {
    let midi = freq_to_midi(440.0);
    assert!((midi - 69.0).abs() < 0.01);
}

#[test]
fn test_midi_69_is_440() {
    let freq = midi_to_freq(69.0);
    assert!((freq - 440.0).abs() < 0.1);
}

#[test]
fn test_roundtrip_freq_midi() {
    for freq in [100.0, 220.0, 440.0, 880.0] {
        let midi = freq_to_midi(freq);
        let back = midi_to_freq(midi);
        assert!((back - freq).abs() < 0.01, "Roundtrip failed for {freq}");
    }
}

#[test]
fn test_chromatic_snaps_to_nearest() {
    let snapped = snap_to_scale(freq_to_midi(450.0), 0, ScaleType::Chromatic);
    assert_eq!(snapped, 69);
    let snapped = snap_to_scale(freq_to_midi(460.0), 0, ScaleType::Chromatic);
    assert_eq!(snapped, 70);
}

#[test]
fn test_c_major_skips_black_keys() {
    let snapped = snap_to_scale(61.0, 0, ScaleType::Major);
    assert!(snapped == 60 || snapped == 62);
    let snapped = snap_to_scale(66.0, 0, ScaleType::Major);
    assert!(snapped == 65 || snapped == 67);
}

#[test]
fn test_key_offset_d_major() {
    let snapped = snap_to_scale(63.0, 2, ScaleType::Major);
    assert!(snapped == 62 || snapped == 64);
}

#[test]
fn test_correct_pitch_full_amount_near_target() {
    // A note very close to A4 (440Hz) should snap almost exactly to A4
    let result = correct_pitch(442.0, 0, ScaleType::Chromatic, 1.0).unwrap();
    assert!((result - 440.0).abs() < 2.0, "Near-target should snap hard, got {result}");
}

#[test]
fn test_correct_pitch_zero_amount_no_correction() {
    // With amount=0, output should equal input
    let result = correct_pitch(445.0, 0, ScaleType::Chromatic, 0.0).unwrap();
    assert!((result - 445.0).abs() < 1.0, "Zero amount should not correct, got {result}");
}

#[test]
fn test_correct_pitch_between_notes_softer() {
    // A note exactly between A4 (440) and A#4 (466.16) should get less correction
    // than a note close to A4
    let midpoint = 453.0; // roughly between A4 and A#4
    let near_a4 = 442.0;

    let mid_result = correct_pitch(midpoint, 0, ScaleType::Chromatic, 1.0).unwrap();
    let near_result = correct_pitch(near_a4, 0, ScaleType::Chromatic, 1.0).unwrap();

    let mid_error = (mid_result - 440.0).abs().min((mid_result - 466.16).abs());
    let near_error = (near_result - 440.0).abs();

    // The midpoint should have MORE residual error than the near-target note
    assert!(mid_error > near_error, "Midpoint should be corrected less aggressively");
}

#[test]
fn test_correct_pitch_out_of_range() {
    assert!(correct_pitch(30.0, 0, ScaleType::Chromatic, 1.0).is_none());
    assert!(correct_pitch(1500.0, 0, ScaleType::Chromatic, 1.0).is_none());
}
