use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};

// ── MCP Lifecycle ──

pub enum McpCommand {
    Start { bind_addr: String, port: u16 },
    Stop,
    Reconfigure { bind_addr: String, port: u16 },
    /// Set the shared port_owner command sender for serial operations
    SetPortOwner(Option<std::sync::mpsc::Sender<crate::port_owner::PortCommand>>),
}

/// Access log entry sent from MCP server to GUI
#[derive(Clone, serde::Serialize)]
pub struct McpAccessLogEntry {
    pub timestamp: String,
    pub client_ip: String,
    pub action: String,
    pub detail: String,
}

pub enum McpStatus {
    Running { addr: String },
    Stopped,
    Error(String),
}

/// Handle to control the MCP server from the GUI.
pub struct McpHandle {
    cmd_tx: mpsc::Sender<McpCommand>,
    status_rx: mpsc::Receiver<McpStatus>,
    log_rx: mpsc::Receiver<McpAccessLogEntry>,
}

impl McpHandle {
    pub fn start() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (status_tx, status_rx) = mpsc::channel();
        let (log_tx, log_rx) = mpsc::channel();
        std::thread::spawn(move || mcp_manager(cmd_rx, status_tx, log_tx));
        Self { cmd_tx, status_rx, log_rx }
    }

    pub fn send(&self, cmd: McpCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    pub fn poll_status(&self) -> Option<McpStatus> {
        self.status_rx.try_recv().ok()
    }

    pub fn poll_log(&self) -> Option<McpAccessLogEntry> {
        self.log_rx.try_recv().ok()
    }

    pub fn cmd_tx(&self) -> mpsc::Sender<McpCommand> {
        self.cmd_tx.clone()
    }
}

/// Shared state between MCP manager and handler threads
struct McpShared {
    port_owner: Option<mpsc::Sender<crate::port_owner::PortCommand>>,
    /// Active client count for concurrency control
    active_clients: std::sync::atomic::AtomicUsize,
    /// Access log entries (IP, time, action)
    access_log: std::sync::Mutex<Vec<AccessLogEntry>>,
}

#[derive(Clone, serde::Serialize)]
struct AccessLogEntry {
    timestamp: String,
    client_ip: String,
    action: String,
    detail: String,
}

fn mcp_manager(cmd_rx: mpsc::Receiver<McpCommand>, status_tx: mpsc::Sender<McpStatus>, log_tx: mpsc::Sender<McpAccessLogEntry>) {
    let mut running = false;
    let mut stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut current_thread: Option<std::thread::JoinHandle<()>> = None;
    let shared = Arc::new(Mutex::new(McpShared {
        port_owner: None,
        active_clients: std::sync::atomic::AtomicUsize::new(0),
        access_log: std::sync::Mutex::new(Vec::new()),
    }));

    loop {
        match cmd_rx.recv() {
            Ok(McpCommand::Start { bind_addr, port }) => {
                if running { continue; }
                stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
                let sf = stop_flag.clone();
                let stx = status_tx.clone();
                let sh = shared.clone();
                let ltx = log_tx.clone();
                let handle = std::thread::spawn(move || {
                    run_mcp_listener(&bind_addr, port, sf, stx, sh, ltx);
                });
                current_thread = Some(handle);
                running = true;
            }
            Ok(McpCommand::Stop) => {
                if !running { continue; }
                stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                if let Some(h) = current_thread.take() { let _ = h.join(); }
                running = false;
                let _ = status_tx.send(McpStatus::Stopped);
            }
            Ok(McpCommand::Reconfigure { bind_addr, port }) => {
                if running {
                    stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    if let Some(h) = current_thread.take() { let _ = h.join(); }
                    running = false;
                    let _ = status_tx.send(McpStatus::Stopped);
                }
                stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
                let sf = stop_flag.clone();
                let stx = status_tx.clone();
                let sh = shared.clone();
                let ltx = log_tx.clone();
                let handle = std::thread::spawn(move || {
                    run_mcp_listener(&bind_addr, port, sf, stx, sh, ltx);
                });
                current_thread = Some(handle);
                running = true;
            }
            Ok(McpCommand::SetPortOwner(po)) => {
                if let Ok(mut sh) = shared.lock() {
                    sh.port_owner = po;
                }
            }
            Err(_) => break,
        }
    }

    stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
    if let Some(h) = current_thread.take() { let _ = h.join(); }
}

