# 发布与升级

## 0.1.11 全屏响应、贴边错误与本地日志

macOS 窗口操作仍串行执行；忙碌期间的新按键在入口丢弃，避免首次切换等待结束后集中重放。AX 查询采用本进程 250ms 超时，应用暂时无响应时提示重试，不再误报辅助功能未授权。全屏应用收起前，切到另一个已运行应用的实际窗口并确认离开源桌面，保留全屏；不再仅依赖可能没有窗口的 Finder 激活请求。

窗口切换错误现在在 Navy 玻璃贴边内显示，保留快捷键列表、主线和流光，增加圆角暖色状态区。错误默认显示 6 秒，新错误替换旧错误，成功立即清除；关闭普通贴边提示时仍能看见错误。整体百分比缩放继续有效，拥挤布局的图标保持圆形比例。

设置 → 通用新增「导出 TXT」。诊断仅保存在本机标准日志目录，记录版本、启动、操作耗时、错误及忙碌按键丢弃数，不记录窗口标题或内容。保留今天及此前六个 UTC 日期的日志，每天最多 2 MiB，启动及运行期间定期清理；使用系统保存框导出，取消不会报错。

固定 macOS 证书与 updater 公钥保持不变。Mac mini 使用独立测试窗口验收；Codex 自身受电脑操作工具限制，未直接控制，MBP 与 Windows 实机结果仍需用户确认。Windows 窗口控制逻辑未改动；错误展示与日志功能为平台共用。

## 0.1.7 macOS 窗口控制与前缀标签

修复 macOS 原生组件阻塞主循环，导致已运行应用与前台窗口状态不刷新的问题。窗口切换过滤非标准辅助窗口，保留已最小化窗口；激活目标窗口优先设置 AXMain，必要时回退 AXRaise，并检查应用激活请求结果。

macOS 设置面板的快捷键前缀显示 Control、Option、Shift、Command；Windows 仍显示 Ctrl、Alt、Shift、Win。原有配置值和快捷键语义不变，Windows 窗口控制与共享切换逻辑未修改。发版准备流程直接读取草稿创建响应中的 Release ID，避免新草稿列表尚未刷新时首次构建失败。

Mac mini 已验证原生应用列表刷新、双窗口逐窗激活 / 最小化 / 恢复，以及设置标签显示。真实全局快捷键端到端、MBP 和 Windows 实机回归尚未完成。每台 Mac 仍需为 AppFlash 开启辅助功能权限；本次发版不自动修改系统权限或替换本机已安装应用。

## 0.1.6 macOS 窗口恢复

修复关闭设置后应用仍驻留、从 Finder 再次打开却没有窗口的问题。macOS 的重新打开事件现在会恢复设置窗口；最小化窗口也会恢复并获得焦点。普通启动显示设置，`--background` 登录启动保持静默，关闭设置继续驻留。保留原自动更新公钥、应用标识、配置路径和免费 ad-hoc 签名方式。

## 0.1.5 macOS 分发修复

macOS 两种架构均改为在打包时对完整应用包执行免费 ad-hoc 签名，修复 Apple Silicon 包只有链接器签名、严格校验失败的问题。CI 在 macOS 构建后执行 `codesign --verify --deep --strict`，任一架构失败会阻止正式发布。保留原自动更新公钥、应用标识和配置路径。首次启动仍可能需要在「隐私与安全性」中手动允许；此版本不包含 Developer ID 证书或 Apple 公证。

GitHub CLI 已取得 workflow 授权，正式工作流位于 .github/workflows/release.yml，仅推送 v* 标签才触发发布。普通 main 提交不发布安装包。已配置更新签名 Secret；v0.1.1 已正式发布，公开更新清单及三个平台的下载地址已验证 HTTP 200。

远端：`https://github.com/loyranworks/AppFlash.git`。源码与双语 README 已提交并通过 HTTPS 推送；项目使用独立 Git 仓库，不修改父目录仓库 remote。仓库已由维护者公开，客户端无需 GitHub Token。

## 仓库迁移（2026-10-30）

当前仓库为 loyranworks/AppFlash，Git remote、客户端更新源和清单生成脚本已同步修改。0.1.1 的旧、新清单地址均已匿名验证 HTTP 200；旧地址仍依赖 GitHub 重定向，请勿在原地址新建同名仓库。签名公钥、应用标识和配置路径保持不变，升级不会迁移或清空用户配置。0.1.2 已在新仓库正式发布，包含贴边显示顺序修正。旧、新清单入口都返回 0.1.2，三平台包已匿名下载并通过原公钥签名及版本校验。长按 Prefix 的实机效果仍需验收。

## 一次性准备

1. 源码与工作流已准备。使用 HTTPS / gh 登录推送即可；SSH 或连接器授权不是发布前提。发布前核对 main 的测试结果和版本，不要误改父目录仓库。
2. **使用同一个已公开的 AppFlash 仓库。** 新客户端 Release endpoint 使用 loyranworks/AppFlash；发布前检查源码与历史中没有凭据、私钥和私人配置。客户端不携带 GitHub Token；仅公开源码不够，仍需含完整更新清单的正式 Release。
3. 更新签名私钥已生成在 `C:\Users\luoyi8\.tauri\app-flash.key`，不在项目内。请自行安全备份；文件内容已配置为仓库 Actions Secret `TAURI_SIGNING_PRIVATE_KEY`；如密钥有密码，配置 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。客户端仅包含匹配的公钥。不要将私钥提交到 Git，也不要更换公钥导致已安装客户端失去更新能力。
4. 开启 GitHub Actions。首次发布仍需用户下载并安装安装包，开发目录的裸 exe 不作为正式便携版更新支持。

