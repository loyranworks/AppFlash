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
- [x] 接通日志退出排空和运行时维护、全套测试、两平台 callback 分支检查、独立审查；Windows正式CI也已通过。
- [x] 固定签名候选实机验收，恢复配置；统一 GitHub 发版并核验公开附件。
- [x] 用官方资料对比共享核心/平台适配方案，单独报告长期建议，不在本版重构内核。
- [x] 用户曾追加版本化备份与回滚，随后明确取消；本轮不新增回滚功能，恢复三项修复的发版范围。

## 已完成的三项修复证据

- 托盘事件红例复现中文、英文 footer 未同步；修复后15项设置会话回归通过，未保存提示仍优先。
- 提取真实 callback/Event/Gate，控制 Tauri 状态及系统注册边界；旧 Windows 分支明确失败于“Busy presses accumulated before execution”，Mac 对照通过。共享 permit 后两分支均通过，原三个 gate 和五个窗口循环回归通过。这不是 Windows 原生窗口实机验收，真实 worker 的 permit 生命周期由独立审查核对。
- 日志旧全局路径在真实存储锁被持有时不能返回；后台队列阻塞 send 的红例也明确失败。改为 try_lock/try_send 后，定向17例证明1条执行中＋128条排队时生产者仍返回、额外记录丢弃且导出可见；受控导出阻塞和正截止时间退出均通过。删去未被生产使用的 flush 备用API，只保留实际 export FIFO 屏障和 shutdown(timeout)。
- 此阶段完整59项 JavaScript、88项 Rust通过；独立审查及 API 收敛复核均无确认缺陷。四个窗口策略文件与基线33e1820逐字节相同；摘要保存在 `/tmp/appflash-nonblocking-0117/source-verification.json`。
- 三项修复候选0.1.17已固定签名构建并验证公有pin。菜单栏自动化读取 SystemUIServer 超时，未声称完成该入口实机点击验收；其事件处理已由生产controller回归验证。备份回滚追加范围已由用户明确取消，尚未提交发布标签。
- 平台架构当前只完成日志、输入准入共用模块的必要提取；统一 WindowBackend trait、能力模型及共享核心契约尚未实现，本轮维持原平台窗口策略，不把长期建议称为已落地。

## Mini 安装候选验收

- 固定签名0.1.17原位替换0.1.16，未重新授予辅助功能权限；旧应用保留于本轮临时目录。仅运行自有空白单窗口，使用解锁与精确前台PID守卫的组合键，真实验证激活、最小化、恢复三次成功；没有将它称为Windows实机验收或再次测试全屏策略。
- 设置页暂停/恢复按钮与底部提示一致。托盘入口自动化读取超时，其事件逻辑由真实controller的中英、两平台、未保存优先回归及独立审查确认，未声称实际点击托盘验收通过。
- 用真实系统保存对话框导出 `native-export-0.1.17.txt`，界面显示成功路径；文件有0.1.17启动及三次实际switch.done、UTC+08:00、累计丢弃数（本轮0）。该项证明真实导出与异步队列接通，阻塞IO不等待的保证由受控存储并发回归单独证明。
- 原配置逐字节恢复，测试fixture已退出，候选后台存活；完整记录 `acceptance.json` 与 `rig-observations.jsonl` 保存在 `/tmp/appflash-nonblocking-0117`。不修改任何私人应用内容。

## 长期架构建议的依据

参考 [Qt QPA](https://doc.qt.io/qt-6/qpa.html) 的公共平台接口与不同平台后端，以及 [Tauri架构](https://v2.tauri.app/concept/architecture/) 对WebView/runtime的分层。针对本项目的建议是共享配置、暂停/输入准入、调度、日志与错误契约，用Rust trait包装平台端口，保留Win32与AX/Spaces机制和当前两种Cycle策略。共同契约测试加各平台实机验收后再逐项收敛产品语义，不直接把Mac算法移植到Windows；无需为外观统一而更换整个UI框架。此处为建议，不是本版已实现的完整重构。

## 正式发布结果

- 发行提交 `7ff98c6804fe3fc19e2a54e73569f1bbeb49fc55`，标签 `v0.1.17`；[Actions 41273108651](https://github.com/loyranworks/AppFlash/actions/runs/41273108651) 的prepare、两个macOS架构、Windows及publish全成功。Windows实际CI运行并通过npm test与NSIS构建；源callback受控回归仍不等同Windows原生窗口实机验收。
- [Release 430360853](https://github.com/loyranworks/AppFlash/releases/tag/v0.1.17) 于2026-11-01 12:34:44（Asia/Shanghai）发布，公开正文已确认保留三项修复、完整重构未实施及实测边界。
- 九个公开附件全部匿名下载，大小与GitHub API SHA-256匹配；三个更新包通过原客户端公钥、0.1.17签名版本及篡改拒绝测试。两个Mac架构的归档与只读DMG应用通过完整固定签名、pin、requirement、标识、版本及架构校验，挂载均已卸载。latest更新入口为0.1.17。
- 正式产物记录在 `/tmp/appflash-release-v0117/{release,download-verification,package-verification,latest-from-endpoint}.json`。Mini当前运行已验收本地候选，正式发行包另行抽验，不将二者称为相同二进制。
- 最后只读核对原配置逐字节相同、候选后台PID56209存活。旧0.1.16应用已在测试前移出Applications保留于本轮临时目录；fixture退出。dot-doctor已结束、退出0，仍报告既有软链/目录/同步与SSH公钥备份权限待处理，未改动无关系统配置。
- 用户询问完整平台重构风险，已说明中等规模、一次合并算法可能导致回归；后续先保持线程/调用顺序/成功判断原样封装接口，分别完成Windows/Mac实机基线，再逐步迁移。没有在本版承诺或实现此重构。
