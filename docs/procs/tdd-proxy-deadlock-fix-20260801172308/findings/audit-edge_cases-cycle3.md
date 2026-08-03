---
doc_type: proc
brief: "Fidelity audit: 边界覆盖 (cycle 3)"
source_skill: execute
audit_phase: fidelity
audit_angle: edge_cases
audit_cycle: 3
confidence: verified
---

# 审查角度: 边界覆盖 (Cycle 3 — 终轮复核)

**审查依据**: AC1/AC3/AC10 / Step 5/6/9/11/12
**审查周期**: 3/3
**Fix 依据**: logs/audit-fix-cycle2.md（Fix D：新增 `control_socket_rebind_exhaustion_exits`）
**审查方法**: 源码 + 测试逐行复核；**实证验证**（测试连跑 + 三组变异体实验）替代仅读日志。

## 评分明细

| # | 检查项 | 评分 | 证据 | 严重程度 |
|---|--------|------|------|----------|
| 1 | AC1 时间界：死锁回归测试带超时断言（3s 界） | 10/10 | proxy_contract.rs:388-443 复核无变化（BUDGET=3s + recv_timeout 2s + 控制线程 recv_timeout 双路径有界）；本次全套复跑 13/13 ok | — |
| 2 | AC10 stop 超时三态：无文件/拒绝（stale）/无响应全覆盖 | 10/10 | 复核无变化（proxy_contract.rs:607-680 无响应+无文件、689-724 stale；时间界 2.5s vs <1s 可区分）；套件内 2/2 ok | — |
| 3 | AC3 占端口三态：占者存活/非 0 退出/诊断文本/不 panic | 10/10 | 复核无变化（proxy_contract.rs:767+ occupant listener 保活 + wait_child_exit 5s 有界 + 诊断断言）；套件内 ok | — |
| 4 | EADDRINUSE 耗尽路径：分支可达、收敛契约有直接测试 | **9/10** | **Fix D 复核见偏离 1**：测试真实存在、确定性命中分支（1.59-1.63s×3 实证循环执行）、四断言齐全、变异体 2/3 必红；残余 = 重探测节奏（3×500ms）本身未 pin（变异体 A 绿） | 已修复 |
| 5 | lsof 降级全谱：四守卫齐备；缺失态有测试 | 9/10 | 复核无变化（proxy.rs:472-487：`.output().ok()?` / `!status.success()` / `from_utf8().ok()?` / 非空 filter 四守卫）；失败/空/非 UTF-8 三子路径仍无专门用例 | 次要 |
| 6 | 诊断端口来源：实际绑定端口非硬编码 19191 | 10/10 | 复核无变化（launch_proxy_contract.rs:320 `format!("port {port} already in use")` 动态端口 needle） | — |
| 7 | 双启动收敛界：败者 ≤2s、恰一存活、无 panic；serial 隔离 | 10/10 | 套件内 double_start_race_one_wins ok（13/13 全套） | — |
| 8 | 僵尸恢复竞态窗口：双非 panic 出口各有契约测试 | 9/10 | 复核无变化（proxy.rs:245-253 exit(1) + launch.rs:163-173 bail 各自有测试）；组合交错仍无确定性手段 | 次要 |
| 9 | 就绪探测边界：3×500ms 耗尽 → bail，不挂起 | 9/10 | 复核无变化（launch.rs:163-173）；静默应答者最坏形态未测 | 次要 |
| 10 | 并发关闭：shutdown 先响应后删文件；accept 错误 sleep 100ms | 9/10 | 复核无变化（proxy.rs:525-528 控制侧 sleep 100ms；HTTP `Err(_) => continue` 为前序遗留）；accept 错误分支无测试 | 次要 |

## 偏离详情

### 偏离 1（Cycle 2 主导，Fix D 已应用）: EADDRINUSE 重探测耗尽分支无直接测试

