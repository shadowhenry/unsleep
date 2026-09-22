<h1 align="center">
  <img src="assets/pumpkin-source.png" alt="UnSleep logo" width="44" height="44" valign="middle">
  UnSleep - 跨平台防熄屏工具
</h1>

<p align="center">基于 <b>Tauri 2.x</b> 的轻量桌面 / 移动端防睡眠应用，调用系统级能力保持设备清醒，切到其他软件工作仍可继续生效。</p>

## 功能特点

- **系统级防睡眠**：通过平台原生 API 阻止系统休眠 / 熄屏，不依赖网页焦点
- **跨平台**：桌面端覆盖 macOS / Windows / Linux，移动端支持 iOS
- **菜单栏 / 托盘常驻**：启动默认不弹窗，仅保留菜单栏或托盘图标
- **关闭即隐藏**：点击主窗口关闭按钮（X）只隐藏窗口，不退出应用；只有菜单里的“退出”才会完全退出
- **一键开关**：点按开关即可启用 / 关闭防睡眠，并实时显示已保持清醒的时长
- **原生实现**：macOS `caffeinate` / Windows `SetThreadExecutionState` / Linux `systemd-inhibit` / iOS `idleTimerDisabled`
- **浏览器预览调试**：无需打包即可在浏览器预览界面，使用 Screen Wake Lock API 作为调试兜底（不作为 PWA 发布）

## 技术架构

| 层 | 技术 |
| --- | --- |
| 界面 | 原生 HTML / CSS / JS（`web/`，无前端框架），由 Tauri WebView 渲染 |
| 核心 | Rust（Tauri 2.x），负责防睡眠状态机、托盘、窗口事件与 IPC 命令 |
| 防睡眠 | 平台原生 API：`caffeinate` / `SetThreadExecutionState` / `systemd-inhibit` / `idleTimerDisabled` |
| 打包 | Tauri bundler（macOS dmg / Windows nsis / Linux AppImage·deb·rpm / iOS IPA） |

```
UnSleep/
├── web/                  # 前端界面（原生 HTML/CSS/JS，Tauri WebView 渲染）
│   ├── index.html        # 主界面
│   ├── app.js            # 开关状态、计时、与 Tauri 通信
│   ├── styles.css        # 样式
│   └── icon.png
├── src-tauri/            # Tauri 桌面端（Rust）
│   ├── src/main.rs       # 入口：防睡眠状态机、托盘菜单、窗口关闭事件
│   ├── build.rs          # 构建脚本
│   ├── Cargo.toml        # Rust 依赖与配置
│   ├── tauri.conf.json   # Tauri 配置（窗口、打包、权限）
│   ├── capabilities/     # 权限能力声明（default.json）
│   ├── icons/            # 各平台图标（macOS / Windows / Linux / iOS / Android）
│   └── Info.plist        # macOS 打包元数据
├── assets/               # 设计素材（logo 源文件等）
├── package.json          # 前端脚本与 Tauri CLI 依赖
└── LICENSE
```

## 环境准备

需要安装：

- Node.js
- Rust / Cargo
- 对应平台的 Tauri 系统依赖

macOS 安装 Rust：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

macOS 如未安装 Xcode 命令行工具：

```bash
xcode-select --install
```

安装项目依赖：

```bash
npm install
```

## 快速开发

```bash
npm install        # 安装依赖（@tauri-apps/cli）
npm run dev        # 启动 Tauri 桌面应用
```

这会启动 Tauri 桌面应用。点击“打开防睡眠”后，会启用系统级防睡眠。

其它命令：

```bash
npm run preview    # 浏览器预览界面（http://localhost:5173，Screen Wake Lock API 调试兜底）
npm run build      # 编译当前平台安装包
npm run ios:init   # 初始化 iOS 工程（需 macOS + Xcode，仅首次）
npm run ios:dev    # 在 iOS 模拟器 / 设备开发运行
npm run ios:build  # 编译 iOS release 包
```

## 编译与打包

Tauri 默认编译“当前系统对应的平台应用”——在哪台机器执行，就编译哪台机器的包；最稳妥的跨平台方式是分别在对应系统上打包（或用 GitHub Actions 在 `macos-latest` / `windows-latest` / `ubuntu-latest` 上执行）。

### 通用编译

```bash
npm install
npm run build      # 等价于 npx tauri build
```

### macOS

```bash
npm run build      # 编译当前架构
```

如需同时兼容 Apple Silicon 与 Intel Mac，先安装 Intel Rust target：

```bash
rustup target add x86_64-apple-darwin
npm run build:mac:universal   # Universal macOS（Apple + Intel）
```

也可分别编译：

```bash
npm run build:mac:apple   # Apple Silicon
npm run build:mac:intel   # Intel
```

常见产物：

```text
src-tauri/target/release/bundle/macos/
src-tauri/target/release/bundle/dmg/
```

### Windows

```powershell
npm install
npm run build
```

常见产物：

```text
src-tauri\target\release\bundle\nsis\
src-tauri\target\release\bundle\msi\
```

### Linux

```bash
npm install
npm run build
```

常见产物：

```text
src-tauri/target/release/bundle/appimage/
src-tauri/target/release/bundle/deb/
src-tauri/target/release/bundle/rpm/
```

### iOS

iOS 命令只能在 macOS 上运行，需 Xcode、iOS target 与 Apple 签名环境。生成的 IPA 通常在：

```text
src-tauri/gen/apple/build/arm64/UnSleep.ipa
```

注意：当前 iOS 版本可编译客户端框架，但防睡眠原生实现仍需接入 iOS 的 `idleTimerDisabled` 能力。

## 菜单栏 / 托盘

桌面客户端会停靠在系统菜单栏或托盘中。应用启动默认不弹出界面，只保留菜单栏 / 托盘图标，主窗口隐藏；macOS 上以纯菜单栏应用运行（无 Dock 图标、不参与 Cmd+Tab），Windows / Linux 上仅隐藏启动窗口。

菜单项：

- 显示窗口
- 打开防睡眠 / 关闭防睡眠
- 退出

## 开源协议

本项目基于 [MIT 协议](LICENSE) 开源。
