use crate::state::{AppState, LogLevel, T};
use crate::theme;
use eframe::egui;

pub fn render_log_panel(ui: &mut egui::Ui, state: &mut AppState) {
    let lang = state.language;
    let c = theme::get_colors(state.theme);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(T::log_viewer(lang)).strong().color(c.text_primary));
        ui.separator();
        ui.label(egui::RichText::new(format!("Entries: {}", state.log_entries.len())).color(c.text_muted));
    });

    ui.separator();

    let available_height = ui.available_height() - 40.0;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .max_height(available_height)
        .show(ui, |ui| {
            for entry in &state.log_entries {
                let (color, level_str) = match entry.level {
                    LogLevel::Info => (c.log_info, "INFO"),
                    LogLevel::Warning => (c.log_warning, "WARN"),
                    LogLevel::Error => (c.log_error, "ERR "),
                };

                let timestamp = chrono::DateTime::from_timestamp_millis(entry.timestamp)
                    .map(|t| t.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S%.3f").to_string())
                    .unwrap_or_default();

                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(format!("[{}]", timestamp)).color(c.timestamp_color).monospace());
                    ui.label(egui::RichText::new(level_str).color(color).strong());
                    ui.label(egui::RichText::new(&entry.message).color(c.text_primary));
                });
            }
        });

    ui.separator();

    ui.horizontal(|ui| {
        if ui.button(T::clear_logs(lang)).clicked() {
            state.log_entries.clear();
        }

        if ui.button(T::export_logs(lang)).clicked() {
            if let Some(path) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).save_file() {
                let mut content = String::from("timestamp,level,message\n");
                for entry in &state.log_entries {
                    let ts = chrono::DateTime::from_timestamp_millis(entry.timestamp)
                        .map(|t| t.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S%.3f").to_string())
                        .unwrap_or_default();
                    let level = match entry.level {
                        LogLevel::Info => "INFO",
                        LogLevel::Warning => "WARN",
                        LogLevel::Error => "ERROR",
                    };
                    content.push_str(&format!("{},{},\"{}\"\n", ts, level, entry.message.replace('"', "\"\"")));
                }
                if let Err(e) = std::fs::write(&path, content) {
                    state.add_log_entry(LogLevel::Error, &format!("Export failed: {}", e));
                } else {
                    state.add_log_entry(LogLevel::Info, &format!("Logs exported to {}", path.display()));
                }
            }
        }
    });
}
