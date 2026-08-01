---
doc_type: proc
brief: "Fidelity audit: 边界覆盖 (cycle 1)"
source_skill: execute
audit_phase: fidelity
audit_angle: edge_cases
audit_cycle: 1
confidence: verified
---

# 审查角度: 边界覆盖

**审查依据**: AC1/AC3/AC10 / Step 5/6/9/11/12
**审查周期**: 1/3

## 评分明细

| # | 检查项 | 评分 | 证据 | 严重程度 |
|---|--------|------|------|----------|
| 1 | AC1 时间界：死锁回归测试带超时断言（3s 界） | 10/10 | tests/proxy_contract.rs:388-443 — BUDGET=3s；控制线程 20 次 status 经 `recv_timeout(BUDGET)` 有界等待；HTTP GET 2s 读超时 + `http_elapsed < BUDGET` 断言；挂死 CI 不可能（双路径均有界） | — |
| 2 | AC10 stop 超时三态：无文件/拒绝（stale）/无响应全覆盖 | 7/10 | tests/proxy_contract.rs:604-677 — ①无响应（≤2.5s 非 0 + stderr Error + 不误报 not running）②无文件（<1s exit 0 + 消息）均有测试；③stale 拒绝态仅代码路径（main.rs:239-246 → shutdown_proxy 连接拒绝快速非 0）+ tdd.md:50 语义记录，**无直接测试** | 主要 |
| 3 | AC3 占端口三态：(a) TCP 被占+无控制 socket→lsof 报错；(b) 控制仍响应+TCP 被占→健康复用（AC11）；(c) 控制死+TCP 被占→报错 | 10/10 | (a) tests/proxy_contract.rs:719-766（非 0 + 诊断 + 无 panic + 占用者存活）；(b) tests/launch_proxy_contract.rs:178-215 `reuses_live_proxy`（PID 存活 + READY mtime 未重写）+ docs/references/install-script.md:143-153 迁移文档明示"控制仍响应→视为健康复用、唯一修复路径手动 kill"；(c) tests/launch_proxy_contract.rs:306-330（Err + "port already in use" + 未 spawn）+ 子进程路径 tests/proxy_contract.rs:719-766 | — |
| 4 | EADDRINUSE 耗尽路径：3×500ms 收敛；先 TCP 后控制下是否仍可达/可测试 | 8/10 | src/proxy.rs:263-281 — 3×500ms 重探测循环原样保留作僵尸安全网；但方向 A（TCP 先行）后双启动败者在 TCP EADDRINUSE 处直接 exit(1)（proxy.rs:245-253），根本走不到控制 bind——耗尽分支在正常双启动下**不可达、无直接测试**；收敛语义等价迁移到 TCP 仲裁层（20/20 + SIGSTOP 10/10 证据），findings/double_start_race_one_wins-analysis.md:79-100 有完整论证 | 次要 |
| 5 | lsof 降级全谱：缺失/失败/空/非 UTF-8 → None → 降级文本 | 9/10 | src/proxy.rs:472-499 — 四态代码全实现（`.output().ok()?` 缺失 / `!status.success()` 失败 / `.lines().next()...filter(非空)` 空 / `String::from_utf8().ok()?` 非 UTF-8 → None → 降级文本）；单测仅覆盖缺失态（proxy.rs:901-914 PATH=/nonexistent），失败/空/非 UTF-8 三子路径无专门用例 | 次要 |
| 6 | 诊断端口来源：实际绑定端口非硬编码 19191 | 10/10 | src/proxy.rs:244-249 — TCP bind 用 `port` 参数（`proxy_port()` 读 CCT_PROXY_PORT）；tests/proxy_contract.rs:748 断言动态端口文本 `port {port} already in use`；tests/launch_proxy_contract.rs:320 同；tcp_port_owner 单测用 bind-0 实际端口（proxy.rs:920-933） | — |
| 7 | 双启动收敛界：败者 ≤2s、恰一存活、无 panic；20/20 复跑证据；serial 隔离 | 10/10 | tests/proxy_contract.rs:777-839 — 2s 预算轮询 + 恰一存活 + 输家非 0 + 两 stderr 合计无 "panic"；复跑证据 logs/double_start_race_one_wins-fix-attempt-1.md:36（循环 20 次 20/20 零 flake）+ tdd.md:62；SIGSTOP 确定性复现 10/10（fix-attempt-1.md:37）；proxy_contract/launch_proxy_contract 全部 `#[serial]` | — |
| 8 | 僵尸恢复竞态窗口：父 bind 后 spawn 窗口被抢 → 子 TCP 失败报错 + 父就绪耗尽报错——双非 panic 出口；无遗漏 expect | 9/10 | 双非 panic 出口均实现且各自有契约测试：子 TCP 失败 eprintln+exit(1)（proxy.rs:245-253，测于 proxy_contract.rs:719-766）+ 父就绪耗尽 bail（launch.rs:163-173，测于 launch_proxy_contract.rs:268-298）；diff 证实竞态路径无新增 expect/panic（旧 `bind(...).expect` 与 `panic!("proxy bind")` 均已移除）；但"父 drop 后端口被第三方抢走"的完整交错无确定性测试 | 次要 |
| 9 | 就绪探测边界：3×500ms 耗尽 → bail 明确报错，≤2s 不挂起 | 9/10 | launch.rs:163-173 — PROBE_RETRIES×PROBE_TIMEOUT 耗尽 bail "did not become healthy"；launch_proxy_contract.rs:268-298 断言 Err + 消息 + elapsed≤2s；但 ≤2s 仅对快速失败（connect 拒绝）场景成立——静默应答者（bind 但不回包）最坏 ~2.5s（3×500ms 探测 + 2×500ms sleep），该形态未测 | 次要 |
| 10 | 并发关闭：shutdown 删文件不删 in-flight 连接；accept 错误 sleep 100ms 防忙循环 | 9/10 | proxy.rs:600-615 — shutdown 分支先写响应再 `remove_file` 再 exit(0)，响应先于清理送达；run_control_socket accept 错误 sleep 100ms（proxy.rs:525-528）符合 plan Step 2；shutdown_removes_socket_file 测试（proxy_contract.rs:849-871）；但 accept 错误分支无直接测试（难诱发），HTTP accept `Err(_) => continue`（proxy.rs:301）为前序代码无 sleep | 次要 |

