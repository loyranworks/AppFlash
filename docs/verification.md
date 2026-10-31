# AppFlash 验证记录

## 2026-10-31 v0.1.7 正式发布验收

源码提交 `ecba0cc051a1e7f3b40dbcccac148d7c114cf7a8` 与 `v0.1.7` 标签已原子推送。GitHub Actions [41181227266](https://github.com/loyranworks/AppFlash/actions/runs/41181227266) 第一次运行即完成 prepare、Windows x64、macOS ARM、macOS Intel 和 publish，全部成功。两个 macOS 构建的严格签名门禁均通过。

[正式 Release](https://github.com/loyranworks/AppFlash/releases/tag/v0.1.7) 于 2026-10-31 07:44:49 UTC 发布，已成为 latest，非草稿、非预发布；发布说明包含实际验证范围。全部 9 个附件均已匿名下载，SHA-256 与 GitHub 元数据一致。公开 `latest.json` 版本为 0.1.7，三平台地址指向本标签的公开附件，签名文本与相应 `.sig` 一致。三个更新包均通过客户端原公钥验签、签名版本和篡改字节拒绝检查。

两个 macOS 更新归档内应用均为 0.1.7，架构分别为 arm64 与 x86_64，完整应用包严格签名校验通过；两个 DMG 的 `hdiutil verify` 均通过。Apple Silicon DMG SHA-256：`f391d060a4d386e185545e54cdd8b5f6f4d752f19eaf38a8b85d4d21df570cbf`；Intel DMG：`084736ec5ef33fe267aeec4f8b8726cfbb57defba38bacf4170fcf41efe1aee6`；Windows 安装器：`36f3c2feac404cfe13f3da9acc3194596bc90c923c0295d2f473176d2478dce8`。

只发布并验收下载产物，未替换本机安装的 v0.1.6。真实全局快捷键端到端、MBP / Windows 实机以及客户端跨版本升级仍待验收。本机永久产物目录仅有两个历史版本，未清理已有文件。

## 2026-10-31 v0.1.7 发布前检查

最终源码的 28 项 JavaScript 与 17 项 Rust 测试通过，版本一致性、Rust 格式和差异空白检查通过；本地 v0.1.7 正式候选应用的严格签名校验通过。首次沙盒测试受 Swift 全局模块缓存写入限制失败，获准在正常宿主环境重跑后完整通过，未绕过测试。已有 `updates.rs:100` 不可达代码告警仍存在。

发版准备改为读取创建草稿响应中的 ID，已有草稿直接读取 `databaseId`。新增实际执行工作流 bash 的回归测试，模拟草稿列表尚未可见：旧脚本失败，新脚本对新建和已有草稿均通过。线上结果见本页正式发布验收记录。

## 2026-10-31 平台前缀标签

macOS 设置面板的四个前缀改为 Control、Option、Shift、Command；Windows 保持 Ctrl、Alt、Shift、Win。只更新标签文本，不改 `Control / Alt / Shift / Super` 输入值、持久化配置或注册快捷键的语义。新增两平台标签回归用例，先失败后通过；完整 27 项 JavaScript 与 17 项 Rust 测试通过。正式候选应用 v0.1.7 已重建，真实 macOS 界面确认四个键名正确、原有 Alt 配置仍选中 Option、默认窗口布局无截断。

与 v0.1.6 比较，Windows 平台适配、共享窗口切换逻辑、快捷键注册、运行时及平台构建开关均未修改。macOS Swift 组件只在 macOS 构建条件中编译，新增原生测试也仅在 macOS 运行。没有执行 Windows 实机回归，未声称所有 Windows 运行表现已验收。本节记录发布前候选验收。

## 2026-10-31 macOS 原生窗口控制候选修复

在 Mac mini 上使用已安装的 `/Applications/AppFlash.app` v0.1.6 验证。按用户确认开启 AppFlash 辅助功能权限，临时创建计算器 Option+K 绑定；本轮测试结束后已恢复原空应用列表，权限按用户授权保留开启。MBP 未直接连接，不把 Mac mini 的结果当作 MBP 验收。

旧长驻 Swift 组件主线程阻塞于 `readLine()`，没有运行 AppKit 主循环，导致 `NSWorkspace` 应用列表与前台状态长期不刷新。实机对照中，新进程确认前台已经是 AppFlash，旧长驻组件仍返回计算器 active。修复为后台读取输入、主线程串行处理 IPC、持续运行 `NSApplication.run()`。真实回归用例启动并停止无窗口测试应用：旧组件超时失败，新组件及时感知应用启动 / 退出并正常处理 EOF。此默认测试不申请辅助功能权限，但需要 macOS 图形会话。

计算器还暴露非业务 `AXDialog` 窗口，原代码只按非空标题筛选，导致它混入切换队列。现在只保留标准 `AXWindow` 或已最小化的 `AXWindow`；实测最小化后的计算器会暂时变成 `AXDialog` 子角色，因此必须保留最小化窗口。未最小化的 dialog、floating 和 panel 不加入普通窗口循环。计算器的 `AXRaise` 返回 `-25205`，即使恢复已成功也会被原组件误报失败；修复为优先对目标引用设置可写 `AXMain`，不支持时退回 `AXRaise`，并检查应用激活请求结果。

候选组件的 `tools/test-macos-native.py --window-control` 已通过两个自有空白窗口的逐窗激活、全部最小化、逐窗恢复及真实 active / minimized 状态检查；计算器的最小化与恢复同样通过。代码审查未发现阻断项。42 项 JavaScript / Rust 测试通过，其中新增 1 项原生 IPC 和应用列表刷新回归。

v0.1.7 候选正式 `.app` 已从源码构建并通过完整签名校验，未替换本机安装的 v0.1.6。全局快捷键端到端验证尚未完成：当前电脑操作工具的按键未触发独立 Carbon 测试热键；未通过 CGEvent 注入系统级按键。用户随后明确要求发版，本节保留实际验收范围。

## 2026-10-31 macOS 窗口恢复

官方 v0.1.5 ARM 应用首次启动能显示设置；关闭设置后进程继续运行，从 Finder 再次打开却不恢复窗口。使用同一包实机复现，源码缺少 macOS `RunEvent::Reopen` 处理。现在使用 `App::run` 处理该事件，恢复设置并取消最小化；显示窗口失败会记录错误。

修复逻辑在本机源码构建的调试应用中完成 GUI 验收：首次启动显示设置、关闭后 Finder 重开恢复、最小化后 Finder 重开恢复。`--background` 冷启动日志确认没有调用显示窗口逻辑；测试工具查询应用会触发 `Reopen`，经临时生命周期日志区分后确认，诊断日志代码已移除。最终 v0.1.6 源码的 25 项 JavaScript 和 16 项 Rust 测试通过，格式和发布版本检查通过；已有 `updates.rs:100` 不可达代码编译告警仍存在，与本次窗口逻辑无关。只读代码审查未发现阻断风险。

正式优化版 v0.1.6 `.app` 已在本机从源码编译、完成 ad-hoc 签名，并实机确认首次显示及关闭后 Finder 重开恢复窗口。Tauri 的本地 DMG 流程停在 Finder AppleScript 布局步骤，结束该子进程后，改用 HFS+ 简单封装已编译应用，交付 `../artifacts/AppFlash_0.1.6_aarch64.dmg`。最终 DMG 完整性校验通过，重新只读挂载后确认版本为 0.1.6、架构为 arm64、完整应用包严格签名校验通过；SHA-256 为 `325d97db22df82ac3026dbda8e1275b7f62b961b2b84cfcf4a90e50d63290992`。本地构建仅禁用当前包的 updater 归档生成，未改动正式配置或更新公钥。源码提交 `4ccb984` 和 `v0.1.6` 标签已推送，GitHub Actions 41169580197 在复用已有草稿后进入三个平台构建。

## 2026-10-31 macOS ARM 签名修复

官方 v0.1.4 Apple Silicon DMG 与更新归档的 SHA-256 均与 GitHub Release 一致，DMG 内部校验通过。ARM 应用只有 linker ad-hoc 签名，完整应用包的严格校验报 `code has no resources but signature indicates they must be present`；同版本 Intel 应用完全未签名。对 ARM 应用补全 ad-hoc 签名后，`codesign --verify --deep --strict` 通过；`syspolicy_check distribution` 的代码签名错误消失，仍报告临时签名和缺少公证。

源码设置 `bundle.macOS.signingIdentity` 为 `-`，发布工作流在两个 macOS 架构构建完成后执行严格签名校验；失败会阻止正式发布。25 项 JavaScript 测试、发布配置检查和 `git diff --check` 通过；只读代码审查未发现阻断问题。本机没有 Rust，未执行 Rust 测试或源码重编译，未推送代码或发布新 Release。

交付 `../artifacts/AppFlash_0.1.4_aarch64_adhoc.dmg`，基于已核对哈希的官方 v0.1.4 ARM 二进制补签重打包。最终 DMG 的 `hdiutil verify` 通过；重新只读挂载后，包内应用的严格签名校验通过，架构为 arm64。补签应用使用临时配置目录启动并持续运行 10 秒，随后停止测试进程；未提前退出或报告 AppFlash 启动失败，但 stderr 包含 `sandbox_extension_issue_file_to_process` 告警，未据此声称完整界面或辅助功能验收。未关闭 Gatekeeper 或移除隔离属性，首次启动的系统手动允许仍需用户完成。

## 0.1.3 远端发布验证

正式版本标签 v0.1.3 指向 d586a9f。Actions 41036956626 首次在创建草稿后立即读取 Release ID 时未找到新草稿；确认唯一草稿存在后只重跑失败流程。第二次准备、Windows、macOS ARM、macOS Intel 及 publish 全部成功，共用一份 Release；未手工合并安装包。2026-10-30 18:07:02（北京时间）正式发布，非草稿、非预发布，公开 latest.json 为 0.1.3。三个公开更新包均匿名下载成功，并通过原公钥、签名版本及篡改拒绝测试。未执行已安装客户端的实际升级或 macOS 实机验收。创建草稿后的即时列表可见性仍可能造成后续首次运行失败，工作流未在本次额外改动。

## 0.1.3 工作室署名（已正式发布）

左侧 APP / FLASH 下方新增工作室名称和口号，随现有中英文语言设置即时切换，保留 AppFlash 应用口号。VI 原字体压缩为仅包含固定署名字形的 Regular WOFF2（合计 4420 字节），附原 SIL OFL 许可证；所有目标字形均已检查存在。托盘 / 菜单栏使用原生 About，显示实际包版本及对应语言的工作室文本，分别配置 Windows comments 和 macOS credits。品牌与国际化回归测试通过；macOS 原生 About、系统托盘点击及视觉排版仍待实机验收。本次浏览器访问宿主不可用，未启动或替换用户安装的程序。

## 0.1.3 任务栏图标修正（已正式发布）

已安装的 0.1.2 EXE 从原路径及独立临时副本提取均为闪电图标。发现 Tauri 的 set_icon 只更新 Windows ICON_SMALL，未同步 ICON_BIG。现为设置窗口单独更新并持有任务栏大图标，颜色和暂停状态共用现有图像生成逻辑；替换时释放旧句柄。原生隐藏窗口回归测试验证 ICON_BIG 存在、刷新后更换句柄且不改 ICON_SMALL。未清理系统图标缓存、未替换用户正在运行的程序，实际任务栏显示仍待新版安装后验收。

日期：2026-10-30。平台：Windows x64。

## 已执行

- `npm test`：26 项测试通过（10 JavaScript，16 Rust），含字母排序、手动移动边界、旧配置迁移、96px PNG 尺寸、更新确认 / 未保存设置保护 / 失败恢复、发布配置检查与签名拒绝篡改。
- `node --check ui/settings.js`、`node --check ui/overlay.js`：通过。
- `cargo check --manifest-path src-tauri/Cargo.toml`：通过。
- `cargo fmt --manifest-path src-tauri/Cargo.toml`：已执行。
- `npm run build -- --no-bundle`：通过，产物 `src-tauri/target/release/app-flash.exe`。
- `npm run build -- --bundles nsis`：通过，生成 3.17 MiB Windows 安装包与 `.sig` 更新签名。通过 `APP_FLASH_VERIFY_INSTALLER` 测试真实安装包与客户端公钥匹配，修改后的文件拒绝验签。
- HTTPS `git ls-remote` 与临时目录 `gh repo clone` 成功；仓库为私有，当前 gh 用户 royluo2026 拥有 ADMIN 权限。未修改父仓库 remote，没有提交、推送或线上发布。
- 原生只读扫描测试：当时发现 6 个应用，两次扫描与检查总计约 2.194 ms。这不是切换延迟，也不是与 AHK 的对比。
- Windows Explorer 原生图标提取：通过。
- 调试版本曾启动，Windows 工具枚举返回标题为 AppFlash 的实际窗口。随后已关闭本次调试进程，以完成构建。

## 本次修改

移除启动参数，贴边不显示应用名称并接入 Windows 原生 blur / macOS vibrancy；默认延迟 500ms。新增 A–Z / 手动顺序选择和应用列表上移下移按钮。新增直接读写系统启动项的开关，登录后静默启动。

远端升级客户端与 CI 流程已实现；签名私钥保存在工作区外。已确定调试后公开同一仓库；私有阶段无法匿名下载，远端尚未发布 Release，线上链路未验证。

## 2026-10-30 双语与导航更新

设置拆为应用、快捷键、贴边提示、通用四栏，语言即时保存，导航不重建编辑字段。静态翻译覆盖、状态重译和导航保留编辑测试通过；含旧配置语言默认值、英文读写与非法语言校验的全部 29 项测试通过。Rust check、Release 配置校验与 Windows NSIS 构建通过。中英文 README 已更新；仓库保持私有，未提交、推送或发布。

## 尚未验证

2026-10-30：恢复最初闪电图标；新增原生图标颜色、暂停灰色、恢复颜色、透明留白与非法颜色测试，以及设置预览状态测试。全部 32 项测试与 Rust check 通过。设置预览采用草稿色；托盘使用已保存颜色，暂停不改写配置。文件图标保持静态。Windows / macOS 托盘切色的视觉实机验收仍未完成。

Computer Use 的页面捕获授权等待超时，未完成截图、批量导入点击、设置保存和实际快捷键触发的 UI 端到端检查。没有以模拟通过替代实机结果。

macOS 代码没有在 Mac 上编译或运行；Swift API、辅助功能授权、原生组件签名、各应用窗口兼容性仍需要验证。

没有实际启用系统开机启动项，也没有重启系统验证登录启动。磨砂外观仍未截图验收。

尚未检查管理员窗口、多屏与混合 DPI、26 项提示布局、长时间运行资源占用、真实 AHK 性能基准、真实跨版本更新与线上 CI。已生成 Windows 安装器，但未做操作系统代码签名或 macOS 公证。旧 AHK / INI 未改动，本工作区未初始化独立 Git。


## 0.1.1 远端发布验收（2026-10-30）

Windows x64、macOS Apple Silicon / Intel 的 GitHub 构建成功。首次 publish 地址校验失败，修复清单后通过 CLI 发布同一构建；下次 tag 的自动 publish 尚待验证。公开 latest.json 返回 200，版本 0.1.1；三平台包匿名下载均返回 200。线上 Windows 安装包及清单中的签名通过内嵌公钥验证，篡改被拒绝。未自动安装或终止现有客户端；实际跨版本升级、重启、配置保留及桌面图标刷新待用户验收。


## 0.1.2 仓库迁移与远端验收（2026-10-30）

仓库迁移至 loyranworks/AppFlash，Git remote、客户端 endpoint、发布脚本与双语 README 已同步。三平台线上构建成功。并发创建导致两个同标签草稿，本次合并附件并验证复制摘要一致后通过 CLI 发布；冗余 Intel 草稿改为 v0.1.2-build-intel 并保留，不对外发布。工作流已改为预先创建单一草稿并传递 Release ID；该新流程的下一次线上执行仍待验证。旧、新更新入口均匿名返回 200 / 0.1.2，三平台包匿名下载均返回 200，并通过原公钥、签名版本与篡改拒绝测试。未执行用户客户端安装、重启或长按 Prefix 实机验收。

## 0.1.8 合并验收（2026-10-31）

本机固定签名 A（0.1.7）→B（0.1.8）在同一 `/Applications/AppFlash.app` 测试。只为 A 移除旧条目并重新授权一次，B 没有改动辅助功能列表，真实全局 CGEvent 组合键仍成功激活、最小化和恢复计算器。主程序 SHA-256 不同，designated requirement 相同：`identifier "local.appflash.desktop" and certificate root = H"f00dc7f30609d86353fa08ac71ca68b53e78c9b5"`。固定公有证书校验通过；临时、异证书、被篡改代码的拒绝测试通过。

首次合并自动测试为 35 项 JavaScript + 35 项 Rust 全通过；后续原生全屏诊断修复仍需最终重跑。设置页实际输入 80%，滑杆同步，确认保存后配置落盘 80%；原生贴边窗口实际为 96×96pt（100% 最小尺寸 120×120pt），右侧保持 12pt 留白。未以 WebView zoom 叠加第二次缩放。

全屏原生验收增加真实 fixture 的 didEnter/didExit 通知、每窗原生 ID、isKeyWindow、isOnActiveSpace、fullScreen、minimized、hidden 状态；单独 AXFocusedWindow 成功不再作为实际桌面切换的充分证据。已复现 AXWindows/AXChildren 在全屏桌面省略其他桌面的正常窗口，有效缓存引用仍可读取。候选列表合并公开属性和有效缓存，并仅在精确 WindowServer ID 缺失时按需进行隔离的兼容发现。

兼容发现使用可选私有符号，按精确 PID/CGWindowID、AXWindow 根和标准窗口条件匹配，无标题或位置猜测，无对象内存偏移读取。扫描有预算和限频；未确认完整窗口集合时禁止结束本轮收起，避免把部分发现当作全量。符号或应用不支持时可能提示重试，不能保证所有第三方窗口兼容。

`dot-doctor` 已运行：既有未纳管配置和断链提示仍在；签名目录属于秘密存储，保持在 Git 之外，未自动纳入 dotfiles。

后续强化验收：`tools/test-macos-native.py <helper> --fullscreen --check-space` 已通过两种从全屏前台直接收起场景：同时含普通窗口、关闭普通窗口后只剩全屏窗口。系统在全屏期间会拒绝直接 hide；实现先公开激活已运行的 Finder，确认目标全屏窗口离开当前 Space，再要求实际 AXHidden=true。恢复要求原全屏窗口重新成为 key 且在 active Space，fullScreen 始终为 true。关闭窗口对应的 AX 根失效后，系统 CG 库存仍可能保留该窗口，现按此前已验证根与会话唯一 CGWindowID 的精确关系排除离屏残留，不把未知窗口猜成已关闭。

已安装 B 的真实全局组合键连续六次，验证两轮“窗口 1 / 窗口 2 / 全部最小化”；从实际全屏 fixture 通过 K 切到计算器，源窗口全屏保留。贴边首次实测发现普通 NSWindow 即使有跨全屏标志也未 on-screen；按设计升级为真实非激活 NSPanel，显示、焦点及比例由下一轮实机验收确认。

最终 NSPanel 实机验收通过：在实际全屏 fixture 为 key、active Space 且前台不变时，按真实四修饰键前缀，WindowServer 明确将 AppFlash hint 面板标记为 on-screen；原 Tao 代理窗保持隐藏。80% 首次显示为 96×125pt，设置改为 150% 保存后为 180×234pt，右边距始终 12pt，中心位置保持；实际截图确认清透圆角材质、Navy 字母、主线与连接线。macOS 提示条底部修饰键改为原生 ⌃ ⌥ ⇧ ⌘ 符号。

最终安装的 B 通过真实 Carbon 全局快捷键再跑两轮含全屏窗口的循环：普通窗与全屏窗各置前一次 → 整应用 hidden=true → 恢复并重复；每次全屏窗 key 与 active Space 一致，fullScreen 全程 true。没有为 B 重新授权。源码最终完整回归为 35 项 JavaScript + 37 项 Rust 全通过（另含 15 个原生分类断言）；固定证书 release app 构建与校验通过。测试过程中曾发现并修复原生句柄 placeholder 契约与 Tao/GCD 几何异步时序，最终尺寸检查覆盖了首次显示及比例变更。

正式发行 v0.1.9 已完成：源码 `6ad6c9fdce47cee1b9027f60a6b3c794ad8a7396`，GitHub Actions run `41198456342`，Windows x64、macOS Apple Silicon 与 Intel 的构建、回归测试、签名门禁及 publish 全部成功。失败的 v0.1.8 标签/草稿保留，未对外发布；0.1.9 修复了 CI 临时测试钥匙串搜索列表及签名 identity 就绪检查，没有换证书或放宽校验。

公开 Release `429904533` 的全部 9 个附件均匿名下载成功，大小及 GitHub 提供的 SHA-256 全匹配。三个平台更新包均通过客户端原公钥、可信版本 0.1.9 与修改字节拒绝测试。两种 macOS 架构的更新归档与只读 DMG 内应用均通过完整严格签名、固定证书、固定 requirement、包标识、版本和架构校验；两张 DMG 校验通过并已卸载。公开 latest 更新入口返回 0.1.9。

额外完成真实内置升级：从本机 B 0.1.8 点击检查更新、确认下载并升级，自动安装 GitHub 正式 0.1.9 并重启；设置显示实际 0.1.9，已保存配置与更新备份一致。没有修改辅助功能授权列表，三次实际 CGEvent K 快捷键成功激活、最小化、恢复计算器。安装后的正式主程序与实际运行的原生组件均验证为同一固定证书。正式版面板截图确认 macOS 修饰键符号和 Navy 清透材质。测试临时绑定及快捷键最终恢复，原始恢复备份保留。
