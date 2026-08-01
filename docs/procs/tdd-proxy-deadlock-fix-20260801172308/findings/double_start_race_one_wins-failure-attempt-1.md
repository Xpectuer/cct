---
title: "double_start_race_one_wins — FAILURE (attempt 1)"
brief: "double_start_race_one_wins — 双启动竞态 flake（TCP/控制双锁交叉）"
doc_type: finding
created: 2026-08-01
step: 12
attempt: 1
---
## double_start_race_one_wins FAILURE (attempt 1)

**Timestamp**: 2026-08-01T12:05Z
**Phase**: Refactor 后验证运行（TC-13 Refactor agent 的 `cargo test --test proxy_contract` 全量运行之一；agent 自报 10/10 通过，hook 捕获到一次真实失败）

**失败形态**：
```
thread 'double_start_race_one_wins' panicked at tests/proxy_contract.rs:775:5:
exactly one proxy must survive the double-start race — check_proxy_running(".../race.sock") was false after 2.0164175s
(status_a=Some(ExitStatus(unix_wait_status(256))), status_b=None)
```
- status_a=Some(256)：进程 A 以非 0 退出（exit(1)）
- status_b=None：进程 B 存活
- check_proxy_running false：race.sock 无响应（A 的控制 listener 已死，B 无控制通道）

**时序推演（假设）**：TCP 端口与控制 socket 是两个独立锁，双启动无原子性。可能的交叉：B 先 bind TCP 成功；A 先 bind 控制 socket 成功；B 控制 bind EEXIST → 探测 A（若 500ms 探测窗口内 A 的 accept loop 未调度/超时误判，或探测结果 true 但 B 已在 TCP 上）→ 任一方因 TCP 冲突或误判退出 → 幸存者（B）持有 TCP 但无控制通道（A 退出时控制 socket 文件残留）→ check_proxy_running false → 测试失败。TC-12 Refactor 的 delete-on-conflict 消除了"先删后 bind"的 ~7% 窗口，但未消除 TCP/控制交叉竞态（~2-7% 间歇）。

**影响**：间歇 flake（hook 捕获 1 次；TC-12 Refactor 后验证 agent 3 次全绿、TC-13 Refactor agent 报告全绿）。Step 14 全量门与 CI 会不稳定。

**关联**：
- TC-12 Refactor log: logs/double_start_race_one_wins-refactor.md（delete-on-conflict 设计 + "zero-flake 设计超出 refactor scope" 备注）
- 约束 #5/#10（双启动收敛语义）
