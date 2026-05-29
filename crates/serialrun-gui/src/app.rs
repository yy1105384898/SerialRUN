use crate::state::{AppState, Language, Theme, T, UserPrefs};
use crate::ui;
use eframe::egui;
use std::sync::{Arc, Mutex};

fn config_path() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        std::path::PathBuf::from(home).join(".serialrun").join("config.toml")
    } else {
        std::path::PathBuf::from(".serialrun").join("config.toml")
    }
}

fn load_prefs() -> UserPrefs {
    let path = config_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| toml::from_str(&c).ok())
        .unwrap_or_default()
}

pub fn save_prefs_from_state(state: &AppState) {
    let prefs = UserPrefs {
        theme: state.theme,
        language: state.language,
        baud_rate: state.config.baud_rate,
        data_bits: match state.config.data_bits {
            serialrun_core::config::DataBits::Five => "5".into(),
            serialrun_core::config::DataBits::Six => "6".into(),
            serialrun_core::config::DataBits::Seven => "7".into(),
            serialrun_core::config::DataBits::Eight => "8".into(),
        },
        stop_bits: match state.config.stop_bits {
            serialrun_core::config::StopBits::One => "1".into(),
            serialrun_core::config::StopBits::Two => "2".into(),
        },
        parity: match state.config.parity {
            serialrun_core::config::Parity::None => "None".into(),
            serialrun_core::config::Parity::Odd => "Odd".into(),
            serialrun_core::config::Parity::Even => "Even".into(),
        },
    };
    save_prefs(&prefs);
}

pub fn save_prefs(prefs: &UserPrefs) {
    let path = config_path();
    if let Ok(content) = toml::to_string_pretty(prefs) {
        let _ = std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new(".")));
        let _ = std::fs::write(&path, content);
    }
}

pub struct SerialRunApp {
    state: Arc<Mutex<AppState>>,
    current_theme: Theme,
    mcp_handle: crate::mcp_server::McpHandle,
    last_mcp_po_tx: Option<std::sync::mpsc::Sender<crate::port_owner::PortCommand>>,
}

impl SerialRunApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, mcp_handle: crate::mcp_server::McpHandle) -> Self {
        let prefs = load_prefs();
        let mut state = AppState::new();
        state.language = prefs.language;
        state.theme = prefs.theme;
        state.config.baud_rate = prefs.baud_rate;
        // Restore data bits
        match prefs.data_bits.as_str() {
            "5" => state.config.data_bits = serialrun_core::config::DataBits::Five,
            "6" => state.config.data_bits = serialrun_core::config::DataBits::Six,
            "7" => state.config.data_bits = serialrun_core::config::DataBits::Seven,
            _ => state.config.data_bits = serialrun_core::config::DataBits::Eight,
        }
        // Restore stop bits
        match prefs.stop_bits.as_str() {
            "2" => state.config.stop_bits = serialrun_core::config::StopBits::Two,
            _ => state.config.stop_bits = serialrun_core::config::StopBits::One,
        }
        // Restore parity
        match prefs.parity.as_str() {
            "Odd" => state.config.parity = serialrun_core::config::Parity::Odd,
            "Even" => state.config.parity = serialrun_core::config::Parity::Even,
            _ => state.config.parity = serialrun_core::config::Parity::None,
        }
        // Auto-refresh port list on startup
        state.refresh_ports();

        // Do NOT apply visuals here — eframe overrides them after new() returns.
        // Instead, use a sentinel value so update() applies visuals on the first frame.
        Self { state: Arc::new(Mutex::new(state)), current_theme: Theme::Light, mcp_handle, last_mcp_po_tx: None }
    }
}

