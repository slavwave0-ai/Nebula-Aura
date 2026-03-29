use std::sync::{Arc, Mutex};

use atomic_float::AtomicF32;
use egui::{
    epaint::Rgba, pos2, vec2, Align2, Color32, ComboBox, FontId, Frame, RichText, Sense, Stroke,
    TextEdit,
};
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiState};

use crate::{dsp::SpectrumFrame, NebulaAura, NebulaAuraParams};

pub fn create_editor(
    params: Arc<NebulaAuraParams>,
    spectrum: Arc<Mutex<SpectrumFrame>>,
    gain_reduction: Arc<AtomicF32>,
    _executor: AsyncExecutor<NebulaAura>,
    is_a: bool,
    midi_enabled: bool,
    preset_names: Vec<String>,
) -> Box<dyn Editor> {
    let mut selected_preset = String::new();
    let mut scale: f32 = 1.0;

    create_egui_editor(
        params.editor_state.clone(),
        (),
        |_, _| {},
        move |egui_ctx, _setter, _| {
            egui_ctx.set_pixels_per_point(scale.clamp(0.5, 2.5));
            Frame::none()
                .fill(Color32::from_rgb(2, 4, 18))
                .stroke(Stroke::new(1.0, Color32::from_rgb(0, 255, 255)))
                .show(egui_ctx, |ui| {
                    let rect = ui.max_rect();
                    paint_background(ui, rect, egui_ctx.input(|i| i.time) as f32);

                    ui.horizontal(|ui| {
                        ui.heading(
                            RichText::new("NEBULA AURA").color(Color32::from_rgb(0, 255, 255)),
                        );
                        ui.label(RichText::new("v1.0").color(Color32::from_rgb(255, 0, 255)));
                        ui.label(
                            RichText::new("Made by Nebula Audio")
                                .color(Color32::from_rgb(255, 255, 0)),
                        );
                        ui.add(egui::Slider::new(&mut scale, 0.5..=2.5).text("UI Scale"));
                    });

                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        knob_with_field(ui, "Input", &params.input_level, "dB");
                        knob_with_field(ui, "Output", &params.output_level, "dB");
                        knob_with_field(ui, "Harm", &params.harmonics, "%");
                        knob_with_field(ui, "Freq", &params.frequency, "Hz");
                        knob_with_field(ui, "Slope", &params.slope, "dB/oct");
                        knob_with_field(ui, "Odd", &params.odd, "dB");
                        knob_with_field(ui, "Even", &params.even, "dB");
                        knob_with_field(ui, "Mix", &params.mix, "%");
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        ComboBox::from_label("Oversampling")
                            .selected_text(format!("{:?}", params.oversampling.value()))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut params.oversampling,
                                    crate::OversamplingMode::Off,
                                    "Off",
                                );
                                ui.selectable_value(
                                    &mut params.oversampling,
                                    crate::OversamplingMode::X2,
                                    "2x",
                                );
                                ui.selectable_value(
                                    &mut params.oversampling,
                                    crate::OversamplingMode::X4,
                                    "4x",
                                );
                                ui.selectable_value(
                                    &mut params.oversampling,
                                    crate::OversamplingMode::X6,
                                    "6x",
                                );
                                ui.selectable_value(
                                    &mut params.oversampling,
                                    crate::OversamplingMode::X8,
                                    "8x",
                                );
                            });

                        ui.checkbox(&mut params.phase_flip, "Phase");
                        ui.checkbox(&mut params.fx_bypass, "FX Bypass");
                        ui.label(if is_a { "A/B: A" } else { "A/B: B" });
                        ui.label(if midi_enabled {
                            "MIDI Learn: On"
                        } else {
                            "MIDI Learn: Off"
                        });
                    });

                    ui.horizontal(|ui| {
                        ComboBox::from_label("Presets")
                            .selected_text(if selected_preset.is_empty() {
                                "Select preset".to_owned()
                            } else {
                                selected_preset.clone()
                            })
                            .show_ui(ui, |ui| {
                                for name in &preset_names {
                                    ui.selectable_value(&mut selected_preset, name.clone(), name);
                                }
                            });
                        ui.add(TextEdit::singleline(&mut selected_preset).hint_text("Preset name"));
                    });

                    ui.separator();
                    paint_spectrum(ui, &spectrum);

                    let gr = gain_reduction.load(std::sync::atomic::Ordering::Relaxed);
                    ui.label(
                        RichText::new(format!("Gain reduction: {gr:.1} dB"))
                            .color(Color32::from_rgb(255, 0, 255)),
                    );
                });
        },
    )
}

