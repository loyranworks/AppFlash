# 多屏收起修复实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复一次全屏收起快捷键使其他屏幕应用连续跳转的问题，保持全屏与真实隐藏确认。

**Architecture:** 只读选定一个离开窗口后最多激活一次；以源应用失活作为允许尝试隐藏的条件。主动切离后按目标覆盖的显示器等待视觉切换，只有该路径刷新清单；最终隐藏独立核对实际状态。

**Tech Stack:** Swift AppKit/Accessibility、Python 原生回归、Rust/Tauri。

**Spec:** `docs/superpowers/specs/2026-11-01-multidisplay-collapse-design.md`

## Global Constraints

- 用户既定要求为保留全屏、隐藏代替最小化；不移动窗口或修改系统设置。
- 原地分支实现；不改变 Windows、签名或更新密钥。
- GUI 操作待 Mini 手动解锁；不控制宿主 Codex/ChatGPT。
- 源状态不明、目标切换失败时明确失败，忙碌按键不重放。

## Review Focus

- 全部候选失败时只能有一次激活动作。
- 缓存候选失败后不能继续另选应用。
- 源窗口仍可见但应用明确失活时不应继续试切。
- 系统焦点与前台 PID 不一致时不能成功。
- 两个全屏应用都隐藏时仍需独立普通离开窗口，实际输入位置不能丢。

### Task 1: 准备流程及回归

**Files:** `src-tauri/native/macos.swift`、`tools/test-macos-departure.py`、`tools/test-macos-departure-visibility.py`、`tools/test-macos-runtime.py`、`src-tauri/tests/macos_native.rs`。

**Interfaces:** `prepareToHide(_:windows:deadline:) throws -> Bool` 返回本次是否主动激活了目标；prepare-collapse 只在 true 时刷新清单，false 时维持已有快照。保持最终 `setHidden` 实测隐藏状态与 Rust 事务提交语义。

- [x] 提取真实生产准备函数到非 GUI 系统边界回归，先确认旧代码连续激活多个候选。
- [x] 最小修改：已失活即允许最终隐藏；优先可见窗口，只读选择、一次激活、严格确认、明确失败；增加诊断。
- [x] 三/四/七屏、多全屏窗口、跨屏目标、镜像、负坐标、上下与不同大小布局；插拔选择前/激活中改变布局需停止，不继续试切。
- [x] 非 GUI 回归、全套 `npm test`、格式与版本校验。
- [x] 自有 fixture 全屏、普通/多窗口、缺值和引用重建实际键盘回归；独立审查。

### Task 2: 固定证书与统一发布

**Files:** 五个版本文件、发布说明与本计划。

- [x] 固定证书候选构建、原位升级和真实快捷键验收，恢复配置。
- [ ] 提交 GitHub、统一发版、核对三平台与公开附件签名、摘要及更新入口。
- [ ] 明确报告物理双屏热插拔尚未本机实测，提供 MBP 更新与复核步骤。

## 证据与过程中发现的问题

- 原函数非 GUI 回归首先失败：一次激活拒绝后记录 `[2, 3, 4]`，缓存拒绝后为 `[4, 2, 3]`。源窗口失活但仍可见、前台状态矛盾、缺失布局和选中后布局变化均有失败先行用例。最终流程与真实几何辅助函数回归通过。
- 流程测试直接提取生产 prepareToHide；外部系统状态和 settlement helper 被控制，后者由独立 visibility 脚本提取真实生产实现测试，GUI 用例再组合两者。不是把所有组合测试都称为仅替换外部 API。
- 移除全局离屏约束的第一版以及仅要求目标上屏的一版，都使真实冷启动回归只发现 1 窗口而非 2。改为按相关显示器等待后又发现源已离屏时 SDK 会省略可选 kCGWindowIsOnscreen；临时真实日志证实 exact record 存在、property=nil、onscreen-membership=false。最初单测把缺键当未知也写错了；现按 SDK 定义纠正其预期，未知完整记录、错误 owner、错误字段类型仍拒绝。修正后冷启动实际 1→2、精确普通窗口激活和保留全屏隐藏全部通过。
- 非 GUI 几何覆盖 3/4/7 屏、同应用多个全屏窗口、跨两屏目标、镜像、负坐标、上下排列、不同显示尺寸/小数 Quartz 窗口坐标，以及目标屏消失。这里只验证坐标和状态逻辑，不等同于物理混合 DPI 或插拔测试。
- 最终完整测试 44 项 JavaScript、64 项 Rust 全通过。七组实际窗口回归均退出 0：latency、single/multi、busy、cold fullscreen、mixed fullscreen、引用重建后双全屏输入、系统焦点缺值后双全屏输入；两种输入场景各四轮且检查原插入位置。
- 最终生产 Swift SHA-256：`c85319cdf82b716063f64ee9a442daa3e2041e477414d414848e15619ff86ba0`。GUI fixture 副本仅在本轮唯一 bundle ID 范围内选择离开目标；所有实际按键前核对解锁控制台和精确测试 PID，fixture 启动及信号动作也拒绝锁屏。
- 固定证书候选 0.1.15 原位替换 0.1.14，完整签名、公有 pin 和 designated requirement 连续性通过，未重新授权。实际安装版通过真实全局组合键执行三轮全屏隐藏/恢复，每次恰好激活自有 Anchor PID 29884，Source PID 29935，全屏状态保留。实际运行版不能注入候选过滤，这一项是事后精确断言；本次没有选择个人应用。
- 手动运行时工具首次发送器存在 `.005` Swift 编译拼写错误，未发键且原配置恢复；已修正。最终报告确认 config_restored/fixtures_stopped；另行只读验证 AppFlash PID 30761 真实运行、配置与安装前逐字节相同。工具字段仅将 Popen 声称为 restart_requested，不以请求本身代替恢复验证。
- 独立审查已关闭冷启动 Important 回归，流程与几何函数检查通过；实际运行时脚本的离开目标断言从“非 Source”收紧为精确 Anchor，并使用已经取得的三轮记录重核通过。

Mini 只有一块活动显示器。未物理测试 MBP 双屏/三屏或热插拔；未控制宿主 Codex/ChatGPT。原配置与后台运行已恢复，临时 fixture 已退出。不能把模拟多屏元数据或单屏实机通过说成真实多屏已验收。
