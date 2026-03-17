use sauce::dsp::note_snap::{freq_to_midi, midi_to_freq, snap_to_scale, ScaleType};

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
