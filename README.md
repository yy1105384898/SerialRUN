# SerialTap

A cross-platform serial port assistant for embedded developers, built with Rust.

## Features

- **Cross-platform**: Windows, macOS, Linux, iOS, Android
- **CLI & GUI**: Command-line interface and graphical desktop application
- **Protocol Support**: Modbus RTU/TCP, custom protocol parsing
- **Data Visualization**: Real-time charts and statistics
- **Script Recording**: Record and replay serial communication scripts
- **File Transfer**: XMODEM/YMODEM/ZMODEM support
- **Plugin System**: Extensible architecture with dynamic plugin loading
- **HEX Mode**: Send and receive data in hexadecimal format
- **Auto Reply**: Automatically respond to patterns
- **Logging**: Comprehensive logging with export capabilities

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
```

### Pre-built Binaries

Download the latest release from [GitHub Releases](https://github.com/yourusername/SerialTap/releases).

## Usage

### CLI

```bash
# List available ports
serialtap list

# Connect to a port
serialtap connect COM1 -b 115200

# Send data
serialtap send COM1 "Hello\r\n"

# Monitor port
serialtap monitor COM1 -t

# Replay script
serialtap replay COM1 script.txt

# Record script
serialtap record COM1 -o script.txt

# Agent mode (JSON output)
serialtap agent list-ports
serialtap agent COM1 send "test"
serialtap agent COM1 read --timeout 1000
```

### GUI

```bash
# Run the GUI application
serialtap-gui
```

## Configuration

SerialTap uses TOML configuration files. Example:

```toml
[serial]
port_name = "COM1"
baud_rate = 115200
data_bits = "Eight"
stop_bits = "One"
parity = "None"
flow_control = "None"
timeout_ms = 1000

[app]
log_dir = "logs"
auto_reconnect = true
hex_mode = false
timestamp_logs = true
```

## Plugin System

SerialTap supports dynamic plugins. See `plugins/example-plugin/` for an example.

### Plugin API

```rust
// Get plugin information
#[no_mangle]
pub extern "C" fn plugin_get_info() -> *mut c_char;

// Get available commands
#[no_mangle]
pub extern "C" fn plugin_get_commands() -> *mut c_char;

// Execute a command
#[no_mangle]
pub extern "C" fn plugin_execute(command: *const c_char, params: *const c_char) -> *mut c_char;

// Free allocated strings
#[no_mangle]
pub extern "C" fn plugin_free_string(s: *mut c_char);
```

## Development

### Prerequisites

- Rust 1.70+
- Cargo

### Building

```bash
# Build all crates
cargo build

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Project Structure

```
SerialTap/
├── crates/
│   ├── serialtap-core/      # Core library
│   ├── serialtap-cli/       # CLI application
│   └── serialtap-gui/       # GUI application
├── plugins/                 # Plugin examples
├── tests/                   # Integration tests
└── benches/                 # Performance benchmarks
```

## License

MIT License

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
