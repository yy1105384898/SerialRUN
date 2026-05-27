use crate::app::apply_theme;
use crate::state::{AppState, Language, T, Theme};
use eframe::egui;
use serialtap_core::SerialConfig;

pub fn render_connection_panel(ui: &mut egui::Ui, state: &mut AppState, ctx: &egui::Context) {
    let lang = state.language;

    ui.horizontal(|ui| {
        // Logo
        ui.label(
            egui::RichText::new("S")
                .size(20.0)
                .strong()
                .color(egui::Color32::from_rgb(0, 180, 120)),
        );
        ui.label(
            egui::RichText::new("SerialTap")
                .size(16.0)
                .strong(),
        );

        ui.add_space(12.0);

        // Port selector
        ui.label(T::port(lang));
        let port_names: Vec<String> = state.ports.iter().map(|p| p.name.clone()).collect();
        let selected = state.selected_port.clone().unwrap_or_default();
        egui::ComboBox::from_id_salt("port_select")
            .width(130.0)
            .selected_text(if selected.is_empty() { "—" } else { &selected })
            .show_ui(ui, |ui| {
                for name in &port_names {
                    ui.selectable_value(&mut state.selected_port, Some(name.clone()), name);
                }
            });

        if ui.small_button(T::refresh_ports(lang)).clicked() {
            state.refresh_ports();
        }

        ui.add_space(8.0);

        // Baud rate
        let baud_rates = [9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600];
        egui::ComboBox::from_id_salt("baud_rate")
            .width(90.0)
            .selected_text(format!("{}", state.config.baud_rate))
            .show_ui(ui, |ui| {
                for &rate in &baud_rates {
                    ui.selectable_value(&mut state.config.baud_rate, rate, format!("{}", rate));
                }
            });

        ui.add_space(8.0);

        // Connect / Disconnect button
        if state.is_connected {
            if ui
                .button(egui::RichText::new(T::disconnect(lang)).color(egui::Color32::from_rgb(220, 60, 60)))
                .clicked()
            {
                if let Some(mut port) = state.port.take() {
                    let _ = port.disconnect();
                }
                state.is_connected = false;
                state.add_log_entry(crate::state::LogLevel::Info, "Disconnected");
            }
        } else if ui
            .button(egui::RichText::new(T::connect(lang)).color(egui::Color32::from_rgb(0, 180, 120)))
            .clicked()
        {
            if let Some(ref port_name) = state.selected_port {
                let config =
                    SerialConfig::new(port_name).with_baud_rate(state.config.baud_rate);
                let mut port = serialtap_core::SerialPort::new(config);
                match port.connect() {
                    Ok(()) => {
                        state.is_connected = true;
                        state.port = Some(port);
                        state.add_log_entry(
                            crate::state::LogLevel::Info,
                            &format!("Connected to {}", port_name),
                        );
                    }
                    Err(e) => {
                        state.add_log_entry(crate::state::LogLevel::Error, &e.to_string());
                    }
                }
            }
        }

        // Right side: buttons
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Help
            if ui.small_button("?").clicked() {
                state.show_help = !state.show_help;
            }

            // Theme toggle
            let theme_label = match state.theme {
                Theme::Dark => "\u{263E}", // moon
                Theme::Light => "\u{2600}", // sun
            };
            if ui.small_button(theme_label).clicked() {
                state.theme = match state.theme {
                    Theme::Dark => Theme::Light,
                    Theme::Light => Theme::Dark,
                };
                apply_theme(ctx, state.theme);
            }

            // Language toggle
            let lang_label = match state.language {
                Language::English => "EN",
                Language::Chinese => "中",
            };
            if ui.small_button(lang_label).clicked() {
                state.language = match state.language {
                    Language::English => Language::Chinese,
                    Language::Chinese => Language::English,
                };
            }

            // Chart / Log toggle
            if ui.small_button(T::log(lang)).clicked() {
                state.show_log_window = !state.show_log_window;
            }
            if ui.small_button(T::chart(lang)).clicked() {
                state.show_chart_window = !state.show_chart_window;
            }
        });
    });
}
