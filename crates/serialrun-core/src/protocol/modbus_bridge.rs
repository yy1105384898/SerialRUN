/// Modbus TCP/RTU Bridge.
///
/// Accepts Modbus TCP connections and forwards requests to a serial RTU device.
/// Each TCP client gets its own thread. The serial port is shared via Arc<Mutex<>>.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use super::modbus::ModbusFrame;
use super::modbus_tcp::ModbusTcpFrame;

pub struct BridgeConfig {
    pub tcp_port: u16,
    pub serial_port_name: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            tcp_port: 502,
            serial_port_name: String::new(),
            baud_rate: 9600,
            timeout_ms: 500,
        }
    }
}

pub struct BridgeLogEntry {
    pub timestamp: i64,
    pub client_addr: String,
    pub direction: String, // "TCP->RTU" or "RTU->TCP"
    pub request_hex: String,
    pub response_hex: String,
    pub success: bool,
}

/// Start the Modbus TCP/RTU bridge.
/// Returns (stop_flag, log_receiver, error_receiver).
pub fn start_bridge(
    config: BridgeConfig,
) -> Result<(Arc<AtomicBool>, mpsc::Receiver<BridgeLogEntry>, mpsc::Receiver<String>), String> {
    if config.serial_port_name.is_empty() {
        return Err("No serial port selected".into());
    }

    let stop = Arc::new(AtomicBool::new(false));
    let (log_tx, log_rx) = mpsc::channel();
    let (err_tx, err_rx) = mpsc::channel();

    let stop_clone = stop.clone();
    let serial_port_name = config.serial_port_name.clone();
    let baud_rate = config.baud_rate;
    let timeout_ms = config.timeout_ms;

    std::thread::spawn(move || {
        let addr = format!("0.0.0.0:{}", config.tcp_port);
        let listener = match TcpListener::bind(&addr) {
            Ok(l) => l,
            Err(e) => {
                let _ = err_tx.send(format!("Failed to bind {}: {}", addr, e));
                return;
            }
        };
        listener.set_nonblocking(true).ok();

        let _ = err_tx.send(format!("Bridge listening on {}", addr));

        // Shared serial port
        let serial_config = crate::config::SerialConfig {
            port_name: serial_port_name,
            baud_rate,
            ..Default::default()
        };
        let serial_port = Arc::new(Mutex::new(crate::SerialPort::new(serial_config)));

        // Connect serial port
        {
            let mut port = serial_port.lock().unwrap();
            if port.connect().is_err() {
                let _ = err_tx.send("Failed to open serial port".into());
                return;
            }
        }

        loop {
            if stop_clone.load(Ordering::Relaxed) {
                break;
            }

            match listener.accept() {
                Ok((stream, addr)) => {
                    stream.set_read_timeout(Some(Duration::from_millis(timeout_ms))).ok();
                    stream.set_write_timeout(Some(Duration::from_millis(timeout_ms))).ok();

                    let stop_c = stop_clone.clone();
                    let port_c = serial_port.clone();
                    let log_c = log_tx.clone();
                    let addr_str = addr.to_string();

                    std::thread::spawn(move || {
                        handle_tcp_client(stream, addr_str, stop_c, port_c, log_c);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
            }
        }

        // Disconnect serial
        let mut port = serial_port.lock().unwrap();
        let _ = port.disconnect();
    });

    Ok((stop, log_rx, err_rx))
}

fn handle_tcp_client(
    mut stream: TcpStream,
    client_addr: String,
    stop: Arc<AtomicBool>,
    serial_port: Arc<Mutex<crate::SerialPort>>,
    log_tx: mpsc::Sender<BridgeLogEntry>,
) {
    let mut header = [0u8; 6]; // MBAP header (without unit_id)

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        // Read MBAP header
        match stream.read_exact(&mut header) {
            Ok(()) => {}
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(_) => break, // Client disconnected
        }

        let transaction_id = u16::from_be_bytes([header[0], header[1]]);
        let _protocol_id = u16::from_be_bytes([header[2], header[3]]);
        let length = u16::from_be_bytes([header[4], header[5]]) as usize;

        if length < 1 || length > 256 {
            break;
        }

        let mut pdu = vec![0u8; length];
        if stream.read_exact(&mut pdu).is_err() {
            break;
        }

        // Reconstruct full TCP frame
        let mut full_tcp = Vec::with_capacity(6 + length);
        full_tcp.extend_from_slice(&header);
        full_tcp.extend_from_slice(&pdu);

        let tcp_frame = match ModbusTcpFrame::parse(&full_tcp) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let rtu_frame = tcp_frame.to_rtu_frame();
        let req_bytes = rtu_frame.to_bytes();
        let req_hex = hex_str(&req_bytes);

        // Send via serial
        let response = {
            let mut port = serial_port.lock().unwrap();
            if port.write(&req_bytes).is_err() {
                log_tx.send(BridgeLogEntry {
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    client_addr: client_addr.clone(),
                    direction: "TCP->RTU".into(),
                    request_hex: req_hex.clone(),
                    response_hex: String::new(),
                    success: false,
                }).ok();
                continue;
            }
            std::thread::sleep(Duration::from_millis(50));
            let mut buf = [0u8; 256];
            match port.read(&mut buf) {
                Ok(n) if n >= 4 => buf[..n].to_vec(),
                _ => {
                    log_tx.send(BridgeLogEntry {
                        timestamp: chrono::Utc::now().timestamp_millis(),
                        client_addr: client_addr.clone(),
                        direction: "TCP->RTU".into(),
                        request_hex: req_hex.clone(),
                        response_hex: String::new(),
                        success: false,
                    }).ok();
                    continue;
                }
            }
        };

        let resp_hex = hex_str(&response);

        // Convert RTU response to TCP response
        let rtu_resp = match ModbusFrame::parse(&response) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let tcp_resp = ModbusTcpFrame::from_rtu_frame(&rtu_resp, transaction_id);
        let resp_bytes = tcp_resp.to_bytes();

        if stream.write_all(&resp_bytes).is_err() {
            break;
        }

        log_tx.send(BridgeLogEntry {
            timestamp: chrono::Utc::now().timestamp_millis(),
            client_addr: client_addr.clone(),
            direction: "TCP->RTU".into(),
            request_hex: req_hex,
            response_hex: resp_hex,
            success: true,
        }).ok();
    }
}

fn hex_str(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")
}
