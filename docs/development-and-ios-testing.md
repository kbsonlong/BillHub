# BillHub 开发与 iOS 测试

本文说明如何启动 BillHub 桌面开发环境、iOS 模拟器和 iPhone 真机，并记录常见故障的处理方式。项目使用 Tauri 2、React、Vite 和 Rust；iOS 应用由 `src-tauri` 下的 Tauri 配置生成。

## 1. 环境要求

- macOS 与 Xcode。首次使用 Xcode 时先启动一次并接受许可；确认命令行工具指向所需的 Xcode：

  ```bash
  xcode-select -p
  xcodebuild -version
  xcrun simctl list devices available
  ```

- Node.js 和 pnpm。仓库包含 `pnpm-lock.yaml`，安装依赖时使用锁文件：

  ```bash
  pnpm install --frozen-lockfile
  ```

- Rust。iOS 构建需要 Rustup 管理的 Apple 编译目标。可用以下命令安装：

  ```bash
  rustup target add aarch64-apple-ios aarch64-apple-ios-sim --toolchain stable-aarch64-apple-darwin
  ```

  查看已安装目标：

  ```bash
  rustup target list --installed --toolchain stable-aarch64-apple-darwin
  ```

  如果机器上的 `rustc` 来自 Homebrew，而 iOS targets 安装在 Rustup 工具链中，运行 iOS 命令时需让 Rustup 的 `bin` 目录排在 `PATH` 前面。本文后续命令使用本机路径示例；换一台机器时，将其替换为 `rustup which rustc` 所属工具链的 `bin` 目录。

## 2. 桌面与前端开发

启动桌面应用（同时启动 Vite）：

```bash
pnpm dev
```

只启动前端开发服务器：

```bash
pnpm web:dev
```

前端地址是 `http://localhost:1420`。只在浏览器打开页面适合检查普通布局和交互；Tauri 文件选择、SQLite 等桌面 API 在普通浏览器里不可用。

构建前端并执行 TypeScript 检查：

```bash
pnpm build
```

## 3. iOS 模拟器

### 初始化或重新生成 iOS 工程与图标

首次创建 iOS 工程，或需要按当前 Tauri 配置重新生成工程时，在仓库根目录运行：

```bash
rtk npm run tauri -- ios init
```

项目图标源文件是 `src-tauri/icons/logo.svg`。需要重新生成各平台图标（包括 iOS AppIcon）时运行：

```bash
rtk npm run tauri -- icon src-tauri/icons/logo.svg
```

命令默认将图标生成到 `src-tauri/icons/`，并更新 iOS 工程使用的 AppIcon 资源。初始化完成后可按下文启动模拟器或真机。`src-tauri/gen/apple/` 中的 Xcode 工程属于生成文件；重新初始化前，先将任何需要保留的 Xcode 手工配置迁回 Tauri 配置或源文件。

1. 在 Xcode 的 **Settings > Platforms** 安装 iOS Simulator runtime，并从 **Window > Devices and Simulators** 确认目标模拟器可用。
2. 用 `xcrun simctl list devices available` 查看可用设备名称。当前示例使用 `iPhone 17 Pro`；也可以换成已安装的机型。
3. 在仓库根目录运行：

   ```bash
   PATH="/Users/zengshenglong/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH" pnpm tauri ios dev "iPhone 17 Pro"
   ```

   Tauri CLI 会启动 Vite、编译 Rust/iOS 工程并安装运行应用。保持这个终端进程运行，以便开发服务器和热更新继续工作。结束时按 `Ctrl-C`。

如需在 Xcode 中检查生成的工程或调整签名设置：

```bash
PATH="/Users/zengshenglong/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH" pnpm tauri ios dev --open "iPhone 17 Pro"
```

若模拟器处于异常状态，可在退出开发命令后通过设备 UDID 启动并等待就绪，再重试：

```bash
xcrun simctl list devices available
xcrun simctl boot <SIMULATOR-UDID>
xcrun simctl bootstatus <SIMULATOR-UDID> -b
```

不要对含有需要保留数据的模拟器执行 `erase`。如只是想验证首次启动，可在模拟器设置中卸载应用后重装；这会删除该模拟器中应用的本地数据。

## 4. iPhone 13 真机

### 首次准备

1. 在 iPhone 上打开 **设置 > 隐私与安全性 > 开发者模式**，启用后按系统提示重启并确认。
2. 用数据线连接 Mac 并解锁手机，在手机上确认“信任此电脑”。也可以先在 Xcode 的 **Window > Devices and Simulators** 配对设备，再启用无线连接。
3. 在 Xcode **Settings > Accounts** 登录 Apple 账号。打开工程时，在 iOS 应用 target 的 **Signing & Capabilities** 选择有效的 Team，并确认 Bundle Identifier 可用于该 Team。个人 Team 一般可用于本地开发部署；分发签名则需要符合 Apple 的分发要求。
4. Mac 与手机保持可互相访问的网络连接。首次启动应用时允许 BillHub 访问本地网络；拒绝后可在 iPhone **设置 > BillHub > 本地网络**重新开启。

### 启动真机

先确认 Xcode 已识别设备：

```bash
xcrun xctrace list devices
```

然后在仓库根目录运行（设备名称以命令输出为准）：

```bash
PATH="/Users/zengshenglong/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH" pnpm tauri ios dev "iPhone 13"
```