- **关联检查项**: #4
- **Cycle 2 评分**: 8/10 → **Cycle 3 评分**: 9/10
- **审计员实证复核**（非仅读日志）:
  1. **测试真实存在且通过**: `cargo test --test proxy_contract control_socket_rebind_exhaustion_exits -- --exact` 连跑 3 次 → 1.60s / 1.63s / 1.59s 全绿；全套 `cargo test --test proxy_contract` ×2 → 13 passed（7.78s / 7.81s）。耗时 1.6s 与 fix 日志（1.58-1.63s）一致，且**只有 3×500ms sleep 循环真实执行才可能产生该耗时**——循环执行本身被实证。
  2. **构造确命中耗尽分支**: 空目录占 `CCT_PROXY_SOCKET` 路径 → TCP 动态端口空闲 bind 成功（避开 TCP-first 仲裁）→ 控制 bind 目录 → EADDRINUSE（`is_bind_conflict` 匹配 AddrInUse|AlreadyExists）→ `check_proxy_running` false（connect 目录 → ENOTSOCK 立即失败，探测快速）→ `remove_file` 对目录必然失败（macOS EPERM，`let _ =` 吞掉）→ rebind 仍冲突 → 3×500ms 耗尽 → `exit_bind_failed` → exit(1)。**目录占用者形态无 µs 级竞态**（fix 日志对"循环抢绑线程"方案的 flake 论证成立：remove→rebind 窗口 ~µs，抢绑者需恰落窗口内才赢——审计员认同该设计决策），且 remove 若意外成功则 rebind 成功、proxy 正常启动 → 测试在 wait 预算内挂起红——机制自检无盲区。
  3. **断言四件套逐项核验**（proxy_contract.rs:927-946）: ① `!status.success()`——exit(1) 非 0（silent-success 实现必红）；② `stderr.contains("control socket bind")`——命中 `exit_bind_failed` 的 `[cct-proxy] control socket bind {path:?} failed:` 契约文本（误报 "another live proxy owns" 必红）；③ `!stderr.contains("panic")`——无 panic；④ `elapsed <= 3s`——有界收敛，且比 wait_child_exit 的 5s 预算更严格（3~5s 间退出 → elapsed 断言红）。断言表面与真实代码路径吻合（main 顶层错误处理无关本路径，诊断直接来自 proxy 子进程 stderr）。
  4. **可证伪性——变异体实验**（审计员改 src/proxy.rs 重绑冲突分支，逐组跑测试后恢复，恢复后 diff 与备份逐字节一致 + 测试复绿）:
     - **变异体 A（重探测循环整体删除 → 重绑冲突立即 exit_bind_failed）: 测试绿**（0.05s）。**残余缺口**：3×500ms 节奏本身未被 pin——若回归把循环删掉改为立即退出，收敛契约（非 0 + 消息 + 无 panic + ≤3s）全部保持，测试不红。行为差异仅在时间维度（1.6s vs 0.05s），无下界断言兜底。
     - **变异体 B（`for _ in 0..PROBE_RETRIES` → `loop` 无限重试）: 测试红**（5.17s，wait_child_exit 5s 预算断言 "never exited"）——无限重试/挂起被 pin。
     - **变异体 C（分支内 `panic!`）: 测试红**（0.29s，断言 ② 消息缺失 + ③ panic 双杀）——panic 被 pin。
     - 结论: 任务列出的三种错误实现中 **panic 与无限重试必红**；"重探测分支缺失"（狭义的"删循环立即退出"形态）**不红**——这是与 cycle 2 报告"可证伪性成立"表述之间的唯一实证差异，故评 9 而非 10。
  5. **处置**: 收敛契约（可观测面）全部 pin 且稳定；循环节奏属实现参数（PROBE_TIMEOUT/RETRIES 常量本身亦无 pin 测试，项目 assert-contracts 规则允许）。**低成本加固建议**（一行，不阻塞）: 增补下界断言 `elapsed >= Duration::from_secs(1)`（1.5s sleep 为真实时间，下界 1s 无 flake 风险），即可 pin 循环存在、闭合变异体 A 缺口。

### 偏离 2-6（维持，均次要低优先）: lsof 三子路径 / 僵尸组合交错 / 静默应答者 / accept 错误分支

- **关联检查项**: #5/#8/#9/#10 — 源码复核无变化（Fix D 仅新增测试，未触 src/——git status 核验：src/proxy.rs 的 diff 均为 proc 执行期实现，本次审计前后 diff --stat 一致，无审计期间残留改动）。各维持 cycle 2 评分。

## 角度总评

SCORE: 9

**总分**: 9/10（所有检查项最低分——最低为 #4/#5/#8/#9/#10 的 9/10）
**通过阈值**: ≥ 9

**判定**: ✅ PASS — Fix D 经审计员实证复核成立：`control_socket_rebind_exhaustion_exits` 确定性命中耗尽分支（目录占用者形态无竞态，机制自检闭环）、四断言覆盖非 0/消息/无 panic/时间界、3 次连跑 1.59-1.63s 稳定、变异体实验证明 panic 与无限重试必红。#4 由 8 → 9。残余项（#4 循环节奏未 pin、#5 三子路径、#8 组合交错、#9 静默应答者、#10 accept 错误）均为次要低优先，且 #4 已有闭合路径（一行下界断言），不构成质量阻塞。

**复核明细（Cycle 3 实证动作）**:
- `cargo test --test proxy_contract control_socket_rebind_exhaustion_exits -- --exact` ×3 → 1.60s / 1.63s / 1.59s 全绿
- `cargo test --test proxy_contract` ×2 → 13 passed / 13 passed（7.78s / 7.81s）
- 变异体实验 ×3（A 删循环→绿 0.05s；B 无限重试→红 5.17s；C panic→红 0.29s），恢复后 src/proxy.rs 与备份逐字节一致、测试复绿
- 抽查 #1（388-443 双路径有界）、#2（607-724 三态）、#3（767+ 占者保活）、#5（proxy.rs:472-487 四守卫）、#6（launch_proxy_contract.rs:320 动态端口 needle）——均与 cycle 2 证据一致
- fix 日志耗时声明（1.58-1.63s / 7.63-7.79s）与审计员实测（1.59-1.63s / 7.78-7.81s）一致，无夸大

**剩余建议（均低优先，不阻塞）**: #4 补 `elapsed >= 1s` 下界断言闭合变异体 A 缺口（一行）；#5 补假 lsof 脚本覆盖非 0 退出与非法 UTF-8；#9 补静默应答者 fake 并把界放宽至 ≤3s。接受现状亦合规。
