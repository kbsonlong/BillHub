# BillHub iPhone 安装与 AltStore 自动刷新指引

本文介绍如何用免费的 Apple Personal Team，在 Mac 上安装 AltServer、在 iPhone 上安装 AltStore，并让 AltStore 尝试通过同一 Wi-Fi 刷新 BillHub 的签名。适用于当前通过 Xcode/Tauri 在 iPhone 上测试的 BillHub。

> **先了解限制：**免费 Personal Team 签名通常约 7 天到期。AltStore 可以在一周内尝试刷新由 AltStore 管理的 App，但它不是 Apple 的后台自动续签服务，也不保证每次都能在无人操作时完成。Mac 上的 AltServer 必须运行，iPhone 与 Mac 必须在同一可互相发现的 Wi-Fi 网络。AltStore 不会自动接管原来由 Xcode 安装的 BillHub；需要先把 BillHub 的 IPA 导入 AltStore。

## 1. 工作方式和准备条件

刷新过程如下：

```text
iPhone 上的 AltStore  ──同一 Wi-Fi──>  Mac 上运行的 AltServer  ──互联网──>  Apple
        │                                    │
        └──更新由 AltStore 管理的签名───────┘
```

- **iPhone：**已启用开发者模式，并能安装个人开发者签名的应用。
- **Mac：**macOS 11 或更新版本；安装并运行 AltServer。Mac 需要保持开机、联网，自动刷新时不能处于关机或睡眠状态。
- **Apple 账号：**免费 Apple 账号即可。AltServer 首次安装 AltStore 时需要用 Apple 账号向 Apple 完成签名认证。
- **同一 Wi-Fi：**iPhone 和 Mac 需连接到同一局域网，且路由器不能隔离无线客户端。访客网、公司/学校网络、VPN 或防火墙可能阻止设备发现。
- **应用数量：**免费账号有 Apple 的侧载应用数量限制；AltStore 当前文档说明同一设备最多同时启用 3 个侧载 App。若已达到上限，先检查是否有其他侧载应用占用名额。

## 2. 在 Mac 上安装 AltServer

