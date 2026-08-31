# Luma

Android TV live player built with Tauri 2, React, Rust, and AndroidX Media3.

## Features

- Import M3U/M3U8 playlists from URL or local file
- Channel groups, favorites, and recent history
- Native Media3 full-screen playback on Android TV
- Automatic retry for recoverable live stream errors

## Legal Notice

Luma does not ship any channels or streams. You must only import playlists that you are legally allowed to use.

## Prerequisites

- Node.js 20+
- pnpm 9+
- Rust 1.77+
- Android Studio with SDK 28+
- `ANDROID_HOME` and `NDK_HOME` configured for Tauri Android builds

## Development

```bash
pnpm install
pnpm test
pnpm tauri android init
pnpm tauri android dev
```

## Build APK

```bash
pnpm tauri android build --apk
```

## Signing

Do not commit keystore files or credentials. Configure release signing locally through Android Studio or Gradle properties outside the repository.

## License

MIT