## 偏离详情

### 偏离 1: AC10 stop 三态中"stale 拒绝态"缺直接测试
- **关联检查项**: #2
- **评分**: 7/10
- **证据**: tests/proxy_contract.rs:604-677 — `stop_times_out_on_unresponsive_socket` 只含①（UnixListener 接受连接但 hold 住不回包，≤2.5s 非 0）与②（无 socket 文件，<1s exit 0）两个 case；全测试树 grep 无任何 "refused/ECONNREFUSED/stale" 的 stop 用例（唯一命中是 log_masks_api_key_upstream_error 的上游注释）；tdd.md:50 仅以备注形式记录 case 3 语义（"socket 存在 → shutdown_proxy 传播 connect 错误"）；audit-perspectives.md Angle 2 #9 声称"三 case 全实现"与事实不符（实现的是代码路径，非测试）
- **期望**: 三态各有测试：stale 拒绝 → `cct proxy stop` 快速非 0 退出（<1s 量级）+ stderr 错误 + 不误报 "Proxy is not running."
- **实际**: 代码行为正确（main.rs:239-246 `socket_path.exists()` 为 true → shutdown_proxy → `UnixStream::connect` 立即 ECONNREFUSED → Err 传播 → 非 0 快速退出），但无回归测试 pin 该行为——而该态恰是修复的关键语义变更点（旧实现 check_proxy_running 内核 connect 对 stale socket 返回 false → 误报 "not running" exit 0；新实现必须非 0 报错，AC11 迁移路径依赖此区分）
- **严重程度**: 主要
- **修复建议**: 在 tests/proxy_contract.rs `stop_times_out_on_unresponsive_socket` 追加 case ③：`UnixListener::bind` 临时路径后立即 drop（或 bind 后不在该路径监听），使 socket 文件存在但 connect 被拒 → spawn `cct proxy stop` → 断言 <1s 内退出、非 0、stderr 含 "Error"、stdout 不含 "Proxy is not running."

### 偏离 2: EADDRINUSE 耗尽分支不可达且无测试（方向 A 的等价迁移）
- **关联检查项**: #4
- **评分**: 8/10
- **证据**: src/proxy.rs:263-281 — 控制 bind 冲突后"删→重绑→重绑仍冲突→3×500ms 重探测耗尽 exit_bind_failed"循环保留；但 src/proxy.rs:244-253 TCP bind 已前置，双启动败者在 TCP 处退出，控制段冲突只对真僵尸文件（探测无应答）触发，重绑冲突需"非 proxy 进程占 socket 路径"等异例才能到达（findings/double_start_race_one_wins-analysis.md:91-94 明确此为防御性安全网）
- **期望**: 3×500ms 耗尽收敛路径可证明/可测试
- **实际**: 收敛语义由 TCP 仲裁层等价承担并经 20/20 循环复跑 + SIGSTOP 10/10 确定性复现验证；控制段耗尽循环本身无直接测试，正常双启动下不可达
- **严重程度**: 次要
- **修复建议**: 接受现状（analysis 论证充分、与约束 #3/#4/#5 意图合规），或在 proxy.rs 单测中构造"bind 冲突 + 重绑仍冲突"的最小复现（如先用 std UnixListener 占路径再并发 bind）以 pin 耗尽分支非 panic。低优先。

### 偏离 3: lsof 降级全谱中失败/空/非 UTF-8 三子路径无专门测试
- **关联检查项**: #5
- **评分**: 9/10
- **证据**: src/proxy.rs:472-499 — 四态 → None 守卫全部实现；tests（proxy.rs:901-914）仅 `tcp_port_owner_fallback_when_lsof_missing` 用 PATH 注入覆盖缺失态
- **期望**: 缺失/失败/空/非 UTF-8 四态均有覆盖
- **实际**: 失败（lsof 退出非 0）、空输出、非 UTF-8 输出三路径仅靠代码审查，无用例（lsof 退出非 0 可经 PATH 指向返回非 0 的假 lsof 脚本构造；空输出即绑定不存在的端口；非 UTF-8 需假 lsof 输出非法字节）
- **严重程度**: 次要
- **修复建议**: 低优先——同一 `?`/guard 模式，四态共享一个降级契约；如补，用临时目录假 `lsof` 脚本（PATH 注入）覆盖"非 0 退出"与"非法 UTF-8 输出"两态即可闭环

