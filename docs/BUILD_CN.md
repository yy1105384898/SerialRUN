# SerialTap 构建指南

[English](BUILD.md)

---

## 前提条件

- [Rust](https://www.rust-lang.org/tools/install) 1.70+
- 平台特定的构建工具（见下方）

## Windows

### 要求

安装 [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)，勾选「使用 C++ 的桌面开发」。

### 构建

```bash
rustup target add x86_64-pc-windows-msvc
cargo build --target x86_64-pc-windows-msvc --release -p serialtap-gui
```

输出: `target/x86_64-pc-windows-msvc/release/serialtap-gui.exe`

## macOS

### 要求

```bash
xcode-select --install
```

### 构建

```bash
# Apple Silicon (M1/M2/M3/M4)
rustup target add aarch64-apple-darwin
cargo build --target aarch64-apple-darwin --release -p serialtap-gui

# Intel Mac
rustup target add x86_64-apple-darwin
cargo build --target x86_64-apple-darwin --release -p serialtap-gui
```

### 打包为 .app

```bash
cargo install cargo-bundle
```

在 `crates/serialtap-gui/Cargo.toml` 中添加:

```toml
[package.metadata.bundle]
name = "SerialTap"
identifier = "com.serialtap.app"
category = "DeveloperTool"
short_description = "串口助手 - 嵌入式开发者工具"
```

运行:

```bash
cargo bundle --target aarch64-apple-darwin --release -p serialtap-gui
```

输出: `target/release/bundle/osx/SerialTap.app`

## Linux

### 要求 (Ubuntu/Debian)

```bash
sudo apt install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libgtk-3-dev libatk1.0-dev
```

### 构建

```bash
rustup target add x86_64-unknown-linux-gnu
cargo build --target x86_64-unknown-linux-gnu --release -p serialtap-gui
```

## Android

```bash
cargo install cargo-mobile2
cargo android init
cargo android build --release -p serialtap-gui
```

## iOS

```bash
cargo install cargo-mobile2
cargo ios init
cargo ios build --release -p serialtap-gui
```

## 交叉编译参考

| 目标 | 命令 |
|------|------|
| Windows x64 | `--target x86_64-pc-windows-msvc` |
| macOS ARM | `--target aarch64-apple-darwin` |
| macOS x64 | `--target x86_64-apple-darwin` |
| Linux x64 | `--target x86_64-unknown-linux-gnu` |
| Linux ARM64 | `--target aarch64-unknown-linux-gnu` |
| Android | `--target aarch64-linux-android` |
| iOS | `--target aarch64-apple-ios` |

```bash
rustup target add <target>
cargo build --target <target> --release -p serialtap-gui
```

## 输出路径

```
target/<target>/release/serialtap-gui.exe              # Windows
target/<target>/release/serialtap-gui                  # macOS/Linux
target/release/bundle/osx/SerialTap.app                # macOS .app
target/release/bundle/deb/serialtap-gui_*.deb          # Debian 包
target/release/bundle/rpm/serialtap-gui-*.rpm          # RPM 包
```
