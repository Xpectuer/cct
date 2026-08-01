---
title: "TDD: cct proxy 死锁修复 + Codex 会话可见性验证与文档收尾"
doc_type: proc
status: active
source: docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824
brief: "TDD session: Part A proxy 死锁修复（异步 accept/应用层探测/僵尸自愈/lsof 诊断/脱敏）+ Part B 会话可见性实测验证与文档收尾"
test_cmd: cargo test
full_test_cmd: cargo test
yields_from: [tdd-proxy-deadlock-fix-20260801172308_plan.md]
created: 2026-08-01
updated: 2026-08-01
revision: 2
---

# cct proxy 死锁修复 + Codex 会话可见性验证与文档收尾 - TDD Session

**Started**: 2026-08-01 17:23
**Plan**: `./tdd-proxy-deadlock-fix-20260801172308_plan.md`（plan/ 快照：code-spec.md 25 步 / 4 组；权威测试策略见 plan/verification.md——单元 → 契约（tests/proxy_contract + launch_proxy_contract，真实二进制 + 临时 socket/动态端口）→ L2 实测（poc/ 脚本）→ 文档断言）

## Test Cases

| # | Test Case | Tier | Plan Section | Target File(s) | Depends On | Red | Green | Refactor |
|---|-----------|------|--------------|----------------|------------|-----|-------|----------|
| 1 | proxy_socket_path_override | unit | Step 1 — proxy_socket_path() 支持 CCT_PROXY_SOCKET 覆盖 | src/proxy.rs | [] | [ ] | [ ] | [ ] |
| 2 | check_proxy_running_app_probe | unit | Step 3 — 应用层健康探测常量与 check_proxy_running 升级 | src/proxy.rs | [1] | [ ] | [ ] | [ ] |
| 3 | tcp_port_owner_fallback | unit | Step 5 — 占端口诊断辅助（只读 lsof + 降级文本） | src/proxy.rs | [2] | [ ] | [ ] | [ ] |
| 4 | mask_ctl_and_request_path | unit | Step 8 — 控制命令与请求日志 api_key 脱敏 | src/proxy.rs | [2] | [ ] | [ ] | [ ] |
| 5 | shutdown_proxy_timeout | unit | Step 9 — shutdown_proxy stop 2s 超时 + stop_proxy 区分 | src/proxy.rs, src/main.rs | [2] | [ ] | [ ] | [ ] |
| 6 | concurrent_control_and_http | integration | Step 11 — 7 行为契约（AC1 死锁回归） | tests/proxy_contract.rs | [1,2,3,4,5] | [ ] | [ ] | [ ] |
| 7 | stub_forwarding_with_bearer | integration | Step 11 — 7 行为契约（AC4 Bearer 转发 + SSE） | tests/proxy_contract.rs | [6] | [ ] | [ ] | [ ] |
| 8 | log_masks_api_key | integration | Step 11 — 7 行为契约（AC5 脱敏） | tests/proxy_contract.rs | [6] | [ ] | [ ] | [ ] |
| 9 | stop_times_out_on_unresponsive_socket | integration | Step 11 — 7 行为契约（AC1/10 stop 超时） | tests/proxy_contract.rs | [6] | [ ] | [ ] | [ ] |
| 10 | zombie_recovery_restarts_proxy | integration | Step 11 — 7 行为契约（AC2 自愈） | tests/proxy_contract.rs | [6] | [ ] | [ ] | [ ] |
| 11 | port_occupied_reports_error_keeps_occupant | integration | Step 11 — 7 行为契约（AC3 占端口） | tests/proxy_contract.rs | [6] | [ ] | [ ] | [ ] |
| 12 | double_start_race_one_wins | integration | Step 11 — 7 行为契约（AC9/10 双启动） | tests/proxy_contract.rs | [6] | [ ] | [ ] | [ ] |
| 13 | shutdown_removes_socket_file | integration | Step 7 — shutdown 命令退出前清理 socket 文件 | tests/proxy_contract.rs | [6,7,8,9,10,11,12] | [ ] | [ ] | [ ] |
| 14 | launch_path_writes_no_codex_config | integration | Step 13 — 配置快照回归（AC14） | tests/proxy_contract.rs | [6,7,8,9,10,11,12,13] | [ ] | [ ] | [ ] |
| 15 | spawns_fake_when_none_running | integration | Step 12 — launch 重启契约（AC2 spawn） | tests/launch_proxy_contract.rs | [1,2,3,4,5] | [ ] | [ ] | [ ] |
| 16 | reuses_live_proxy | integration | Step 12 — launch 重启契约（AC9 复用） | tests/launch_proxy_contract.rs | [15] | [ ] | [ ] | [ ] |
| 17 | zombie_socket_triggers_restart | integration | Step 12 — launch 重启契约（AC2 重启） | tests/launch_proxy_contract.rs | [15] | [ ] | [ ] | [ ] |
| 18 | probe_exhaustion_reports_error | integration | Step 12 — launch 重启契约（就绪耗尽 ≤2s） | tests/launch_proxy_contract.rs | [15] | [ ] | [ ] | [ ] |
| 19 | port_occupied_bails_with_diagnosis | integration | Step 12 — launch 重启契约（AC3 未 spawn） | tests/launch_proxy_contract.rs | [15] | [ ] | [ ] | [ ] |
| 20 | run_all_full_pass | e2e | Step 15 — 迁移前置 + 全量 PoC 运行 | poc/（执行 run-all.sh；基线补录 poc/poc.md） | [6,7,8,9,10,11,12,13,14,15,16,17,18,19] | [ ] | [ ] | [ ] |
| 21 | visibility_three_checks | e2e | Step 16 — B006-B008 会话可见性判定 | poc/scripts（执行 B006/B007/B008） | [20] | [ ] | [ ] | [ ] |
| 22 | layered_diag_and_log | e2e | Step 17 — 分层诊断确认 + Results Log 落账 | poc/poc.md | [20,21] | [ ] | [ ] | [ ] |
| 23 | doc_cleanup_final | e2e | Step 19-21 — install-script 迁移 + 五文档清理 + 终审 | docs/references/install-script.md, CLAUDE.md, ARCHITECTURE.md, docs/modules/launch.md, docs/references/codex-home-storage-layout.md, docs/references/codex-backend-development-guide.md | [22] | [ ] | [ ] | [ ] |