1. 在 Mac 上打开 [AltStore 官方 macOS 安装说明](https://faq.altstore.io/altstore-classic/how-to-install-altstore-macos)，从官方链接下载 AltServer。当前说明要求 macOS 11 或更新版本。
2. 解压下载文件，将 `AltServer.app` 拖到“应用程序”文件夹。
3. 从“应用程序”启动 AltServer。启动后，菜单栏会出现 AltServer 图标。若 macOS 询问是否允许网络连接，选择允许。
4. 用数据线连接 iPhone，解锁手机，并在手机上确认“信任此电脑”。此前已经信任过这台 Mac 的话通常不会再次询问。
5. 打开 Finder，在侧边栏选择 iPhone。在“通用”页勾选 **“在 Wi-Fi 网络中显示此 iPhone”**，点“应用”。这一步用于建立无线配对；初次安装 AltStore 建议仍保持 USB 连接。
6. 点击 Mac 菜单栏中的 AltServer 图标，选择 **Install AltStore > [你的 iPhone 名称]**。
7. 按提示输入 Apple 账号。账号凭据用于 AltServer 向 Apple 完成开发者签名认证。只在从官方渠道取得的 AltServer 中输入，不要把密码发给他人或填入非官方网页。若你希望把个人开发用途与日常账号分开，可以使用单独的 Apple 账号。
8. 等待 Mac 出现安装成功提示。如果 iPhone 主屏幕暂时没有 AltStore，稍等片刻或重启 iPhone 后再检查。

AltStore 官方称 Apple 账号凭据只发送给 Apple。请自行确认下载来源是 AltStore 官方页面，并按自己的账号安全要求决定使用哪个账号。

## 3. 在 iPhone 上信任 AltStore

1. 首次打开 AltStore 前，进入 **设置 > 通用 > VPN 与设备管理**（系统版本不同，菜单名称可能略有差异）。
2. 在“开发者 App”下选择对应 Apple 账号，点“信任”，再按屏幕提示确认。
3. 如果 iPhone 尚未启用开发者模式，进入 **设置 > 隐私与安全性 > 开发者模式**并启用，然后按提示重启和确认。
4. 回到主屏幕打开 AltStore。进入 AltStore 的 **Settings**，确认登录的 Apple 账号正确，并开启 **Background Refresh**。
5. 如果 iOS 询问 AltStore 使用本地网络，选择允许。之后可在 **设置 > 隐私与安全性 > 本地网络**检查 AltStore 是否有权限。

## 4. 把 BillHub 加入 AltStore 管理

### 为什么需要重新安装 BillHub

目前手机上的 BillHub 是通过 Xcode 安装的。仅安装 AltStore 不会把这个现有 App 纳入刷新列表。AltStore 只会刷新它自己管理的侧载应用，因此要将 BillHub 的 `.ipa` 文件交给 AltStore 安装。

### 导出 IPA

1. 在 BillHub 仓库根目录打开终端，确认依赖和 iOS 工程已经准备好。
2. 按本项目已使用的方式构建可安装 IPA：

   ```bash
   rtk pnpm tauri ios build --export-method debugging
   ```

3. 等构建完成后，在构建输出中找到导出的 `.ipa` 文件。若命令提示打开 Xcode Organizer 或导出流程，按 Xcode 提示完成导出，并确认最后得到的是 `.ipa`，不是 `.app` 或 `.xcarchive`。
4. 将 IPA 传到 iPhone“文件”App 可访问的位置，例如通过 AirDrop 发送到 iPhone 并选择“存储到文件”，或放入 iCloud Drive。

该 IPA 是独立构建；安装后 BillHub 页面不依赖 Mac 上的 Vite 开发服务器。Wi-Fi 仅用于 AltStore 与 AltServer 通信和签名刷新。

### 迁移现有安装时保护本地数据

BillHub 的账本数据保存在 iPhone 本地。替换签名或安装来源时，iOS 可能允许覆盖安装并保留数据，也可能因签名不兼容而要求先移除旧 App。**如果 AltStore 或 iOS 提示需要删除旧版 BillHub，请先停止，不要直接确认。**删除 App 可能一并删除本地账本数据。先确认账单已有可恢复的备份，再决定是否继续。

如果你还没有验证过本地数据备份和恢复流程，建议先不要卸载当前可用的 BillHub。先安装并验证 AltStore，确认 IPA 可被 AltStore 识别；在处理“覆盖安装/签名不匹配”提示前，先备份账单数据。AltStore 的应用停用备份功能只适用于由 AltStore 管理的应用，不能当作当前 Xcode 安装版 BillHub 的预备份。

### 用 AltStore 安装 IPA

1. 确认 AltServer 正在 Mac 菜单栏运行，iPhone 和 Mac 在同一 Wi-Fi。首次安装 IPA 时可以用 USB 连接，先排除无线发现问题。
2. 在 iPhone 打开 AltStore，进入 **My Apps**。
3. 点页面上的 **“+”**，从“文件”选择 BillHub 的 `.ipa`。
4. 等待 AltStore 完成签名和安装。遇到 Apple 账号登录或确认提示时，按 AltStore 提示完成。
5. 安装完成后，在 iPhone 主屏幕打开 BillHub，检查页面是否正常加载，再确认原有账本数据是否还在。
6. 回到 AltStore 的 **My Apps**，确认 BillHub 出现在列表中并显示有效期倒计时。只有出现在这里的 BillHub 才由 AltStore 管理刷新。

> 若第 3 步找不到 IPA、显示文件格式错误，确认文件扩展名为 `.ipa` 且导出完整。若安装时提示与设备、Bundle ID、签名或应用数量有关，记录完整错误提示再处理；不要为了绕过错误而先删除原 App。

## 5. 打开 Wi-Fi 自动刷新

### 配置 Mac 无线配对

若还没完成第 2 节的 Finder 配置：

1. 用 USB 连接 iPhone 并解锁。
2. 打开 Finder，选择 iPhone，在“通用”中启用 **“在 Wi-Fi 网络中显示此 iPhone”**，点“应用”。
3. 拔掉数据线，确认 iPhone 和 Mac 仍处于同一 Wi-Fi。

### 让 AltServer 持续运行

1. 在 Mac **系统设置 > 通用 > 登录项**中，将 AltServer 加入“登录时打开”。macOS 版本不同，设置名称可能略有不同。
2. 确认菜单栏能看到 AltServer 图标；如果退出 AltServer，iPhone 就没有可连接的刷新服务。
3. 让 Mac 保持开机、联网并避免在计划刷新时进入睡眠。Mac 关机、睡眠、AltServer 退出或 Wi-Fi 中断时，自动刷新就无法完成。

### 验证一次手动刷新

在依赖自动刷新之前，先验证设备之间能通信：

1. iPhone 和 Mac 连接同一 Wi-Fi，Mac 上 AltServer 正在运行。
2. 在 iPhone 打开 AltStore > **My Apps**，点 **Refresh All**，或在 BillHub 条目上执行刷新。
3. 等待刷新完成，并确认 BillHub 倒计时重新变长。
4. 如果提示找不到 AltServer，先保持 USB 连接再试。USB 成功、Wi-Fi 失败通常表示 Finder Wi-Fi 配对、局域网隔离、VPN 或防火墙仍有问题。

手动刷新验证通过后，AltStore 会在后台周期性尝试刷新。为了减少 iOS 后台调度、Mac 睡眠或网络中断的影响，建议每隔几天打开 AltStore 查看倒计时，并在少于 1–2 天时手动点一次 **Refresh All**。

## 6. 日常使用与限制

- **不需要一直插数据线。**完成 Finder 的 Wi-Fi 配对且同网可达后，刷新可以通过 Wi-Fi 进行。USB 可作为无线连接失败时的手动备用方式。
- **Wi-Fi 自动刷新不是保证任务。**iOS 会自行安排 AltStore 的后台运行时机；AltStore 官方表述是“尝试”在一周内自动刷新。不要等到最后几小时才第一次检查。
- **AltServer 必须在线。**手机连 Wi-Fi 本身不够；Mac 关机、睡眠、离开网络或 AltServer 未运行时，刷新会失败。
- **网络要允许设备互相发现。**访客 Wi-Fi、公司/学校网络、客户端隔离、VPN 或防火墙可能阻断 AltStore 到 AltServer 的连接。可换到可信的家庭网络，或临时用 USB 手动刷新。
- **免费签名仍约 7 天有效。**AltStore 通过重新签名延长可运行时间，不会把免费账号变成长期签名。若超过有效期仍未刷新，App 会暂时无法打开；恢复 AltServer 连接后，打开 AltStore 并手动刷新。
- **App 数量有上限。**免费账号在设备上同时启用的侧载 App 数量有限；AltStore 当前说明为最多 3 个。AltStore 还提示免费账号 App ID 数量/有效期存在限制，查看 My Apps 中的 **View App IDs** 可了解当前注册情况。
- **开发服务器与签名是两回事。**使用 `tauri ios dev` 时，BillHub 可能请求 Mac 上的 Vite 开发服务器，离开该网络就无法加载开发页面。用导出的独立 IPA 安装后，日常运行不依赖该服务器；但它仍需要在签名到期前被 AltStore 刷新。

## 7. 故障排查

### AltStore 显示 “Could not find AltServer”

1. 检查 Mac 是否开机、未睡眠，且菜单栏 AltServer 图标仍在。
2. 确认 iPhone 和 Mac 连接同一 Wi-Fi，而不是访客网络或不同 VLAN。
3. 关闭 VPN 后重试；检查路由器是否开启无线客户端隔离。
4. 检查 Finder 是否启用了“在 Wi-Fi 网络中显示此 iPhone”。必要时重新插 USB、解锁手机并在 Finder 中确认设备可见。
5. 插 USB 后再试。如果 USB 可用而 Wi-Fi 不可用，问题在无线配对或网络发现，不是签名本身。

### AltServer 登录 Apple 账号时报 HTTP 503 / `unknown tag html`

这表示 AltServer 收到了 HTTP 503 错误响应；旧版本可能把 Apple 或网络返回的 HTML 错误页误当成登录数据解析，继而显示 `The data is not in the correct format`、`unknown tag html` 或 `NSCocoaErrorDomain Code 3840`。这类响应本身不能证明 Apple 账号密码错误。

1. 在 AltServer 菜单中查看版本，并从 [AltStore 官方页面](https://altstore.io/)更新。AltStore 项目曾记录旧版 AltServer 的 503 问题，并在 1.7.6 发布修复；低于 1.7.6 时先升级再试。
2. 若使用 AltServer 1.8.2 / AltStore 2.3.1，且错误发生在输入双重验证码之后，AltStore 项目区截至 2026-10-26 仍有同版本的 HTTP 503 问题报告。此时不要连续重复登录、反复改密码或新建账号；等待官方更新后再试。
3. 若想排除网络路径，可只做一次对照：让 Mac 改用手机热点或另一条可信网络，再尝试登录。不同网络结果有差异时，可能是特定网络到 Apple 登录接口的路径问题；换网仍失败则更像当前 AltServer/Apple 认证接口兼容问题。
4. 暂时不要安装未经官方发布的修改版 AltServer，也不要删除 iPhone 上现有的 BillHub。AltStore 登录成功之前，自动刷新尚未配置；可继续用 Xcode 在签名到期前重新安装 BillHub。

### AltStore 安装成功，但 BillHub 不在刷新列表

确认 BillHub 是从 AltStore 的 **My Apps > +** 导入 IPA 安装的。由 Xcode 或 AltServer 的 **Sideload .ipa…** 功能直接安装的 App，不等于 AltStore 管理的 App；AltServer 文档说明直接 Sideload 的 App 需要每 7 天手动重新安装。

### AltStore 的有效期已过

先让 Mac 上的 AltServer 恢复运行，再把手机和 Mac 放到同一 Wi-Fi，打开 AltStore 手动刷新。若过期的是 AltStore 自身，AltStore 官方说明可以用 AltServer 重新安装 AltStore 来刷新，通常不需要先删除它。

### 安装提示 App ID 数量已满

AltStore 的 **My Apps > View App IDs** 可查看注册的 App IDs 和到期时间。AltStore 文档说明免费账号最多保留 10 个 App IDs，每个约一周到期；应用包含的扩展也可能占用额外 App ID。等待已注册 ID 过期后重试，或检查当前安装的侧载 App。

### 安装/覆盖时出现签名不匹配或需要删除旧版

停止操作，先保护 BillHub 本地数据库。确认账单备份可恢复之前，不要删除旧版 App。若继续遇到问题，记录完整错误内容、Bundle Identifier 和 AltStore 版本，再针对错误处理。

## 8. 推荐检查清单

- [ ] AltServer 从 AltStore 官方页面下载，并位于 `/Applications`。
- [ ] Mac 登录后 AltServer 会启动，菜单栏图标可见。
- [ ] Finder 已启用“在 Wi-Fi 网络中显示此 iPhone”。
- [ ] AltStore 已信任，开发者模式已启用，Background Refresh 已打开。
- [ ] iPhone 已允许 AltStore 使用本地网络。
- [ ] BillHub 是通过 AltStore 的 My Apps 导入 IPA 安装，且出现在 My Apps 列表中。
- [ ] 同一 Wi-Fi 下手动 Refresh All 成功，倒计时已重置。
- [ ] 原 Xcode 版 BillHub 的本地数据已确认备份后，才处理覆盖/删除提示。
- [ ] 每隔几天检查一次有效期，必要时手动刷新。

## 官方参考

- [AltStore：macOS 安装 AltStore](https://faq.altstore.io/altstore-classic/how-to-install-altstore-macos)
- [AltStore：AltServer 与 Wi-Fi 刷新](https://faq.altstore.io/altstore-classic/altserver)
- [AltStore：应用管理、后台刷新和有效期](https://faq.altstore.io/altstore-classic/your-altstore)
- [AltStore：故障排查](https://faq.altstore.io/altstore-classic/troubleshooting-guide)
- [AltStore：App ID 数量和有效期](https://faq.altstore.io/altstore-classic/app-ids)
- [AltStore：AltServer 直接安装 IPA 的限制](https://faq.altstore.io/release-notes/altserver)
- [Apple：开发者账号与 Personal Team](https://developer.apple.com/help/account/basics/about-your-developer-account)
- [BillHub：开发与 iOS 测试](./development-and-ios-testing.md)
