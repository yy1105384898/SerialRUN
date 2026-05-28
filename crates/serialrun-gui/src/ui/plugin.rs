use crate::state::{AppState, T};
use eframe::egui;

pub fn render_plugin_panel(ui: &mut egui::Ui, state: &mut AppState) {
    let lang = state.language;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(T::plugins(lang)).strong());
        ui.separator();
        if ui.button("Refresh").clicked() {
            discover_plugins(state);
        }
    });
    ui.add_space(4.0);

    ui.label("Place .dylib/.so/.dll plugins in the plugins/ directory next to the executable.");
    ui.add_space(4.0);

    if state.plugins.is_empty() {
        ui.label(egui::RichText::new("No plugins found.").weak());
    } else {
        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            for plugin in &state.plugins {
                ui.horizontal(|ui| {
                    let status_color = if plugin.loaded { egui::Color32::GREEN } else { egui::Color32::GRAY };
                    ui.label(egui::RichText::new(if plugin.loaded { "\u{25CF}" } else { "\u{25CB}" }).color(status_color));
                    ui.label(egui::RichText::new(&plugin.name).strong());
                    ui.label(egui::RichText::new(format!("v{}", plugin.version)).weak());
                    ui.label(egui::RichText::new(&plugin.author).weak());
                });
                ui.separator();
            }
        });
    }
}

fn discover_plugins(state: &mut AppState) {
    state.plugins.clear();

    let plugins_dir = if let Ok(exe_path) = std::env::current_exe() {
        exe_path.parent().map(|d| d.join("plugins")).unwrap_or_default()
    } else {
        std::path::PathBuf::from("plugins")
    };

    if !plugins_dir.exists() {
        state.add_log_entry(crate::state::LogLevel::Info, "No plugins/ directory found");
        return;
    }

    let entries: Vec<_> = match std::fs::read_dir(&plugins_dir) {
        Ok(e) => e.flatten().collect(),
        Err(e) => {
            state.add_log_entry(crate::state::LogLevel::Error, &format!("Failed to read plugins/: {}", e));
            return;
        }
    };

    for entry in entries {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let valid_ext = match std::env::consts::OS {
            "macos" => ext == "dylib",
            "windows" => ext == "dll",
            _ => ext == "so",
        };
        if !valid_ext {
            continue;
        }

        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

        // Try to load the plugin and get real metadata
        match serialrun_core::plugin::LoadedPlugin::load(&path) {
            Ok(loaded) => {
                let info = loaded.info();
                state.plugins.push(crate::state::PluginInfo {
                    name: info.name.clone(),
                    version: info.version.clone(),
                    author: info.author.clone(),
                    loaded: true,
                });
                state.add_log_entry(crate::state::LogLevel::Info, &format!("Loaded plugin: {} v{}", info.name, info.version));
                // Keep the loaded plugin alive by leaking it (simple approach)
                // In production, you'd store it in a Vec<LoadedPlugin> on state
                std::mem::forget(loaded);
            }
            Err(e) => {
                state.plugins.push(crate::state::PluginInfo {
                    name,
                    version: "?".into(),
                    author: "?".into(),
                    loaded: false,
                });
                state.add_log_entry(crate::state::LogLevel::Warning, &format!("Plugin load failed: {}", e));
            }
        }
    }

    if state.plugins.is_empty() {
        state.add_log_entry(crate::state::LogLevel::Info, "No plugins found in plugins/ directory");
    } else {
        state.add_log_entry(crate::state::LogLevel::Info, &format!("Found {} plugin(s)", state.plugins.len()));
    }
}
