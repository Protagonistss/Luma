# Luma

Android TV live player built with Tauri 2, React, Rust, and AndroidX Media3.

## Features

- Import M3U/M3U8 playlists from URL or local file
- Channel groups, favorites, and recent history
- Native Media3 full-screen playback on Android TV
- Automatic retry for recoverable live stream errors
- Windows / macOS / Linux desktop dev for UI and playlist workflows

## Legal Notice

Luma does not ship any channels or streams. You must only import playlists that you are legally allowed to use.

## Prerequisites

- Node.js 20+
- pnpm 9+
- Rust 1.77+
- **Windows 桌面开发**：安装 [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)（Windows 10/11 通常已自带）
- **Android TV 打包**：Android Studio、SDK 28+，并配置 `ANDROID_HOME` / `NDK_HOME`

## Project Layout

- `web/` — React + TypeScript TV UI（Tauri WebView 层）
- `native/` — Tauri 2 Rust 核心、Android TV 配置与 Media3 播放插件

## Windows 桌面开发（推荐日常调试）

在电脑上测试频道列表、导入、收藏、最近观看等业务逻辑：

```bash
pnpm install
pnpm dev:desktop
```

等价命令：

```bash
pnpm tauri dev
```

这会启动 Windows 桌面窗口（1280×720），Rust 后端与 Web UI 都会运行。点击播放时，桌面端会在系统浏览器中打开流地址，便于快速验证；Android TV 上仍走 Media3 原生全屏播放。

仅调试前端样式时，也可以只跑：

```bash
pnpm dev
```

注意：`pnpm dev` 没有 Tauri 后端，无法测试导入、收藏等需要 Rust 的功能。

## 构建 Windows 安装包

```bash
pnpm tauri build
```

产物位于 `native/target/release/bundle/`。

## Android TV 开发与 APK

```bash
pnpm tauri android init   # 首次需要
pnpm tauri android dev
pnpm tauri android build --apk
```

详见 [docs/BUILD_ANDROID.md](docs/BUILD_ANDROID.md)。

## 自动化检查

```bash
pnpm test          # vitest 单测
pnpm typecheck     # tsc 类型检查
pnpm lint          # oxlint（基于 @ithinku/oxlint-config）
pnpm format:check  # oxfmt 格式检查（基于 @ithinku/oxfmt-config）
pnpm build         # 类型检查 + vite 构建
cd native && cargo test
```

## Signing

Do not commit keystore files or credentials. Configure release signing locally through Android Studio or Gradle properties outside the repository.

## License

MIT