fn run_mcp_listener(
    bind_addr: &str,
    port: u16,
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
    status_tx: mpsc::Sender<McpStatus>,
    shared: Arc<Mutex<McpShared>>,
    log_tx: mpsc::Sender<McpAccessLogEntry>,
) {
    let addr = format!("{}:{}", bind_addr, port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            let _ = status_tx.send(McpStatus::Error(format!("Failed to bind: {}", e)));
            return;
        }
    };
    listener.set_nonblocking(true).ok();

    let _ = status_tx.send(McpStatus::Running { addr: addr.clone() });
    eprintln!("MCP server listening on {}", addr);

    loop {
        if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        match listener.accept() {
            Ok((mut stream, addr)) => {
                // Accept inherits non-blocking from listener; reset to blocking for read/write
                let _ = stream.set_nonblocking(false);
                let client_ip = addr.ip().to_string();
                let shared = shared.clone();
                let log_tx = log_tx.clone();
                std::thread::spawn(move || {
                    handle_client(stream, shared, client_ip, log_tx);
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }
        }
    }

    let _ = status_tx.send(McpStatus::Stopped);
    eprintln!("MCP server stopped");
}

#[derive(Serialize, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
struct McpResponse {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Serialize, Deserialize)]
struct McpError {
    code: i32,
    message: String,
}

impl McpResponse {
    fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
    }
    fn error(id: Option<serde_json::Value>, code: i32, message: String) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: None, error: Some(McpError { code, message }) }
    }
}

/// Helper: send a WriteRead command through port_owner and wait for response
fn port_write_read(
    po: &mpsc::Sender<crate::port_owner::PortCommand>,
    data: Vec<u8>,
    timeout_ms: u64,
) -> Result<Vec<u8>, String> {
    let (resp_tx, resp_rx) = mpsc::channel();
    let _ = po.send(crate::port_owner::PortCommand::WriteRead { data, timeout_ms, resp_tx });
    resp_rx.recv().unwrap_or_else(|e| Err(format!("Channel closed: {}", e)))
}

/// Helper: send a Write command through port_owner
fn port_write(
    po: &mpsc::Sender<crate::port_owner::PortCommand>,
    data: Vec<u8>,
) {
    let _ = po.send(crate::port_owner::PortCommand::Write(data));
}

