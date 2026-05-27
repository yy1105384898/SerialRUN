use crate::state::{AppState, LogLevel, T};
use eframe::egui;

pub fn render_log_panel(ui: &mut egui::Ui, state: &AppState) {
    let lang = state.language;
    ui.horizontal(|ui| {
        ui.label(T::log_viewer(lang));
        ui.separator();
        ui.label(format!("Entries: {}", state.log_entries.len()));
    });

    ui.separator();

    let available_height = ui.available_height() - 40.0;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .max_height(available_height)
        .show(ui, |ui| {
            for entry in &state.log_entries {
                let color = match entry.level {
                    LogLevel::Info => egui::Color32::WHITE,
                    LogLevel::Warning => egui::Color32::YELLOW,
                    LogLevel::Error => egui::Color32::RED,
                };

                let level_str = match entry.level {
                    LogLevel::Info => "INFO",
                    LogLevel::Warning => "WARN",
                    LogLevel::Error => "ERR ",
                };

                let timestamp = chrono::DateTime::from_timestamp_millis(entry.timestamp)
                    .map(|t| t.format("%H:%M:%S%.3f").to_string())
                    .unwrap_or_default();

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("[{}]", timestamp)).weak());
                    ui.label(egui::RichText::new(level_str).color(color).strong());
                    ui.label(&entry.message);
                });
            }
        });

    ui.separator();

    ui.horizontal(|ui| {
        if ui.button(T::clear_logs(lang)).clicked() {
            // Note: This requires mutable access to state
            // In a real implementation, use a message passing system
        }

        if ui.button(T::export_logs(lang)).clicked() {
            // Export logs to file
        }
    });
}
