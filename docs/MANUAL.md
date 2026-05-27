# SerialTap User Manual

## Table of Contents

1. [Introduction](#introduction)
2. [Installation](#installation)
3. [Getting Started](#getting-started)
4. [CLI Reference](#cli-reference)
5. [GUI Guide](#gui-guide)
6. [Advanced Features](#advanced-features)
7. [Troubleshooting](#troubleshooting)
8. [FAQ](#faq)

## Introduction

SerialTap is a powerful, cross-platform serial port assistant designed for embedded developers. It provides both command-line and graphical interfaces for serial communication, protocol analysis, and automation.

### Key Features

- **Cross-platform**: Works on Windows, macOS, Linux, iOS, and Android
- **Dual Interface**: CLI for automation, GUI for interactive use
- **Protocol Support**: Built-in Modbus parser, extensible custom protocols
- **Scripting**: Record and replay serial communication sessions
- **File Transfer**: XMODEM, YMODEM, ZMODEM support
- **Plugin System**: Extend functionality with custom plugins

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/SerialTap.git
cd SerialTap

# Build CLI
cargo build --release -p serialtap-cli

# Build GUI
cargo build --release -p serialtap-gui

# The binaries will be in target/release/
```

### Pre-built Binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/yourusername/SerialTap/releases).

## Getting Started

### First Connection

1. Connect your serial device to your computer
2. List available ports:
   ```bash
   serialtap list
   ```
3. Connect to the port:
   ```bash
   serialtap connect /dev/ttyUSB0 -b 115200
   ```
4. Start communicating:
   ```
   > AT
   > AT+RST
   ```

### Quick Commands

```bash
# Send a command
serialtap send /dev/ttyUSB0 "Hello\r\n"

# Monitor traffic
serialtap monitor /dev/ttyUSB0 -t

# Record a session
serialtap record /dev/ttyUSB0 -o session.txt
```

## CLI Reference

### Global Options

- `--help`: Show help information
- `--version`: Show version
- `--format <fmt>`: Output format (text, json)

### Commands

#### `list`

List all available serial ports.

```bash
serialtap list [--format json]
```

#### `connect`

Connect to a serial port in interactive mode.

```bash
serialtap connect <port> [options]

Options:
  -b, --baud <rate>      Baud rate (default: 115200)
  -d, --data-bits <bits> Data bits (5, 6, 7, 8; default: 8)
  -s, --stop-bits <bits> Stop bits (1, 2; default: 1)
  -p, --parity <parity>  Parity (none, odd, even; default: none)
  -f, --flow <control>   Flow control (none, software, hardware; default: none)
```

#### `send`

Send data to a serial port.

```bash
serialtap send <port> <data> [options]

Options:
  -b, --baud <rate>    Baud rate (default: 115200)
  -x, --hex            Send as hex data
```

#### `monitor`

Monitor serial port data.

```bash
serialtap monitor <port> [options]

Options:
  -b, --baud <rate>    Baud rate (default: 115200)
  -l, --log <file>     Log to file
  -t, --timestamp      Show timestamps
  -x, --hex            Hex mode
```

#### `replay`

Replay a script file.

```bash
serialtap replay <port> <script> [options]

Options:
  -b, --baud <rate>    Baud rate (default: 115200)
```

#### `record`

Record a script to a file.

```bash
serialtap record <port> [options]

Options:
  -o, --output <file>  Output file (default: script.txt)
  -b, --baud <rate>    Baud rate (default: 115200)
```

#### `agent`

Agent mode for automation (JSON output).

```bash
serialtap agent [port] <action>

Actions:
  list-ports           List all ports
  port-info <name>     Get port information
  send <data>          Send data
  read                 Read data
  run-script <script>  Run a script
```

## GUI Guide

### Launching the GUI

```bash
serialtap-gui
```

### Interface Overview

1. **Top Bar**: Connection controls, port selection, baud rate
2. **Left Panel**: Settings and configuration
3. **Center Panel**: Terminal display
4. **Bottom Bar**: Status information

### Connecting

1. Click "Refresh Ports" to detect devices
2. Select a port from the dropdown
3. Choose baud rate and other settings
4. Click "Connect"

### Terminal Display

- **Text Mode**: Default UTF-8 display
- **HEX Mode**: Toggle for hexadecimal view
- **Timestamps**: Enable/disable time display
- **Auto-scroll**: Follow new data automatically

### Sending Data

1. Type in the input field at the bottom
2. Press Enter or click "Send"
3. Data appears in the terminal with TX prefix

### Charts

- Click "Chart" to open data visualization
- Shows real-time data rates
- Displays transfer statistics

### Logging

- Click "Log" to open log viewer
- View application events
- Export logs for analysis

## Advanced Features

### Script Recording

1. Click "Start Recording" in settings
2. Perform serial operations
3. Click "Stop Recording"
4. Scripts save as JSON or text format

### Script Format

**JSON Format**:
```json
{
  "name": "test_script",
  "description": "Test sequence",
  "commands": [
    {"delay_ms": 0, "action": "Send", "data": "AT+RST"},
    {"delay_ms": 1000, "action": "Wait"},
    {"delay_ms": 0, "action": "Send", "data": "AT+CWMODE=1"}
  ]
}
```

**Text Format**:
```
# Script: test_script
# Description: Test sequence
SEND 0 AT+RST
WAIT 1000
SEND 0 AT+CWMODE=1
```

### Protocol Parsing

SerialTap includes a built-in Modbus parser:

```bash
# Monitor Modbus traffic
serialtap monitor /dev/ttyUSB0 -x -t
```

### Custom Protocols

Define custom protocols in code or via plugins:

```rust
use serialtap_core::protocol::ProtocolParser;

let mut parser = ProtocolParser::new();
parser.add_pattern("MyProtocol", r"^MY:", "Custom protocol")?;
```

### File Transfer

Transfer files using standard protocols:

```bash
# Transfer with XMODEM (via script)
serialtap replay /dev/ttyUSB0 transfer_script.txt
```

### Plugin Development

Create plugins in Rust:

```rust
#[no_mangle]
pub extern "C" fn plugin_get_info() -> *mut c_char {
    // Return plugin information
}

#[no_mangle]
pub extern "C" fn plugin_execute(command: *const c_char, params: *const c_char) -> *mut c_char {
    // Execute plugin command
}
```

## Troubleshooting

### Port Not Found

**Symptoms**: No ports listed in `serialtap list`

**Solutions**:
1. Check device connection
2. Install appropriate drivers
3. On Linux, check permissions:
   ```bash
   ls -la /dev/tty*
   sudo usermod -a -G dialout $USER
   ```

### Connection Failed

**Symptoms**: "Connection failed" error

**Solutions**:
1. Verify baud rate matches device
2. Check for port conflicts
3. Ensure proper cable connection
4. Try different settings

### Data Corruption

**Symptoms**: Garbled output or incorrect data

**Solutions**:
1. Verify parity settings (usually None)
2. Check data bits (usually 8)
3. Ensure stop bits match (usually 1)
4. Monitor for buffer overflow

### Permission Denied

**Symptoms**: "Permission denied" when connecting

**Solutions**:
- **Linux**: Add user to dialout group
  ```bash
  sudo usermod -a -G dialout $USER
  ```
- **macOS**: Usually works out of the box
- **Windows**: Run as administrator if needed

### Performance Issues

**Symptoms**: Slow data transfer or missed data

**Solutions**:
1. Use higher baud rate
2. Optimize buffer sizes
3. Use flow control
4. Consider async I/O

## FAQ

### Q: What baud rates are supported?

A: Standard rates from 1200 to 921600 baud. Custom rates may work depending on hardware.

### Q: Can I use SerialTap with Arduino?

A: Yes! SerialTap works with any USB-to-serial device, including Arduino.

### Q: Does it support Bluetooth serial?

A: Yes, if the Bluetooth device appears as a serial port on your system.

### Q: Can I use it for Modbus communication?

A: Yes, SerialTap includes a built-in Modbus parser and can send/receive Modbus frames.

### Q: How do I create custom protocols?

A: Use the `ProtocolParser` API in code or create a plugin with custom parsing logic.

### Q: Is there a mobile version?

A: Yes, SerialTap supports iOS and Android, though mobile platforms have some limitations.

### Q: Can I automate tests?

A: Yes, use the CLI with scripts or the agent mode for JSON output.

### Q: How do I report bugs?

A: Please open an issue on GitHub with detailed reproduction steps.

## Support

- **Documentation**: See `docs/` directory
- **Issues**: GitHub Issues
- **Community**: GitHub Discussions

## License

MIT License - see LICENSE file for details.
