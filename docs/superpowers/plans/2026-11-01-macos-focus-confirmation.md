# macOS 焦点确认修复实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复系统焦点查询缺值造成的误报，验证切换后的实际键盘接收与延迟后发布 0.1.14。

**Architecture:** 保留系统精确焦点的正常路径，只为明确不可用的系统属性增加有多重证据的窗口确认。使用测试副本在 AX 读取边界注入缺值，其他 WindowServer、AppKit、窗口和键盘均使用真实组件。

**Tech Stack:** Swift AppKit/Accessibility、Rust/Tauri、Python 原生验收工具。

**Spec:** `docs/superpowers/specs/2026-11-01-macos-focus-confirmation.md`

## Global Constraints

- 原地分支 `fix/macos-focus-confirmation`，按用户已授权方式执行，不另建工作树。
- 不改变 Windows 行为、证书、更新密钥、视觉设计或用户内容。
- 不控制宿主 Codex/ChatGPT，不操作系统认证界面。
- 总激活预算 1.2 秒、最多两次修复不增加；缺值只允许白名单错误。

## Review Focus

- 同应用错误窗口不能被前台 PID 掩盖：精确窗口负例。
- 全屏动画未结束不能只凭本地缓存成功：精确 WindowServer 可见性负例及真实键盘接收。
- 系统明确聚焦其他应用不能被降级掩盖：冲突状态负例。
- AX 忙碌、权限、失效不属于缺值：错误状态负例。
- 成功切窗不等于网页输入框聚焦：实际文本插入位置验收与日志来源区分。

### Task 1: 有证据的焦点确认与回归

**Files:** `src-tauri/native/macos.swift`、`tools/test-macos-native.py`、`tools/test-macos-focus-confirmation.py`、`src-tauri/tests/macos_native.rs`。

**Interfaces:** 保持 `hasExactFocus(_:deadline:observed:)` 的布尔契约或等价调用方更新；新增纯降级判定函数供生产与表驱动用例共用。激活日志保留真实系统观测 PID，并附加确认来源。

- [x] 扩展独立双全屏测试，注入仅 system-wide AXFocusedApplication 的 `.noValue`，并核对真实文本插入位置；运行确认旧代码激活超时失败。
- [x] 表驱动负例要求 wrong PID/window/offscreen/minimized/hidden/hard AX error 均不通过，先确认失败原因符合预期。
- [x] 实现最小降级及来源日志；没有证据要求改变焦点写入顺序，本轮保留原顺序。
- [x] 重跑缺值双全屏键盘、普通/多窗口/全屏/重建根用例及完整 `npm test`。

### Task 2: 性能、正式构建与发版

**Files:** 五个版本字段、发布记录及验收证据文档。

**Interfaces:** Task 1 通过的正式原生代码进入固定证书候选；现有发布工作流和签名校验工具保持使用。

- [x] 与 0.1.13 基线比较 Safari/Hermes 真实快捷键延迟和输入位置，不把日志完成时间等同动画结束。
- [x] 独立审查最终分支，解决阻断项；完整测试、格式和版本检查通过。
- [ ] 固定证书构建安装验收，恢复测试配置和窗口；提交 main 与 v0.1.14，确认 CI 三平台和正式附件签名/校验。
- [ ] 记录已测及未测边界，提供 MBP 更新链接，不把 Mini fixture 等同 MBP ChatGPT 实测。

## 发布前证据（2026-11-01）

- 完整最终测试：44 项 JavaScript、62 项 Rust；版本检查及 Rust 格式检查通过。
- 仅系统 AXFocusedApplication 缺值的旧代码回归失败：实际 fixture 已 active/key/onActiveSpace，原生激活仍在 1.2 秒后报错。修复后完全隔离的两个全屏 fixture 连续四轮实际文本插入、隐藏/恢复通过；CGEvent 常驻发送器在成功应答后 100ms 内发键，不等待焦点补齐。输入从位置 4 开始，逐次检查字符串及新插入位置。
- 同 CGWindowID 的 AX 根重建后，四轮双全屏输入与隐藏/恢复通过。单/三窗口、busy、冷启动全屏、混合全屏、关闭窗口清理、查询延迟回归通过。
- 同条件六轮原生激活对照：Safari 中位数 260.8ms → 164.8ms；Hermes 29.4ms → 22.9ms。这是 IPC 激活确认时间，不等于完整动画时间。
- Safari 独立本地页面全屏测试，常驻发送器在应答后 5.6–73.8ms 发出的四个字符全部进入位置 8，最终位置 12。原先用每次新启动发送器的测试存在启动开销，不能单独作为“立即输入”的证据，已用常驻发送器重测。
- Mini 原位安装 0.1.14 固定证书候选，designated requirement 与 0.1.13 相同；八次实际全局热键全部成功，未修改辅助功能授权。Safari 按键到日志完成 211.0–238.6ms；Hermes 63.8–110.4ms（首个冷操作 110.4ms）。日志实际使用新来源字段。原配置逐字节恢复，设置实际显示 0.1.14，自有 Safari 普通测试窗口已关闭。
- 源码与测试工具经独立审查，无 Critical/Important。非阻断限制：纯策略负例尚未覆盖全部 AX 读取映射；Anchor 启动失败会安全失败，但缺少单独就绪/错误输出诊断。

## 测试失败与修正边界

初始冷启动用例在原版和候选均失败：直接 Popen 的 departure fixture 的 launchDate=nil，排序落在 Hermes 后；切到其他全屏桌面不会暴露源普通桌面窗口。测试工具现为全屏模式编译生产源码副本，仅允许本轮 UUID 的自有 fixture 作为离开目标；先将自有普通窗口激活到其真实桌面，再启动源应用，保留源首次 AX 查询发生于全屏后。隔离后的原版/候选冷启动均通过。这不是对生产目标选择策略的改动。

隔离又暴露了双全屏用例原先依赖个人应用作为第三个离开目标；现新增自有普通 Anchor fixture，四轮缺值及根重建回归通过。一次 fixture 启动未写出状态，后续加进程退出码和 stderr 诊断后重测通过；不将该次超时宣称为已定位的生产问题。

所有全屏源码验收均明确打印未注入源码的 SHA-256；候选为 `f19a3c660c280c83af1c14287cbdb3e444d179817bb00e4038420a031e34f865`，原版为 `dc9ec6ec5eb980de5ed95628e22984a006c223ab5c81f9bce4a5d0083aa4d6e5`。这些用例不等同于已签名二进制验收；后者由原位安装及实际全局快捷键单独验证。

未直接操作 MBP 或宿主 Codex/ChatGPT；修复其报告的缺值机制不等于已在该应用实测。短时间锁屏基线没有发现首次额外卡顿，未覆盖候选长时间锁屏、所有应用或完整视觉动画。
