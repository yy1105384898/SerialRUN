use crate::state::{AppState, T};
use eframe::egui;
use serialtap_core::file_transfer::{FileTransfer, TransferProtocol};
use std::cell::RefCell;
use std::rc::Rc;

pub fn render_file_transfer_panel(ui: &mut egui::Ui, state: &mut AppState) {
    let lang = state.language;
    ui.label(T::protocol(lang));
    ui.horizontal(|ui| {
        let mut current = state.file_transfer_protocol;
        egui::ComboBox::from_id_salt("ft_proto").selected_text(match current { TransferProtocol::Xmodem => "XMODEM", TransferProtocol::XmodemCrc => "XMODEM-CRC", TransferProtocol::Ymodem => "YMODEM", TransferProtocol::Zmodem => "ZMODEM" }).show_ui(ui, |ui| {
            ui.selectable_value(&mut current, TransferProtocol::Xmodem, "XMODEM");
            ui.selectable_value(&mut current, TransferProtocol::XmodemCrc, "XMODEM-CRC");
            ui.selectable_value(&mut current, TransferProtocol::Ymodem, "YMODEM");
            ui.selectable_value(&mut current, TransferProtocol::Zmodem, "ZMODEM");
        });
        state.file_transfer_protocol = current;
    });
    ui.add_space(8.0);
    if state.file_transfer_done { ui.label(egui::RichText::new("Done").color(egui::Color32::GREEN)); }
    else if let Some(ref e) = state.file_transfer_error { ui.label(egui::RichText::new(format!("Error: {}", e)).color(egui::Color32::RED)); }
    else if state.file_transfer_sending { ui.label("Sending..."); }
    else if state.file_transfer_receiving { ui.label("Receiving..."); }
    else { ui.label("Ready"); }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let can = state.is_connected && !state.file_transfer_sending && !state.file_transfer_receiving;
        if ui.add_enabled(can, egui::Button::new(T::send_file(lang))).clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                state.file_transfer_sending = true;
                state.file_transfer_done = false;
                state.file_transfer_error = None;
                if let Some(port) = state.port.take() {
                    let shared = Rc::new(RefCell::new(port));
                    let proto = state.file_transfer_protocol;
                    let transfer = FileTransfer::new(proto);
                    let p = shared.clone();
                    let wf = move |d: &[u8]| -> Result<(), serialtap_core::file_transfer::TransferError> { p.borrow_mut().write(d).map_err(|e| serialtap_core::file_transfer::TransferError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?; Ok(()) };
                    let p = shared.clone();
                    let rf = || -> Result<u8, serialtap_core::file_transfer::TransferError> { let mut b = [0u8; 1]; match p.borrow_mut().read(&mut b) { Ok(1) => Ok(b[0]), _ => Ok(0) } };
                    let result = transfer.send_file(&path, wf, rf, |_| {});
                    match Rc::try_unwrap(shared) {
                        Ok(cell) => { state.port = Some(cell.into_inner()); }
                        Err(_) => { state.add_log_entry(crate::state::LogLevel::Error, "Port reference leak"); }
                    }
                    state.file_transfer_sending = false;
                    match result { Ok(()) => { state.file_transfer_done = true; } Err(e) => { state.file_transfer_error = Some(e.to_string()); } }
                }
            }
        }
        if ui.add_enabled(can, egui::Button::new(T::receive_file(lang))).clicked() {
            if let Some(path) = rfd::FileDialog::new().add_filter("All", &["*"]).save_file() {
                state.file_transfer_receiving = true;
                state.file_transfer_done = false;
                state.file_transfer_error = None;
                if let Some(port) = state.port.take() {
                    let shared = Rc::new(RefCell::new(port));
                    let proto = state.file_transfer_protocol;
                    let transfer = FileTransfer::new(proto);
                    let p = shared.clone();
                    let wf = move |d: &[u8]| -> Result<(), serialtap_core::file_transfer::TransferError> { p.borrow_mut().write(d).map_err(|e| serialtap_core::file_transfer::TransferError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?; Ok(()) };
                    let p = shared.clone();
                    let rf = || -> Result<u8, serialtap_core::file_transfer::TransferError> { let mut b = [0u8; 1]; match p.borrow_mut().read(&mut b) { Ok(1) => Ok(b[0]), _ => Ok(0) } };
                    let result = transfer.receive_file(&path, wf, rf, |_| {});
                    match Rc::try_unwrap(shared) {
                        Ok(cell) => { state.port = Some(cell.into_inner()); }
                        Err(_) => { state.add_log_entry(crate::state::LogLevel::Error, "Port reference leak"); }
                    }
                    state.file_transfer_receiving = false;
                    match result { Ok(()) => { state.file_transfer_done = true; } Err(e) => { state.file_transfer_error = Some(e.to_string()); } }
                }
            }
        }
    });
}
