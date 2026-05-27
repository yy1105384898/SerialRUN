<div align="center">

# SerialTap

**面向嵌入式开发者的跨平台串口助手**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20iOS%20%7C%20Android-blue.svg)]()

[English](README.md) | [中文](#功能特性)

</div>

---

## 功能特性

- **跨平台** — Windows、macOS、Linux、iOS、Android
- **CLI & GUI** — 命令行用于自动化，桌面客户端用于交互式使用
- **协议支持** — Modbus RTU/TCP 解析，自定义协议模式匹配
- **数据可视化** — 实时图表和统计信息
- **脚本录制** — 录制和回放串口通信会话
- **文件传输** — 支持 XMODEM / YMODEM / ZMODEM
- **插件系统** — 可扩展架构，支持动态加载插件
- **十六进制模式** — 以十六进制格式收发数据
- **自动回复** — 自动响应匹配的模式
- **双语界面** — 英文/中文语言切换，深色/浅色主题

## 快速开始

### 安装

```bash
git clone https://github.com/yourusername/SerialTap.git
cd SerialTap
cargo build --release
```

### 命令行使用

```bash
# 列出可用串口
serialtap list

# 连接串口
serialtap connect COM1 -b 115200

# 发送数据
serialtap send COM1 "Hello\r\n"

# 带时间戳监听
serialtap monitor COM1 -t -l output.log

# 录制脚本
serialtap record COM1 -o script.txt

# 回放脚本
serialtap replay COM1 script.txt
```

### 桌面客户端使用

```bash
serialtap-gui
```

### GUI 快速开始

1. 通过 USB 连接串口设备
2. 点击「刷新」检测端口
3. 选择端口和波特率，点击「连接」
4. 在输入框输入命令，按回车发送

## 项目结构

```
SerialTap/
├── crates/
│   ├── serialtap-core/       # 核心串口逻辑库
│   ├── serialtap-cli/        # 命令行工具
│   └── serialtap-gui/        # 桌面客户端 (egui)
├── plugins/
│   └── example-plugin/       # 插件示例 (C FFI)
├── docs/                     # 文档
├── tests/                    # 集成测试
└── benches/                  # 性能测试
```

## 跨平台构建

| 平台 | 命令 |
|------|------|
| Windows (MSVC) | `cargo build --target x86_64-pc-windows-msvc --release` |
| macOS (Apple Silicon) | `cargo build --target aarch64-apple-darwin --release` |
| macOS (Intel) | `cargo build --target x86_64-apple-darwin --release` |
| Linux | `cargo build --target x86_64-unknown-linux-gnu --release` |

详见 [docs/BUILD_CN.md](docs/BUILD_CN.md) 了解 Android、iOS 构建及 .app 打包。

## Agent 模式 (自动化)

```bash
serialtap agent list-ports                # 列出端口 (JSON)
serialtap agent COM1 send "AT+RST"        # 发送数据
serialtap agent COM1 read --timeout 1000  # 读取数据
serialtap agent COM1 run-script test.txt  # 运行脚本
```

## 插件开发

```rust
#[no_mangle]
pub extern "C" fn plugin_get_info() -> *mut c_char { /* ... */ }

#[no_mangle]
pub extern "C" fn plugin_execute(command: *const c_char, params: *const c_char) -> *mut c_char { /* ... */ }
```

完整示例见 [plugins/example-plugin/](plugins/example-plugin/)。

## 文档

| 文档 | 说明 |
|------|------|
| [docs/MANUAL_CN.md](docs/MANUAL_CN.md) | 用户手册 |
| [docs/SKILL_CN.md](docs/SKILL_CN.md) | 技能参考 |
| [docs/BUILD_CN.md](docs/BUILD_CN.md) | 构建指南 |
| [CLAUDE_CN.md](CLAUDE_CN.md) | Agent 操作指南 |

## 开发

```bash
cargo build       # 构建所有 crate
cargo test        # 运行测试
cargo bench       # 运行性能测试
```

## 许可证

[MIT 许可证](LICENSE)

---

<div align="center">

**为嵌入式开发者用心打造**

</div>
