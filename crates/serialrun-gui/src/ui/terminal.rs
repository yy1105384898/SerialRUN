use crate::port_owner::PortCommand;
use crate::state::{AppState, ChecksumMode, Direction, Language, LineEnding, ScriptAction, ScriptCommand, T};
use crate::theme;
use eframe::egui;

pub fn render_terminal_panel(ui: &mut egui::Ui, state: &mut AppState) {
    let lang = state.language;
    let c = theme::get_colors(state.theme);

    // Toolbar row 1
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(T::terminal(lang)).strong().size(14.0));
        ui.separator();
        ui.checkbox(&mut state.hex_mode, T::tx_hex(lang));
        ui.checkbox(&mut state.rx_hex_mode, T::rx_hex(lang));
        ui.checkbox(&mut state.show_timestamp, T::show_timestamp(lang));
        ui.checkbox(&mut state.auto_scroll, T::auto_scroll(lang));

        ui.separator();
        ui.label(T::crc_label(lang));
        let checksum = state.terminal_checksum_mode;
        egui::ComboBox::from_id_salt("term_crc").width(100.0).selected_text(checksum.label(lang)).show_ui(ui, |ui| {
            for &mode in ChecksumMode::all() {
                ui.selectable_value(&mut state.terminal_checksum_mode, mode, mode.label(lang));
            }
        });
        ui.label(egui::RichText::new("?").color(egui::Color32::from_rgb(100, 150, 220)).strong())
            .on_hover_text(crc_hover_text(lang));
    });

    // Toolbar row 2: DTR/RTS + auto-send + save
    ui.horizontal(|ui| {
        // DTR/RTS controls
        if state.is_connected {
            let old_dtr = state.dtr;
            let old_rts = state.rts;
            ui.checkbox(&mut state.dtr, "DTR");
            ui.checkbox(&mut state.rts, "RTS");
            if state.dtr != old_dtr {
                if let Some(ref po) = state.port_owner {
                    po.send(PortCommand::SetDtr(state.dtr));
                }
            }
            if state.rts != old_rts {
                if let Some(ref po) = state.port_owner {
                    po.send(PortCommand::SetRts(state.rts));
                }
            }
        }

        ui.separator();
        // Auto-send
        let auto_label = if state.auto_send_enabled { T::stop_auto(lang) } else { T::auto_send(lang) };
        if ui.small_button(auto_label).clicked() {
            state.auto_send_enabled = !state.auto_send_enabled;
            state.auto_send_last_time = chrono::Utc::now().timestamp_millis();
        }
        if state.auto_send_enabled {
            ui.add(egui::DragValue::new(&mut state.auto_send_interval_ms).range(100..=60000).suffix("ms"));
        }

        ui.add_space(8.0);
        if ui.button(T::clear(lang)).clicked() {
            state.terminal_buffer.clear();
            state.save_terminal();
        }

        if ui.button(T::save_btn(lang)).clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Text", &["txt"])
                .add_filter("All", &["*"])
                .save_file()
            {
                let mut content = String::new();
                for line in &state.terminal_buffer {
                    let ts = chrono::DateTime::from_timestamp_millis(line.timestamp)
                        .map(|t| t.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S%.3f").to_string())
                        .unwrap_or_default();
                    content.push_str(&format!("[{}] {} {}\n", ts, line.direction, line.content));
                }
                let _ = std::fs::write(&path, content);
                state.add_log_entry(crate::state::LogLevel::Info, &format!("Terminal log saved to {}", path.display()));
            }
        }
    });

    ui.separator();

    // Terminal display area
    let available_height = ui.available_height() - 50.0;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(state.auto_scroll)
        .max_height(available_height)
        .show(ui, |ui| {
            for line in &state.terminal_buffer {
                let (color, prefix) = match line.direction {
                    Direction::Rx => (c.rx_color, "\u{2193} RX"),
                    Direction::Tx => (c.tx_color, "\u{2191} TX"),
                    Direction::System => (c.sys_color, "\u{2699} SYS"),
                };
                let content_color = c.text_primary;
                let ts_color = c.timestamp_color;

                let timestamp = if state.show_timestamp {
                    let time = chrono::DateTime::from_timestamp_millis(line.timestamp)
                        .map(|t| t.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S%.3f").to_string())
                        .unwrap_or_default();
                    format!("[{}] ", time)
                } else {
                    String::new()
                };

                let content = if line.is_hex {
                    line.content.clone()
                } else {
                    line.content
                        .replace("\r\n", "\u{21B5}\n")
                        .replace("\r", "\u{21B5}")
                        .replace("\n", "\u{21B5}\n")
                };

                // Render row with right-aligned copy button (TX/RX only)
                if line.direction != Direction::System {
                    let copy_label = if lang == Language::Chinese { "复制" } else { "Copy" };
                    let now = chrono::Utc::now().timestamp_millis();
                    let just_copied = state.term_copied_ts == line.timestamp && (now - state.term_copied_time) < 1500;

                    let (btn_text, btn_color) = if just_copied {
                        (if lang == Language::Chinese { "已复制!" } else { "Copied!" }, egui::Color32::from_rgb(60, 180, 80))
                    } else {
                        (copy_label, egui::Color32::from_rgb(140, 140, 140))
                    };

                    ui.horizontal(|ui| {
                        if !timestamp.is_empty() {
                            ui.label(egui::RichText::new(&timestamp).color(ts_color).size(13.0).monospace());
                        }
                        ui.label(egui::RichText::new(prefix).color(color).size(13.0).strong());
                        ui.label(egui::RichText::new(&content).color(content_color).size(14.0));
                        // Copy button — always rendered, subtle gray, right-aligned
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let btn = ui.add(egui::Button::new(
                                egui::RichText::new(btn_text).color(btn_color).small()
                            ).fill(egui::Color32::TRANSPARENT).frame(false));
                            if btn.clicked() {
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    let _ = clipboard.set_text(content.clone());
                                    state.term_copied_ts = line.timestamp;
                                    state.term_copied_time = now;
                                }
                            }
                        });
                    });
                } else {
                    // SYS line — no copy button
                    ui.horizontal(|ui| {
                        if !timestamp.is_empty() {
                            ui.label(egui::RichText::new(&timestamp).color(ts_color).size(13.0).monospace());
                        }
                        ui.label(egui::RichText::new(prefix).color(color).size(13.0).strong());
                        ui.label(egui::RichText::new(&content).color(content_color).size(14.0));
                    });
                }
            }
        });

    ui.separator();

    // Input area
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.keep_input, T::keep_input(lang));
        ui.text_edit_singleline(&mut state.input_buffer);

        // Line ending selector — between input and send button
        ui.label(T::line_ending(lang));
        let le = state.line_ending;
        egui::ComboBox::from_id_salt("le_input").width(90.0).selected_text(le.label(lang)).show_ui(ui, |ui| {
            ui.selectable_value(&mut state.line_ending, LineEnding::None, LineEnding::None.label(lang));
            ui.selectable_value(&mut state.line_ending, LineEnding::CR, LineEnding::CR.label(lang));
            ui.selectable_value(&mut state.line_ending, LineEnding::LF, LineEnding::LF.label(lang));
            ui.selectable_value(&mut state.line_ending, LineEnding::CRLF, LineEnding::CRLF.label(lang));
        });

        // Send button with colored background
        let send_btn = ui.add(egui::Button::new(
            egui::RichText::new(T::send(lang)).color(c.btn_send_text).strong()
        ).fill(c.btn_send).min_size(egui::vec2(60.0, 24.0)));
        if send_btn.clicked() && !state.input_buffer.is_empty() {
            do_send(state);
        }
    });
}

