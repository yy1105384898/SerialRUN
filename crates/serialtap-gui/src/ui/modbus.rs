use crate::state::{AppState, ModbusFrameLogEntry, ModbusFunctionCode, MonitorEntry, T};
use eframe::egui;
use serialtap_core::protocol::{ModbusFrame, ModbusParser};

pub fn render_modbus_panel(ui: &mut egui::Ui, state: &mut AppState) {
    let lang = state.language;
    ui.collapsing(T::quick_request(lang), |ui| { render_quick_request(ui, state); });
    ui.separator();
    ui.collapsing(T::register_monitor(lang), |ui| { render_register_monitor(ui, state); });
    ui.separator();
    ui.collapsing(T::frame_log(lang), |ui| { render_frame_log(ui, state); });
}

fn render_quick_request(ui: &mut egui::Ui, state: &mut AppState) {
    let lang = state.language;
    egui::Grid::new("modbus_qr").num_columns(2).show(ui, |ui| {
        ui.label(T::slave_id(lang));
        ui.add(egui::DragValue::new(&mut state.modbus.slave_id).range(0..=247));
        ui.end_row();
        ui.label(T::function_code(lang));
        let fc = state.modbus.function_code;
        egui::ComboBox::from_id_salt("modbus_fc").selected_text(fc.label(lang)).show_ui(ui, |ui| {
            for &f in ModbusFunctionCode::all() { ui.selectable_value(&mut state.modbus.function_code, f, f.label(lang)); }
        });
        ui.end_row();
        ui.label(T::start_address(lang));
        ui.text_edit_singleline(&mut state.modbus.start_addr);
        ui.end_row();
        if state.modbus.function_code.is_read() {
            ui.label(T::quantity(lang));
            ui.text_edit_singleline(&mut state.modbus.quantity);
        } else {
            ui.label(T::write_value(lang));
            ui.text_edit_singleline(&mut state.modbus.write_value);
        }
        ui.end_row();
    });
    ui.add_space(4.0);
    if ui.button(T::send_request(lang)).clicked() { do_modbus_request(state); }
    if let Some(ref err) = state.modbus.last_error { ui.colored_label(egui::Color32::RED, err.as_str()); }
    if !state.modbus.last_request_hex.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new(T::last_request(lang)).strong());
        ui.label(egui::RichText::new(&state.modbus.last_request_hex).monospace());
        ui.label(egui::RichText::new(T::last_response(lang)).strong());
        ui.label(egui::RichText::new(&state.modbus.last_response_hex).monospace());
    }
}

fn do_modbus_request(state: &mut AppState) {
    state.modbus.last_error = None;
    let addr: u16 = match state.modbus.start_addr.parse() { Ok(v) => v, Err(_) => { state.modbus.last_error = Some("Invalid address".into()); return; } };
    let frame = if state.modbus.function_code.is_read() {
        let qty: u16 = match state.modbus.quantity.parse() { Ok(v) => v, Err(_) => { state.modbus.last_error = Some("Invalid quantity".into()); return; } };
        ModbusParser::build_read_request(state.modbus.slave_id, state.modbus.function_code.to_core_function(), addr, qty)
    } else {
        let val: u16 = match state.modbus.write_value.parse() { Ok(v) => v, Err(_) => { state.modbus.last_error = Some("Invalid value".into()); return; } };
        ModbusParser::build_write_single(state.modbus.slave_id, addr, val)
    };
    let req_bytes = frame.to_bytes();
    let req_hex = hex_str(&req_bytes);
    let mut buf = [0u8; 256];
    let result = (|| -> Result<Vec<u8>, String> {
        let port = state.port.as_mut().ok_or("Not connected")?;
        port.write(&req_bytes).map_err(|e| e.to_string())?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        let n = port.read(&mut buf).map_err(|e| e.to_string())?;
        if n < 4 { return Err("No response".into()); }
        Ok(buf[..n].to_vec())
    })();
    match result {
        Ok(resp) => {
            let resp_hex = hex_str(&resp);
            if let Ok(f) = ModbusFrame::parse(&resp) {
                state.modbus.last_request_hex = req_hex.clone();
                state.modbus.last_response_hex = resp_hex.clone();
                state.modbus.frame_log.push_back(ModbusFrameLogEntry { timestamp: chrono::Utc::now().timestamp_millis(), request_hex: req_hex, response_hex: resp_hex, decoded: ModbusParser::format_frame(&f), is_error: false });
                if state.modbus.frame_log.len() > 200 { state.modbus.frame_log.pop_front(); }
            }
        }
        Err(e) => { state.modbus.last_error = Some(e); }
    }
}