### 偏离 4: 僵尸恢复竞态窗口的组合交错无确定性测试
- **关联检查项**: #8
- **评分**: 9/10
- **证据**: 两个出口各自被契约测试覆盖——子进程 TCP 失败（tests/proxy_contract.rs:719-766）+ 父就绪耗尽（tests/launch_proxy_contract.rs:268-298）；但"父试探 bind 成功后、子进程 bind 前，端口被第三方抢占"的完整交错无测试；diff 核实竞态路径新增代码零 expect/panic（旧 `expect("bind proxy control socket")` 与 `panic!("proxy bind {addr}")` 均已移除，见 git diff HEAD -- src/proxy.rs）
- **期望**: 双非 panic 出口 + 无遗漏 expect（满足）；组合窗口可复现验证
- **实际**: 组合窗口需可控抢占者（第三方在父 drop 与子 bind 之间抢端口），无确定性测试手段，仅靠两出口分别验证 + 逻辑推演
- **严重程度**: 次要
- **修复建议**: 接受现状（两出口均已 contract 化，组合为两已验证路径的交错）；如追求确定性，可在 launch 契约中注入"fake 目标 sleep 后 bind 已被测试进程占用的端口"的脚本复现组合形态。低优先。

### 偏离 5: 就绪探测 ≤2s 断言仅覆盖快速失败场景，静默应答者形态未测
- **关联检查项**: #9
- **评分**: 9/10
- **证据**: tests/launch_proxy_contract.rs:268-298 — `probe_exhaustion_reports_error` 用立即退出脚本（connect 拒绝，elapsed≈1s）；最坏形态（spawn 目标 bind 控制 socket 但永不应答 status）下 3 次探测各 500ms 超时 + 2 次 sleep 500ms = ~2.5s，超出测试的 ≤2s 断言但仍在有界范围（plan Step 12 Verify 的 ≤2s 以 connect-拒绝场景为口径）
- **期望**: 3×500ms 耗尽 → bail 明确报错，有界不挂起
- **实际**: bail 报错 ✓、有界 ✓（≤2.5s 最坏）、测试 ≤2s 在快失败场景成立；静默应答者形态无用例
- **严重程度**: 次要
- **修复建议**: 如要求产品侧硬性 ≤2s，将 launch.rs:163-173 的 sleep 改为 400ms（3×400+2×400=2.0s）；否则补充静默应答者 fake（bind socket 不回包）测试并把断言放宽到 ≤3s。低优先。

### 偏离 6: accept 错误 100ms 防忙循环无测试；HTTP accept 错误分支为前序代码无 sleep
- **关联检查项**: #10
- **评分**: 9/10
- **证据**: src/proxy.rs:525-528 — 控制 socket accept 错误 sleep 100ms 已按 plan Step 2 实现；tests/proxy_contract.rs:849-871 覆盖 shutdown 文件清理但无 accept 错误用例（需资源耗尽诱发）；src/proxy.rs:301 HTTP accept `Err(_) => continue` 为 HEAD 前序代码（git show HEAD:src/proxy.rs 证实），不在本修复范围
- **期望**: shutdown 先响应后删文件（满足：proxy.rs:602-613）；accept 错误有界退避（控制侧满足；HTTP 侧为前序遗留）
- **实际**: 控制侧实现 ✓；两处 accept 错误分支均无测试；HTTP 侧无 sleep（前序，忙循环风险存在但非本次引入）
- **严重程度**: 次要
- **修复建议**: 接受现状或将 HTTP accept 错误分支并入 100ms sleep（2 行，顺带消除前序忙循环隐患，属超范围改动需单独决策）；测试诱发困难，可依赖 code review。低优先。

## 角度总评

SCORE: 7

**总分**: 7/10（所有检查项最低分）
**通过阈值**: ≥ 9

**判定**: ❌ NEEDS_REWORK — 共 6 个偏离需修复

**主导偏离（决定总分）**: #2 AC10 stop 三态中 stale 拒绝态缺直接测试——该态正是修复的关键语义变更点（旧实现误报 "not running" exit 0，新实现必须快速非 0 报错），且 audit-perspectives.md Angle 2 #9 "三 case 全实现"的表述与测试事实不符。修复成本低（在 stop 契约测试追加 case ③）。

**其余 5 项为低优先次要偏离**（#4 EADDRINUSE 耗尽分支等价迁移至 TCP 仲裁、#5 lsof 三子路径、#8 组合交错、#9 静默应答者形态、#10 accept 错误分支测试），均不影响 AC 边界正确性，多数已由 analysis 文档充分论证或代码结构上平凡正确。
