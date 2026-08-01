---
doc_type: proc
brief: "Fidelity audit: 代码诚实度 (cycle 1)"
source_skill: execute
audit_phase: fidelity
audit_angle: honesty
audit_cycle: 1
confidence: verified
---

# 审查角度: 代码诚实度

**审查依据**: Decisions 5/6/7 / Step 10-15
**审查周期**: 1/3

## 评分明细
| # | 检查项 | 评分 | 证据 | 严重程度 |
|---|--------|------|------|----------|
| 1 | vacuous Red 记录完整性：TC-7/9/10/16/17/18/19 逐一核对 red 日志注明 vacuous + 理由；Exit code 口径与事实一致 | 10/10 | logs/stub_forwarding_with_bearer-red.md:3（"真空 Red：AC4 行为在 HEAD 基线已实现"）+ :26-38（三原因逐一核实：switch 分支 `git show HEAD:src/proxy.rs` 第 423 行、Bearer 注入 :333-335、SSE 转发）+ 手动 E2E 交叉验证；stop_times_out_on_unresponsive_socket-red.md:29（vacuous，理由=TC-5 Green 已实现，耗时 2.16s≈2s 超时证据）；zombie_recovery_restarts_proxy-red.md:22（vacuous，理由=TC-15 Green 实现 ensure_proxy_running 全路径，4 条非空断言枚举）；reuses_live_proxy-red.md:29（vacuous，理由=复用路径 pre-fix 已存在）；zombie_socket_triggers_restart-red.md:23 / probe_exhaustion_reports_error-red.md:23 / port_occupied_bails_with_diagnosis-red.md:22（均注明 vacuous + 理由 + 耗时佐证 0.57s/1.53s/0.06s）；launch_path_writes_no_codex_config-red.md:9（vacuous + 首轮 exit 101 flake 如实记录）。全部 exit 0 与输出块一致；对照组 TC-1..6/8/11/12/13/15 red 均为真实 exit 101（如 double_start_race_one_wins-red.md:9 真实 panic 证据） | 无 |
| 2 | vacuous 用例 Green 断言强度：断言非空转（stub 记录/DELTA/退出码/stderr/耗时/进程存在性变化） | 10/10 | tests/proxy_contract.rs:451-513（TC-7：stub 恰 1 条 POST /v1/chat + Bearer sk-contract-key（客户端不带 Authorization 头）+ 200 + text/event-stream + chunked + DELTA + 事件顺序 created<delta<completed）；:604-677（TC-9：非 0 退出 + ≤2.5s + stderr 含 Error + 无误报 not running + stdout 空 + ② 快速 exit 0）；:686-710（TC-10：SIGKILL 后 sock.exists + 探测 false + ensure Ok + 恢复健康）；tests/launch_proxy_contract.rs:178-215（TC-16：PID 存活 try_wait None + READY mtime 未变）；:221-261（TC-17：READY 重出现=重新 spawn 硬证据）；:268-298（TC-18：Err + "did not become healthy" + ≤2s）；:306-330（TC-19：Err + "port X already in use" + !ready.exists()） | 无 |
| 3 | 空断言扫描：assert!(true)、let _ =、未使用结果、脚本固定输出 PASS | 10/10 | src/ tests/ 全库 grep assert!(true) 零命中；let _ = 均属合理（remove_file 清理、write_control_response 错误应答、crossterm 清理，src/proxy.rs:269/538/613 等）；poc/scripts 无固定 PASS 输出——每处 exit 0 前有真实条件（如 verify-B011-migration-docs.sh:19-21 grep 文档内容后才 PASS；verify-B015-layered-diag.sh:37-39 真实 curl 退出码）；唯一静默跳过为 src/proxy.rs:921-923（lsof 缺失时 return，注释说明且由姊妹测试覆盖 PATH=/nonexistent 场景） | 无 |
| 4 | TC-14 快照守卫：glob 真实路径；前后对比真做；CODEX_HOME env 真设 | 10/10 | tests/proxy_contract.rs:876-898 snapshot_codex_home 递归真实遍历临时 codex_home；:933 before 快照 → :937 ensure_proxy_running + :941-958 switch_profile → :961-965 assert_eq!(before, after) 真对比 + :966-970 禁止名单检查；:927-930 EnvVarsGuard 真设 CODEX_HOME=codex_home + CCT_CONFIG=临时空 profiles.toml（子进程继承，不落真实 ~/.codex）；switch 重试仅限连接级瞬时错误（:948-951），状态级错误立即 panic——不削弱断言；red 日志首轮 ENOTCONN flake 如实记录（launch_path_writes_no_codex_config-red.md:34-43） | 无 |
| 5 | Refactor 不添加行为：TC-5 status_to_result 等；TC-12 竞态修复可追溯到 failure-dispatch | 10/10 | logs/shutdown_proxy_timeout-refactor.md:9（status_to_result 纯提取声明：同一 status 比较/同一错误构造/同一超时窗口）；代码核对 src/proxy.rs:161-169 被 switch_profile:156 与 shutdown_proxy:180 共用，语义与 plan Step 9 New 代码逐字一致；TC-12 可追溯：double_start_race_one_wins-red.md:9（exit 101，真实 panic at src/proxy.rs:219）→ findings/double_start_race_one_wins-analysis.md + fix-attempt-1.md:14-50（先 TCP 后控制，消息文本逐字未动，20/20 循环 + SIGSTOP 确定性复现 10/10 修复前 24/24 对照）→ refactor-verify.md:22-39（3/3 套件全绿）——竞态修复由真实失败触发，非 Refactor 夹带行为 | 无 |
| 6 | fake 脚本真实性：真 rm + accept + 应答 status | 10/10 | tests/launch_proxy_contract.rs:27-62 write_fake_proxy：真 `rm -f "$SOCK"` + `: > "$READY"` + `exec python3` 真 accept 循环（AF_UNIX bind/listen/settimeout 0.5/读至换行/回 `{"status":"ok"}`，socket 删除后自终止）；probe_exhaustion fake 为 `#!/bin/bash\nexit 0` 立即退出（:271）——与设计一致；无假应答、无短路 | 无 |
| 7 | TODO/stub 扫描：TODO\|FIXME\|unimplemented!\|todo!\|dbg! 零命中；无残留 bind panic expect | 10/10 | src/ tests/ grep 全部零命中；src/proxy.rs 全部 bind（TCP :245-254、控制 :261-286）经 match + exit(1) 处理，无 .expect panic 路径；残存 expect 均为测试代码（:777/814/841/866/924）或 runtime 构造（:193/205 "build proxy tokio runtime"，非 bind 路径）；`is_bind_conflict`（:225-230）覆盖 macOS EEXIST 并注释 | 无 |
| 8 | run-all 15/15 自证性：汇总来自真实退出码；无 Skip 掩盖、无 \|\| echo PASS 兜底 | 8/10 | run-all.sh:22-32 聚合真实退出码（RC=77→Skip，无 \|\| echo PASS）；Skip 掩盖已消除（B005 改为 stderr 重定向真断言 verify-B005-log-masking.sh:24-25/40-48，B015 自起实例不再恒 SKIP）；**偏离**：最终 15/15/0/0 运行的原始输出未留存——doc_cleanup_final-green.md:14 仅有汇总行 "run-all 终态: 15/15/0/0"，/tmp 仅有 run-all-fix1.log（13/15，logs/run_all_full_pass-fix-attempt-1.md 引用）；早期各次运行（3/11/1、卡死、13/2）均有原始输出，最强声明反而无原始转录；15/15 由证据链间接支撑（13/15 + B011/B013 修复后单独 PASS 于 doc_cleanup_final-green.md:11-13，且我复核文档内容真实存在） | 次要 |
| 9 | 脱敏无死角：outbound 泄漏覆盖；mask_ctl_line 用解析后 cmd.api_key | 10/10 | src/proxy.rs:551 `mask_ctl_line(line.trim(), cmd.api_key.as_deref())`——用解析后字段值，按字段名掩码（不依赖 sk- 前缀，:628-633）；outbound 路径全掩：:370-374 "-> upstream" 经 mask_request_path、:443 reqwest 错误（内嵌 URL）同样过 mask（TC-4 备注的潜在泄漏由 TC-8 扩展用例 log_masks_api_key_upstream_error 覆盖，tests/proxy_contract.rs:565-597 断言错误路径无明文 + sk-*** 反真空守卫）；契约测试 log_masks_api_key（:521-559）断言 ctl+请求两路径无明文且 sk-*** 必现 | 无 |
| 10 | 基线证据引用：refs/proxy-deadlock-diagnosis.md 存在且支持"修复前 FAIL"声明 | 10/10 | docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/refs/proxy-deadlock-diagnosis.md（4.1K）存在：死锁机制（std UnixListener 阻塞 current_thread runtime）、实测证据（PID 29182 sample 100% __accept、ESTABLISHED/CLOSE_WAIT、curl 8s 超时）——支持 B001/B003/B005/B015 修复前 FAIL 声明；poc.md Results Log 基线行（2026-08-01 修复前 15/11/4/0）引用该文件 + session-log | 无 |

