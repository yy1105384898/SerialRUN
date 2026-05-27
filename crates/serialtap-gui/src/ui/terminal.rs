use crate::state::{AppState, Direction, T};
use eframe::egui;

pub fn render_terminal_panel(ui: &mut egui::Ui, state: &mut AppState) {
    let lang = state.language;
    ui.horizontal(|ui| {
        ui.label(T::terminal(lang));
        ui.separator();
        ui.checkbox(&mut state.hex_mode, "HEX");
        ui.checkbox(&mut state.show_timestamp, T::show_timestamp(lang));
        ui.checkbox(&mut state.auto_scroll, T::auto_scroll(lang));

        if ui.button(T::clear(lang)).clicked() {
            state.terminal_buffer.clear();
        }
    });

    ui.separator();

    let available_height = ui.available_height() - 40.0;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(state.auto_scroll)
        .max_height(available_height)
        .show(ui, |ui| {
            for line in &state.terminal_buffer {
                let (color, prefix) = match line.direction {
                    Direction::Rx => (egui::Color32::GREEN, "RX"),
                    Direction::Tx => (egui::Color32::BLUE, "TX"),
                    Direction::System => (egui::Color32::YELLOW, "SYS"),
                };

                let timestamp = if state.show_timestamp {
                    let time = chrono::DateTime::from_timestamp_millis(line.timestamp)
                        .map(|t| t.format("%H:%M:%S%.3f").to_string())
                        .unwrap_or_default();
                    format!("[{}] ", time)
                } else {
                    String::new()
                };

                let content = if line.is_hex {
                    line.content.clone()
                } else {
                    line.content.replace("\r\n", "↵\n").replace("\r", "↵").replace("\n", "↵\n")
                };

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("{}[{}]", timestamp, prefix)).color(color));
                    ui.label(&content);
                });
            }
        });

    ui.separator();

    ui.horizontal(|ui| {
        let response = ui.text_edit_singleline(&mut state.input_buffer);

        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if !state.input_buffer.is_empty() && state.is_connected {
                let data = state.input_buffer.clone();
                state.input_buffer.clear();

                if let Some(ref mut port) = state.port {
                    let bytes = if state.hex_mode {
                        parse_hex(&data).unwrap_or_default()
                    } else {
                        data.as_bytes().to_vec()
                    };

                    match port.write(&bytes) {
                        Ok(n) => {
                            state.tx_count += n as u64;
                            state.add_terminal_line(Direction::Tx, data, state.hex_mode);
                            state.add_log_entry(crate::state::LogLevel::Info, &format!("Sent {} bytes", n));
                        }
                        Err(e) => {
                            state.add_log_entry(crate::state::LogLevel::Error, &e.to_string());
                        }
                    }
                }
            }
        }

        if ui.button(T::send(lang)).clicked() && !state.input_buffer.is_empty() && state.is_connected {
            let data = state.input_buffer.clone();
            state.input_buffer.clear();

            if let Some(ref mut port) = state.port {
                let bytes = if state.hex_mode {
                    parse_hex(&data).unwrap_or_default()
                } else {
                    data.as_bytes().to_vec()
                };

                match port.write(&bytes) {
                    Ok(n) => {
                        state.tx_count += n as u64;
                        state.add_terminal_line(Direction::Tx, data, state.hex_mode);
                        state.add_log_entry(crate::state::LogLevel::Info, &format!("Sent {} bytes", n));
                    }
                    Err(e) => {
                        state.add_log_entry(crate::state::LogLevel::Error, &e.to_string());
                    }
                }
            }
        }
    });
}

fn parse_hex(hex_str: &str) -> Option<Vec<u8>> {
    let hex_str = hex_str.replace(" ", "").replace("0x", "").replace("0X", "");
    if hex_str.len() % 2 != 0 {
        return None;
    }

    let mut bytes = Vec::new();
    for i in (0..hex_str.len()).step_by(2) {
        let byte = u8::from_str_radix(&hex_str[i..i + 2], 16).ok()?;
        bytes.push(byte);
    }

    Some(bytes)
}
