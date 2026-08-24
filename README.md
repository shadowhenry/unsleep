# UnSleep

一个轻量、简约的跨平台桌面防熄屏应用。

- 桌面端：使用 Tauri 调用系统级防睡眠能力，切到其他软件工作时仍可继续生效。
- 浏览器预览：仅用于调试界面，不作为 PWA 发布。

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

## 开发运行

```bash
npm run dev
```

这会启动 Tauri 桌面应用。点击“打开防睡眠”后，会启用系统级防睡眠。

## 编译当前平台应用

Tauri 默认编译“当前系统对应的平台应用”。也就是说：

- 在 macOS 上执行，会编译 macOS 应用
- 在 Windows 上执行，会编译 Windows 应用
- 在 Linux 上执行，会编译 Linux 应用

通用编译指令：

```bash
npm run build
```

等价于：

```bash
npx tauri build
```

## macOS 编译

在 macOS 上运行：

```bash
npm run build
```

常见产物位置：

```text
src-tauri/target/release/bundle/macos/
src-tauri/target/release/bundle/dmg/
```

通常会生成 `.app` 和 `.dmg`。

## Windows 编译

在 Windows 上运行：

```powershell
npm install
npm run build
```

常见产物位置：

```text
src-tauri\target\release\bundle\nsis\
src-tauri\target\release\bundle\msi\
```

通常会生成 `.exe` 安装包，具体取决于 Tauri bundler 配置和本机环境。

## Linux 编译

在 Linux 上运行：

```bash
npm install
npm run build
```

常见产物位置：

```text
src-tauri/target/release/bundle/appimage/
src-tauri/target/release/bundle/deb/
src-tauri/target/release/bundle/rpm/
```

通常会生成 `.AppImage`、`.deb` 或 `.rpm`，具体取决于系统依赖和 Tauri 配置。

## 跨平台打包说明

Tauri 不建议在一台机器上直接编译所有平台安装包。最稳定的方式是：

- macOS 包：在 macOS 上编译
- Windows 包：在 Windows 上编译
- Linux 包：在 Linux 上编译

如果要自动化发布，可以使用 GitHub Actions 分别在 `macos-latest`、`windows-latest`、`ubuntu-latest` 上执行：

```bash
npm install
npm run build
```

## 浏览器预览界面

如果只想预览界面：

```bash
npm run preview
```

然后打开：

```text
http://localhost:5173
```

浏览器预览只用于调试界面。正式防睡眠能力以 Tauri 客户端为准。

## 防睡眠实现

- macOS：调用 `caffeinate`
- Windows：调用 `SetThreadExecutionState`
- Linux：调用 `systemd-inhibit`
- 浏览器预览：调用 `Screen Wake Lock API` 作为调试兜底

## 菜单栏 / 托盘

桌面客户端会停靠在系统菜单栏或托盘中。

菜单项：

- 显示窗口
- 打开防睡眠 / 关闭防睡眠
- 退出

macOS 上显示在顶部菜单栏；Windows 和 Linux 上显示在系统托盘区域。iOS 没有系统托盘概念，因此不支持这个入口。

## iOS 编译

iOS 命令只能在 macOS 上使用，并且需要 Xcode、iOS target 和 Apple 签名环境。

首次初始化 iOS 工程：

```bash
npm run tauri ios init
```

或使用项目脚本：

```bash
npm run ios:init
```

开发运行到模拟器或设备：

```bash
npm run tauri ios dev
```

或：

```bash
npm run ios:dev
```

编译 iOS release 包：

```bash
npm run tauri ios build
```

或：

```bash
npm run ios:build
```

打开 Xcode 工程进行归档、签名和发布：

```bash
npm run tauri ios build -- --open
```

或：

```bash
npm run ios:open
```

App Store Connect 导出：

```bash
npm run tauri ios build -- --export-method app-store-connect
```

生成的 IPA 通常在：

```text
src-tauri/gen/apple/build/arm64/UnSleep.ipa
```

注意：当前 iOS 版本可用于编译客户端框架，但防睡眠原生实现还需要额外接入 iOS 的 `idleTimerDisabled` 能力。

## 开源协议

MIT
