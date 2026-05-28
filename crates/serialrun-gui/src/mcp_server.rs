use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};

// ── MCP Lifecycle ──

pub enum McpCommand {
    Start { bind_addr: String, port: u16 },
    Stop,
    Reconfigure { bind_addr: String, port: u16 },
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
}

impl McpHandle {
    pub fn start() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (status_tx, status_rx) = mpsc::channel();
        std::thread::spawn(move || mcp_manager(cmd_rx, status_tx));
        Self { cmd_tx, status_rx }
    }

    pub fn send(&self, cmd: McpCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    pub fn poll_status(&self) -> Option<McpStatus> {
        self.status_rx.try_recv().ok()
    }

    pub fn cmd_tx(&self) -> mpsc::Sender<McpCommand> {
        self.cmd_tx.clone()
    }
}

fn mcp_manager(cmd_rx: mpsc::Receiver<McpCommand>, status_tx: mpsc::Sender<McpStatus>) {
    let mut running = false;
    let mut stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut current_thread: Option<std::thread::JoinHandle<()>> = None;

    loop {
        match cmd_rx.recv() {
            Ok(McpCommand::Start { bind_addr, port }) => {
                if running { continue; }
                stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
                let sf = stop_flag.clone();
                let stx = status_tx.clone();
                let handle = std::thread::spawn(move || {
                    run_mcp_listener(&bind_addr, port, sf, stx);
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
                // Stop current
                if running {
                    stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    if let Some(h) = current_thread.take() { let _ = h.join(); }
                    running = false;
                    let _ = status_tx.send(McpStatus::Stopped);
                }
                // Start with new config
                stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
                let sf = stop_flag.clone();
                let stx = status_tx.clone();
                let handle = std::thread::spawn(move || {
                    run_mcp_listener(&bind_addr, port, sf, stx);
                });
                current_thread = Some(handle);
                running = true;
            }
            Err(_) => break,
        }
    }

    stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
    if let Some(h) = current_thread.take() { let _ = h.join(); }
}

fn run_mcp_listener(bind_addr: &str, port: u16, stop_flag: Arc<std::sync::atomic::AtomicBool>, status_tx: mpsc::Sender<McpStatus>) {
    let mcp = Arc::new(Mutex::new(SerialRunMcp::new()));

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
            Ok((stream, _)) => {
                let mcp = mcp.clone();
                std::thread::spawn(move || {
                    handle_client(stream, mcp);
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

struct SerialRunMcp {
    port: Option<serialrun_core::SerialPort>,
    connected_port: Option<String>,
}

impl SerialRunMcp {
    fn new() -> Self {
        Self { port: None, connected_port: None }
    }

    fn handle_request(&mut self, request: McpRequest) -> McpResponse {
        match request.method.as_str() {
            "initialize" => {
                let result = serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "serialrun-mcp",
                        "version": "0.1.0"
                    }
                });
                McpResponse::success(request.id, result)
            }
            "tools/list" => {
                let tools = serde_json::json!({
                    "tools": [
                        {
                            "name": "list_ports",
                            "description": "List available serial ports",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "connect",
                            "description": "Connect to a serial port",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "port": { "type": "string", "description": "Port name (e.g., COM1, /dev/ttyUSB0)" },
                                    "baud_rate": { "type": "integer", "description": "Baud rate (default: 115200)" }
                                },
                                "required": ["port"]
                            }
                        },
                        {
                            "name": "disconnect",
                            "description": "Disconnect from current serial port",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "send",
                            "description": "Send data to serial port",
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
                                    "timeout_ms": { "type": "integer", "description": "Read timeout in milliseconds (default: 1000)" },
                                    "max_bytes": { "type": "integer", "description": "Maximum bytes to read (default: 1024)" }
                                }
                            }
                        },
                        {
                            "name": "send_command",
                            "description": "Send a command and wait for response",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "command": { "type": "string", "description": "Command to send" },
                                    "timeout_ms": { "type": "integer", "description": "Response timeout in milliseconds (default: 1000)" }
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
                                    "quantity": { "type": "integer", "description": "Number of registers to read (default: 1)" }
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
                            "description": "Read all registers from a PLC preset (Siemens/Mitsubishi/Delta/Omron)",
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
                                    "value": { "type": "number", "description": "Value to write (will be scaled if register has scale_factor)" }
                                },
                                "required": ["address", "value"]
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

                        let config = serialrun_core::config::SerialConfig {
                            port_name: port_name.to_string(),
                            baud_rate,
                            ..Default::default()
                        };
                        let mut port = serialrun_core::SerialPort::new(config);
                        match port.connect() {
                            Ok(()) => {
                                self.port = Some(port);
                                self.connected_port = Some(port_name.to_string());
                                McpResponse::success(request.id, serde_json::json!({
                                    "content": [{ "type": "text", "text": format!("Connected to {} at {} baud", port_name, baud_rate) }]
                                }))
                            }
                            Err(e) => McpResponse::error(request.id, -1, e.to_string())
                        }
                    }
                    "disconnect" => {
                        if let Some(mut port) = self.port.take() {
                            let _ = port.disconnect();
                        }
                        self.connected_port = None;
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

                        let Some(ref mut port) = self.port else {
                            return McpResponse::error(request.id, -1, "Not connected".into());
                        };

                        let bytes = if hex {
                            data.split_whitespace()
                                .filter_map(|s| u8::from_str_radix(s, 16).ok())
                                .collect::<Vec<_>>()
                        } else {
                            data.as_bytes().to_vec()
                        };

                        match port.write(&bytes) {
                            Ok(n) => McpResponse::success(request.id, serde_json::json!({
                                "content": [{ "type": "text", "text": format!("Sent {} bytes", n) }]
                            })),
                            Err(e) => McpResponse::error(request.id, -1, e.to_string())
                        }
                    }
                    "read" => {
                        let _timeout_ms = arguments.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(1000) as u64;
                        let max_bytes = arguments.get("max_bytes").and_then(|v| v.as_u64()).unwrap_or(1024) as usize;

                        let Some(ref mut port) = self.port else {
                            return McpResponse::error(request.id, -1, "Not connected".into());
                        };

                        let mut buf = vec![0u8; max_bytes];
                        match port.read(&mut buf) {
                            Ok(n) => {
                                let data_hex = buf[..n].iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
                                let data_text = String::from_utf8_lossy(&buf[..n]).to_string();
                                McpResponse::success(request.id, serde_json::json!({
                                    "content": [{ "type": "text", "text": format!("Read {} bytes\nHEX: {}\nText: {}", n, data_hex, data_text) }]
                                }))
                            }
                            Err(e) => McpResponse::error(request.id, -1, e.to_string())
                        }
                    }
                    "send_command" => {
                        let command = arguments.get("command").and_then(|v| v.as_str()).unwrap_or("");
                        let timeout_ms = arguments.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(1000) as u64;

                        if command.is_empty() {
                            return McpResponse::error(request.id, -32602, "Command is required".into());
                        }

                        let Some(ref mut port) = self.port else {
                            return McpResponse::error(request.id, -1, "Not connected".into());
                        };

                        let mut cmd_bytes = command.as_bytes().to_vec();
                        if !command.ends_with("\r\n") && !command.ends_with('\n') && !command.ends_with('\r') {
                            cmd_bytes.extend_from_slice(b"\r\n");
                        }

                        if let Err(e) = port.write(&cmd_bytes) {
                            return McpResponse::error(request.id, -1, e.to_string());
                        }

                        std::thread::sleep(std::time::Duration::from_millis(timeout_ms));
                        let mut buf = vec![0u8; 4096];
                        match port.read(&mut buf) {
                            Ok(n) => {
                                let response = String::from_utf8_lossy(&buf[..n]).to_string();
                                McpResponse::success(request.id, serde_json::json!({
                                    "content": [{ "type": "text", "text": response }]
                                }))
                            }
                            Err(e) => McpResponse::error(request.id, -1, e.to_string())
                        }
                    }
                    "modbus_read" => {
                        let slave_id = arguments.get("slave_id").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
                        let address = match arguments.get("address").and_then(|v| v.as_u64()) {
                            Some(a) => a as u16,
                            None => return McpResponse::error(request.id, -32602, "address is required".into()),
                        };
                        let quantity = arguments.get("quantity").and_then(|v| v.as_u64()).unwrap_or(1) as u16;

                        let Some(ref mut port) = self.port else {
                            return McpResponse::error(request.id, -1, "Not connected".into());
                        };

                        use serialrun_core::protocol::{ModbusFrame, ModbusParser, ModbusFunction};
                        let frame = ModbusParser::build_read_request(slave_id, ModbusFunction::ReadHoldingRegisters, address, quantity);
                        let req = frame.to_bytes();
                        if let Err(e) = port.write(&req) {
                            return McpResponse::error(request.id, -1, format!("Write failed: {}", e));
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        let mut buf = [0u8; 256];
                        match port.read(&mut buf) {
                            Ok(n) if n >= 4 => {
                                match ModbusFrame::parse(&buf[..n]) {
                                    Ok(f) => {
                                        let hex = buf[..n].iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
                                        let mut values = Vec::new();
                                        let data = &f.data;
                                        let mut i = 1; // skip byte count
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

                        let Some(ref mut port) = self.port else {
                            return McpResponse::error(request.id, -1, "Not connected".into());
                        };

                        use serialrun_core::protocol::{ModbusFrame, ModbusParser};
                        let frame = ModbusParser::build_write_single(slave_id, address, value);
                        let req = frame.to_bytes();
                        if let Err(e) = port.write(&req) {
                            return McpResponse::error(request.id, -1, format!("Write failed: {}", e));
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        let mut buf = [0u8; 256];
                        match port.read(&mut buf) {
                            Ok(n) if n >= 4 => {
                                let hex = buf[..n].iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
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

                        let Some(ref mut port) = self.port else {
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
                            if port.write(&req).is_err() {
                                results.push(serde_json::json!({"addr": reg.addr, "name": reg.name, "error": "write failed"}));
                                continue;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(50));
                            let mut buf = [0u8; 256];
                            match port.read(&mut buf) {
                                Ok(n) if n >= 4 => {
                                    if let Ok(f) = ModbusFrame::parse(&buf[..n]) {
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

                        let Some(ref mut port) = self.port else {
                            return McpResponse::error(request.id, -1, "Not connected".into());
                        };

                        use serialrun_core::protocol::{ModbusFrame, ModbusParser};
                        let raw_val = value as u16;
                        let frame = ModbusParser::build_write_single(slave_id, address, raw_val);
                        let req = frame.to_bytes();
                        if let Err(e) = port.write(&req) {
                            return McpResponse::error(request.id, -1, format!("Write failed: {}", e));
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        let mut buf = [0u8; 256];
                        match port.read(&mut buf) {
                            Ok(n) if n >= 4 => {
                                McpResponse::success(request.id, serde_json::json!({
                                    "content": [{ "type": "text", "text": format!("Wrote {} to {} register 0x{:04X} (slave {})", value, brand_name, address, slave_id) }]
                                }))
                            }
                            _ => McpResponse::error(request.id, -1, "No response".into())
                        }
                    }
                    _ => McpResponse::error(request.id, -32601, format!("Unknown tool: {}", tool_name))
                }
            }
            "notifications/initialized" => {
                // Client notification, no response needed
                McpResponse::success(request.id, serde_json::json!({}))
            }
            _ => McpResponse::error(request.id, -32601, format!("Unknown method: {}", request.method))
        }
    }
}

fn handle_client(stream: TcpStream, mcp: Arc<Mutex<SerialRunMcp>>) {
    let reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = stream;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let request: McpRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                let response = McpResponse::error(None, -32700, format!("Parse error: {}", e));
                let _ = writeln!(writer, "{}", serde_json::to_string(&response).unwrap());
                let _ = writer.flush();
                continue;
            }
        };

        let mut mcp = mcp.lock().unwrap();
        let response = mcp.handle_request(request);

        let _ = writeln!(writer, "{}", serde_json::to_string(&response).unwrap());
        let _ = writer.flush();
    }
}
