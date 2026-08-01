---
title: "Execution Log — cct proxy 死锁修复 + Codex 会话可见性验证与文档收尾"
brief: "23/23 steps, fidelity audit 9/10"
doc_type: proc
created: 2026-08-02T10:30:00+08:00
---

# Execution Log

| Step | Status | Notes |
|------|--------|-------|
| TC-1 — proxy_socket_path_override | ✅ | Red 101 / Green 0 / Refactor 0；CCT_PROXY_SOCKET env 优先 |
| TC-2 — check_proxy_running_app_probe | ✅ | 应用层探测（status 命令 + PROBE_TIMEOUT 500ms×3）；send_control 签名保持（接口冻结） |
| TC-3 — tcp_port_owner_fallback | ✅ | lsof 只读诊断 + 降级文本；PATH 隔离单测 |
| TC-4 — mask_ctl_and_request_path | ✅ | 字段名脱敏（任意 api_key 形态）+ sk- 值扫描；发现 outbound 泄漏留 TC-8 |
| TC-5 — shutdown_proxy_timeout | ✅ | shutdown_proxy 2s 超时传播错误；stop_proxy 区分无 socket/无响应；提取 status_to_result |
| TC-6 — concurrent_control_and_http | ✅ | 死锁回归：同步 accept → tokio 异步 accept + spawn_blocking；3s 双界 |
| TC-7 — stub_forwarding_with_bearer | ✅ | vacuous Red（AC4 HEAD 已实现）；回归守卫 + NO_PROXY 隔离 |
| TC-8 — log_masks_api_key | ✅ | 真实 Red：outbound 日志泄漏 → mask_request_path 修复；upstream error 路径也脱敏 |
| TC-9 — stop_times_out_on_unresponsive_socket | ✅ | 三态覆盖（无文件/拒绝/无响应）；审计补 stop_rejects_stale_socket |
| TC-10 — zombie_recovery_restarts_proxy | ✅ | vacuous Red（重启路径 TC-15 已实现）；RestartEnvGuard 清理 |
| TC-11 — port_occupied_reports_error_keeps_occupant | ✅ | TCP bind 失败 → lsof 诊断 exit(1) 非 panic；占用者存活 |
| TC-12 — double_start_race_one_wins | ✅ | 竞态 flake → failure-dispatch：先 TCP 后控制 bind 顺序；20/20 无 flake + SIGSTOP 10/10 |
| TC-13 — shutdown_removes_socket_file | ✅ | handle_control 签名 + shutdown 前删 socket 文件 |
| TC-14 — launch_path_writes_no_codex_config | ✅ | AC14 快照守卫（CODEX_HOME 下无 config.toml/auth.json/profile-*.config.toml） |
| TC-15..19 — launch_proxy_contract 5 契约 | ✅ | CCT_PROXY_BIN fake 注入；spawn/复用/僵尸重启/耗尽/占端口未 spawn 全绿 |
| TC-20 — run_all_full_pass | ✅ | 迁移前置（29182 已不存在，用户确认）；PoC harness 修复（wait/trap/SSE item-based）；15/15/0/0 |
| TC-21 — visibility_three_checks | ✅ | B006/B007/B008 全 PASS → 无 cct 层 bug（AC6 兜底未触发） |
| TC-22 — layered_diag_and_log | ✅ | B015 分层诊断 PASS；poc.md Results Log 落账 |
| TC-23 — doc_cleanup_final | ✅ | install-script 迁移说明 + 五文档 AC13 清理 + resume 语义；B011/B013/B014 PASS |
| 回归门 | ✅ | cargo test 193/193 全绿（7 suites） |
| 保真度审计 | ✅ | Overall 9/10（completeness 9 / fidelity 10 / honesty 10 / edge_cases 9），3 循环，fix A-D。See findings/audit-*.md |

**执行偏差记录**（均有 findings/ 分析文档支撑）：
1. TC-12 bind 顺序：控制在前 → 先 TCP 后控制（双启动竞态由 TCP 仲裁收敛；delete-on-conflict + EEXIST 重探测保留作防御）——findings/double_start_race_one_wins-analysis.md
2. PoC 脚本 harness：B001 无参 wait 挂起、EXIT trap 空 PID 覆写退出码、stub SSE 补 item-based 事件（codex 0.146 契约）——findings/run_all_full_pass-failure-attempt-1.md
3. B006/B007 断言可证伪化（session-id/rollout 计数语义，spec AC6/AC7 明文要求）——audit 修复产物
4. install-script.md 双平台 socket 路径（macOS ~/Library/Application Support）——audit 修复产物