pub fn do_send(state: &mut AppState) {
    // Check connection first
    if state.port_owner.is_none() {
        let msg = if state.language == Language::Chinese {
            "未连接串口，请先连接设备"
        } else {
            "Not connected. Please connect to a serial port first."
        };
        state.show_error(msg);
        return;
    }

    let data = if state.keep_input {
        state.input_buffer.clone()
    } else {
        std::mem::take(&mut state.input_buffer)
    };
    let hex_mode = state.hex_mode;
    let checksum_mode = state.terminal_checksum_mode;
    let line_ending = state.line_ending;
    state.hex_error = None;

    let mut bytes = if hex_mode {
        match parse_hex(&data) {
            Some(b) => b,
            None => {
                let msg = if state.language == Language::Chinese {
                    "HEX 格式错误：只允许 0-9, A-F, a-f, 空格"
                } else {
                    "Invalid HEX: only 0-9, A-F, a-f, spaces allowed"
                };
                state.show_error(msg);
                state.input_buffer = data;
                return;
            }
        }
    } else {
        let mut b = data.as_bytes().to_vec();
        b.extend_from_slice(line_ending.suffix());
        b
    };

    bytes = checksum_mode.append_checksum(&bytes);

    let display = if hex_mode {
        data.clone()
    } else {
        data.replace("\r", "\\r").replace("\n", "\\n")
    };

    state.tx_count += bytes.len() as u64;
    state.add_chart_data(bytes.len() as f64);
    state.add_terminal_line(Direction::Tx, display, false);
    let hex_preview = format_hex_bytes(&bytes);
    let text_preview = String::from_utf8_lossy(&bytes).to_string();
    state.add_log_entry(crate::state::LogLevel::Info, &format!("TX {} bytes: {} | {}", bytes.len(), hex_preview, text_preview));
    super::data_logger::log_data(state, "TX", &bytes);
    if state.recording {
        let now = chrono::Utc::now().timestamp_millis();
        let delay = if state.recording_last_time > 0 {
            (now - state.recording_last_time).max(0) as u64
        } else {
            0
        };
        // Record wait if delay > 50ms (avoid noise)
        if delay > 50 {
            state.script_commands.push(ScriptCommand {
                delay_ms: delay,
                action: ScriptAction::Wait,
                data: None,
            });
        }
        state.script_commands.push(ScriptCommand {
            delay_ms: 0,
            action: ScriptAction::Send,
            data: Some(data.clone()),
        });
        state.recording_last_time = now;
    }

    // Write through port owner (connection already verified at top of function)
    if let Some(ref po) = state.port_owner {
        po.send(PortCommand::Write(bytes));
    }
}

