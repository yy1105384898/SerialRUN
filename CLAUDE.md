# SerialTap — Agent Operation Guide

This document provides instructions for Claude Code agents to operate the SerialTap serial port assistant.

## Quick Commands

### List Ports

```bash
serialtap list                    # Text format
serialtap list --format json      # JSON format
```

### Connect

```bash
serialtap connect /dev/ttyUSB0 -b 115200
serialtap connect COM1 -b 9600 -d 7 -s 2 -p odd -f hardware
```

### Send Data

```bash
serialtap send COM1 "Hello\r\n"               # Text
serialtap send COM1 "48 65 6C 6C 6F" --hex    # HEX
```

### Monitor

```bash
serialtap monitor COM1 -t                  # With timestamps
serialtap monitor COM1 -x                  # HEX mode
serialtap monitor COM1 -t -l output.log    # With logging
```

### Scripts

```bash
serialtap record COM1 -o script.txt    # Record
serialtap replay COM1 script.txt       # Replay
```

## Agent Mode (JSON Output)

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
      "description": "USB Device 0403:6001",
      "manufacturer": "FTDI",
      "vid": 1027,
      "pid": 24577
    }
  ]
}
```

### Send Data

```bash
serialtap agent COM1 send "Hello" -b 115200
```

Output:

```json
{ "success": true, "bytes_written": 5 }
```

### Read Data

```bash
serialtap agent COM1 read --timeout 1000 --max-bytes 1024
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
serialtap agent COM1 run-script script.txt
```

## Common Workflows

### ESP8266/ESP32 AT Command Testing

```bash
serialtap connect COM3 -b 115200
# Then in interactive mode:
> AT
> AT+RST
> AT+CWMODE=1
> AT+CWJAP="WiFi","password"
```

### Modbus Traffic Capture

```bash
serialtap monitor /dev/ttyUSB0 -x -t -l modbus.log
serialtap send /dev/ttyUSB0 "01 03 00 00 00 0A C5 CD" --hex
```

### Automated Testing

```bash
serialtap record COM1 -o test.txt
serialtap replay COM1 test.txt
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Port not found | `serialtap list` to check |
| Permission denied | `sudo usermod -a -G dialout $USER` (Linux) |
| Connection failed | Verify baud rate matches device |
| No data received | Check cable and flow control |
