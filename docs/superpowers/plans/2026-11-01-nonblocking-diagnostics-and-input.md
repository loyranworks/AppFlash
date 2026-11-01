# 非阻塞日志与跨平台按键入口实施计划

**Goal:** 修复共用提示及热路径阻塞，加入 Windows 防积压并发布，保留窗口策略。

**Architecture:** 日志用单后台消费者和有界队列；按键入口共享已有 RAII permit，平台窗口算法独立保留。

**Spec:** `docs/superpowers/specs/2026-11-01-nonblocking-diagnostics-and-input-design.md`

## 约束与审查重点

- `platform/windows.rs`、`switching.rs`、`macos_cycle.rs`、`native/macos.swift` 不改。
- 日志提交不等待文件、导出或排空锁；满队列有计数，时间为生产时刻；保留七天和每日上限。
- 导出在阻塞任务里等待顺序屏障；退出排空有截止时间。
- permit 覆盖等待和执行，发送失败与异常提前返回都释放；忙碌按键不进入动作队列。
- 托盘事件不清除草稿或手动暂停，提示按真实状态刷新。

## 任务

- [x] 日志代理仅修改 `diagnostics.rs`、对应测试及新日志辅助模块，先红后绿；给出根调用点需要的精确 API。
- [x] 主线程先重现托盘状态红例，修复并覆盖干净、未保存与英语显示。
- [x] 主线程共享 input gate，补慢执行与失败回收回归，保证窗口算法不变。
- [ ] 接通日志退出排空和运行时维护、全套测试、Windows CI 分支检查、独立审查。
- [ ] 固定签名候选实机验收，恢复配置；统一 GitHub 发版并核验公开附件。
- [ ] 用官方资料对比共享核心/平台适配方案，单独报告长期建议，不在本版重构内核。
- [ ] 用户追加本轮版本化备份与回滚，待确认配置回滚或整应用版本回退后实现，一并验收再发版。

## 已完成的三项修复证据

- 托盘事件红例复现中文、英文 footer 未同步；修复后15项设置会话回归通过，未保存提示仍优先。
- 提取真实 callback/Event/Gate，控制 Tauri 状态及系统注册边界；旧 Windows 分支明确失败于“Busy presses accumulated before execution”，Mac 对照通过。共享 permit 后两分支均通过，原三个 gate 和五个窗口循环回归通过。这不是 Windows 原生窗口实机验收，真实 worker 的 permit 生命周期由独立审查核对。
- 日志旧全局路径在真实存储锁被持有时不能返回；后台队列阻塞 send 的红例也明确失败。改为 try_lock/try_send 后，定向17例证明1条执行中＋128条排队时生产者仍返回、额外记录丢弃且导出可见；受控导出阻塞和正截止时间退出均通过。删去未被生产使用的 flush 备用API，只保留实际 export FIFO 屏障和 shutdown(timeout)。
- 此阶段完整59项 JavaScript、88项 Rust通过；独立审查及 API 收敛复核均无确认缺陷。四个窗口策略文件与基线33e1820逐字节相同；摘要保存在 `/tmp/appflash-nonblocking-0117/source-verification.json`。
- 三项修复候选0.1.17已固定签名构建并验证公有pin。菜单栏自动化读取 SystemUIServer 超时，未声称完成该入口实机点击验收；其事件处理已由生产controller回归验证。新增回滚功能仍需实现和最终重建、统一实测，尚未提交发布标签。