首次构建可能需要较长时间，按终端提示完成证书或设备信任操作。保持命令运行；要停止开发服务器和应用调试会话，在终端按 `Ctrl-C`。

项目的 Vite 配置通过 `TAURI_DEV_HOST` 绑定可供手机访问的地址，并将 HMR WebSocket 放在 1421 端口。Tauri CLI 会为移动开发设置此主机信息。若网络不可达，确认 Mac 和 iPhone 在同一可路由网络、VPN/防火墙未阻断开发端口，并在系统提示时允许本地网络访问。若设备通过 Xcode 的网络隧道连接，Tauri CLI 的 `--force-ip-prompt` 可用于手动选择设备可访问的主机地址：

```bash
PATH="/Users/zengshenglong/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH" pnpm tauri ios dev --force-ip-prompt "iPhone 13"
```

## 5. 建议的移动端验收流程

模拟器和真机均建议逐项检查，并记录设备型号、iOS 版本、构建提交和结果：

1. 首次安装启动、关闭后再次启动；确认页面可加载且本地数据仍在。
2. 检查主导航、每日任务、记一笔、流水明细、导入中心和成长记录在窄屏上的布局、滚动、键盘遮挡及返回路径。
3. 新增一笔收入和支出，检查保存结果、金额/日期/分类及流水详情；重新启动后确认数据持久化。
4. 检查导入预览、提交、重复数据提示及导入后的流水和 XP/成长记录。使用专门的测试账单，避免导入隐私数据。
5. 检查每日任务完成/撤销、重复点击、跨日期显示、XP 更新和成长记录日期归属。
6. 验证文件选择、权限提示、取消操作和错误提示；验证离线启动及本地数据访问。
7. 真机检查安全区、刘海/灵动岛、底部手势区域、系统键盘、深浅外观和不同字号下的可读性。

应用的数据保存在各自运行环境的本地存储中。模拟器和 iPhone 的数据库相互独立，卸载应用可能清除对应环境的数据；测试前请使用可丢弃的数据副本并备份重要账单。

## 6. iOS Web 内容调试

- 模拟器：Safari 的 **Develop** 菜单可选择启动中的模拟器及 BillHub WebView。
- 真机：在 Mac Safari **Settings > Advanced** 打开 Web 开发者功能；在 iPhone **设置 > Safari > 高级 > Web 检查器**启用检查器。设备连接并信任后，在 Mac Safari **Develop** 菜单选择 iPhone 及 BillHub 页面。
- 如页面不能连接热更新，先检查开发终端是否仍在运行，再检查本地网络权限和 Wi-Fi/VPN。改动 Rust 命令或原生配置后需重新编译部署，单靠前端 HMR 不会更新原生二进制。

## 7. 常见问题

### `can't find crate for std` / `can't find crate for core`

通常是当前选中的 `rustc` 工具链没有安装 iOS target，或执行构建的 `rustc` 和安装 target 的 Rustup 工具链不同。检查 `which rustc` 与 target 列表，然后安装目标并优先使用 Rustup 工具链：

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim --toolchain stable-aarch64-apple-darwin
PATH="/Users/zengshenglong/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH" pnpm tauri ios dev "iPhone 17 Pro"
```

### Xcode 提示签名或 provisioning profile 错误

在 Xcode **Settings > Accounts** 确认 Apple 账号已添加；用 `--open` 打开工程，在 iOS target 的 **Signing & Capabilities** 选择 Team 并启用自动签名。确认连接的 iPhone 已解锁、受信任且 Developer Mode 已打开。

### `CoreSimulator` 启动/安装失败

确认 runtime 和设备仍可用，先启动模拟器并等待 `bootstatus` 完成。若仍失败，退出 Xcode Simulator 与 Tauri 开发进程后再启动；先收集 Xcode 和模拟器错误信息，不要用清除模拟器数据作为常规修复。

### 真机显示无法连接开发服务器

确认手机已授权 BillHub 本地网络访问、Mac 与 iPhone 网络互通、开发命令未退出。暂时关闭 VPN 或检查防火墙是否拦截 Vite/HMR 端口；需要时运行 `--force-ip-prompt` 并选取 iPhone 可访问的地址。

### 设备上仍显示旧前端

确认当前设备连到本次运行的 Vite 地址；重启应用。前端更新通常由 HMR 推送，Rust 或 Xcode 原生部分变化则需结束并重新运行 `tauri ios dev`。

## 8. 生成的 iOS 工程与版本控制

`src-tauri/gen/apple/` 是 Tauri CLI 生成的 Xcode 工程，默认不纳入版本控制；`src-tauri/gen/apple/Assets.xcassets/AppIcon.appiconset/` 的应用图标资源是项目例外，已保留跟踪。日常开发不需要手动编辑生成工程；若 Xcode 中临时调整了生成文件，先确认是否应将设置迁回 `src-tauri/tauri.conf.json` 或项目源资源，避免把可重建文件当作稳定配置提交。

## 9. 官方参考

- [Tauri 2 开发与移动端开发](https://v2.tauri.app/develop/)
- [Tauri 2 iOS 签名与分发](https://v2.tauri.app/distribute/sign/ios/)
- [Apple：在 Xcode 中管理设备](https://developer.apple.com/documentation/xcode/connecting-a-device-to-your-mac)
