# 贴边错误与本地日志 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** 将窗口错误带到用户当前桌面的贴边，并保留可导出的七天本地诊断，连同全屏切离/去积压修复发布 0.1.11。

**Architecture:** Rust 独立日志模块管理时间、大小和导出；Rust 通知状态管理 6 秒有效期，复用既有 overlay 与原生 NSPanel。前端只显示状态与导出入口，系统保存对话框由 Rust 发起。

**Tech Stack:** Tauri 2 / Rust / Swift / HTML CSS JS；复用现有 time 0.3.55，增加官方 dialog 插件。

**Spec:** docs/superpowers/specs/2026-10-31-edge-errors-and-local-logs-design.md

## Global Constraints

- Navy `#244b70`、B 清透玻璃、主线和流光保留，整体 50–150% 缩放保留。
- 错误保留 6 秒，成功立即清除；普通提示关闭时也能显示；保留快捷键列表。
- 日志七个日文件，每日最多 2 MiB；启动及运行期间定期清理，仅操作自己的日志文件。
- 不记录窗口内容；导出由用户在系统保存框选 TXT 路径；日志失败不阻止切换。
- Codex 自身禁止 CUA 操作，使用独立测试窗口，不绕过工具限制。

## Review Focus

- 新错误更新但不制造通知队列；成功/过期后恢复正常几何尺寸。
- 原生玻璃 resize 与 50%/150% 比例下长错误仍可读且不裁掉绑定列表。
- 跨日和过期日志清理不能删除其他文件，满额后不能无限增长。
- 导出取消或磁盘失败不修改配置、不影响切换；保存对话框不阻塞窗口控制线程。
- 忙碌按键被丢弃、keyup 保持正确、后续新按键与单/多/全屏循环正常。

### Task 1: 本地日志模块

**Files:** src-tauri/src/diagnostics.rs; src-tauri/tests/diagnostics.rs
**Interfaces:** `init(PathBuf, &str)->io::Result<()>`, `record(&str,&str)`, `maintenance()`, `export(&Path)->Result<(),String>`; Logger 实例方法支持注入 SystemTime 测试。
- [x] 写真实文件测试：七天边界、非日志文件保留、大小上限、并发整行、TXT 时间顺序和 Unicode、不可写路径。
- [x] 先失败，再实现按日 UTF-8 记录、去换行注入、2 MiB 上限、七日清理、合并导出。
- [x] 单测通过。模块只负责文件，不接触 UI 或窗口控制。

### Task 2: 融合错误浮层与导出入口

**Files:** ui/overlay.html/js/css, ui/index.html, ui/settings.js/css, ui/i18n.js, relevant node tests
**Interfaces:** overlay event `switch-notice` payload `{id:number,message:string}` or `null`; export `invoke('export_logs')->string|null`；前端不控制原生 TTL。
**Geometry:** 正常 rail 120pt；有错误时 280pt 宽、正常行布局上方额外 104pt，整体跟随原缩放。Rust 控制窗口边界，CSS 留同等空间。
- [x] 写 JS 行为测试：通知更新/清空、HTML 文本安全、配置变更保留错误、导出成功/取消/异常。
- [x] 使用同一玻璃容器、主线和流光，融合 Blush 与深暖色状态强调；保留应用行，支持长中文/英文、浅深色及减少动态效果。
- [x] 通用页增加导出按钮与状态，翻译完整，不将窗口错误继续只放设置红字。
- [x] `node --test ui/*.test.js` 全绿；主任务负责实际截图与原生全屏验收。

### Task 3: 运行时与系统保存框接入

**Files:** src-tauri/src/overlay_notice.rs, runtime.rs, main.rs, platform/macos.rs, overlay_layout.rs, Cargo.toml/lock, tests
- [x] 通知状态的 6 秒边界、替换和成功清除先红后绿。
- [x] 初始化标准日志目录；记录启动、切换结果/耗时、原生阶段、丢弃按键汇总；读取 helper stderr 进入日志；现有循环每小时维护。
- [x] worker 失败写通知状态；overlay 线程独立于 prefix/普通开关显示通知，更新 geometry、超时恢复；显示失败限速记录。
- [x] Rust async `export_logs` 只接受 settings 窗口请求，在后台线程调用系统保存框，不接收前端任意写入路径。
- [x] 完整 npm test、版本、格式、签名与真实热键/全屏/错误浮层/导出/配置恢复通过。

### Task 4: 审查、发布、正式包验证
- [x] 独立代码审查，修复 Important/Critical。
- [ ] GitHub 统一发布 0.1.11，三平台及 publish 全绿。
- [ ] 下载摘要/签名/公钥篡改拒绝/固定证书核对，正式包验收，恢复原配置；记录未实测 Codex/MBP 的范围。