pub fn parse_hex(hex_str: &str) -> Option<Vec<u8>> {
    // Strip spaces and 0x/0X prefixes per byte
    let hex_str: String = hex_str.split_whitespace()
        .filter_map(|token| {
            let t = token.strip_prefix("0x").or_else(|| token.strip_prefix("0X")).unwrap_or(token);
            if t.is_empty() { return None; }
            // Validate each character is hex
            if t.chars().all(|c| c.is_ascii_hexdigit()) {
                Some(t.to_string())
            } else {
                None
            }
        })
        .collect();
    if hex_str.is_empty() || hex_str.len() % 2 != 0 {
        return None;
    }

    let mut bytes = Vec::new();
    for i in (0..hex_str.len()).step_by(2) {
        let byte = u8::from_str_radix(&hex_str[i..i + 2], 16).ok()?;
        bytes.push(byte);
    }

    Some(bytes)
}

/// Format raw bytes as space-separated hex string
pub fn format_hex_bytes(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")
}

fn crc_hover_text(lang: Language) -> String {
    if lang == Language::Chinese {
        "CRC16/MODBUS: Modbus RTU 标准校验 (0xA001)\nCRC16/CCITT: CCITT 标准 (0x1021)\nCRC32: 32位循环冗余校验\nLRC: 纵向冗余校验 (Modbus ASCII)\nSUM8: 8位累加和校验".into()
    } else {
        "CRC16/MODBUS: Modbus RTU standard (0xA001)\nCRC16/CCITT: CCITT standard (0x1021)\nCRC32: 32-bit CRC\nLRC: Longitudinal Redundancy Check (Modbus ASCII)\nSUM8: 8-bit additive checksum".into()
    }
}