impl eframe::App for SerialRunApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());

        // Force-sync theme visuals every frame to guarantee button text matches colors
        if state.theme != self.current_theme {
            let mut visuals = match state.theme {
                Theme::Dark => egui::Visuals::dark(),
                Theme::Light => {
                    let mut v = egui::Visuals::light();
                    v.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(230, 230, 235);
                    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 30, 30));
                    v.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(200, 200, 210);
                    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 0, 0));
                    v.widgets.active.weak_bg_fill = egui::Color32::from_rgb(170, 170, 185);
                    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 0, 0));
                    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 50));
                    v
                }
            };
            visuals.window_rounding = egui::Rounding::same(8.0);
            visuals.widgets.noninteractive.rounding = egui::Rounding::same(6.0);
            visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
            visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
            visuals.widgets.active.rounding = egui::Rounding::same(6.0);
            ctx.set_visuals(visuals);
            self.current_theme = state.theme;
            save_prefs_from_state(&state);
        }

        // Poll MCP status
        while let Some(status) = self.mcp_handle.poll_status() {
            match status {
                crate::mcp_server::McpStatus::Running { addr } => {
                    state.mcp_running = true;
                    state.mcp_status = addr;
                }
                crate::mcp_server::McpStatus::Stopped => {
                    state.mcp_running = false;
                    state.mcp_status.clear();
                }
                crate::mcp_server::McpStatus::Error(e) => {
                    state.mcp_running = false;
                    state.mcp_status = format!("Error: {}", e);
                }
            }
        }
        if state.mcp_cmd_tx.is_none() {
            state.mcp_cmd_tx = Some(self.mcp_handle.cmd_tx());
        }

        // Poll MCP access log entries
        let mut mcp_log_changed = false;
        while let Some(entry) = self.mcp_handle.poll_log() {
            state.mcp_access_log.push_back(entry);
            if state.mcp_access_log.len() > 200 {
                state.mcp_access_log.pop_front();
            }
            mcp_log_changed = true;
        }
        if mcp_log_changed {
            state.save_mcp_log();
        }

        // Sync port_owner to MCP server whenever it changes
        {
            let has_po = state.port_owner.is_some();
            let had_po = self.last_mcp_po_tx.is_some();
            if has_po != had_po {
                let po_tx = state.port_owner.as_ref().map(|po| po.cmd_tx());
                if let Some(ref cmd_tx) = state.mcp_cmd_tx {
                    let _ = cmd_tx.send(crate::mcp_server::McpCommand::SetPortOwner(po_tx.clone()));
                }
                self.last_mcp_po_tx = po_tx;
            }
        }

        // Poll port owner events (persistent reader)
        let events: Vec<_> = if let Some(ref port_owner) = state.port_owner {
            let mut evts = Vec::new();
            while let Some(evt) = port_owner.poll() {
                evts.push(evt);
            }
            evts
        } else {
            Vec::new()
        };
        for evt in events {
            match evt {
                crate::port_owner::PortEvent::Data(data) => {
                    state.rx_count += data.len() as u64;
                    state.add_chart_data(data.len() as f64);
                    let (received, is_hex_display) = if state.rx_hex_mode {
                        (crate::ui::terminal::format_hex_bytes(&data), true)
                    } else {
                        (String::from_utf8_lossy(&data).to_string(), false)
                    };
                    state.add_terminal_line(crate::state::Direction::Rx, received.clone(), is_hex_display);
                    let hex_preview = crate::ui::terminal::format_hex_bytes(&data);
                    let text_preview = String::from_utf8_lossy(&data).to_string();
                    state.add_log_entry(crate::state::LogLevel::Info, &format!("RX {} bytes: {} | {}", data.len(), hex_preview, text_preview));
                    super::ui::data_logger::log_data(&mut state, "RX", &data);
                    // Auto-reply
                    if state.auto_reply_enabled && !state.auto_reply_pattern.is_empty() && !state.auto_reply_response.is_empty() {
                        if received.contains(&state.auto_reply_pattern) {
                            let reply = state.auto_reply_response.clone();
                            let reply_bytes = reply.as_bytes().to_vec();
                            if let Some(ref po) = state.port_owner {
                                po.send(crate::port_owner::PortCommand::Write(reply_bytes));
                            }
                            state.add_terminal_line(crate::state::Direction::Tx, reply.clone(), false);
                            state.add_log_entry(crate::state::LogLevel::Info, &format!("Auto-reply sent: {}", reply));
                        }
                    }
                }
                crate::port_owner::PortEvent::Opened(ok, msg) => {
                    if ok {
                        state.is_connected = true;
                        state.add_log_entry(crate::state::LogLevel::Info, &format!("Connected to {}", msg));
                    } else {
                        state.is_connected = false;
                        state.show_error(&msg);
                    }
                }
                crate::port_owner::PortEvent::Closed => {
                    state.is_connected = false;
                }
                crate::port_owner::PortEvent::Written(_) => {}
                crate::port_owner::PortEvent::Error(e) => {
                    state.show_error(&e);
                }
            }
        }

        // Replay logic
        if state.replay_running && state.is_connected {
            let now = chrono::Utc::now().timestamp_millis();
            let elapsed = (now - state.replay_start_time) as u64;

            // Calculate cumulative delay for current index
            let mut cumulative_delay: u64 = 0;
            for i in 0..state.replay_index {
                cumulative_delay += state.replay_commands[i].delay_ms;
            }

            while state.replay_index < state.replay_commands.len() {
                let cmd = &state.replay_commands[state.replay_index];
                if cmd.action == crate::state::ScriptAction::Wait {
                    cumulative_delay += cmd.delay_ms;
                    state.replay_index += 1;
                    continue;
                }
                // Send command if we've reached its timestamp
                if elapsed >= cumulative_delay {
                    if let Some(ref data) = cmd.data {
                        let hex_mode = state.hex_mode;
                        let checksum_mode = state.terminal_checksum_mode;
                        let line_ending = state.line_ending;

                        let mut bytes = if hex_mode {
                            match crate::ui::terminal::parse_hex(data) {
                                Some(b) => b,
                                None => { state.replay_index += 1; continue; }
                            }
                        } else {
                            let mut b = data.as_bytes().to_vec();
                            b.extend_from_slice(line_ending.suffix());
                            b
                        };
                        bytes = checksum_mode.append_checksum(&bytes);

                        let display = if hex_mode { data.clone() } else { data.replace("\r", "\\r").replace("\n", "\\n") };
                        let idx = state.replay_index;
                        let byte_len = bytes.len();
                        let hex_preview = crate::ui::terminal::format_hex_bytes(&bytes);
                        let text_preview = String::from_utf8_lossy(&bytes).to_string();
                        state.tx_count += byte_len as u64;
                        state.add_chart_data(byte_len as f64);
                        state.add_terminal_line(crate::state::Direction::Tx, display, false);
                        state.add_log_entry(crate::state::LogLevel::Info, &format!("Replay [{}] TX {} bytes: {} | {}", idx, byte_len, hex_preview, text_preview));
                        super::ui::data_logger::log_data(&mut state, "TX", &bytes);

                        if let Some(ref po) = state.port_owner {
                            po.send(crate::port_owner::PortCommand::Write(bytes));
                        }
                    }
                    state.replay_index += 1;
                } else {
                    break;
                }
            }

            // Check if replay is complete
            if state.replay_index >= state.replay_commands.len() {
                state.replay_running = false;
                state.replay_commands.clear();
                state.replay_index = 0;
                state.add_log_entry(crate::state::LogLevel::Info, "Replay completed");
            }

            // Request repaint to keep replay loop running
            ctx.request_repaint();
        }

        // Auto-send logic
        if state.auto_send_enabled && state.is_connected && !state.input_buffer.is_empty() {
            let now = chrono::Utc::now().timestamp_millis();
            if now - state.auto_send_last_time >= state.auto_send_interval_ms as i64 {
                state.auto_send_last_time = now;
                super::ui::terminal::do_send(&mut state);
            }
            ctx.request_repaint();
        }

        let lang = state.language;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui::connection::render_connection_panel(ui, &mut state, ctx);
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui::status::render_status_bar(ui, &mut state);
        });

        egui::SidePanel::left("side_panel").resizable(false).default_width(200.0).show(ctx, |ui| {
            ui::connection::render_connection_controls(ui, &mut state);
            ui::settings::render_settings_panel(ui, &mut state, ctx);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui::terminal::render_terminal_panel(ui, &mut state);
        });

        // Floating windows
        macro_rules! toggle_window {
            ($show:expr, $title:expr, $render:expr, $w:expr, $h:expr) => {
                let mut show = $show;
                if show {
                    egui::Window::new($title).open(&mut show).default_width($w).default_height($h).show(ctx, |ui| { $render(ui, &mut state); });
                }
                $show = show;
            };
        }

        toggle_window!(state.show_chart_window, T::data_chart(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::charts::render_chart_panel(ui, s), 400.0, 300.0);
        toggle_window!(state.show_log_window, T::log_viewer(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::log_viewer::render_log_panel(ui, s), 400.0, 350.0);
        toggle_window!(state.show_help, T::help(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::help::render_help_panel(ui, s), 600.0, 500.0);
        toggle_window!(state.show_modbus_window, T::modbus_panel(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::modbus::render_modbus_panel(ui, s), 520.0, 450.0);
        toggle_window!(state.show_plc_window, T::plc_control(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::plc::render_plc_panel(ui, s), 600.0, 500.0);
        toggle_window!(state.show_bridge_window, T::bridge(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::bridge::render_bridge_panel(ui, s), 520.0, 450.0);
        toggle_window!(state.show_simulator_window, T::simulator(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::simulator::render_simulator_panel(ui, s), 520.0, 500.0);
        toggle_window!(state.show_checksum_window, T::checksum(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::checksum::render_checksum_panel(ui, s), 400.0, 350.0);
        toggle_window!(state.show_file_transfer_window, T::file_transfer(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::file_transfer::render_file_transfer_panel(ui, s), 420.0, 300.0);
        toggle_window!(state.show_frame_builder_window, T::frame_builder(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::frame_builder::render_frame_builder_panel(ui, s), 450.0, 350.0);
        toggle_window!(state.show_data_logger_window, T::data_logger(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::data_logger::render_data_logger_panel(ui, s), 400.0, 250.0);
        toggle_window!(state.show_can_window, T::can_analyzer(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::can_analyzer::render_can_analyzer_panel(ui, s), 550.0, 400.0);
        toggle_window!(state.show_i2c_spi_window, T::i2c_spi(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::i2c_spi::render_i2c_spi_panel(ui, s), 450.0, 380.0);
        toggle_window!(state.show_scope_window, T::oscilloscope(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::serial_scope::render_serial_scope_panel(ui, s), 600.0, 480.0);
        toggle_window!(state.show_flasher_window, T::flasher(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::flasher::render_flasher_panel(ui, s), 420.0, 350.0);
        toggle_window!(state.show_register_editor_window, T::register_editor(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::register_editor::render_register_editor_panel(ui, s), 500.0, 400.0);
        toggle_window!(state.show_plugin_window, T::plugins(lang), |ui: &mut egui::Ui, s: &mut AppState| ui::plugin::render_plugin_panel(ui, s), 480.0, 400.0);

        // MCP Access Log popup
        if state.show_mcp_log_popup {
            let title = if lang == Language::Chinese { "MCP 访问日志" } else { "MCP Access Log" };
            let mut open = state.show_mcp_log_popup;
            let c = crate::theme::get_colors(state.theme);
            egui::Window::new(title)
                .open(&mut open)
                .default_width(500.0)
                .default_height(400.0)
                .resizable(true)
                .show(ctx, |ui| {
                    let count = state.mcp_access_log.len();
                    ui.label(egui::RichText::new(
                        format!("{} {}", count, if lang == Language::Chinese { "条记录" } else { "entries" })
                    ).color(c.text_muted));
                    ui.separator();

                    egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                        for entry in state.mcp_access_log.iter().rev().take(100) {
                            let color = match entry.action.as_str() {
                                "CONNECT" => c.mcp_connect,
                                "DISCONNECT" => c.mcp_disconnect,
                                "CALL" => c.mcp_call,
                                _ => c.text_muted,
                            };
                            ui.horizontal_wrapped(|ui| {
                                ui.label(egui::RichText::new(&entry.timestamp).color(c.timestamp_color).monospace().small());
                                ui.label(egui::RichText::new(&entry.client_ip).color(c.text_primary).strong().small());
                                ui.label(egui::RichText::new(&entry.action).color(color).strong().small());
                                ui.label(egui::RichText::new(&entry.detail).color(c.text_secondary).small());
                            });
                        }
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button(egui::RichText::new(
                            if lang == Language::Chinese { "清空日志" } else { "Clear Log" }
                        ).color(c.error)).clicked() {
                            state.mcp_access_log.clear();
                            state.save_mcp_log();
                        }
                    });
                });
            state.show_mcp_log_popup = open;
        }
    }
}
