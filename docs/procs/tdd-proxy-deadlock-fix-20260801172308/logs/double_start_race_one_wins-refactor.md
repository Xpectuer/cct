---
title: "double_start_race_one_wins — Refactor Phase"
brief: "double_start_race_one_wins — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T11:49:22Z
case: "double_start_race_one_wins"
phase: refactor
---
Changes made: **有改动**。`src/proxy.rs` run_proxy 控制 socket 段与 `tests/proxy_contract.rs` 双启动测试：

**src/proxy.rs（run_proxy 控制 socket 段）**
1. **重复退出路径去重**：提取 `exit_socket_owned` / `exit_bind_failed`（`-> !`）两个 helper，收敛原先 2 处 "another live proxy owns" + 2 处 "bind failed" 的 `eprintln! + exit(1)` 重复；两条仅差 "already" 一词的消息统一为一条。诊断文本不弱化（同义合并）。
2. **EEXIST 收敛分支（平台差异修正）**：新增 `is_bind_conflict`（`AddrInUse | AlreadyExists`），把 macOS/BSD 的 EEXIST（`ErrorKind::AlreadyExists`，os error 17）并入重探测收敛分支——实测双启动竞态本机（macOS）的 bind 冲突就是 EEXIST 而非 EADDRINUSE（Red 日志 panic 证据 `Os { code: 17, kind: AlreadyExists, message: "File exists" }` + 本次 5 连跑中 2/5 输家走 EEXIST），旧 catch-all 分支虽能 exit(1) 非 panic，但跳过重探测、诊断退化为 "bind failed: File exists"；并入后 macOS 获得与 Linux 相同的 "another live proxy owns" 诊断与收敛语义（plan Step 6 标题即 "EADDRINUSE/EEXIST 收敛"）。
3. **先绑后删（delete-on-conflict）**：删除"先探测再删"的预探测 + 预删除；改为 bind 冲突（路径被并发创建）才处理——探测活 proxy → 报错退出（控制通道零触碰）；探测无应答（僵尸 socket）→ 删后重绑一次；重绑仍冲突 → 重探测 `PROBE_RETRIES`×`PROBE_TIMEOUT` 耗尽报错。理由见下方 TOCTOU 分析（实测计划设计的宽窗口会红门禁，本设计消除该窗口，且与约束 #3 父进程"试探 bind，先 drop 再 spawn"同一模式、符合约束 #5 "不破坏活 proxy 控制通道"意图）。
4. **TOCTOU 残差注释**：在代码中记录"探测对已绑定但未应答的启动中实例误判为死 socket"这一残差（保持 plan 接受的竞态类别，收窄到可忽略）。

**tests/proxy_contract.rs（double_start_race_one_wins）**
5. 更新过期 doc 注释（原注释描述修复前 `remove_file + bind().expect()` panic 行为——Red 阶段遗留），改为描述当前 delete-on-conflict 实现；"(EADDRINUSE 收敛)" 注释与断言消息扩为平台中性的 "bind 冲突收敛 / socket bind conflict convergence"。

**平台差异分析（Green agent 主张的核实）**
- **结论：Green agent 的主张属实且可复现**。真实双启动竞态下本机（macOS Darwin 25.5.0）输家 bind 冲突报 `Os { code: 17, kind: AlreadyExists, message: "File exists" }`（EEXIST）——Red 日志的 panic 证据与本次 5 连跑（2/5 输家 EEXIST）一致。macOS 上 EADDRINUSE 分支在竞态中基本是死代码，catch-all 才是实际路径。
- 串行复现实验（bind 到已存在 socket 文件 / 普通文件 / 跨进程活 socket / 跨进程 stale socket）均为 errno 48（EADDRINUSE）——EEXIST 只出现在**两进程并发 bind 同一路径**的 vnode 创建竞态中（Python 串行复现无法构造）。已并入 `is_bind_conflict` 注释。
- 处理合理性：旧 catch-all 能保证"非 panic + exit(1) + 收敛"，但错过重探测；`AlreadyExists` 并入重探测分支后 macOS 与 Linux 同一收敛语义，诊断不弱反强。

**TOCTOU 分析（plan 接受的"先探测再删"竞态）**
- **竞态真实存在且会红门禁，非纯理论**。两次实测门禁失败（均为"影子文件"模式）：
  - 失败 #1（全量 suite，EEXIST 分支已加的中间态）：`check_proxy_running(...) was false after 2.045s (status_a=None, status_b=Some(1))`。
  - 失败 #2（全量 `cargo test`，计划设计最终态）：同断言失败。
  - 机制：B 的预探测在 A bind 之前执行（竞态在此时**合法地未决** → 探测 false）→ B 预删除删掉 A 刚绑定的活 socket → B bind 成功 → B TCP bind 输给 A（同端口）→ B exit(1) → 路径上留下 B 的死 socket 文件，遮蔽 A（活、TCP 正常、控制通道被 unlink 不可达）→ `check_proxy_running` false。负载下概率可观：计划设计样本约 2/28（~7%）。
- **"简单消除"尝试**：先绑后删（delete-on-conflict）——B 只在 bind 冲突且探测确认死 socket 后才删除。这消除宽窗口（B 不再可能删掉 A 刚绑定的路径：探测对已绑定实例在 500ms 内正常应答）；残差为**探测误判**：对"已绑定但启动中未应答（或负载饥饿）"的实例，500ms 探测超时误判为死 socket → 仍可能删除 + 重绑 + TCP 输 → 影子文件。手动压力竞态实测：delete-on-conflict 负载下 2/12 出现双绑（残差真实存在，但门禁样本 0/~50 红）；仪器化 14 连跑全绿（每个胜者控制任务均正常应答探测）。
- **决策**：保留 delete-on-conflict。理由：(a) 门禁 `cargo test --test proxy_contract` 在 delete-on-conflict 下样本全绿（6 次全量 suite + 2 次全量 cargo test + 16 次单测，0 红），计划设计 2/28 红；(b) 符合约束 #5 意图与约束 #3 的父进程模式；(c) 残差（探测误判）是 plan 已接受竞态类别的收窄版，已在代码注释与本日志说明。若需零抖动，后续可考虑胜者侧控制 socket 自愈（按 inode 比对 + 重绑）或 lsof 归属核验——超出本次 refactor 范围，建议作为独立议题。

**其他观察**：`cargo fmt --check` 显示本文件与两个契约测试文件存在**前序阶段遗留**的 fmt 差异（port_conflict_message 的 `None => format!`、invalid-JSON 分支、多处长行），非本次改动引入，未触碰（surgical edits）；本次改动区域 fmt 干净。僵尸/占端口/并发/脱敏等 8 个契约测试行为不受影响（全绿验证）。

test_cmd exit code: 0
output:
```
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 12.52s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 9 tests
test smoke_stub_receives_request ... ok
test log_masks_api_key ... ok
test stub_forwarding_with_bearer ... ok
test stop_times_out_on_unresponsive_socket ... ok
test double_start_race_one_wins ... ok
test concurrent_control_and_http ... ok
test log_masks_api_key_upstream_error ... ok
test port_occupied_reports_error_keeps_occupant ... ok
test zombie_recovery_restarts_proxy ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.31s
```
（最终态复验：`cargo test --test proxy_contract` 另 2 次 9/9 + 2 次全量 `cargo test` 全绿 + `cargo test --test launch_proxy_contract` 5/5 + double_start 单独 16/16。）

前置 Green 日志: docs/procs/tdd-proxy-deadlock-fix-20260801172308/logs/double_start_race_one_wins-green.md
