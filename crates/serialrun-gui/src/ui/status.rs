use crate::state::{AppState, Language, T};
use crate::theme;
use eframe::egui;

/// Logo green — consistent across status bar, MCP, and other indicators
pub const LOGO_GREEN: egui::Color32 = egui::Color32::from_rgb(76, 175, 80);

pub fn render_status_bar(ui: &mut egui::Ui, state: &mut AppState) {
    let lang = state.language;
    let c = theme::get_colors(state.theme);

    // Auto-expire error display
    state.clear_error_if_expired();

    ui.horizontal(|ui| {
        let status_color = if state.is_connected { c.success } else { c.error };

        let status_text = if state.is_connected {
            format!("{}: {}", T::connected(lang), state.selected_port.as_deref().unwrap_or("N/A"))
        } else {
            T::disconnected(lang).to_string()
        };

        ui.label(egui::RichText::new(status_text).color(status_color));

        ui.separator();

        ui.label(egui::RichText::new(format!("{}: {}", T::baud_rate(lang), state.config.baud_rate)).color(c.text_secondary));

        ui.separator();

        ui.label(egui::RichText::new(format!("RX: {} {}", state.rx_count, T::bytes(lang))).color(c.text_secondary));
        ui.label(egui::RichText::new(format!("TX: {} {}", state.tx_count, T::bytes(lang))).color(c.text_secondary));

        if state.recording {
            ui.separator();
            ui.label(egui::RichText::new(format!("● {}", T::recording(lang))).color(c.error));
        }

        // Show current error/warning message inline (red for errors)
        if let Some(ref err) = state.global_error {
            ui.separator();
            ui.label(egui::RichText::new("\u{2716}").color(c.error).size(13.0));
            ui.label(egui::RichText::new(err.as_str()).color(c.error).strong());
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Version
            ui.label(egui::RichText::new("SerialRUN v0.1.0").color(c.text_muted));

            // Warning history: red dot + count
            let warning_count = state.warning_history.len();
            if warning_count > 0 {
                ui.separator();
                let dot_label = format!("\u{25CF} {}", warning_count);
                if ui.add(egui::Button::new(
                    egui::RichText::new(&dot_label).color(c.error).strong().size(12.0)
                ).fill(egui::Color32::TRANSPARENT).frame(false)).on_hover_text(
                    if lang == Language::Chinese { "点击查看警告/错误历史" } else { "Click to view warning/error history" }
                ).clicked() {
                    state.show_warning_popup = !state.show_warning_popup;
                }
            }
        });
    });

    // Warning history popup window
    if state.show_warning_popup {
        let popup_title = if lang == Language::Chinese { "⚠ 警告 / 错误历史" } else { "⚠ Warning / Error History" };
        let mut open = state.show_warning_popup;
        egui::Window::new(popup_title)
            .open(&mut open)
            .default_width(420.0)
            .default_height(350.0)
            .resizable(true)
            .show(ui.ctx(), |ui| {
                let warning_count = state.warning_history.len();
                ui.label(egui::RichText::new(
                    format!("{} {}", warning_count, if lang == Language::Chinese { "条记录" } else { "entries" })
                ).color(c.text_muted));
                ui.separator();

                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    for entry in state.warning_history.iter().rev().take(50) {
                        let ts = chrono::DateTime::from_timestamp_millis(entry.timestamp)
                            .map(|t| t.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S%.3f").to_string())
                            .unwrap_or_default();
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new(format!("[{}]", ts)).color(c.timestamp_color).monospace());
                            ui.label(egui::RichText::new(&entry.message).color(c.error));
                        });
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new(
                        if lang == Language::Chinese { "清空历史" } else { "Clear History" }
                    ).color(c.error)).clicked() {
                        state.warning_history.clear();
                        state.save_warnings();
                        state.show_warning_popup = false;
                    }
                });
            });
        state.show_warning_popup = open;
    }
}
