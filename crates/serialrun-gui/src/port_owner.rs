use serialrun_core::config::SerialConfig;
use serialrun_core::port::ClearBuffer;
use serialrun_core::SerialPort;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub enum PortCommand {
    Open(SerialConfig),
    Close,
    Write(Vec<u8>),
    /// Write data, then read response with timeout (for request-response protocols)
    WriteRead {
        data: Vec<u8>,
        timeout_ms: u64,
        resp_tx: mpsc::Sender<Result<Vec<u8>, String>>,
    },
    ClearBuffers,
    ChangeBaud(u32),
    /// Pause/resume the continuous read loop (for CAN/Scope exclusive access)
    ReadPause,
    ReadResume,
    SetDtr(bool),
    SetRts(bool),
    /// Wait until port is released (for CAN/Scope exclusive access)
    WaitForRelease {
        resp_tx: mpsc::Sender<()>,
    },
}

pub enum PortEvent {
    Opened(bool, String),
    Closed,
    Written(usize),
    Data(Vec<u8>),
    Error(String),
}

pub struct PortOwnerHandle {
    cmd_tx: Option<mpsc::Sender<PortCommand>>,
    pub evt_rx: mpsc::Receiver<PortEvent>,
    handle: Option<thread::JoinHandle<()>>,
}

impl PortOwnerHandle {
    pub fn start() -> Self {
        let (cmd_tx_inner, cmd_rx) = mpsc::channel();
        let (evt_tx, evt_rx) = mpsc::channel();
        let handle = thread::spawn(move || Self::run(cmd_rx, evt_tx));
        Self { cmd_tx: Some(cmd_tx_inner), evt_rx, handle: Some(handle) }
    }

    /// Get a clone of the command sender for use by other threads (e.g., Modbus, PLC).
    pub fn cmd_tx(&self) -> mpsc::Sender<PortCommand> {
        self.cmd_tx.clone().expect("PortOwnerHandle already shut down")
    }