fn handle_request(
    request: McpRequest,
    shared: &Mutex<McpShared>,
) -> McpResponse {
    match request.method.as_str() {
        "initialize" => {
            let result = serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "serialrun-mcp", "version": "0.2.0" }
            });
            McpResponse::success(request.id, result)
        }
        "tools/list" => {
            let tools = serde_json::json!({
                "tools": [
                    {
                        "name": "list_ports",
                        "description": "List available serial ports",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "connect",
                        "description": "Connect to a serial port via the GUI's port owner",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "port": { "type": "string", "description": "Port name (e.g., COM1, /dev/ttyUSB0)" },
                                "baud_rate": { "type": "integer", "description": "Baud rate (default: 115200)" },
                                "data_bits": { "type": "integer", "description": "Data bits: 5, 6, 7, 8 (default: 8)" },
                                "stop_bits": { "type": "integer", "description": "Stop bits: 1, 2 (default: 1)" },
                                "parity": { "type": "string", "description": "Parity: None, Odd, Even (default: None)" }
                            },
                            "required": ["port"]
                        }
                    },
                    {
                        "name": "disconnect",
                        "description": "Disconnect from current serial port",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "send",
                        "description": "Send data to serial port (text or hex)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "data": { "type": "string", "description": "Data to send (text or hex string)" },
                                "hex": { "type": "boolean", "description": "If true, data is interpreted as hex" }
                            },
                            "required": ["data"]
                        }
                    },
                    {
                        "name": "read",
                        "description": "Read data from serial port",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "timeout_ms": { "type": "integer", "description": "Read timeout in ms (default: 1000)" },
                                "max_bytes": { "type": "integer", "description": "Maximum bytes to read (default: 1024)" }
                            }
                        }
                    },
                    {
                        "name": "send_command",
                        "description": "Send a command and wait for response (write-read)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "command": { "type": "string", "description": "Command to send" },
                                "timeout_ms": { "type": "integer", "description": "Response timeout in ms (default: 1000)" }
                            },
                            "required": ["command"]
                        }
                    },
                    {
                        "name": "modbus_read",
                        "description": "Read Modbus RTU holding registers",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "slave_id": { "type": "integer", "description": "Slave ID (1-247)", "default": 1 },
                                "address": { "type": "integer", "description": "Start register address" },
                                "quantity": { "type": "integer", "description": "Number of registers (default: 1)" }
                            },
                            "required": ["address"]
                        }
                    },
                    {
                        "name": "modbus_write",
                        "description": "Write a Modbus RTU holding register",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "slave_id": { "type": "integer", "description": "Slave ID (1-247)", "default": 1 },
                                "address": { "type": "integer", "description": "Register address" },
                                "value": { "type": "integer", "description": "Value to write (u16)" }
                            },
                            "required": ["address", "value"]
                        }
                    },
                    {
                        "name": "plc_read",
                        "description": "Read all registers from a PLC preset",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "brand": { "type": "string", "description": "PLC brand: Siemens, Mitsubishi, Delta, Omron", "default": "Siemens" },
                                "slave_id": { "type": "integer", "description": "Slave ID (1-247)", "default": 1 }
                            }
                        }
                    },
                    {
                        "name": "plc_write",
                        "description": "Write to a PLC register by address",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "brand": { "type": "string", "description": "PLC brand: Siemens, Mitsubishi, Delta, Omron", "default": "Siemens" },
                                "slave_id": { "type": "integer", "description": "Slave ID (1-247)", "default": 1 },
                                "address": { "type": "integer", "description": "Register address" },
                                "value": { "type": "number", "description": "Value to write" }
                            },
                            "required": ["address", "value"]
                        }
                    },
                    {
                        "name": "get_access_log",
                        "description": "Get MCP access log (client IPs, tool calls, timestamps)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "limit": { "type": "integer", "description": "Max entries to return (default: 50)" }
                            }
                        }
                    }
                ]
            });
            McpResponse::success(request.id, tools)
        }
        "tools/call" => {
            let tool_name = request.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = request.params.get("arguments").cloned().unwrap_or(serde_json::json!({}));

            match tool_name {
                "list_ports" => {
                    match serialrun_core::SerialPort::list_ports() {
                        Ok(ports) => {
                            let ports_json: Vec<serde_json::Value> = ports.iter().map(|p| {
                                serde_json::json!({
                                    "name": p.name,
                                    "description": p.description,
                                    "manufacturer": p.manufacturer,
                                })
                            }).collect();
                            McpResponse::success(request.id, serde_json::json!({
                                "content": [{ "type": "text", "text": serde_json::to_string_pretty(&ports_json).unwrap() }]
                            }))
                        }
                        Err(e) => McpResponse::error(request.id, -1, e.to_string())
                    }
                }
                "connect" => {
                    let port_name = arguments.get("port").and_then(|v| v.as_str()).unwrap_or("");
                    let baud_rate = arguments.get("baud_rate").and_then(|v| v.as_u64()).unwrap_or(115200) as u32;

                    if port_name.is_empty() {
                        return McpResponse::error(request.id, -32602, "Port name is required".into());
                    }

                    let sh = match shared.lock() {
                        Ok(sh) => sh,
                        Err(_) => return McpResponse::error(request.id, -1, "Internal error".into()),
                    };
                    let Some(ref po) = sh.port_owner else {
                        return McpResponse::error(request.id, -1, "Not connected to port owner. Connect via GUI first.".into());
                    };

                    let config = serialrun_core::config::SerialConfig {
                        port_name: port_name.to_string(),
                        baud_rate,
                        ..Default::default()
                    };
                    let _ = po.send(crate::port_owner::PortCommand::Open(config));
                    // Wait for connection result via a one-shot channel
                    let (verify_tx, verify_rx) = mpsc::channel();
                    let po_clone = po.clone();
                    std::thread::spawn(move || {
                        // Poll for Opened event by sending a test WriteRead
                        std::thread::sleep(std::time::Duration::from_millis(300));
                        let (rtx, rrx) = mpsc::channel();
                        let _ = po_clone.send(crate::port_owner::PortCommand::WriteRead {
                            data: vec![],
                            timeout_ms: 100,
                            resp_tx: rtx,
                        });
                        let result = rrx.recv().unwrap_or_else(|_| Err("Timeout".into()));
                        let _ = verify_tx.send(result.is_ok());
                    });
                    let connected = verify_rx.recv().unwrap_or(false);
                    if connected {
                        McpResponse::success(request.id, serde_json::json!({
                            "content": [{ "type": "text", "text": format!("Connected to {} at {} baud", port_name, baud_rate) }]
                        }))
                    } else {
                        McpResponse::error(request.id, -1, format!("Failed to connect to {}", port_name))
                    }
                }
                "disconnect" => {
                    let sh = match shared.lock() {
                        Ok(sh) => sh,
                        Err(_) => return McpResponse::error(request.id, -1, "Internal error".into()),
                    };
                    let Some(ref po) = sh.port_owner else {
                        return McpResponse::error(request.id, -1, "Not connected to port owner".into());
                    };
                    let _ = po.send(crate::port_owner::PortCommand::Close);
                    McpResponse::success(request.id, serde_json::json!({
                        "content": [{ "type": "text", "text": "Disconnected" }]
                    }))
                }
                "send" => {
                    let data = arguments.get("data").and_then(|v| v.as_str()).unwrap_or("");
                    let hex = arguments.get("hex").and_then(|v| v.as_bool()).unwrap_or(false);

                    if data.is_empty() {
                        return McpResponse::error(request.id, -32602, "Data is required".into());
                    }

                    let sh = match shared.lock() {
                        Ok(sh) => sh,
                        Err(_) => return McpResponse::error(request.id, -1, "Internal error".into()),
                    };
                    let Some(ref po) = sh.port_owner else {
                        return McpResponse::error(request.id, -1, "Not connected".into());
                    };

                    let bytes = if hex {
                        data.split_whitespace()
                            .filter_map(|s| u8::from_str_radix(s, 16).ok())
                            .collect::<Vec<_>>()
                    } else {
                        data.as_bytes().to_vec()
                    };

                    let len = bytes.len();
                    let _ = po.send(crate::port_owner::PortCommand::Write(bytes));
                    McpResponse::success(request.id, serde_json::json!({
                        "content": [{ "type": "text", "text": format!("Sent {} bytes", len) }]
                    }))
                }
                "read" => {
                    let timeout_ms = arguments.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(1000);

                    let sh = match shared.lock() {
                        Ok(sh) => sh,
                        Err(_) => return McpResponse::error(request.id, -1, "Internal error".into()),
                    };
                    let Some(ref po) = sh.port_owner else {
                        return McpResponse::error(request.id, -1, "Not connected".into());
                    };

                    // Use WriteRead with empty data to just read
                    match port_write_read(po, vec![], timeout_ms) {
                        Ok(data) => {
                            let data_hex = data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
                            let data_text = String::from_utf8_lossy(&data).to_string();
                            McpResponse::success(request.id, serde_json::json!({
                                "content": [{ "type": "text", "text": format!("Read {} bytes\nHEX: {}\nText: {}", data.len(), data_hex, data_text) }]
                            }))
                        }
                        Err(e) => McpResponse::error(request.id, -1, e)
                    }
                }
                "send_command" => {
                    let command = arguments.get("command").and_then(|v| v.as_str()).unwrap_or("");
                    let timeout_ms = arguments.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(1000);

                    if command.is_empty() {
                        return McpResponse::error(request.id, -32602, "Command is required".into());
                    }

                    let sh = match shared.lock() {
                        Ok(sh) => sh,
                        Err(_) => return McpResponse::error(request.id, -1, "Internal error".into()),
                    };
                    let Some(ref po) = sh.port_owner else {
                        return McpResponse::error(request.id, -1, "Not connected".into());
                    };

                    let mut cmd_bytes = command.as_bytes().to_vec();
                    if !command.ends_with("\r\n") && !command.ends_with('\n') && !command.ends_with('\r') {
                        cmd_bytes.extend_from_slice(b"\r\n");
                    }

                    match port_write_read(po, cmd_bytes, timeout_ms) {
                        Ok(data) => {
                            let response = String::from_utf8_lossy(&data).to_string();
                            McpResponse::success(request.id, serde_json::json!({
                                "content": [{ "type": "text", "text": response }]
                            }))
                        }
                        Err(e) => McpResponse::error(request.id, -1, e)
                    }
                }
                "modbus_read" => {
                    let slave_id = arguments.get("slave_id").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
                    let address = match arguments.get("address").and_then(|v| v.as_u64()) {
                        Some(a) => a as u16,
                        None => return McpResponse::error(request.id, -32602, "address is required".into()),
                    };
                    let quantity = arguments.get("quantity").and_then(|v| v.as_u64()).unwrap_or(1) as u16;

                    let sh = match shared.lock() {
                        Ok(sh) => sh,
                        Err(_) => return McpResponse::error(request.id, -1, "Internal error".into()),
                    };
                    let Some(ref po) = sh.port_owner else {
                        return McpResponse::error(request.id, -1, "Not connected".into());
                    };

                    use serialrun_core::protocol::{ModbusFrame, ModbusParser, ModbusFunction};
                    let frame = ModbusParser::build_read_request(slave_id, ModbusFunction::ReadHoldingRegisters, address, quantity);
                    let req = frame.to_bytes();

                    match port_write_read(po, req, 200) {
                        Ok(resp) if resp.len() >= 4 => {
                            match ModbusFrame::parse(&resp) {
                                Ok(f) => {
                                    let hex = resp.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
                                    let mut values = Vec::new();
                                    let data = &f.data;
                                    let mut i = 1;
                                    while i + 1 < data.len() {
                                        values.push(u16::from_be_bytes([data[i], data[i+1]]));
                                        i += 2;
                                    }
                                    McpResponse::success(request.id, serde_json::json!({
                                        "content": [{ "type": "text", "text": format!("Read {} registers from slave {}\nHEX: {}\nValues: {:?}", quantity, slave_id, hex, values) }]
                                    }))
                                }
                                Err(e) => McpResponse::error(request.id, -1, format!("Parse error: {}", e))
                            }
                        }
                        _ => McpResponse::error(request.id, -1, "No response".into())
                    }
                }
                "modbus_write" => {
                    let slave_id = arguments.get("slave_id").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
                    let address = match arguments.get("address").and_then(|v| v.as_u64()) {
                        Some(a) => a as u16,
                        None => return McpResponse::error(request.id, -32602, "address is required".into()),
                    };
                    let value = match arguments.get("value").and_then(|v| v.as_u64()) {
                        Some(v) => v as u16,
                        None => return McpResponse::error(request.id, -32602, "value is required".into()),
                    };

                    let sh = match shared.lock() {
                        Ok(sh) => sh,
                        Err(_) => return McpResponse::error(request.id, -1, "Internal error".into()),
                    };
                    let Some(ref po) = sh.port_owner else {
                        return McpResponse::error(request.id, -1, "Not connected".into());
                    };

                    use serialrun_core::protocol::ModbusParser;
                    let frame = ModbusParser::build_write_single(slave_id, address, value);
                    let req = frame.to_bytes();

                    match port_write_read(po, req, 200) {
                        Ok(resp) if resp.len() >= 4 => {
                            let hex = resp.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
                            McpResponse::success(request.id, serde_json::json!({
                                "content": [{ "type": "text", "text": format!("Wrote {} to register 0x{:04X} (slave {})\nResponse: {}", value, address, slave_id, hex) }]
                            }))
                        }
                        _ => McpResponse::error(request.id, -1, "No response".into())
                    }
                }
                "plc_read" => {
                    let brand_name = arguments.get("brand").and_then(|v| v.as_str()).unwrap_or("Siemens");
                    let slave_id = arguments.get("slave_id").and_then(|v| v.as_u64()).unwrap_or(1) as u8;

                    let brand = match brand_name {
                        "Siemens" | "siemens" => crate::state::PlcBrand::Siemens,
                        "Mitsubishi" | "mitsubishi" => crate::state::PlcBrand::Mitsubishi,
                        "Delta" | "delta" => crate::state::PlcBrand::Delta,
                        "Omron" | "omron" => crate::state::PlcBrand::Omron,
                        _ => return McpResponse::error(request.id, -32602, format!("Unknown brand: {}. Use Siemens, Mitsubishi, Delta, Omron", brand_name)),
                    };

                    let models = crate::plc_presets::get_models(brand);
                    let regs = models.first().map(|m| m.registers.clone()).unwrap_or_default();
                    if regs.is_empty() {
                        return McpResponse::error(request.id, -1, "No registers defined for this brand".into());
                    }

                    let sh = match shared.lock() {
                        Ok(sh) => sh,
                        Err(_) => return McpResponse::error(request.id, -1, "Internal error".into()),
                    };
                    let Some(ref po) = sh.port_owner else {
                        return McpResponse::error(request.id, -1, "Not connected".into());
                    };

                    use serialrun_core::protocol::{ModbusFrame, ModbusParser, ModbusFunction};
                    let mut results = Vec::new();
                    for reg in &regs {
                        let qty = match reg.data_type {
                            crate::state::PlcDataType::U32 | crate::state::PlcDataType::Float32 => 2,
                            _ => 1,
                        };
                        let frame = ModbusParser::build_read_request(slave_id, ModbusFunction::ReadHoldingRegisters, reg.addr, qty);
                        let req = frame.to_bytes();
                        match port_write_read(po, req, 200) {
                            Ok(resp) if resp.len() >= 4 => {
                                if let Ok(f) = ModbusFrame::parse(&resp) {
                                    let data = &f.data;
                                    let val_str = match reg.data_type {
                                        crate::state::PlcDataType::Bool => {
                                            let raw = data.get(1).copied().unwrap_or(0);
                                            if raw != 0 { "ON".into() } else { "OFF".into() }
                                        }
                                        crate::state::PlcDataType::U16 => {
                                            let raw = data.get(1..3).map(|d| u16::from_be_bytes([d[0], d[1]])).unwrap_or(0);
                                            if reg.scale_factor != 1.0 { format!("{:.2}", raw as f64 * reg.scale_factor) } else { format!("{}", raw) }
                                        }
                                        crate::state::PlcDataType::I16 => {
                                            let raw = data.get(1..3).map(|d| u16::from_be_bytes([d[0], d[1]])).unwrap_or(0) as i16;
                                            if reg.scale_factor != 1.0 { format!("{:.2}", raw as f64 * reg.scale_factor) } else { format!("{}", raw) }
                                        }
                                        crate::state::PlcDataType::U32 => {
                                            let raw = data.get(1..5).map(|d| u32::from_be_bytes([d[0], d[1], d[2], d[3]])).unwrap_or(0);
                                            if reg.scale_factor != 1.0 { format!("{:.2}", raw as f64 * reg.scale_factor) } else { format!("{}", raw) }
                                        }
                                        crate::state::PlcDataType::Float32 => {
                                            let raw = data.get(1..5).map(|d| u32::from_be_bytes([d[0], d[1], d[2], d[3]])).unwrap_or(0);
                                            let f = f32::from_bits(raw);
                                            if reg.scale_factor != 1.0 { format!("{:.3}", f as f64 * reg.scale_factor) } else { format!("{:.3}", f) }
                                        }
                                    };
                                    results.push(serde_json::json!({"addr": reg.addr, "name": reg.name, "type": reg.data_type.label(), "value": val_str, "unit": reg.unit}));
                                } else {
                                    results.push(serde_json::json!({"addr": reg.addr, "name": reg.name, "error": "parse error"}));
                                }
                            }
                            _ => {
                                results.push(serde_json::json!({"addr": reg.addr, "name": reg.name, "error": "no response"}));
                            }
                        }
                    }

                    McpResponse::success(request.id, serde_json::json!({
                        "content": [{ "type": "text", "text": format!("{} PLC ({}) slave {} - {} registers:\n{}", brand_name, models.first().map(|m| m.model).unwrap_or("?"), slave_id, results.len(), serde_json::to_string_pretty(&results).unwrap()) }]
                    }))
                }
                "plc_write" => {
                    let brand_name = arguments.get("brand").and_then(|v| v.as_str()).unwrap_or("Siemens");
                    let slave_id = arguments.get("slave_id").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
                    let address = match arguments.get("address").and_then(|v| v.as_u64()) {
                        Some(a) => a as u16,
                        None => return McpResponse::error(request.id, -32602, "address is required".into()),
                    };
                    let value = match arguments.get("value") {
                        Some(v) => v.as_f64().unwrap_or(0.0),
                        None => return McpResponse::error(request.id, -32602, "value is required".into()),
                    };

                    let _brand = match brand_name {
                        "Siemens" | "siemens" => crate::state::PlcBrand::Siemens,
                        "Mitsubishi" | "mitsubishi" => crate::state::PlcBrand::Mitsubishi,
                        "Delta" | "delta" => crate::state::PlcBrand::Delta,
                        "Omron" | "omron" => crate::state::PlcBrand::Omron,
                        _ => return McpResponse::error(request.id, -32602, format!("Unknown brand: {}", brand_name)),
                    };

                    let sh = match shared.lock() {
                        Ok(sh) => sh,
                        Err(_) => return McpResponse::error(request.id, -1, "Internal error".into()),
                    };
                    let Some(ref po) = sh.port_owner else {
                        return McpResponse::error(request.id, -1, "Not connected".into());
                    };

                    use serialrun_core::protocol::ModbusParser;
                    let raw_val = value as u16;
                    let frame = ModbusParser::build_write_single(slave_id, address, raw_val);
                    let req = frame.to_bytes();

                    match port_write_read(po, req, 200) {
                        Ok(resp) if resp.len() >= 4 => {
                            McpResponse::success(request.id, serde_json::json!({
                                "content": [{ "type": "text", "text": format!("Wrote {} to {} register 0x{:04X} (slave {})", value, brand_name, address, slave_id) }]
                            }))
                        }
                        _ => McpResponse::error(request.id, -1, "No response".into())
                    }
                }
                "get_access_log" => {
                    let limit = arguments.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
                    let sh = match shared.lock() {
                        Ok(sh) => sh,
                        Err(_) => return McpResponse::error(request.id, -1, "Internal error".into()),
                    };
                    let active = sh.active_clients.load(std::sync::atomic::Ordering::Relaxed);
                    let log_entries = match sh.access_log.lock() {
                        Ok(log) => {
                            let start = if log.len() > limit { log.len() - limit } else { 0 };
                            log[start..].iter().map(|e| {
                                serde_json::json!({
                                    "time": e.timestamp,
                                    "ip": e.client_ip,
                                    "action": e.action,
                                    "detail": e.detail,
                                })
                            }).collect::<Vec<_>>()
                        }
                        Err(_) => vec![],
                    };
                    McpResponse::success(request.id, serde_json::json!({
                        "content": [{ "type": "text", "text": format!("Active clients: {}\n\nAccess Log (last {}):\n{}", active, log_entries.len(), serde_json::to_string_pretty(&log_entries).unwrap()) }]
                    }))
                }
                _ => McpResponse::error(request.id, -32601, format!("Unknown tool: {}", tool_name))
            }
        }
        "notifications/initialized" => {
            McpResponse::success(request.id, serde_json::json!({}))
        }
        _ => McpResponse::error(request.id, -32601, format!("Unknown method: {}", request.method))
    }
}