> **用例分组与并行**（源自 plan Execution Order DAG）：TC-1→5（G1 单元，串行链）；TC-6..14 与 TC-15..19 为并行组（分别对应 plan step 11 与 step 12 的并行性）；TC-20 需全部契约用例 Green；TC-21/22/23 依次依赖。TC-20 执行前须完成迁移前置（终止 PID 29182，需用户确认）。
> **TC-13 说明**：shutdown 清理 socket 契约（plan Step 7 Verify 要求"启动 → shutdown → 断言 socket 文件不存在"）。plan Step 11 的 7 行为枚举未列此项（源遗漏），本用例补足——实现时作为 tests/proxy_contract.rs 第 8 个行为契约。
> **TC-9 case 3**（stale socket 快错误路径）：socket 文件存在但 connect 立即拒绝（旧版遗留死 socket）→ `cct proxy stop` 快速非 0 退出（Step 9 语义：socket 存在 → shutdown_proxy 传播 connect 错误；用户提示由迁移文档覆盖）。
> **Step 10 / 14 / 18 备注**：Step 10 = tests/proxy_contract.rs 基础设施 smoke（stub 收到请求，TC-6 实现前置，无独立用例）；Step 14 = G2 收尾门（`cargo clippy --all-targets` + verify-B014/B010，并入 TC-20 前检查）；Step 18 = OQ3 TUI picker 可视化确认（🖐️ MANUAL 可选，不阻塞任何 TC）。

## Agent Tool Log

| # | Case | Outcome | Notes | Timestamp |
|---|------|---------|-------|-----------|

## Status

**Current case**: 1 / 23
**Progress**: 0% (0/23 complete)
**Blocked**: None

---
**Updated**: 2026-08-01 17:23
