---
title: "launch_path_writes_no_codex_config — Refactor Phase"
brief: "launch_path_writes_no_codex_config — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T21:15:31Z
case: "launch_path_writes_no_codex_config"
phase: refactor
---

## Changes made（tests/proxy_contract.rs，无 src 改动）

1. **新增通用 `EnvVarsGuard`（RAII env 守卫）**，替换测试内手动 save/restore 与
   `RestartEnvGuard` 的内联还原逻辑——还原逻辑收敛为一份，且 **panic 时也还原**
   （原实现 restore 只在成功路径执行：若 `ensure_proxy_running` / `switch_profile`
   的 expect panic，CODEX_HOME/CCT_CONFIG 会以临时路径残留到同一进程内的后续测试）。
   `RestartEnvGuard` 改为持有 `_env: EnvVarsGuard`（仅 Drop 副作用，下划线抑制 dead_code）。
2. **flake (a) 加固——switch 紧连竞态重试**（Red 阶段 1/12 次 os error 57 ENOTCONN，
   11/11 后续通过）：switch_profile 最多尝试 3 次，**仅**重试连接级瞬时错误
   （`NotConnected` / `ConnectionRefused`）；状态级错误（proxy 应答 err）与重试
   耗尽后仍失败时立即 panic，原断言消息保留，另有最终 `assert!(switched)` 兜底——
   "switch 必须被应答"的契约强度不变。
3. **flake (b) 加固——RestartEnvGuard drop 回收确认**（偶尔不 reap 进程）：
   shutdown send 成功后轮询控制 socket 文件消失（≤2s 有界；proxy.rs shutdown 分支
   在 exit(0) 前 remove_file，即 TC-13 契约），确认 daemon 已处理 shutdown 并退出；
   send 失败（socket 已死）则跳过轮询，保持尽力而为语义。3 次全量运行后
   `ps` 检查无残留 `cct proxy start` daemon。

未改动断言：snapshot 前后一致 `assert_eq!(before, after)`、禁止名单
（config.toml / auth.json / profile-*.config.toml）、"switch 必须成功"三者均保持原强度。

## 测试质量观察（针对本用例）

- **断言强度 vs AC14**：双断言结构合理——snapshot 相等是主守卫（空目录 + 前后一致
  ⇒ 启动链路零写入），禁止名单是显式契约表达（防未来 seed 目录后 snapshot 空转）。
  两者互补，无需改动。
- **flake (a) 评估**：ENOTCONN 出现在 ensure_proxy_running 就绪探测成功之后的 switch
  连接上，属 macOS unix socket 紧连竞态（1/12 次，手动复现稳定通过）。重试仅覆盖
  连接级错误且最终仍强制成功，不会掩盖死锁类契约回归（若控制通道真实失效，重试后
  仍失败——测试转红，仅晚 200ms）。
- **flake (b) 评估**：原实现只发 shutdown 不确认回收，残留 daemon 属真实（虽小）的
  资源泄漏面；socket 消失轮询以 TC-13 契约为锚点，成本低（正常路径 <100ms），
  有界不阻塞。`launch_proxy_contract.rs` 有独立守卫副本，本用例范围外未动，可作后续
  同类加固候选。
- **可简化性**：测试主体已足够短；唯一重复（两处 env 还原 match）已由 EnvVarsGuard
  消除。命名：`_env_guard` → `_restart_guard`（与新增 `_codex_env` 区分）。

## test_cmd exit code: 0

验证命令：`cargo test --test proxy_contract`（工作树根目录，rtk proxy 原始模式），
连续 3 次执行均 exit 0 / 11 passed / 无编译 warning。

## output

第 3 次（最终）执行完整输出：

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 11 tests
test smoke_stub_receives_request ... ok
test log_masks_api_key ... ok
test log_masks_api_key_upstream_error ... ok
test concurrent_control_and_http ... ok
test launch_path_writes_no_codex_config ... ok
test shutdown_removes_socket_file ... ok
test port_occupied_reports_error_keeps_occupant ... ok
test stop_times_out_on_unresponsive_socket ... ok
test double_start_race_one_wins ... ok
test stub_forwarding_with_bearer ... ok
test zombie_recovery_restarts_proxy ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.97s
```
