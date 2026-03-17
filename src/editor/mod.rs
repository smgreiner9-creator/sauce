pub mod widgets;

use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui};
use nih_plug_egui::egui::Color32;
use std::sync::Arc;
use atomic_float::AtomicF32;

use crate::{SauceParams, MusicalKey, Scale};

const BG_COLOR: Color32 = Color32::from_rgb(10, 10, 15);
const PURPLE: Color32 = Color32::from_rgb(108, 43, 238);
const CYAN: Color32 = Color32::from_rgb(0, 240, 255);

pub fn create(
    params: Arc<SauceParams>,
    detected_pitch: Arc<AtomicF32>,
    target_pitch: Arc<AtomicF32>,
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |egui_ctx, _| {
            let mut style = (*egui_ctx.style()).clone();
            style.visuals.dark_mode = true;
            style.visuals.panel_fill = BG_COLOR;
            style.visuals.window_fill = BG_COLOR;
            style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(18, 18, 28);
            style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(28, 28, 42);
            style.visuals.widgets.active.bg_fill = PURPLE;
            style.visuals.selection.bg_fill = PURPLE;
            egui_ctx.set_style(style);
        },
        move |egui_ctx, setter, _state| {
            let params = &params;
            let det_freq = detected_pitch.load(std::sync::atomic::Ordering::Relaxed);
            let tgt_freq = target_pitch.load(std::sync::atomic::Ordering::Relaxed);

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(BG_COLOR))
                .show(egui_ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(12.0);
                        draw_title(ui);
                        ui.add_space(16.0);

                        // Key and Scale dropdowns
                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() * 0.15);

                            let mut key = params.key.value();
                            let key_options: Vec<(MusicalKey, &str)> = vec![
                                (MusicalKey::C, "C"), (MusicalKey::CSharp, "C#"),
                                (MusicalKey::D, "D"), (MusicalKey::DSharp, "D#"),
                                (MusicalKey::E, "E"), (MusicalKey::F, "F"),
                                (MusicalKey::FSharp, "F#"), (MusicalKey::G, "G"),
                                (MusicalKey::GSharp, "G#"), (MusicalKey::A, "A"),
                                (MusicalKey::ASharp, "A#"), (MusicalKey::B, "B"),
                            ];
                            if widgets::neon_dropdown(ui, "KEY", &mut key, &key_options) {
                                setter.begin_set_parameter(&params.key);
                                setter.set_parameter(&params.key, key);
                                setter.end_set_parameter(&params.key);
                            }

                            ui.add_space(24.0);

                            let mut scale = params.scale.value();
                            let scale_options: Vec<(Scale, &str)> = vec![
                                (Scale::Chromatic, "Chromatic"),
                                (Scale::Major, "Major"),
                                (Scale::Minor, "Minor"),
                            ];
                            if widgets::neon_dropdown(ui, "SCALE", &mut scale, &scale_options) {
                                setter.begin_set_parameter(&params.scale);
                                setter.set_parameter(&params.scale, scale);
                                setter.end_set_parameter(&params.scale);
                            }
                        });

                        ui.add_space(20.0);

                        // Four knobs
                        ui.horizontal(|ui| {
                            let spacing = (ui.available_width() - 4.0 * 64.0) / 5.0;
                            ui.add_space(spacing);

                            let mut ig = util::gain_to_db(params.input_gain.value());
                            if widgets::neon_knob(ui, "INPUT", &mut ig, -24.0..=24.0, &|v| format!("{v:.1} dB")) {
                                setter.begin_set_parameter(&params.input_gain);
                                setter.set_parameter(&params.input_gain, util::db_to_gain(ig));
                                setter.end_set_parameter(&params.input_gain);
                            }
                            ui.add_space(spacing);

                            let mut fs = params.formant_shift.value();
                            if widgets::neon_knob(ui, "FORMANT", &mut fs, -12.0..=12.0, &|v| format!("{v:.1} st")) {
                                setter.begin_set_parameter(&params.formant_shift);
                                setter.set_parameter(&params.formant_shift, fs);
                                setter.end_set_parameter(&params.formant_shift);
                            }
                            ui.add_space(spacing);

                            let mut dw = params.dry_wet.value();
                            if widgets::neon_knob(ui, "DRY/WET", &mut dw, 0.0..=1.0, &|v| format!("{:.0}%", v * 100.0)) {
                                setter.begin_set_parameter(&params.dry_wet);
                                setter.set_parameter(&params.dry_wet, dw);
                                setter.end_set_parameter(&params.dry_wet);
                            }
                            ui.add_space(spacing);

                            let mut og = util::gain_to_db(params.output_gain.value());
                            if widgets::neon_knob(ui, "OUTPUT", &mut og, -24.0..=24.0, &|v| format!("{v:.1} dB")) {
                                setter.begin_set_parameter(&params.output_gain);
                                setter.set_parameter(&params.output_gain, util::db_to_gain(og));
                                setter.end_set_parameter(&params.output_gain);
                            }
                        });

                        ui.add_space(16.0);
                        widgets::pitch_meter(ui, det_freq, tgt_freq);
                        ui.add_space(8.0);
                    });
                });
        },
    )
}

fn draw_title(ui: &mut egui::Ui) {
    let painter = ui.painter();
    let center_x = ui.available_rect_before_wrap().center().x;
    let y = ui.cursor().top();
    let font = egui::FontId::proportional(72.0);
    let text = "SAUCE";

    // 3D purple extrusion
    for i in (1..=4).rev() {
        let offset = i as f32 * 1.5;
        painter.text(
            egui::Pos2::new(center_x + offset, y + offset),
            egui::Align2::CENTER_TOP, text, font.clone(),
            Color32::from_rgba_premultiplied(PURPLE.r(), PURPLE.g(), PURPLE.b(), (180 - i * 30) as u8),
        );
    }

    // Cyan glow bloom
    for spread in [3.0, 2.0, 1.0] {
        for dx in [-spread, 0.0, spread] {
            for dy in [-spread, 0.0, spread] {
                painter.text(
                    egui::Pos2::new(center_x + dx, y + dy),
                    egui::Align2::CENTER_TOP, text, font.clone(),
                    Color32::from_rgba_premultiplied(CYAN.r(), CYAN.g(), CYAN.b(), 30),
                );
            }
        }
    }

    // Main white text
    painter.text(
        egui::Pos2::new(center_x, y),
        egui::Align2::CENTER_TOP, text, font,
        Color32::from_rgb(230, 245, 255),
    );

    ui.add_space(80.0);
}
