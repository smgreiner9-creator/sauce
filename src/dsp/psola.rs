/// TD-PSOLA (Time-Domain Pitch Synchronous Overlap and Add) pitch shifter.

pub struct PsolaShifter {
    sample_rate: f32,
}

impl PsolaShifter {
    pub fn new(sample_rate: f32) -> Self {
        Self { sample_rate }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub fn process(&mut self, input: &[f32], source_freq: f32, target_freq: f32) -> Vec<f32> {
        let len = input.len();
        if len == 0 || source_freq <= 0.0 || target_freq <= 0.0 {
            return input.to_vec();
        }

        let source_period = self.sample_rate / source_freq;
        let target_period = self.sample_rate / target_freq;
        let ratio = source_freq / target_freq; // >1 for pitch down, <1 for pitch up

        let source_marks = self.find_pitch_marks(input, source_period);
        if source_marks.len() < 2 {
            return input.to_vec();
        }

        let target_marks = self.generate_target_marks(len, target_period);

        let mut output = vec![0.0f32; len];
        let mut window_sum = vec![0.0f32; len];
        // Use target period as grain radius for proper overlap-add at new pitch
        let radius = target_period as usize;
        if radius < 2 {
            return input.to_vec();
        }

        // Map each target mark to a source mark using phase-locked sequential mapping
        // Target mark k at position t_k maps to source position t_k * ratio
        // Then find the nearest source mark to that position
        for &target_mark in &target_marks {
            let mapped_pos = (target_mark as f32 * ratio) as usize;
            let source_mark = self.nearest_source_mark(&source_marks, mapped_pos);

            for i in 0..(2 * radius) {
                let src_idx = source_mark as isize - radius as isize + i as isize;
                let dst_idx = target_mark as isize - radius as isize + i as isize;

                if src_idx < 0 || src_idx >= len as isize || dst_idx < 0 || dst_idx >= len as isize {
                    continue;
                }

                let t = i as f32 / (2 * radius) as f32;
                let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * t).cos());

                output[dst_idx as usize] += input[src_idx as usize] * window;
                window_sum[dst_idx as usize] += window;
            }
        }

        for i in 0..len {
            if window_sum[i] > 1e-6 {
                output[i] /= window_sum[i];
            }
        }

        output
    }

    fn find_pitch_marks(&self, signal: &[f32], period: f32) -> Vec<usize> {
        let period_usize = period as usize;
        if period_usize < 2 || signal.is_empty() { return vec![]; }

        let mut marks = Vec::new();
        let search_end = (period_usize * 2).min(signal.len());
        if search_end == 0 { return vec![]; }

        let first_peak = signal[..search_end]
            .iter().enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i).unwrap_or(0);
        marks.push(first_peak);

        loop {
            let expected = *marks.last().unwrap() + period_usize;
            if expected >= signal.len() { break; }

            let margin = period_usize / 4;
            let search_start = expected.saturating_sub(margin);
            let search_end = (expected + margin + 1).min(signal.len());
            if search_start >= search_end || search_start >= signal.len() { break; }

            let peak = signal[search_start..search_end]
                .iter().enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(i, _)| search_start + i).unwrap_or(expected);

            marks.push(peak);
        }

        marks
    }

    fn generate_target_marks(&self, len: usize, target_period: f32) -> Vec<usize> {
        let mut marks = Vec::new();
        let mut pos = target_period;
        while (pos as usize) < len {
            marks.push(pos as usize);
            pos += target_period;
        }
        marks
    }

    fn nearest_source_mark(&self, source_marks: &[usize], target_pos: usize) -> usize {
        source_marks.iter()
            .min_by_key(|&&m| (m as isize - target_pos as isize).unsigned_abs())
            .copied().unwrap_or(target_pos)
    }

    pub fn reset(&mut self) {}
}
