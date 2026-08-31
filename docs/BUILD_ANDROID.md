# Android TV 构建与验证

## 环境准备

1. 安装 Android Studio，并确保 SDK Platform 28+ 与 Build Tools 已安装。
2. 设置环境变量：

```powershell
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
$env:NDK_HOME = "$env:ANDROID_HOME\ndk\<version>"
```

3. 安装 Rust Android targets：

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

## 初始化 Android 工程

```bash
pnpm install
pnpm tauri android init
```

## 开发与构建

```bash
pnpm tauri android dev
pnpm tauri android build --apk
```

## 自动化检查

```bash
pnpm test
pnpm lint
pnpm build
cd src-tauri && cargo test
```

## 真机 / 模拟器验证清单

- [ ] 从 URL 导入 M3U
- [ ] 从本地文件导入 M3U
- [ ] 分类、收藏、最近观看切换
- [ ] 遥控器方向键与返回键导航
- [ ] 进入原生全屏播放并返回
- [ ] 断网后按 2/5/10 秒策略重试
- [ ] 应用重启后收藏与最近观看保留

## 签名

Release 签名密钥与 `local.properties` 不得提交到仓库。请在本地 Gradle 或 Android Studio 中配置。