fn render_register_monitor(ui: &mut egui::Ui, state: &mut AppState) {
    let lang = state.language;
    egui::Grid::new("modbus_mon").num_columns(2).show(ui, |ui| {
        ui.label(T::slave_id(lang)); ui.add(egui::DragValue::new(&mut state.modbus.monitor_slave_id).range(0..=247)); ui.end_row();
        ui.label(T::start_address(lang)); ui.text_edit_singleline(&mut state.modbus.monitor_start_addr); ui.end_row();
        ui.label(T::quantity(lang)); ui.text_edit_singleline(&mut state.modbus.monitor_quantity); ui.end_row();
        ui.label(T::poll_interval(lang)); ui.add(egui::DragValue::new(&mut state.modbus.monitor_interval_ms).range(100..=10000)); ui.end_row();
    });
    ui.add_space(4.0);
    let label = if state.modbus.monitor_polling { T::stop_monitor(lang) } else { T::start_monitor(lang) };
    if ui.button(label).clicked() {
        if state.modbus.monitor_polling { state.modbus.monitor_polling = false; state.modbus.monitor_entries.clear(); }
        else if state.is_connected { state.modbus.monitor_polling = true; state.modbus.last_poll_time = 0; }
    }
    if state.modbus.monitor_polling && state.is_connected {
        let now = chrono::Utc::now().timestamp_millis();
        if now - state.modbus.last_poll_time >= state.modbus.monitor_interval_ms as i64 {
            do_monitor_poll(state);
            state.modbus.last_poll_time = now;
        }
    }
    if !state.modbus.monitor_entries.is_empty() {
        ui.separator();
        egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
            for entry in &state.modbus.monitor_entries {
                ui.horizontal(|ui| {
                    ui.label(format!("0x{:04X}", entry.addr));
                    ui.label(format!("0x{:04X}", entry.raw_value));
                    ui.label(&entry.display_value);
                });
            }
        });
    }
}

fn do_monitor_poll(state: &mut AppState) {
    let addr: u16 = match state.modbus.monitor_start_addr.parse() { Ok(v) => v, Err(_) => return };
    let qty: u16 = match state.modbus.monitor_quantity.parse() { Ok(v) => v, Err(_) => return };
    let frame = ModbusParser::build_read_request(state.modbus.monitor_slave_id, state.modbus.monitor_function.to_core_function(), addr, qty);
    let req = frame.to_bytes();
    let mut buf = [0u8; 256];
    if let Ok(resp) = (|| -> Result<Vec<u8>, String> {
        let p = state.port.as_mut().ok_or("Not connected")?;
        p.write(&req).map_err(|e| e.to_string())?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        let n = p.read(&mut buf).map_err(|e| e.to_string())?;
        if n < 4 { return Err("No response".into()); }
        Ok(buf[..n].to_vec())
    })() {
        if let Ok(f) = ModbusFrame::parse(&resp) {
            let data = &f.data;
            if data.len() >= 2 {
                state.modbus.monitor_entries.clear();
                let mut i = 1;
                while i + 1 < data.len() {
                    let val = u16::from_be_bytes([data[i], data[i + 1]]);
                    state.modbus.monitor_entries.push(MonitorEntry { addr: addr + (state.modbus.monitor_entries.len() as u16), raw_value: val, display_value: format!("{}", val), last_update: chrono::Utc::now().timestamp_millis(), error: None });
                    i += 2;
                }
            }
        }
    }
}

fn render_frame_log(ui: &mut egui::Ui, state: &mut AppState) {
    let lang = state.language;
    egui::ScrollArea::vertical().max_height(200.0).stick_to_bottom(true).show(ui, |ui| {
        for entry in state.modbus.frame_log.iter().rev() {
            let ts = chrono::DateTime::from_timestamp_millis(entry.timestamp).map(|t| t.format("%H:%M:%S%.3f").to_string()).unwrap_or_default();
            let color = if entry.is_error { egui::Color32::RED } else { egui::Color32::GREEN };
            ui.horizontal(|ui| { ui.label(egui::RichText::new(format!("[{}]", ts)).weak()); ui.label(egui::RichText::new(&entry.decoded).color(color)); });
            ui.horizontal(|ui| { ui.label(egui::RichText::new("TX:").weak().monospace()); ui.label(egui::RichText::new(&entry.request_hex).monospace()); });
            ui.horizontal(|ui| { ui.label(egui::RichText::new("RX:").weak().monospace()); ui.label(egui::RichText::new(&entry.response_hex).monospace().color(color)); });
            ui.separator();
        }
    });
    if ui.button(T::clear_frame_log(lang)).clicked() { state.modbus.frame_log.clear(); }
}

fn hex_str(bytes: &[u8]) -> String { bytes.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ") }