fn handle_client(mut stream: TcpStream, shared: Arc<Mutex<McpShared>>, client_ip: String, log_tx: mpsc::Sender<McpAccessLogEntry>) {
    // Log client connect
    log_access(&shared, &client_ip, "CONNECT", "client connected", &log_tx);
    eprintln!("[MCP] Client connected: {}", client_ip);

    // Increment active client count
    {
        let sh = shared.lock().unwrap();
        sh.active_clients.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];

    loop {
        let n = match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        buf.extend_from_slice(&tmp[..n]);

        // Process all complete lines in buffer
        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line_bytes).trim().to_string();
            if line.is_empty() { continue; }

            let request: McpRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    let response = McpResponse::error(None, -32700, format!("Parse error: {}", e));
                    let _ = write!(stream, "{}\n", serde_json::to_string(&response).unwrap());
                    let _ = stream.flush();
                    continue;
                }
            };

            // Log tool call
            let tool_name = request.params.get("name").and_then(|v| v.as_str()).unwrap_or(&request.method);
            log_access(&shared, &client_ip, "CALL", tool_name, &log_tx);

            let response = handle_request(request, &shared);
            let _ = write!(stream, "{}\n", serde_json::to_string(&response).unwrap());
            let _ = stream.flush();
        }
    }

    // Decrement active client count and log disconnect
    {
        let sh = shared.lock().unwrap();
        sh.active_clients.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
    log_access(&shared, &client_ip, "DISCONNECT", "client disconnected", &log_tx);
    eprintln!("[MCP] Client disconnected: {}", client_ip);
}

fn log_access(shared: &Arc<Mutex<McpShared>>, client_ip: &str, action: &str, detail: &str, log_tx: &mpsc::Sender<McpAccessLogEntry>) {
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

    // Send to GUI via channel
    let gui_entry = McpAccessLogEntry {
        timestamp: ts.clone(),
        client_ip: client_ip.to_string(),
        action: action.to_string(),
        detail: detail.to_string(),
    };
    let _ = log_tx.send(gui_entry);

    // Also store in shared log for get_access_log tool
    let entry = AccessLogEntry {
        timestamp: ts,
        client_ip: client_ip.to_string(),
        action: action.to_string(),
        detail: detail.to_string(),
    };
    if let Ok(sh) = shared.lock() {
        if let Ok(mut log) = sh.access_log.lock() {
            log.push(entry);
            if log.len() > 500 {
                log.drain(0..100);
            }
        }
    }
}
