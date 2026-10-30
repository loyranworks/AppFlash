# AppFlash 0.1.0 验证记录

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
