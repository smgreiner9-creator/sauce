use nih_plug_egui::egui;
use nih_plug_egui::egui::{Color32, Pos2, Stroke, Vec2};

pub const PURPLE: Color32 = Color32::from_rgb(108, 43, 238);
pub const CYAN: Color32 = Color32::from_rgb(0, 240, 255);
pub const BG_DARK: Color32 = Color32::from_rgb(10, 10, 15);
pub const BG_PANEL: Color32 = Color32::from_rgb(18, 18, 28);
pub const TEXT_DIM: Color32 = Color32::from_rgb(140, 140, 160);

pub fn neon_knob(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    format_value: &dyn Fn(f32) -> String,
) -> bool {
    let size = Vec2::new(64.0, 84.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());
    let knob_center = Pos2::new(rect.center().x, rect.top() + 32.0);
    let knob_radius = 24.0;

    let mut changed = false;
    if response.dragged() {
        let delta = -response.drag_delta().y * 0.005;
        let min = *range.start();
        let max = *range.end();
        *value = (*value + delta * (max - min)).clamp(min, max);
        changed = true;
    }

    let painter = ui.painter_at(rect);
    painter.circle_filled(knob_center, knob_radius + 2.0, BG_PANEL);

    let start_angle = std::f32::consts::PI * 0.75;
    let end_angle = std::f32::consts::PI * 2.25;
    draw_arc(&painter, knob_center, knob_radius, start_angle, end_angle, Stroke::new(3.0, Color32::from_rgb(30, 30, 45)));

    let min = *range.start();
    let max = *range.end();
    let normalized = if (max - min).abs() > f32::EPSILON { (*value - min) / (max - min) } else { 0.5 };
    let fill_angle = start_angle + normalized * (end_angle - start_angle);
    draw_arc(&painter, knob_center, knob_radius, start_angle, fill_angle, Stroke::new(3.5, PURPLE));

    let dot_x = knob_center.x + knob_radius * fill_angle.cos();
    let dot_y = knob_center.y + knob_radius * fill_angle.sin();
    painter.circle_filled(Pos2::new(dot_x, dot_y), 4.0, CYAN);

    let value_text = format_value(*value);
    painter.text(Pos2::new(knob_center.x, knob_center.y + 2.0), egui::Align2::CENTER_CENTER, &value_text, egui::FontId::proportional(10.0), CYAN);
    painter.text(Pos2::new(rect.center().x, rect.bottom() - 4.0), egui::Align2::CENTER_BOTTOM, label, egui::FontId::proportional(11.0), TEXT_DIM);

    changed
}

fn draw_arc(painter: &egui::Painter, center: Pos2, radius: f32, start: f32, end: f32, stroke: Stroke) {
    let segments = 32;
    let step = (end - start) / segments as f32;
    let points: Vec<Pos2> = (0..=segments)
        .map(|i| {
            let angle = start + step * i as f32;
            Pos2::new(center.x + radius * angle.cos(), center.y + radius * angle.sin())
        })
        .collect();
    for i in 0..points.len() - 1 {
        painter.line_segment([points[i], points[i + 1]], stroke);
    }
}

pub fn neon_dropdown<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    label: &str,
    current: &mut T,
    options: &[(T, &str)],
) -> bool {
    let mut changed = false;
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(label).color(TEXT_DIM).size(10.0));
        let current_label = options.iter().find(|(v, _)| v == current).map(|(_, l)| *l).unwrap_or("?");
        egui::ComboBox::from_label("")
            .selected_text(egui::RichText::new(current_label).color(CYAN))
            .show_ui(ui, |ui| {
                for (value, name) in options {
                    if ui.selectable_value(current, value.clone(), *name).changed() {
                        changed = true;
                    }
                }
            });
    });
    changed
}

pub fn pitch_meter(ui: &mut egui::Ui, detected_freq: f32, target_freq: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, BG_PANEL);

    if detected_freq > 0.0 && target_freq > 0.0 {
        let detected_name = freq_to_note_name(detected_freq);
        let target_name = freq_to_note_name(target_freq);
        let text = format!("{}  →  {}", detected_name, target_name);
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, &text, egui::FontId::monospace(13.0), CYAN);
    } else {
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, "listening...", egui::FontId::monospace(12.0), Color32::from_rgb(60, 60, 80));
    }
}

fn freq_to_note_name(freq: f32) -> String {
    let midi = 69.0 + 12.0 * (freq / 440.0).log2();
    let note_num = midi.round() as i32;
    let octave = (note_num / 12) - 1;
    let note_idx = ((note_num % 12) + 12) % 12;
    let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    format!("{}{}", names[note_idx as usize], octave)
}
