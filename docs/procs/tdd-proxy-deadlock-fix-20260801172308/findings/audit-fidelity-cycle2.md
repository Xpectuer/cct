---
doc_type: proc
brief: "Fidelity audit: 计划忠实度 (cycle 2)"
source_skill: execute
audit_phase: fidelity
audit_angle: fidelity
audit_cycle: 2
confidence: verified
---

# 审查角度: 计划忠实度 (cycle 2 复核)

**审查依据**: plan 全部 step / Execution Order（ref/plan/code-spec.md 25 步 + DAG）+ cycle 1 报告（findings/audit-fidelity-cycle1.md 6 个偏离）
**审查周期**: 2/3
**复核方法**: 修复后文件逐行对照——tests/proxy_contract.rs:599-725（TC-9 三态 + 新测试）、docs/references/install-script.md:143-155（迁移小节）、docs/references/codex-backend-development-guide.md 全文 full_auto 语义与 src/config.rs:66-122 / src/app.rs:108-223 / src/ui.rs:84-96,146-150,244-252 / src/launch.rs:193-206 代码交叉核对、poc.md:70-81 Results Log；实跑 `cargo test`（194/194 全绿，raw 输出 12/12 含 `stop_rejects_stale_socket ... ok`）+ `verify-B011`（exit 0）+ `verify-B013`（exit 0）；mtime 核查修复范围（cycle 1 报告之后仅 5 个文件被改动：2 个 fix 日志 + tests/proxy_contract.rs + install-script.md + codex-backend-development-guide.md + poc.md——与 fix 日志声明一致，无 src/ 改动）。

