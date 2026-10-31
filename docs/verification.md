# AppFlash 验证记录

## 2026-10-31 macOS 窗口恢复

官方 v0.1.5 ARM 应用首次启动能显示设置；关闭设置后进程继续运行，从 Finder 再次打开却不恢复窗口。使用同一包实机复现，源码缺少 macOS `RunEvent::Reopen` 处理。现在使用 `App::run` 处理该事件，恢复设置并取消最小化；显示窗口失败会记录错误。

修复逻辑在本机源码构建的调试应用中完成 GUI 验收：首次启动显示设置、关闭后 Finder 重开恢复、最小化后 Finder 重开恢复。`--background` 冷启动日志确认没有调用显示窗口逻辑；测试工具查询应用会触发 `Reopen`，经临时生命周期日志区分后确认，诊断日志代码已移除。最终 v0.1.6 源码的 25 项 JavaScript 和 16 项 Rust 测试通过，格式和发布版本检查通过；已有 `updates.rs:100` 不可达代码编译告警仍存在，与本次窗口逻辑无关。只读代码审查未发现阻断风险。

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
