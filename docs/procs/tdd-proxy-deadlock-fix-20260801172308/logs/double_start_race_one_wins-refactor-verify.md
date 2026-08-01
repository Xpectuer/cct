---
title: "double_start_race_one_wins — Refactor Verification"
brief: "double_start_race_one_wins — Refactor verify: PASS"
doc_type: proc
created: 2026-08-01T11:53:16Z
case: "double_start_race_one_wins"
phase: refactor
---
Verification: PASS
孤儿进程: 无测试残留孤儿。before/after `pgrep -fl "cct.*proxy start"` 均为同一进程：PID 29182 `/Users/zhengjiaye/.local/bin/cct proxy start`（启动于 2026-08-01 14:42:10）——用户自己的实例（~/.local/bin/cct 路径），允许存在，未 kill；after 侧无新增 cct proxy 进程、无 proxy_contract 测试进程残留。
stub_forwarding 稳定性: 3/3 全绿（exit 0 ×3，各 0.12s，1 passed / 8 filtered out）
suite 稳定性: 3/3 全绿（exit 0 ×3：run1 9 passed 5.32s；run2 9 passed 5.32s；run3 9 passed 5.37s）

代码状态确认（src/proxy.rs run_proxy 控制 socket 段，lines 199-278）：
- `exit_socket_owned`（line 211，`-> !`，打印 "another live proxy owns control socket" 后 exit(1)）已定义。
- `exit_bind_failed`（line 217，`-> !`，打印 bind 失败后 exit(1)）已定义。
- bind-conflict 分支：`is_bind_conflict`（line 225-230）匹配 `io::ErrorKind::AddrInUse | io::ErrorKind::AlreadyExists`，并注释了 macOS/BSD 实测 EEXIST（os error 17）语义。
- delete-on-conflict 时序（line 239-270）完整：不预删路径 → bind 冲突时先 `check_proxy_running` 探测 → 活 proxy 则 `exit_socket_owned`（零触碰控制通道）；探测无应答（僵尸）才 `remove_file` 后重绑一次；重绑仍冲突 → `PROBE_RETRIES`×`PROBE_TIMEOUT` 重探测耗尽后 `exit_bind_failed`。控制 socket 任务退出时删除路径（line 277）。

备注：rtk tee 目录（~/Library/Application Support/rtk/tee/）中 19:12:53 的旧失败日志（`double_start_race_one_wins` FAILED，8 passed/1 failed，`status_a=None, status_b=Some(ExitStatus(unix_wait_status(256)))`）属于本次 refactor agent 中间运行（hook 捕获的失败之一），时间戳早于本次验证运行（19:51+），与当前代码状态无关。

Full output:
```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 9 tests
test smoke_stub_receives_request ... ok
test stop_times_out_on_unresponsive_socket ... ok
test port_occupied_reports_error_keeps_occupant ... ok
test stub_forwarding_with_bearer ... ok
test log_masks_api_key_upstream_error ... ok
test concurrent_control_and_http ... ok
test log_masks_api_key ... ok
test double_start_race_one_wins ... ok
test zombie_recovery_restarts_proxy ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.37s
```