## 评分明细
| # | 检查项 | Cycle 1 | Cycle 2 | 证据（cycle 2） | 严重程度 |
|---|--------|---------|---------|----------------|----------|
| 1 | TC-1（Step 1）：CCT_PROXY_SOCKET env 优先；单测 set_var/remove_var；Red 真失败 | 10 | 10 | 修复未触及，cycle 1 证据成立 | — |
| 2 | TC-2（Step 3）：应用层探测；三常量；send_control 签名保持（#9） | 10 | 10 | 同上 | — |
| 3 | TC-3（Step 5）：单次 lsof + 降级链；降级文本含 "lsof -iTCP"；真隔离 | 10 | 10 | 同上 | — |
| 4 | TC-4（Step 8）：两 helper 集中；outbound 泄漏闭环于 TC-8 | 10 | 10 | 同上 | — |
| 5 | TC-5（Step 9）：shutdown_proxy STOP_TIMEOUT 传播；stop_proxy 两态；Refactor 只移代码 | 10 | 10 | 同上 | — |
| 6 | TC-6（Step 11）：骨架一致（≥20 status + GET + 3s 界）；依赖链无跳步 | 10 | 10 | 同上 | — |
| 7 | TC-7（Step 11）：vacuous Red 理由可信；Green 真实验证转发/Bearer/SSE | 10 | 10 | 同上 | — |
| 8 | TC-8（Step 11）：stderr piped；ctl + 请求两路径 | 10 | 10 | 同上 | — |
| 9 | **TC-9（Step 11）：三 case 全覆盖——修复闭环** | 7 | **10** | **Fix B 已验证**：tests/proxy_contract.rs:689-725 `stop_rejects_stale_socket`（bind→drop 残留文件 + `stale.exists()` 前置 :696-699 → spawn 真实 `cct proxy stop` → 四断言：非 0 退出、<1s 快速、stdout 不误报 "Proxy is not running."、stderr 含 "Error" 反真空守卫）；原测试文档注释已更新为三态说明（:599-604）；raw 输出 `stop_rejects_stale_socket ... ok`，12/12 全绿（6.22s）；全量 194/194（cycle 1 193 + 新增 1）。**可证伪性成立**：旧实现（connect 错误吞掉 → exit 0 + 打 "Proxy is not running."）下断言 1/2 必红，stderr 守卫封死静默退化 | 已闭环 |
| 10 | TC-10（Step 11）：SIGKILL→残留→重启 Ok；CCT_PROXY_BIN 注入 | 10 | 10 | 未触及，cycle 1 证据成立 | — |
| 11 | TC-11（Step 11）：占用者存活断言 | 10 | 10 | 同上 | — |
| 12 | TC-12（Step 11）：**偏差 1** bind 顺序先 TCP 后控制 | 8 | 8 | 复核 src/proxy.rs:239-243（TCP 先行注释 = 代码内决策记录，指向 analysis 文档）、:261-286（EADDRINUSE 分支**非死代码**：delete-on-conflict 仍服务僵尸文件、重绑冲突重探测 3×500ms 保留、`exit_socket_owned` 防御保留）；AC10 契约在 TCP 层仲裁（tests 20/20 未改断言）。**残差未变**：code-spec.md Step 6（:272 起）仍为 plan 原文，未补 bind 顺序决策注记；tdd.md 亦无该记录（grep TC-12/竞态/EADDRINUSE 仅命中 RGR 表行 :35）——建议级，非必需修复 | 次要 |
| 13 | TC-13（Step 7）：shutdown 清理 socket；handle_control 签名变更 | 10 | 10 | 未触及 | — |
| 14 | TC-14（Step 13）：临时 CODEX_HOME + 快照对比 | 10 | 10 | 未触及 | — |
| 15 | TC-15..19（Step 12）：fake 真实应答；5 契约断言 | 10 | 10 | 未触及 | — |
| 16 | **TC-20（Step 15）：15/15 原始输出归档——修复闭环** | 8 | **10** | **偏差 2 闭环**：logs/run_all_full_pass-audit-fix1.md 为终态 15/15 完整原始转录（15 个逐脚本 PASS 行含真实 session-id 019fbe81-… 与 Terminated 清理噪音、`Total: 15 \| Pass: 15 \| Fail: 0 \| Skip: 0`、`All checks passed.`、起点时间 02:06:11 与完整性修复周期吻合）；plan Step 15 Verify 门证据已归档；基线补录行（poc.md:73-80 共 8 行）与 fix1 转录互证 | 已闭环 |
| 17 | TC-21（Step 16）：B006/B007/B008 id 级核对成立 | 10 | 10 | 未触及（completeness 角度另行复核） | — |
| 18 | TC-22（Step 17）：B015 PASS + Results Log 当日行 | 10 | 10 | 未触及 | — |
| 19 | **TC-23（Step 19-21）：迁移小节 socket 路径——修复闭环** | 7 | **10** | **Fix C1 已验证**：install-script.md:151-152 双平台措辞（`~/.config/cc-tui/proxy.sock`，Linux；`~/Library/Application Support/cc-tui/proxy.sock`，macOS）与代码 `dirs::config_dir()/cc-tui/proxy.sock`（src/proxy.rs:71-74）两平台语义一致；迁移三要素（29182 lsof→kill / 遗留 socket / 死锁实例）完整保留（:147-154）；verify-B011 断言模式不含路径，实跑 PASS（exit 0） | 已闭环 |
| 20 | TC-23 范围核查（**偏差 4**）：full_auto 表/正文一致性 | 8 | **10** | **Fix C2 已验证（选项 b 全文对齐）**，表/正文/代码三向一致：guide:39 表、:66 字符串语义 + 遗留 bool 注（`true`→`danger`、`false`→unset，与 config.rs:66-87 `deserialize_approval` 逐项吻合）、:83 示例 `"never"`、:103 标签 "Approval"（app.rs:13 一致）、:114 索引表、:119-121 表单映射（y/yes→danger 向后兼容，app.rs:199-204 一致）、:142-145 着色（untrusted 绿→never 黄→danger 红、unset 白，ui.rs:146-150 一致）+ 详情 `approval: <level>`（ui.rs:245-248，approval_label unset→on-request，config.rs:96-103 一致）、:176 旗标映射（launch.rs:196-206：danger→bypass、never/untrusted→`--ask-for-approval` 一致）、:210 `toggle_full_auto` 字符串形（config.rs:702-722 一致）；B013 实跑 PASS（exit 0）。**新残差**（cycle 2 新发现，预存 drift）：guide:146 "footer hint changes to `s: Full-auto`" 与代码 ui.rs:90 `[s] Approval` 不符——该行自 1e94754 引入、a67aeeb 改 footer 标签后未更新，非本轮修复引入，一行可修 | 已闭环 + 新残差极轻 |
| 21 | 终端步骤（22-25）：Proof-Read/Cross-Check/review.md/提交 | 8 | 8 | 未触及，设计预期（tdd.md:68 审计循环为前置门）；回归门已实跑 194/194（cycle 1 193 复验 + 新测试） | 设计预期 |
| 22 | 执行顺序：DAG 与 tdd.md 依赖列一致 | 9 | 9 | 未触及，cycle 1 判定（TC-15..19 先于 infra 属良性并行）成立 | 极轻 |