    pub fn send(&self, cmd: PortCommand) {
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(cmd);
        }
    }

    pub fn poll(&self) -> Option<PortEvent> {
        self.evt_rx.try_recv().ok()
    }

    /// Pause the continuous read loop (for CAN/Scope exclusive port access)
    pub fn pause_reads(&self) {
        self.send(PortCommand::ReadPause);
    }

    /// Resume the continuous read loop
    pub fn resume_reads(&self) {
        self.send(PortCommand::ReadResume);
    }

    /// Wait until the port is released (for CAN/Scope exclusive access).
    /// Sends Close, waits for port_owner to release, then caller can open the port.
    /// Returns true if released, false if timed out or port_owner already gone.
    pub fn wait_for_release(&self) -> bool {
        let (resp_tx, resp_rx) = mpsc::channel();
        self.send(PortCommand::WaitForRelease { resp_tx });
        // Timeout after 200ms to avoid hanging the GUI thread
        resp_rx.recv_timeout(std::time::Duration::from_millis(200)).is_ok()
    }

    /// Send a write-read request and wait for the response (with timeout).
    /// Used by Modbus, PLC, I2C/SPI, Flasher, etc. for request-response protocols.
    pub fn write_read(&self, data: Vec<u8>, timeout_ms: u64) -> Result<Vec<u8>, String> {
        let (resp_tx, resp_rx) = mpsc::channel();
        self.send(PortCommand::WriteRead { data, timeout_ms, resp_tx });
        // Use recv_timeout to avoid hanging if port_owner thread is stuck
        resp_rx.recv_timeout(std::time::Duration::from_millis(timeout_ms + 500))
            .unwrap_or_else(|_| Err("Timeout: port owner did not respond".into()))
    }

    fn run(cmd_rx: mpsc::Receiver<PortCommand>, evt_tx: mpsc::Sender<PortEvent>) {
        let mut port: Option<SerialPort> = None;
        let mut read_paused = false;

        loop {
            // Drain all pending commands first
            let mut had_command = true;
            while had_command {
                match cmd_rx.try_recv() {
                    Ok(cmd) => match cmd {
                        PortCommand::Open(config) => {
                            if port.is_some() {
                                let _ = evt_tx.send(PortEvent::Error("Port already open".into()));
                                continue;
                            }
                            let mut p = SerialPort::new(config.clone());
                            match p.connect() {
                                Ok(()) => {
                                    let _ = p.set_timeout(Duration::from_millis(50));
                                    let _ = evt_tx.send(PortEvent::Opened(true, config.port_name));
                                    port = Some(p);
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(PortEvent::Opened(false, e.to_string()));
                                }
                            }
                        }
                        PortCommand::Close => {
                            if let Some(mut p) = port.take() {
                                let _ = p.disconnect();
                            }
                            let _ = evt_tx.send(PortEvent::Closed);
                        }
                        PortCommand::Write(data) => {
                            if let Some(ref mut p) = port {
                                match p.write(&data) {
                                    Ok(n) => { let _ = evt_tx.send(PortEvent::Written(n)); }
                                    Err(e) => { let _ = evt_tx.send(PortEvent::Error(e.to_string())); }
                                }
                            } else {
                                let _ = evt_tx.send(PortEvent::Error("Not connected".into()));
                            }
                        }
                        PortCommand::WriteRead { data, timeout_ms, resp_tx } => {
                            if let Some(ref mut p) = port {
                                let _ = p.set_timeout(Duration::from_millis(timeout_ms));
                                if let Err(e) = p.write(&data) {
                                    let _ = p.set_timeout(Duration::from_millis(50));
                                    let _ = resp_tx.send(Err(format!("Write failed: {}", e)));
                                    continue;
                                }
                                let mut buf = [0u8; 4096];
                                let result = match p.read(&mut buf) {
                                    Ok(n) if n > 0 => Ok(buf[..n].to_vec()),
                                    Ok(_) => Err("No response".into()),
                                    Err(e) => Err(e.to_string()),
                                };
                                let _ = p.set_timeout(Duration::from_millis(50));
                                let _ = resp_tx.send(result);
                            } else {
                                let _ = resp_tx.send(Err("Not connected".into()));
                            }
                        }
                        PortCommand::ClearBuffers => {
                            if let Some(ref p) = port {
                                let _ = p.clear_buffer(ClearBuffer::All);
                            }
                        }
                        PortCommand::ReadPause => { read_paused = true; }
                        PortCommand::ReadResume => { read_paused = false; }
                        PortCommand::SetDtr(dtr) => {
                            if let Some(ref mut p) = port {
                                let _ = p.write_data_terminal_ready(dtr);
                            }
                        }
                        PortCommand::SetRts(rts) => {
                            if let Some(ref mut p) = port {
                                let _ = p.write_request_to_send(rts);
                            }
                        }
                        PortCommand::WaitForRelease { resp_tx } => {
                            if let Some(mut p) = port.take() {
                                let _ = p.disconnect();
                            }
                            // Brief delay for OS to release the port handle
                            std::thread::sleep(std::time::Duration::from_millis(50));
                            let _ = resp_tx.send(());
                        }
                        PortCommand::ChangeBaud(baud) => {
                            if let Some(ref mut p) = port {
                                let mut cfg = p.config().clone();
                                cfg.baud_rate = baud;
                                let _ = p.disconnect();
                                thread::sleep(Duration::from_millis(50));
                                let mut new_port = SerialPort::new(cfg);
                                match new_port.connect() {
                                    Ok(()) => {
                                        let _ = new_port.set_timeout(Duration::from_millis(50));
                                        let _ = evt_tx.send(PortEvent::Opened(true, format!("Baud changed to {}", baud)));
                                        port = Some(new_port);
                                    }
                                    Err(e) => {
                                        let _ = evt_tx.send(PortEvent::Error(e.to_string()));
                                    }
                                }
                            }
                        }
                    },
                    Err(mpsc::TryRecvError::Empty) => { had_command = false; }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        if let Some(mut p) = port.take() {
                            let _ = p.disconnect();
                        }
                        return;
                    }
                }
            }

            // Non-blocking read from port (skip when paused for CAN/Scope)
            if read_paused {
                thread::sleep(Duration::from_millis(10));
            } else if let Some(ref mut p) = port {
                let mut buf = [0u8; 4096];
                match p.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        let _ = evt_tx.send(PortEvent::Data(buf[..n].to_vec()));
                    }
                    _ => {}
                }
            } else {
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

impl Drop for PortOwnerHandle {
    fn drop(&mut self) {
        // Send Close and drop the sender. The background thread will see
        // Disconnected on its next try_recv() and exit on its own.
        // We do NOT call join() here because on Windows, the serial port read
        // may not respect the timeout, causing the thread to block indefinitely
        // and freezing the GUI.
        if let Some(tx) = self.cmd_tx.take() {
            let _ = tx.send(PortCommand::Close);
            drop(tx);
        }
        // The thread will exit within ~50ms when it sees Disconnected.
        // OS will clean up the thread on process exit.
    }
}