## 发布新版本

同步修改 `package.json`、`src-tauri/Cargo.toml` 与 `src-tauri/tauri.conf.json` 的版本，例如 `0.1.1`；相应更新 lock 文件。执行 `npm test` 和 `node tools/check-release.mjs`。自行提交，再推送对应 `v0.1.1` tag。

`.github/workflows/release.yml` 构建 Windows x64 NSIS、Apple Silicon 和 Intel macOS 应用。先创建一个共享草稿，三个平台按同一 Release ID 上传，所有平台成功且 `latest.json` 包含三个平台的签名和地址后才正式发布。普通代码 push 不发布；不一致版本或缺失密钥会失败；失败构建不会将半完成版本推给客户端。

Windows 本地构建签名安装包：

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = "$HOME\.tauri\app-flash.key"
npm run build -- --bundles nsis
Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY
```

## 客户端行为

- 启动后约 3 秒、之后每 6 小时检查；设置提供手动检查。网络 / 未发布 / 私有源错误会提示，不影响快捷切换。
- 新版本在设置里显示，托盘悬停文本提示；不强制弹窗、不自动安装。
- 用户确认后才下载并签名校验，要求签名包含与清单相同的版本，显示进度，安装后重启。未保存的设置会阻止升级；安装期间设置暂不可编辑。
- 安装前备份已保存配置到同目录 `config.pre-update.json`；备份失败中止。保留原配置、快捷键与系统启动项；下载或签名失败不执行安装。
- 更新签名不等于操作系统代码签名。当前 Windows 未做代码签名；macOS 0.1.8 起使用固定自签名证书签署应用和原生组件，尚未做 Developer ID 签名 / 公证，首次启动仍需在「系统设置 → 隐私与安全性」中手动允许。发布流程会严格校验两个 macOS 架构的应用包签名，校验失败则不发布。

## macOS 提示「AppFlash 已损坏」

2026-10-31 检查官方 v0.1.4 Apple Silicon 产物：DMG 的 SHA-256 与 GitHub Release 一致，`hdiutil verify` 通过，应用为 arm64。DMG 与更新归档内的 `.app` 均只有可执行文件的 linker ad-hoc 签名，没有绑定 Info.plist 或封存资源；`codesign --verify --deep --strict` 报 `code has no resources but signature indicates they must be present`。

修复是在 `bundle.macOS` 中设置 `signingIdentity: "-"`，让 Tauri 在打包时签署完整 `.app`，不能只依赖链接器签名。对下载副本补全签名后，严格签名校验通过；Gatekeeper 检查仍为 rejected，因此此修复不代表免除系统安全提示。已发布的 v0.1.4 文件不会因源码修复而改变，需要重新构建并发布新版本。要让公开下载的应用正常通过 Gatekeeper，仍需配置 Developer ID Application 证书和 Apple 公证凭据，参见 [Tauri macOS 签名说明](https://v2.tauri.app/distribute/sign/macos/)。

## 验证限制

本地已构建 Windows NSIS 安装包及更新签名，测试使用客户端公钥验证真实安装包并确认篡改后的字节被拒绝。v0.1.1 的三个线上构建均成功。首次自动 publish 被 API / 草稿附件地址校验拦截；已按真实附件生成公开下载地址，校验远端 Windows 安装包签名后通过 CLI 正式发布。工作流已加入相同的地址转换，下次 tag 尚待验证完整自动发布。真实跨版本安装由现有客户端通过更新界面验收，不手动覆盖本地程序。Mac 两种架构的构建、签名 / 公证及登录启动需 Mac 实机验收。仓库已公开；更新清单与三个平台包已匿名下载成功；Windows 包使用内嵌公钥校验通过，篡改包被拒绝。客户端点击升级、安装后重启及快捷方式图标刷新仍待用户验收。


## 0.1.9 固定 macOS 签名

仓库 Actions Secrets 使用 `MACOS_SIGN_CERTIFICATE`（加密 PKCS#12 的 Base64）和 `MACOS_SIGN_PASSWORD`。私钥与密码仅存维护者受限签名目录及仓库 Secrets，不能提交。`tools/macos-signing-cert.pem` 只有公有证书，必须保留在仓库；导入前及构建后校验固定证书，原生组件和应用均需匹配，禁止临时签名退路。

每个 macOS job 创建独立临时钥匙串，导入并赋予 codesign 工具访问，最后恢复搜索列表并删除该钥匙串。Tauri updater 继续使用原有的独立更新签名密钥与客户端公钥，不做轮换。

固定证书 SHA-256：`51100acc6e4f2d6e40e781618187b2f75b779ea6587d33c24f93002ee5f15a2f`。证书丢失、替换或过期可能需要用户重新授权，维护者应保留加密备份。自签名不等于 Apple Developer ID 或公证，首次安装的 Gatekeeper 提示仍需按系统流程处理。

本机验证：A（0.1.7）和 B（0.1.8）使用同一固定证书，在 `/Applications/AppFlash.app` 原位替换。只为 A 从系统列表重新添加一次辅助功能权限，B 没有再次授权；两版均经真实 CGEvent 组合键切换、最小化和恢复计算器。此证据验证这两份实际构建的身份连续性，不承诺任意后续换证书/包标识/安装路径的构建保留权限。

0.1.8 的标签与失败草稿保留。该次 GitHub macOS job 成功导入正式证书，但隔离签名测试的临时 identity 不可检索，发布门禁阻止上线。0.1.9 显式注册临时测试钥匙串搜索列表，并在测试与正式导入后等待精确 identity 就绪；没有改变证书 pin、用户系统信任或应用窗口功能。
