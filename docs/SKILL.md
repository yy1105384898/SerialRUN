# SerialTap Skill Documentation

## Overview

SerialTap is a cross-platform serial port assistant designed for embedded developers. It provides both CLI and GUI interfaces for serial port communication, protocol analysis, and automation.

## Core Capabilities

### 1. Serial Port Management

- **Port Enumeration**: Automatically detect all available serial ports
- **Connection Management**: Connect, disconnect, and reconnect to ports
- **Configuration**: Support for various baud rates, data bits, stop bits, parity, and flow control
- **Platform Support**: Windows (COM ports), Linux (ttyUSB/ttyS), macOS (cu.usbserial)

### 2. Data Communication

- **Text Mode**: UTF-8 encoded text send/receive
- **HEX Mode**: Hexadecimal data display and transmission
- **Mixed Mode**: Simultaneous text and HEX display
- **Binary Transfer**: Raw binary data handling

### 3. Protocol Analysis

- **Modbus RTU/TCP**: Parse and display Modbus frames
- **Custom Protocols**: Define custom protocol patterns with regex
- **Protocol Logging**: Record and analyze protocol traffic

### 4. Script Operations

- **Recording**: Record serial communication sessions
- **Replay**: Replay recorded scripts
- **Script Editing**: Create and modify scripts manually
- **Automation**: Automated testing and command sequences

### 5. File Transfer

- **XMODEM**: Classic file transfer protocol
- **YMODEM**: Enhanced XMODEM with batch transfer
- **ZMODEM**: Advanced file transfer with resume capability

### 6. Data Visualization

- **Real-time Charts**: Live data rate visualization
- **Statistics**: Transfer statistics and metrics
- **Logging**: Comprehensive logging with export

### 7. Plugin System

- **Dynamic Loading**: Load plugins at runtime
- **Custom Commands**: Add new commands via plugins
- **Extensible Architecture**: Easy to extend functionality

## Usage Patterns

### Embedded Development Workflow

1. **Discovery**: List ports to find connected devices
2. **Connection**: Connect with appropriate settings (usually 115200 8N1)
3. **Testing**: Send AT commands or test sequences
4. **Debugging**: Monitor responses and analyze protocols
5. **Automation**: Record and replay test scripts

### Automated Testing

1. **Script Creation**: Record manual tests as scripts
2. **Parameterization**: Modify scripts for different test cases
3. **CI Integration**: Use CLI for automated test execution
4. **Result Analysis**: Parse JSON output for test results

### Protocol Analysis

1. **Traffic Capture**: Monitor serial traffic with timestamps
2. **Protocol Parsing**: Use built-in Modbus parser or define custom protocols
3. **Pattern Matching**: Identify communication patterns
4. **Export**: Save analysis results for documentation

## Integration Points

### CLI Integration

```bash
# Scriptable CLI for automation
serialtap list --format json | jq '.ports[0].name'

# Pipe operations
serialtap send COM1 "test" && serialtap monitor COM1 -t -l output.log
```

### Agent Integration

JSON output mode for programmatic access:

```bash
# Get structured data
serialtap agent list-ports

# Execute commands
serialtap agent COM1 send "command"
serialtap agent COM1 read --timeout 1000
```

### Plugin Development

Extend SerialTap with custom plugins:

```rust
// Plugin API
#[no_mangle]
pub extern "C" fn plugin_get_info() -> *mut c_char;

#[no_mangle]
pub extern "C" fn plugin_execute(command: *const c_char, params: *const c_char) -> *mut c_char;
```

## Best Practices

### Port Configuration

- Use matching baud rates between devices
- Verify data format (data bits, stop bits, parity)
- Enable flow control for high-throughput applications
- Set appropriate timeouts for your use case

### Data Handling

- Use HEX mode for binary protocols
- Enable timestamps for debugging
- Log important sessions for later analysis
- Use auto-reply for simple test automation

### Script Development

- Start with manual recording
- Add delays between commands for device response
- Include error handling in scripts
- Test scripts thoroughly before automation

### Performance

- Use appropriate buffer sizes
- Monitor data rates to detect bottlenecks
- Optimize polling intervals for real-time applications
- Use async I/O for high-throughput scenarios

## Troubleshooting Guide

### Common Issues

1. **Port Not Found**
   - Check device connection
   - Verify driver installation
   - Check port permissions (Linux/macOS)

2. **Connection Failed**
   - Verify baud rate matches device
   - Check for port conflicts
   - Ensure proper cable connection

3. **Data Corruption**
   - Verify parity settings
   - Check for buffer overflow
   - Monitor data rates

4. **Performance Issues**
   - Optimize buffer sizes
   - Use appropriate timeouts
   - Consider async I/O

### Debug Steps

1. List available ports
2. Verify port information
3. Test with minimal configuration
4. Check system logs
5. Monitor data traffic

## Platform-Specific Notes

### Windows

- COM port naming: COM1, COM2, etc.
- Driver installation may be required
- Admin privileges sometimes needed

### Linux

- Device naming: /dev/ttyUSB0, /dev/ttyS0
- User must be in `dialout` group
- May need udev rules for custom devices

### macOS

- Device naming: /dev/cu.usbserial-*
- Typically works out of the box
- May need to close other applications using the port

### Mobile (iOS/Android)

- USB OTG support required
- Platform-specific permissions needed
- Limited port access compared to desktop

## API Reference

See `CLAUDE.md` for detailed API documentation and code examples.
