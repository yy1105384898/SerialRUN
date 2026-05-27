# SerialTap User Manual

[中文版](MANUAL_CN.md)

---

## Table of Contents

- [Introduction](#introduction)
- [Installation](#installation)
- [CLI Reference](#cli-reference)
- [GUI Guide](#gui-guide)
- [Troubleshooting](#troubleshooting)

## Introduction

SerialTap is a cross-platform serial port assistant designed for embedded developers. It provides both CLI and GUI interfaces for serial communication, protocol analysis, and automation.

## Installation

```bash
git clone https://github.com/yourusername/SerialTap.git
cd SerialTap
cargo build --release
```

Binaries will be in `target/release/`.

## CLI Reference

### `serialtap list`

List all available serial ports.

```bash
serialtap list                # Text output
serialtap list --format json  # JSON output
```

### `serialtap connect <port> [options]`

Connect to a serial port in interactive mode.

| Option | Default | Description |
|--------|---------|-------------|
| `-b, --baud` | 115200 | Baud rate |
| `-d, --data-bits` | 8 | Data bits (5/6/7/8) |
| `-s, --stop-bits` | 1 | Stop bits (1/2) |
| `-p, --parity` | none | Parity (none/odd/even) |
| `-f, --flow` | none | Flow control (none/software/hardware) |

### `serialtap send <port> <data> [options]`

```bash
serialtap send COM1 "Hello\r\n"              # Send text
serialtap send COM1 "48 65 6C 6C 6F" --hex   # Send HEX
```

### `serialtap monitor <port> [options]`

```bash
serialtap monitor COM1 -t                  # Timestamps
serialtap monitor COM1 -x                  # HEX mode
serialtap monitor COM1 -t -l output.log    # Log to file
```

### `serialtap record <port> [options]`

```bash
serialtap record COM1 -o script.txt
```

### `serialtap replay <port> <script>`

```bash
serialtap replay COM1 script.txt
```

### `serialtap agent [port] <action>`

JSON output mode for automation. See [CLAUDE.md](../CLAUDE.md) for details.

## GUI Guide

```bash
serialtap-gui
```

### Interface Overview

- **Top bar**: Port selector, baud rate, Connect/Disconnect, Chart/Log/Language/Theme/Help buttons
- **Left panel**: Serial config (collapsible), Display options, Auto reply, Recording
- **Center**: Terminal display with TX/RX/SYS indicators
- **Bottom**: Status bar with connection state and byte counts

### Language and Theme

- Click **中** / **EN** to switch language
- Click **☀** / **☾** to switch theme
- Click **?** to open the help guide

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Port not found | Click Refresh, check device connection |
| Permission denied (Linux) | `sudo usermod -a -G dialout $USER` |
| Garbled text | Check baud rate matches device setting |
| No data | Verify cable, check flow control settings |
