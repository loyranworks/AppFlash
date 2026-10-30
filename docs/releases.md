# 发布与升级

GitHub CLI 已取得 workflow 授权，正式工作流位于 .github/workflows/release.yml，仅推送 v* 标签才触发发布。普通 main 提交不发布安装包。尚未配置更新签名 Secret，也未执行线上发布验收。

远端：`https://github.com/royluo2026/AppFlash.git`。源码与双语 README 已提交并通过 HTTPS 推送；项目使用独立 Git 仓库，不修改父目录仓库 remote。仓库继续保持私有。

## 一次性准备

1. 源码与工作流已准备。使用 HTTPS / gh 登录推送即可；SSH 或连接器授权不是发布前提。发布前核对 main 的测试结果和版本，不要误改父目录仓库。
2. **使用同一个 AppFlash 仓库，调试完成后再公开。** 调试阶段保持私有，不另建发布仓库。Release endpoint 不变；普通客户端在私有阶段无法匿名读取。公开前检查源码与历史中没有凭据、私钥和私人配置，再由维护者明确变更可见性并发布正式 Release。客户端不携带 GitHub Token。
3. 更新签名私钥已生成在 `C:\Users\luoyi8\.tauri\app-flash.key`，不在项目内。请自行安全备份，再将文件内容填入仓库 Actions Secret `TAURI_SIGNING_PRIVATE_KEY`；如密钥有密码，配置 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。客户端仅包含匹配的公钥。不要将私钥提交到 Git，也不要更换公钥导致已安装客户端失去更新能力。
4. 开启 GitHub Actions。首次发布仍需用户下载并安装安装包，开发目录的裸 exe 不作为正式便携版更新支持。

## 发布新版本

同步修改 `package.json`、`src-tauri/Cargo.toml` 与 `src-tauri/tauri.conf.json` 的版本，例如 `0.1.1`；相应更新 lock 文件。执行 `npm test` 和 `node tools/check-release.mjs`。自行提交，再推送对应 `v0.1.1` tag。

`.github/workflows/release.yml` 构建 Windows x64 NSIS、Apple Silicon 和 Intel macOS 应用。先上传到草稿 Release，所有平台成功且 `latest.json` 包含三个平台的签名和地址后才正式发布。普通代码 push 不发布；不一致版本或缺失密钥会失败；失败构建不会将半完成版本推给客户端。

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
- 更新签名不等于操作系统代码签名。当前 Windows 未做代码签名，macOS 未做 Developer ID 签名 / 公证，分发仍需处理系统安全提示。

## 验证限制

本地已构建 Windows NSIS 安装包及更新签名，测试使用客户端公钥验证真实安装包并确认篡改后的字节被拒绝。尚未推送、设置远端密钥、运行线上 Actions 或真实跨版本安装。Mac 两种架构的构建、签名 / 公证及登录启动需 Mac 实机验收。已确定后续公开同一仓库；当前保持私有且没有 Release，公开和正式发布前不能声称客户端已经能收到线上更新。
