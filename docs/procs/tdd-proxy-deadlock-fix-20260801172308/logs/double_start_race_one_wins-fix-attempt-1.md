---
title: "double_start_race_one_wins — Fix (attempt 1)"
brief: "双启动竞态修复：先 TCP 后控制 bind 顺序"
doc_type: proc
created: 2026-08-01T13:00:16Z
case: "double_start_race_one_wins"
phase: fix
---

# double_start_race_one_wins — Fix (attempt 1)

## Changes made

`src/proxy.rs` `run_proxy`：调换 bind 顺序——**TCP bind 段移到控制 socket bind 段之前**（方向 A，见 `findings/double_start_race_one_wins-analysis.md`）。

- TCP bind 段（`127.0.0.1:{port}` → EADDRINUSE → `port_conflict_message` 诊断 + exit(1)）原样前置，成为双启动竞态的唯一仲裁者。
- 控制 socket 段（delete-on-conflict：bind 冲突 → 探测 → 活实例 `exit_socket_owned` / 僵尸 → `remove_file` → 重绑；重绑冲突重探测耗尽 `exit_bind_failed`）逻辑原样保留，仅更新注释。
- 无任何消息文本改动（`"port {port} already in use"`、`"[cct-proxy] TCP bind ... failed"`、`"another live proxy owns control socket"` 等契约断言文本逐字未动）。
- `log_proxy!("control socket bound")`、`tokio::spawn(run_control_socket)`、`socket_path` 阴影均保持在控制 bind 之后，位置不变。

顺序改变后的语义核对（与 plan Step 6 描述行为的偏差仅限顺序）：

- a. **zombie 场景**（socket 残留无进程）：TCP bind 成功（端口空闲）→ 控制 EEXIST → 探测 false → 删 → 重绑 ✓（自愈不变）
- b. **双启动**：败者 TCP EADDRINUSE → exit(1)（AC3 诊断文本），**根本走不到控制 bind**——不重绑、不删除、不留任何 socket 文件 ✓；胜者完整启动
- c. **占端口**：TCP bind 失败 → `port_conflict_message` 诊断 exit(1)（AC3 消息不变）✓
- d. **控制段活实例探测分支**（`exit_socket_owned`）原样保留作防御（非 proxy 进程占路径等异例）✓

## test results

| Check | Result |
|-------|--------|
| `cargo build` | 通过 |
| `cargo test --test proxy_contract` | 10/10 通过（exit 0，5.47s） |
| `cargo test --test launch_proxy_contract` | 5/5 通过（exit 0，2.78s） |
| `cargo test`（全量） | 192/192 通过（7 suites，19.41s） |
| `double_start_race_one_wins` 循环 20 次 | **20/20 通过，零 flake**（`for i in $(seq 1 20); do cargo test --test proxy_contract double_start_race_one_wins --quiet ...`） |
| SIGSTOP 冻结确定性复现（/tmp/dsrace_postfix_sigstop.sh，方向 A 复现方法，10 次） | **10/10 通过**：败者 exit(1)（stderr 含 "TCP bind" + "already in use"，且无 "control socket bound"——证明败者在 TCP 处退出、从未触碰控制通道）；胜者 SIGCONT 后健康（status 探测应答）、socket 文件未被删除 |

修复前后对照（同 SIGSTOP 方法）：修复前 24/24 复现"败者 exit(1) + 死文件遮蔽胜者控制通道 + 探测 ECONNREFUSED"；修复后 10/10 干净收敛——败者先退、胜者控制通道完好。

## Notes

与 plan Step 6 的顺序偏差说明：plan Step 6 文字描述的是"先探测再删 + bind 失败报错（TCP + 控制 socket EADDRINUSE）"，未固定控制 bind 与 TCP bind 的相对顺序（constraints.md #5 文字约束的是控制 socket 收敛策略）。本改动把 TCP bind 前置为唯一仲裁者（方向 A，analysis 已论证）：

- 与约束 #3 意图同构：父进程 `ensure_proxy_running` 本就是"试探 bind 判端口空闲 → 再 spawn"——先端口后 spawn；子进程改为"先 TCP 后控制"与此一致。
- 与约束 #4 意图合规：败者消息从 "another live proxy owns" 变为端口占用诊断（lsof PID，信息更强），AC3 断言文本不变。
- 与约束 #5 意图合规：双启动收敛为恰一存活；"不破坏活 proxy 控制通道"从"探测确认后不删"升级为"根本不触碰"。
- 控制段探测-删除只对真僵尸文件执行，正是约束 #3 的意图。

范围：仅 `src/proxy.rs` 的 `run_proxy` 内顺序调整（含注释更新），未改动任何测试、消息文本或其他模块。
