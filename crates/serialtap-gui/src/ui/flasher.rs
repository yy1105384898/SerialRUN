use crate::state::{AppState, McuType, T};
use eframe::egui;

pub fn render_flasher_panel(ui: &mut egui::Ui, state: &mut AppState) {
    let lang = state.language;
    ui.label(egui::RichText::new("Serial Flasher").strong());
    ui.separator();

    ui.horizontal(|ui| {
        ui.label("MCU:");
        for mcu in &[McuType::Stm32, McuType::Esp32] {
            if ui.selectable_label(state.flasher_mcu == *mcu, mcu.label()).clicked() {
                state.flasher_mcu = *mcu;
            }
        }
    });
    ui.add_space(4.0);

    // File selection
    ui.horizontal(|ui| {
        ui.label("Firmware:");
        if ui.button("...").clicked() {
            let filter = match state.flasher_mcu {
                McuType::Stm32 => ("Firmware", &["hex", "bin"] as &[&str]),
                McuType::Esp32 => ("Firmware", &["bin", "elf"] as &[&str]),
            };
            if let Some(path) = rfd::FileDialog::new().add_filter(filter.0, filter.1).pick_file() {
                state.flasher_file = path.display().to_string();
            }
        }
        ui.label(&state.flasher_file);
    });
    ui.add_space(4.0);

    // Progress
    if state.flasher_progress > 0.0 && state.flasher_progress < 1.0 {
        ui.add(egui::ProgressBar::new(state.flasher_progress).text(format!("{}%", (state.flasher_progress * 100.0) as u32)));
    }
    ui.add_space(4.0);

    // Actions
    ui.horizontal(|ui| {
        if ui.button(T::connect(lang)).clicked() {
            flasher_connect(state);
        }
        if ui.button(T::erase(lang)).clicked() {
            flasher_erase(state);
        }
        if ui.button(T::flash(lang)).clicked() {
            flasher_flash(state);
        }
    });
    ui.add_space(4.0);

    // Log
    if !state.flasher_log.is_empty() {
        ui.separator();
        egui::ScrollArea::vertical().max_height(120.0).stick_to_bottom(true).show(ui, |ui| {
            for line in &state.flasher_log {
                ui.label(egui::RichText::new(line).monospace().small());
            }
        });
    }
}

fn flasher_log(state: &mut AppState, msg: &str) {
    state.flasher_log.push_back(msg.to_string());
    if state.flasher_log.len() > 100 { state.flasher_log.pop_front(); }
    state.add_log_entry(crate::state::LogLevel::Info, &format!("Flasher: {}", msg));
}

fn flasher_connect(state: &mut AppState) {
    if state.flasher_mcu == McuType::Stm32 {
        flasher_log(state, "STM32: Sending init sequence (0x7F)...");
        if let Some(ref mut port) = state.port {
            let _ = port.write(&[0x7F]);
            std::thread::sleep(std::time::Duration::from_millis(100));
            let mut buf = [0u8; 32];
            match port.read(&mut buf) {
                Ok(n) if n > 0 => {
                    let hex = buf[..n].iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
                    flasher_log(state, &format!("STM32: ACK received: {}", hex));
                }
                _ => { flasher_log(state, "STM32: No ACK received. Check BOOT0 pin."); }
            }
        } else { flasher_log(state, "Not connected"); }
    } else {
        flasher_log(state, "ESP32: Sending sync sequence...");
        if let Some(ref mut port) = state.port {
            let sync = [0xC0, 0x08, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x07, 0x12, 0x20, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0xC0];
            let _ = port.write(&sync);
            std::thread::sleep(std::time::Duration::from_millis(200));
            let mut buf = [0u8; 64];
            match port.read(&mut buf) {
                Ok(n) if n > 0 => {
                    let hex = buf[..n].iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
                    flasher_log(state, &format!("ESP32: Response: {}", hex));
                }
                _ => { flasher_log(state, "ESP32: No response. Hold BOOT and reset."); }
            }
        } else { flasher_log(state, "Not connected"); }
    }
}

fn flasher_erase(state: &mut AppState) {
    if state.flasher_mcu == McuType::Stm32 {
        flasher_log(state, "STM32: Mass erase command...");
        if let Some(ref mut port) = state.port {
            let erase_cmd = [0x01, 0xFF, 0x00, 0x00]; // erase command
            let _ = port.write(&erase_cmd);
            std::thread::sleep(std::time::Duration::from_millis(500));
            let mut buf = [0u8; 16];
            match port.read(&mut buf) {
                Ok(n) if n > 0 => { flasher_log(state, &format!("STM32: Erase done. {} byte(s) response", n)); }
                _ => { flasher_log(state, "STM32: Erase timeout"); }
            }
        }
    } else {
        flasher_log(state, "ESP32: Erase flash...");
        if let Some(ref mut port) = state.port {
            let erase = [0xC0, 0x00, 0x31, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0];
            let _ = port.write(&erase);
            std::thread::sleep(std::time::Duration::from_millis(500));
            let mut buf = [0u8; 64];
            match port.read(&mut buf) {
                Ok(n) if n > 0 => { flasher_log(state, &format!("ESP32: Erase response: {} byte(s)", n)); }
                _ => { flasher_log(state, "ESP32: Erase timeout"); }
            }
        }
    }
}

fn flasher_flash(state: &mut AppState) {
    if state.flasher_file.is_empty() {
        flasher_log(state, "No firmware file selected");
        return;
    }
    let path = std::path::PathBuf::from(&state.flasher_file);
    match std::fs::read(&path) {
        Ok(data) => {
            flasher_log(state, &format!("Flashing {} byte(s)...", data.len()));
            state.flasher_progress = 0.1;
            if let Some(ref mut port) = state.port {
                let chunk_size = 128;
                let total = data.len();
                let mut offset = 0;
                while offset < total {
                    let end = (offset + chunk_size).min(total);
                    let _ = port.write(&data[offset..end]);
                    offset = end;
                    state.flasher_progress = offset as f32 / total as f32;
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                state.flasher_progress = 1.0;
                flasher_log(state, "Flash complete!");
            } else {
                flasher_log(state, "Not connected");
            }
        }
        Err(e) => { flasher_log(state, &format!("Read error: {}", e)); }
    }
}
