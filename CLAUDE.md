# SerialTap - Agent Operation Manual

This document provides instructions for Claude Code agents to operate the SerialTap serial port assistant.

## Quick Start

### List Available Ports

```bash
# List all serial ports
serialtap list

# List in JSON format (for agent processing)
serialtap list --format json
```

### Connect to a Port

```bash
# Connect with default settings (115200 baud, 8N1)
serialtap connect /dev/ttyUSB0

# Connect with custom settings
serialtap connect COM1 -b 9600 -d 7 -s 2 -p odd -f hardware
```

### Send Data

```bash
# Send text data
serialtap send /dev/ttyUSB0 "Hello World\r\n"

# Send hex data
serialtap send /dev/ttyUSB0 "48 65 6C 6C 6F" --hex

# Send with specific baud rate
serialtap send COM1 "AT+RST\r\n" -b 115200
```

### Monitor Port

```bash
# Monitor with timestamps
serialtap monitor /dev/ttyUSB0 -t

# Monitor in hex mode
serialtap monitor COM1 -x

# Monitor with logging
serialtap monitor /dev/ttyUSB0 -t -l output.log
```

### Script Operations

```bash
# Record a script
serialtap record /dev/ttyUSB0 -o script.txt

# Replay a script
serialtap replay /dev/ttyUSB0 script.txt
```

## Agent Mode (JSON Output)

For automated operations, use the `agent` subcommand which outputs JSON:

### List Ports

```bash
serialtap agent list-ports
```

Output:
```json
{
  "success": true,
  "ports": [
    {
      "name": "/dev/ttyUSB0",
      "description": "USB Serial",
      "manufacturer": "FTDI",
      "vid": "0403",
      "pid": "6001"
    }
  ]
}
```

### Get Port Info

```bash
serialtap agent port-info /dev/ttyUSB0
```

### Send Data

```bash
serialtap agent /dev/ttyUSB0 send "Hello" -b 115200
```

Output:
```json
{
  "success": true,
  "bytes_written": 5
}
```

### Read Data

```bash
serialtap agent /dev/ttyUSB0 read --timeout 1000 --max-bytes 1024
```

Output:
```json
{
  "success": true,
  "bytes_read": 10,
  "data_hex": "48656C6C6F20576F726C64",
  "data_text": "Hello World"
}
```

### Run Script

```bash
serialtap agent /dev/ttyUSB0 run-script script.txt
```

## Common Tasks

### AT Command Testing

```bash
# List ports
serialtap list

# Connect and send AT commands
serialtap connect /dev/ttyUSB0 -b 115200

# In interactive mode:
> AT
> AT+RST
> AT+CWMODE=1
> AT+CWJAP="ssid","password"
```

### Modbus Communication

```bash
# Monitor Modbus traffic
serialtap monitor /dev/ttyUSB0 -x -t

# Send Modbus query (hex)
serialtap send /dev/ttyUSB0 "01 03 00 00 00 0A C5 CD" --hex
```

### Debugging

```bash
# Monitor with detailed logging
serialtap monitor COM1 -t -l debug.log

# Check port status
serialtap agent port-info COM1
```

## Troubleshooting

### Port Not Found

```bash
# Check available ports
serialtap list

# On Linux, check permissions
ls -la /dev/tty*
sudo usermod -a -G dialout $USER
```

### Connection Failed

```bash
# Verify port is not in use
serialtap agent port-info /dev/ttyUSB0

# Try different baud rate
serialtap connect /dev/ttyUSB0 -b 9600
```

### Permission Denied

```bash
# Linux/macOS: Add user to dialout group
sudo usermod -a -G dialout $USER

# Or run with sudo (not recommended for production)
sudo serialtap connect /dev/ttyUSB0
```

## API Reference

### SerialTap Core Library

The `serialtap-core` crate provides the core functionality:

```rust
use serialtap_core::{SerialConfig, SerialPort};

// Create config
let config = SerialConfig::new("/dev/ttyUSB0")
    .with_baud_rate(115200)
    .with_parity(Parity::None);

// Create and connect port
let mut port = SerialPort::new(config);
port.connect()?;

// Send data
port.write(b"Hello")?;

// Read data
let mut buf = [0u8; 1024];
let n = port.read(&mut buf)?;

// Disconnect
port.disconnect()?;
```

### Protocol Parsing

```rust
use serialtap_core::protocol::{ModbusParser, ModbusFrame};

// Parse Modbus frame
let frame = ModbusParser::parse_request(&data)?;

// Build Modbus request
let request = ModbusParser::build_read_request(
    0x01,
    ModbusFunction::ReadHoldingRegisters,
    0x0000,
    0x000A,
);
```

### Script Recording

```rust
use serialtap_core::{ScriptRecorder, ScriptReplayer};

// Record script
let mut recorder = ScriptRecorder::new("test", "Test script");
recorder.start();
recorder.record_send("AT+RST\r\n");
recorder.record_wait(1000);
recorder.stop();
recorder.save(Path::new("script.json"))?;

// Replay script
let mut replayer = ScriptReplayer::load(Path::new("script.json"))?;
replayer.start();
while let Some(cmd) = replayer.next_command() {
    // Execute command
}
```
