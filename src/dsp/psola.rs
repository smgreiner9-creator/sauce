/// TD-PSOLA (Time-Domain Pitch Synchronous Overlap and Add) pitch shifter.
///
/// Maintains cross-block state via an overlap tail buffer so that grains
/// straddling block boundaries are not cut off.

pub struct PsolaShifter {
    sample_rate: f32,
    /// Pre-allocated work buffer for OLA accumulation.
    output_buf: Vec<f32>,
    /// Pre-allocated window-sum buffer for OLA normalisation.
    window_sum_buf: Vec<f32>,
    /// Pre-allocated scratch for source pitch marks.
    source_marks: Vec<usize>,
    /// Pre-allocated scratch for target pitch marks.
    target_marks: Vec<usize>,
    /// Overlap tail carried from previous process() call.
    overlap_tail: Vec<f32>,
    /// How many samples of `overlap_tail` are valid.
    overlap_tail_len: usize,
}

impl PsolaShifter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            output_buf: Vec::new(),
            window_sum_buf: Vec::new(),
            source_marks: Vec::new(),
            target_marks: Vec::new(),
            overlap_tail: Vec::new(),
            overlap_tail_len: 0,
        }
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
        let ratio = (source_freq as f64) / (target_freq as f64);

        // -- Find pitch marks (reuse allocations) --------------------------------
        self.find_pitch_marks(input, source_period);
        if self.source_marks.len() < 2 {
            return input.to_vec();
        }
        self.generate_target_marks(len, target_period);

        let radius = target_period as usize;
        if radius < 2 {
            return input.to_vec();
        }

        // Extended length: grains near the end of the block may extend up to
        // `radius` samples past the block boundary.
        let extended_len = len + radius;

        // Resize work buffers (no realloc if capacity already sufficient).
        self.output_buf.resize(extended_len, 0.0);
        self.output_buf.iter_mut().for_each(|v| *v = 0.0);
        self.window_sum_buf.resize(extended_len, 0.0);
        self.window_sum_buf.iter_mut().for_each(|v| *v = 0.0);

        let grain_len = 2 * radius;
        let hann_denom = (grain_len.max(2) - 1) as f32; // symmetric Hann

        for &target_mark in &self.target_marks {
            // f64 mapped position for precision
            let mapped_pos = ((target_mark as f64) * ratio) as usize;
            let source_mark = Self::nearest_source_mark_static(&self.source_marks, mapped_pos);

            for i in 0..grain_len {
                let src_idx = source_mark as isize - radius as isize + i as isize;
                let dst_idx = target_mark as isize - radius as isize + i as isize;

                if src_idx < 0 || src_idx >= len as isize || dst_idx < 0 || dst_idx >= extended_len as isize {
                    continue;
                }

                let t = i as f32 / hann_denom;
                let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * t).cos());

                self.output_buf[dst_idx as usize] += input[src_idx as usize] * window;
                self.window_sum_buf[dst_idx as usize] += window;
            }
        }

        // Normalise by window sum
        for i in 0..extended_len {
            if self.window_sum_buf[i] > 1e-6 {
                self.output_buf[i] /= self.window_sum_buf[i];
            }
        }

        // -- Cross-block overlap-add -------------------------------------------
        // Add the tail saved from the previous call into the start of this block.
        let tail_to_add = self.overlap_tail_len.min(len);
        for i in 0..tail_to_add {
            self.output_buf[i] += self.overlap_tail[i];
        }

        // Save the new tail (samples beyond `len`) for the next call.
        let new_tail_len = extended_len - len;
        if self.overlap_tail.len() < new_tail_len {
            self.overlap_tail.resize(new_tail_len, 0.0);
        }
        self.overlap_tail[..new_tail_len].copy_from_slice(&self.output_buf[len..extended_len]);

        self.overlap_tail_len = new_tail_len;

        // Return only the block-sized portion.
        self.output_buf[..len].to_vec()
    }

    /// Detect pitch marks using absolute-value peak search.
    fn find_pitch_marks(&mut self, signal: &[f32], period: f32) {
        self.source_marks.clear();
        let period_usize = period as usize;
        if period_usize < 2 || signal.is_empty() {
            return;
        }

        let search_end = (period_usize * 2).min(signal.len());
        if search_end == 0 {
            return;
        }

        // First mark: largest absolute peak in the first two periods.
        let first_peak = signal[..search_end]
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.abs().partial_cmp(&b.abs()).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.source_marks.push(first_peak);

        loop {
            let expected = *self.source_marks.last().unwrap() + period_usize;
            if expected >= signal.len() {
                break;
            }

            let margin = period_usize / 4;
            let search_start = expected.saturating_sub(margin);
            let search_end = (expected + margin + 1).min(signal.len());
            if search_start >= search_end || search_start >= signal.len() {
                break;
            }

            let peak = signal[search_start..search_end]
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.abs().partial_cmp(&b.abs()).unwrap())
                .map(|(i, _)| search_start + i)
                .unwrap_or(expected);

            self.source_marks.push(peak);
        }
    }

    fn generate_target_marks(&mut self, len: usize, target_period: f32) {
        self.target_marks.clear();
        let mut pos = target_period;
        while (pos as usize) < len {
            self.target_marks.push(pos as usize);
            pos += target_period;
        }
    }

    fn nearest_source_mark_static(source_marks: &[usize], target_pos: usize) -> usize {
        source_marks
            .iter()
            .min_by_key(|&&m| (m as isize - target_pos as isize).unsigned_abs())
            .copied()
            .unwrap_or(target_pos)
    }

    pub fn reset(&mut self) {
        self.overlap_tail_len = 0;
        for v in self.overlap_tail.iter_mut() {
            *v = 0.0;
        }
    }
}