## 偏离详情
（仅列出评分 < 10 的检查项）
### 偏离 1: 最终 run-all 15/15 运行缺少原始输出留痕
- **关联检查项**: #8
- **评分**: 8/10
- **证据**: logs/doc_cleanup_final-green.md:14 — "run-all 终态: 15/15/0/0"，无原始转录；同文件 :11-13 仅列出 "B011: PASS / B013: PASS / B014: PASS" 汇总。对比之下早期运行均有原始输出：run 1（15/3/11/1）、run 2 卡死（SIGTERM 143）、13/15 修复运行（/tmp/run-all-fix1.log，logs/run_all_full_pass-fix-attempt-1.md:84-92 内嵌 "Total: 15 | Pass: 13 | Fail: 2 | Skip: 0"）。poc.md Results Log 最终行（:79）将证据指向 doc_cleanup_final-green.md——而该文件仅含汇总声明，未自证。
- **期望**: 最强声明（15/15 全量闭环）应与较弱声明同级留痕：原始输出（完整转录或留存文件路径）入库，使 PASS 汇总可独立复验。
- **实际**: 最终运行无原始输出留痕；15/15 依赖"13/15 + 两 FAIL 项已被修复并单独 PASS"的推断链。无伪造迹象（每条失败路径均有详实记录，文档修复真实可复核），但证据自证性弱于其下的声明。
- **严重程度**: 次要
- **修复建议**: 重跑 `bash docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/run-all.sh`，将完整输出追加至 logs/（或保留 `tee` 文件路径并写入 doc_cleanup_final-green.md / poc.md Results Log Notes），同时补录 B011/B013/B014 单独运行的原始输出；或在 findings/ 下补一份 run_all_full_pass-final-run.md 含完整转录。

## 角度总评
SCORE: 8
**总分**: 8/10（所有检查项最低分）
**通过阈值**: ≥ 9

## 判定
❌ NEEDS_REWORK — 共 1 个偏离需修复（次要：最终 15/15 运行原始输出未留痕，证据自证性不足）
