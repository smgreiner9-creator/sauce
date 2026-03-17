/// Musical note snapping for pitch correction.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleType {
    Chromatic,
    Major,
    Minor,
}

const CHROMATIC: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
const MAJOR: &[u8] = &[0, 2, 4, 5, 7, 9, 11];
const MINOR: &[u8] = &[0, 2, 3, 5, 7, 8, 10];

pub fn freq_to_midi(freq: f32) -> f32 {
    if freq <= 0.0 {
        return f32::NAN;
    }
    69.0 + 12.0 * (freq / 440.0).log2()
}

pub fn midi_to_freq(midi: f32) -> f32 {
    440.0 * 2.0f32.powf((midi - 69.0) / 12.0)
}

pub fn snap_to_scale(midi_note: f32, key_root: u8, scale: ScaleType) -> i32 {
    let offsets = match scale {
        ScaleType::Chromatic => CHROMATIC,
        ScaleType::Major => MAJOR,
        ScaleType::Minor => MINOR,
    };

    let rounded = midi_note.round() as i32;
    let mut best_note = rounded;
    let mut best_dist = f32::MAX;

    for candidate in (rounded - 12)..=(rounded + 12) {
        let degree = ((candidate - key_root as i32) % 12 + 12) % 12;
        if offsets.contains(&(degree as u8)) {
            let dist = (midi_note - candidate as f32).abs();
            if dist < best_dist {
                best_dist = dist;
                best_note = candidate;
            }
        }
    }

    best_note
}

pub fn snap_frequency(freq: f32, key_root: u8, scale: ScaleType) -> Option<f32> {
    if freq < 60.0 || freq > 1200.0 {
        return None;
    }
    let midi = freq_to_midi(freq);
    let target_midi = snap_to_scale(midi, key_root, scale);
    Some(midi_to_freq(target_midi as f32))
}
