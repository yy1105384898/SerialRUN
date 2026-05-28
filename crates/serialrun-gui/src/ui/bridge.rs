use crate::state::{AppState, T};
use eframe::egui;

pub fn render_bridge_panel(ui: &mut egui::Ui, state: &mut AppState) {
    let lang = state.language;

    // Poll bridge log and errors
    poll_bridge_logs(state);

    ui.label(T::tcp_port(lang));
    ui.add(egui::DragValue::new(&mut state.bridge.tcp_port).range(1..=65535));
    ui.add_space(4.0);

    ui.label(T::serial_port(lang));
    egui::ComboBox::from_id_salt("bridge_port")
        .selected_text(if state.bridge.serial_port_name.is_empty() { "--" } else { &state.bridge.serial_port_name })
        .show_ui(ui, |ui| {
            for p in &state.ports {
                ui.selectable_value(&mut state.bridge.serial_port_name, p.name.clone(), &p.name);
            }
        });
    ui.add_space(4.0);

    ui.label(T::baud_rate(lang));
    ui.add(egui::DragValue::new(&mut state.bridge.baud_rate).range(300..=4000000));
    ui.add_space(4.0);

    ui.label(T::timeout_ms(lang));
    ui.add(egui::DragValue::new(&mut state.bridge.timeout_ms).range(100..=5000));
    ui.add_space(8.0);

    // Start/Stop button
    if state.bridge.running {
        if ui.button(T::stop_bridge(lang)).clicked() {
            if let Some(stop) = state.bridge_stop.take() {
                stop.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            state.bridge.running = false;
        }
    } else {
        if ui.button(T::start_bridge(lang)).clicked() {
            let config = serialrun_core::protocol::BridgeConfig {
                tcp_port: state.bridge.tcp_port,
                serial_port_name: state.bridge.serial_port_name.clone(),
                baud_rate: state.bridge.baud_rate,
                timeout_ms: state.bridge.timeout_ms,
            };
            match serialrun_core::protocol::start_bridge(config) {
                Ok((stop, log_rx, err_rx)) => {
                    state.bridge_stop = Some(stop);
                    state.bridge_log_rx = Some(log_rx);
                    state.bridge_err_rx = Some(err_rx);
                    state.bridge.running = true;
                    state.bridge.log.clear();
                }
                Err(e) => {
                    state.bridge.status_msg = Some(e);
                }
            }
        }
    }

    if let Some(ref msg) = state.bridge.status_msg {
        ui.colored_label(egui::Color32::YELLOW, msg.as_str());
    }

    // Log
    ui.separator();
    ui.label(T::bridge_log(lang));
    egui::ScrollArea::vertical().max_height(150.0).stick_to_bottom(true).show(ui, |ui| {
        for entry in state.bridge.log.iter().rev() {
            let ts = chrono::DateTime::from_timestamp_millis(entry.timestamp)
                .map(|t| t.with_timezone(&chrono::Local).format("%H:%M:%S%.3f").to_string())
                .unwrap_or_default();
            let color = if entry.success { egui::Color32::GREEN } else { egui::Color32::RED };
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("[{}]", ts)).weak());
                ui.label(egui::RichText::new(&entry.direction).color(color).monospace());
                ui.label(egui::RichText::new(&entry.client_addr).weak());
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("TX:").weak().monospace());
                ui.label(egui::RichText::new(&entry.request_hex).monospace());
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("RX:").weak().monospace());
                ui.label(egui::RichText::new(&entry.response_hex).monospace().color(color));
            });
        }
    });

    if ui.button(T::clear(lang)).clicked() {
        state.bridge.log.clear();
    }
}

fn poll_bridge_logs(state: &mut AppState) {
    // Poll errors
    if let Some(rx) = &state.bridge_err_rx {
        while let Ok(msg) = rx.try_recv() {
            state.bridge.status_msg = Some(msg);
        }
    }
    // Poll log
    if let Some(rx) = &state.bridge_log_rx {
        while let Ok(entry) = rx.try_recv() {
            state.bridge.log.push_back(crate::state::BridgeLogEntry {
                timestamp: entry.timestamp,
                client_addr: entry.client_addr,
                direction: entry.direction,
                request_hex: entry.request_hex,
                response_hex: entry.response_hex,
                success: entry.success,
            });
            if state.bridge.log.len() > 200 {
                state.bridge.log.pop_front();
            }
        }
    }
    // Check if stop was triggered
    if state.bridge.running && state.bridge_stop.is_none() {
        state.bridge.running = false;
    }
}