## 偏离详情
（仅列出评分 < 10 或新发现的检查项）

### 偏离 1（复核）: TC-12 bind 顺序决策记录未落 plan（维持 8/10）
- **关联检查项**: #12
- **评分**: 8/10（不变）
- **证据**: 修复 agent 未处理此项（fix 日志范围仅 TC-9 测试与 3 文档）；code-spec.md Step 6（:272 起）仍为 plan 原文；tdd.md 无决策注记。代码侧已有充分记录（src/proxy.rs:239-243 注释 + findings/double_start_race_one_wins-analysis.md），EADDRINUSE 控制分支非死代码论证复核成立（:261-286）
- **修复建议**: 无需代码修复。终端步骤执行时在 code-spec.md Step 6 补一行"bind 顺序决策：TCP 先行仲裁（方向 A，见 analysis）"即可闭环；属建议级

### 偏离 6（复核）: 终端步骤 22-25 未执行（设计预期，维持 8/10）
- **关联检查项**: #21
- **评分**: 8/10（不变）
- **证据**: tdd.md:68 明示审计循环为前置门；本次复核 `cargo test` 194/194 复验通过（回归门已实际执行）
- **修复建议**: 三循环审计完成后按 cycle 1 偏离 6 建议执行 Step 22/23/24/25

### 新发现（极轻）: codex-backend-development-guide.md:146 footer 提示文案陈旧
- **关联检查项**: #20（附注，不影响该项评分）
- **证据**: guide:146 "the footer hint changes to `s: Full-auto` on the Codex tab" vs 代码 ui.rs:90 `[s] Approval`（ui.rs:468 测试锁定该文案）；预存 drift（1e94754 引入 guide、a67aeeb 改 footer 标签后未同步），非本轮修复引入，也不在 cycle 1 偏离 5 标注的 6 处布尔叙述之列
- **修复建议**: 一行改为 "the footer hint changes to `s: Approval` on the Codex tab"

## 角度总评
SCORE: 8
**总分**: 8/10（所有检查项最低分，与 cycle 1 同口径）
**通过阈值**: ≥ 9

Cycle 1 判定的 **2 个必需偏离全部修复并经独立复核闭环**：TC-9 case ③（`stop_rejects_stale_socket` 实现 tdd.md:50 承诺的三态全覆盖，四断言含反真空守卫，raw 运行 12/12 + 全量 194/194 无回归）与 install-script.md socket 路径（双平台措辞与 `dirs::config_dir()` 语义一致，B011 PASS）。建议性偏离 2/3/5 亦闭环：15/15 终态原始转录已归档（run_all_full_pass-audit-fix1.md）、full_auto 表/正文与代码三向一致（6 处正文 + 表格逐一比对 src 四模块）、poc.md:73 补 ps -p 29182 确认记录。修复范围经 mtime 核查与 fix 日志声明一致（仅 1 测试文件 + 3 文档 + 2 日志），无越界改动。剩余两项 8 分：偏差 1 的 plan 注记（建议级，一行）、终端步骤 22-25（设计预期，审计循环后执行）。新发现残差仅 guide:146 footer 文案一行（预存 drift，极轻）。

## 判定
✅ 修复充分（NEEDS_REWORK 残留项均为建议级/设计预期）——cycle 1 两个必需偏离已闭环并验证，无新增实质偏离；残余：偏差 1 的 code-spec Step 6 一行注记、终端步骤 22-25、guide:146 footer 文案一行。建议在终端步骤执行时一并补齐，即可达 ≥9 阈值。