fn knob_with_field(ui: &mut egui::Ui, title: &str, param: &FloatParam, unit: &str) {
    ui.vertical(|ui| {
        ui.label(RichText::new(title).color(Color32::from_rgb(0, 255, 255)));
        let mut value = param.value();
        let response = ui.add(egui::Slider::new(
            &mut value,
            param.min_value()..=param.max_value(),
        ));
        if response.changed() {
            param.set_plain_value(value);
        }
        let mut text = format!("{:.2}", param.value());
        let field = ui.add_sized([90.0, 24.0], TextEdit::singleline(&mut text));
        field.context_menu(|ui| {
            if ui.button("Enter numeric value").clicked() {
                if let Ok(parsed) = text.parse::<f32>() {
                    param.set_plain_value(parsed.clamp(param.min_value(), param.max_value()));
                }
                ui.close_menu();
            }
        });
        ui.label(
            RichText::new(unit)
                .small()
                .color(Color32::from_rgb(255, 255, 0)),
        );
    });
}

fn paint_background(ui: &mut egui::Ui, rect: egui::Rect, t: f32) {
    let painter = ui.painter_at(rect);
    let grid = 24.0;
    for x in (0..=((rect.width() / grid) as i32)).map(|n| rect.left() + n as f32 * grid) {
        painter.line_segment(
            [pos2(x, rect.top()), pos2(x, rect.bottom())],
            Stroke::new(0.5, Color32::from_rgba_premultiplied(0, 140, 255, 32)),
        );
    }
    for y in (0..=((rect.height() / grid) as i32)).map(|n| rect.top() + n as f32 * grid) {
        painter.line_segment(
            [pos2(rect.left(), y), pos2(rect.right(), y)],
            Stroke::new(0.5, Color32::from_rgba_premultiplied(255, 0, 255, 28)),
        );
    }
    let scan_y = rect.top() + ((t * 120.0) % rect.height().max(1.0));
    painter.line_segment(
        [pos2(rect.left(), scan_y), pos2(rect.right(), scan_y)],
        Stroke::new(1.5, Color32::from_rgba_premultiplied(255, 255, 0, 56)),
    );
}

fn paint_spectrum(ui: &mut egui::Ui, spectrum: &Arc<Mutex<SpectrumFrame>>) {
    let (response, painter) =
        ui.allocate_painter(vec2(ui.available_width(), 220.0), Sense::hover());
    let rect = response.rect;
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_rgb(0, 255, 255)));

    if let Ok(frame) = spectrum.lock() {
        if frame.bins_hz.len() > 2 {
            let points: Vec<_> = frame
                .bins_hz
                .iter()
                .zip(&frame.magnitudes_db)
                .filter_map(|(hz, db)| {
                    let xnorm = ((*hz).max(20.0).log10() - 20.0f32.log10())
                        / (20_000.0f32.log10() - 20.0f32.log10());
                    if (0.0..=1.0).contains(&xnorm) {
                        let ynorm = ((*db + 96.0) / 96.0).clamp(0.0, 1.0);
                        Some(pos2(
                            egui::lerp(rect.left()..=rect.right(), xnorm),
                            egui::lerp(rect.bottom()..=rect.top(), ynorm),
                        ))
                    } else {
                        None
                    }
                })
                .collect();
            if points.len() > 1 {
                painter.add(egui::Shape::line(
                    points,
                    Stroke::new(1.5, Color32::from_rgb(80, 240, 255)),
                ));
            }
        }
    }

    painter.text(
        rect.left_top() + vec2(12.0, 10.0),
        Align2::LEFT_TOP,
        "Spectrum Analyzer",
        FontId::proportional(13.0),
        Rgba::from_rgb(1.0, 0.0, 1.0).into(),
    );
}
