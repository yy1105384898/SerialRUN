use crate::state::{AppState, T};
use eframe::egui;
use serialrun_core::config::{DataBits, FlowControl, Parity, StopBits};

pub fn render_settings_panel(ui: &mut egui::Ui, state: &mut AppState, _ctx: &egui::Context) {
    let lang = state.language;

    ui.add_space(4.0);

    // Serial config section
    ui.collapsing(T::serial_config(lang), |ui| {
        egui::Grid::new("settings_grid").show(ui, |ui| {
            ui.label(T::data_bits(lang));
            let db_text = match state.config.data_bits { DataBits::Five=>"5", DataBits::Six=>"6", DataBits::Seven=>"7", DataBits::Eight=>"8" };
            egui::ComboBox::from_id_salt("data_bits")
                .width(80.0)
                .selected_text(db_text)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.config.data_bits, DataBits::Five, "5");
                    ui.selectable_value(&mut state.config.data_bits, DataBits::Six, "6");
                    ui.selectable_value(&mut state.config.data_bits, DataBits::Seven, "7");
                    ui.selectable_value(&mut state.config.data_bits, DataBits::Eight, "8");
                });
            ui.end_row();

            ui.label(T::stop_bits(lang));
            let sb_text = match state.config.stop_bits { StopBits::One=>"1", StopBits::Two=>"2" };
            egui::ComboBox::from_id_salt("stop_bits")
                .width(80.0)
                .selected_text(sb_text)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.config.stop_bits, StopBits::One, "1");
                    ui.selectable_value(&mut state.config.stop_bits, StopBits::Two, "2");
                });
            ui.end_row();

            ui.label(T::parity(lang));
            let par_text = match state.config.parity { Parity::None=>"None", Parity::Odd=>"Odd", Parity::Even=>"Even" };
            egui::ComboBox::from_id_salt("parity")
                .width(80.0)
                .selected_text(par_text)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.config.parity, Parity::None, "None");
                    ui.selectable_value(&mut state.config.parity, Parity::Odd, "Odd");
                    ui.selectable_value(&mut state.config.parity, Parity::Even, "Even");
                });
            ui.end_row();

            ui.label(T::flow_control(lang));
            let fc_text = match state.config.flow_control { FlowControl::None=>"None", FlowControl::Software=>"SW", FlowControl::Hardware=>"HW" };
            egui::ComboBox::from_id_salt("flow_control")
                .width(80.0)
                .selected_text(fc_text)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.config.flow_control, FlowControl::None, "None");
                    ui.selectable_value(&mut state.config.flow_control, FlowControl::Software, "SW");
                    ui.selectable_value(&mut state.config.flow_control, FlowControl::Hardware, "HW");
                });
            ui.end_row();
        });
    });

    ui.separator();

    // Display settings
    ui.label(T::display(lang));
    ui.add_space(4.0);
    ui.checkbox(&mut state.hex_mode, T::hex_mode(lang));
    ui.checkbox(&mut state.show_timestamp, T::show_timestamp(lang));
    ui.checkbox(&mut state.auto_scroll, T::auto_scroll(lang));

    ui.separator();

    // Auto reply
    ui.label(T::auto_reply(lang));
    ui.add_space(4.0);
    ui.checkbox(&mut state.auto_reply_enabled, T::auto_reply(lang));
    if state.auto_reply_enabled {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(format!("{}:", T::pattern(lang)));
            ui.text_edit_singleline(&mut state.auto_reply_pattern);
        });
        ui.horizontal(|ui| {
            ui.label(format!("{}:", T::response(lang)));
            ui.text_edit_singleline(&mut state.auto_reply_response);
        });
    }

    ui.separator();

    // MCP Server settings
    ui.collapsing(T::mcp_server(lang), |ui| {
        ui.horizontal(|ui| {
            if ui.checkbox(&mut state.mcp_enabled, T::mcp_enable(lang)).changed() {
                if let Some(ref cmd_tx) = state.mcp_cmd_tx {
                    if state.mcp_enabled {
                        let bind_addr = if state.mcp_bind_lan { "0.0.0.0" } else { "127.0.0.1" }.into();
                        let _ = cmd_tx.send(crate::mcp_server::McpCommand::Start { bind_addr, port: state.mcp_port });
                    } else {
                        let _ = cmd_tx.send(crate::mcp_server::McpCommand::Stop);
                    }
                }
            }
        });
        ui.add_space(4.0);

        egui::Grid::new("mcp_settings").show(ui, |ui| {
            ui.label(T::mcp_port(lang));
            let port_resp = ui.add(egui::DragValue::new(&mut state.mcp_port).range(1024..=65535));
            if port_resp.changed() && state.mcp_running {
                if let Some(ref cmd_tx) = state.mcp_cmd_tx {
                    let bind_addr = if state.mcp_bind_lan { "0.0.0.0" } else { "127.0.0.1" }.into();
                    let _ = cmd_tx.send(crate::mcp_server::McpCommand::Reconfigure { bind_addr, port: state.mcp_port });
                }
            }
            ui.end_row();

            ui.label(T::mcp_bind(lang));
            let bind_text = if state.mcp_bind_lan { T::mcp_lan(lang) } else { T::mcp_localhost(lang) };
            let bind_resp = egui::ComboBox::from_id_salt("mcp_bind").width(140.0).selected_text(bind_text).show_ui(ui, |ui| {
                ui.selectable_value(&mut state.mcp_bind_lan, false, T::mcp_localhost(lang));
                ui.selectable_value(&mut state.mcp_bind_lan, true, T::mcp_lan(lang));
            });
            if bind_resp.response.changed() && state.mcp_running {
                if let Some(ref cmd_tx) = state.mcp_cmd_tx {
                    let bind_addr = if state.mcp_bind_lan { "0.0.0.0" } else { "127.0.0.1" }.into();
                    let _ = cmd_tx.send(crate::mcp_server::McpCommand::Reconfigure { bind_addr, port: state.mcp_port });
                }
            }
            ui.end_row();

            ui.label(T::mcp_status(lang));
            let status_color = if state.mcp_running { egui::Color32::GREEN } else { egui::Color32::GRAY };
            ui.label(egui::RichText::new(if state.mcp_running { T::mcp_running(lang) } else { T::mcp_stopped(lang) }).color(status_color));
            ui.end_row();
        });

        if state.mcp_bind_lan {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(T::mcp_warning(lang)).color(egui::Color32::from_rgb(220, 180, 50)).small());
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let addr = if state.mcp_bind_lan { "0.0.0.0" } else { "127.0.0.1" };
            ui.label(egui::RichText::new(format!("Bind: {}:{}", addr, state.mcp_port)).monospace().small());
        });
        if state.mcp_bind_lan {
            if let Some(local_ip) = get_local_ip() {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("LAN: {}:{}", local_ip, state.mcp_port)).monospace().small().color(egui::Color32::from_rgb(100, 200, 255)));
                });
            }
        }
    });

    ui.separator();

    // Recording
    if state.recording {
        ui.label(egui::RichText::new(format!("● {}", T::recording(lang))).color(egui::Color32::from_rgb(220, 60, 60)));
        if ui.button(T::stop_recording(lang)).clicked() {
            state.recording = false;
            state.add_log_entry(crate::state::LogLevel::Info, "Recording stopped");
        }
    } else {
        if ui.button(T::start_recording(lang)).clicked() {
            state.recording = true;
            state.script_commands.clear();
            state.add_log_entry(crate::state::LogLevel::Info, "Recording started");
        }
    }
}

fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    Some(addr.ip().to_string())
}
