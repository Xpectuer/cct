---
title: "Terminal Steps 22-24: Proof-Read / Cross-Check / Review"
brief: "Post-audit terminal steps — proof-read PASS, AC1-15 cross-check confirmed, review.md written"
doc_type: proc
created: 2026-08-02T02:36:30+08:00
step: "terminal 22-24"
---

# Terminal Steps 22-24（审计后收尾）

前置：Phase 4 保真度审计 3 循环通过（Overall 9/10：completeness 9 / fidelity 10 / honesty 10 / edge_cases 9）。本记录为正式收尾。

## Step 22 — Proof-Read End-to-End（轻量确认）

审计已逐文件核查（fidelity cycle3 22 检查项全 ≥9），本步执行自动化残留扫描 + 格式门禁：

| Check | Command | Result |
|-------|---------|--------|
| TODO/FIXME/todo!/unimplemented! 残留 | `grep -rn "TODO\|FIXME\|todo!\|unimplemented!" src/ tests/` | **零命中** PASS |
| MUTATION 变异残留 | `grep -rn "MUTATION" src/ tests/` | **零命中** PASS |
| 格式门禁 | `cargo fmt --check` | **3 处既有 drift**（记录不修复，见下） |

### cargo fmt --check 3 处 drift（既有，非本次语义缺陷）

均在 `src/proxy.rs`（格式化差异，不影响行为，按任务指示记录不修复）：

1. `:493` — `port_conflict_message` 的 `None =>` 分支多行 `format!` 可压成单行
2. `:543` — `invalid JSON` 分支 `let _ = write_control_response(...)` 需换行
3. `:550` — `log_proxy!("ctl << {}", mask_ctl_line(...))` 需换行

> 备注：`cargo fmt --check` 退出码非 0，但变更集编译与测试全绿（回归门 193/193）。是否纳入 Step 25 提交前统一 `cargo fmt` 由协调者决定；本步不修复。

### Spec intent / constraints 对照

- 变更文件清单与 plan Step 15-21 声明一致（src/proxy.rs、src/launch.rs、src/main.rs、tests/proxy_contract.rs、tests/launch_proxy_contract.rs、docs/references/install-script.md + 五文档改动段）——fidelity cycle3 逐文件核实无越界改动
- constraints.md 逐条对照：共享 ~/.codex 不回退（AC13 五文档清理）、cct 不写 Codex 配置（AC14 快照回归）、不手动编辑 Codex 内部状态（B006-B008 只读验证）、尊重官方 resume 语义（--all 说明保留）、不新增 schema（B014 接口冻结）——全部由审计 completeness #6-15 逐条核实

## Step 23 — Cross-Check Acceptance Criteria（AC1-15 映射，审计已逐条核实）

| Criterion | 实现证据（审计核实，audit-completeness-cycle2） | 审计评分 |
|-----------|-----------------------------------------------|----------|
| AC1 并发响应（死锁回归） | `concurrent_control_and_http` 真实二进制 ≥20 次并发 status + HTTP GET + 3s 时间界（proxy_contract.rs:390-443；proxy.rs:504-531） | 10/10 |
| AC2 僵尸自愈重启 | 三层证据：ensure_proxy_running 探测→bind→spawn→就绪（launch.rs:134-174）+ 子进程先探测再删（proxy.rs:263-285）+ zombie_recovery_restarts_proxy / zombie_socket_triggers_restart / B002 | 10/10 |
| AC3 占端口报错（lsof 诊断、不 kill） | port_occupied_reports_error_keeps_occupant（proxy_contract.rs:719-766）+ bails_with_diagnosis（launch_proxy_contract.rs:306-330）；占用者存活断言；动态端口非硬编码 | 10/10 |
| AC4 stub 转发链路 | stub_forwarding_with_bearer：Bearer + SSE DELTA + 事件顺序（proxy_contract.rs:451-513）；stub 为 SSE 契约实现（stub-sse-upstream.py:37-51） | 10/10 |
| AC5 日志脱敏 | log_masks_api_key stderr 无明文；mask_ctl_line 字段级（任意值形态）+ mask_request_path sk- 扫描；两日志路径（proxy_contract.rs:521-597；proxy.rs:628-650 + 单测 939-1010） | 10/10 |
| AC6 同 provider 可见 | B006：rollout 计数 + session-id 级对比（偏离 1 修复后 6 旗标由 build_codex_proxy_config_args 真实生成，launch.rs:251） | 10/10 |
| AC7 跨 provider 不可见 + 显式恢复 | B007：rollout 计数 ==2 + id 级核对 + 显式 resume 恢复（偏离 2 修复后经真实函数生成旗标） | 10/10 |
| AC8 cwd 过滤 + --all | verify-B008-cwd-filter.sh 真实切换 cwd/仓库目录（:15-53） | 10/10 |
| AC9 活 proxy 双启动报错 + 复用 | double_start_race_one_wins（proxy_contract.rs:777-839）+ reuses_live_proxy（launch_proxy_contract.rs:178-215）；进程数不变 + 不删 socket | 10/10 |
| AC10 契约测试覆盖 7 场景 + 隔离 | 7 行为契约齐备 + CCT_PROXY_SOCKET 临时路径 + 动态端口 + serial + tempfile；唯一遗留：控制 socket EADDRINUSE 重探测耗尽子分支无直接测试（TCP-first 仲裁下不可达，注释注明防御定位，偏离 5 文档化） | 9/10 |
| AC11 迁移说明 | install-script.md:149-151 Linux + macOS 双平台路径；三要素齐备；B011 独立复跑 PASS（偏离 3 已修复） | 10/10 |
| AC12 L2 前置 | B012 预检 + 基线补录 + ps -p 29182 确认记录（偏离 4 已修复；与 run_all_full_pass-green.md:16 一致） | 10/10 |
| AC13 五文档清理 + 语义说明 | 五文档 grep 零陈旧叙述；resume 语义段落 guide:215-223 + layout:174-182；唯一遗留：codex-configuration-reference.md:324 范围外过时指针（偏离 6 文档化） | 9/10 |
| AC14 不写 Codex 配置 + 接口冻结 | launch_path_writes_no_codex_config 快照对比（proxy_contract.rs:916-971）+ verify-B014 接口未变 | 10/10 |
| AC15 分层诊断 | verify-B015 curl --noproxy '*' 先行 + poc.md Results Log 当日行；唯一遗留：门约束下自起实例（偏离 7 文档化） | 9/10 |

**Cross-Check 结论**：AC1-15 全部有实现证据且证据形态可证伪；3 个 9/10 项均为已文档化次要偏离（已接受），无验收证据缺陷。

## Step 24 — Review

按 verification.md Self-Review Checklist 11 项逐项过（详见 `docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/review.md`）：

- **11/11 PASS**：AC 覆盖、Verify 可执行、实例隔离、接口冻结、脱敏覆盖、无新增 panic、超时收敛、文档语义、历史快照不动、MANUAL 显式、并行安全、条件分支兜底
- 审计汇总：completeness 9（cycle2 终）/ fidelity 10（cycle3 终）/ honesty 10（cycle2 终）/ edge_cases 9（cycle3 终）— **Overall 9/10 PASS**
- 终局证据：TDD 23/23；回归门 193 passed / 0 failed exit 0；PoC run-all 15/15/0/0

**产出**：`docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/review.md`（新建，frontmatter: title/doc_type/brief/confidence/created/updated/revision；目标位置确认：draft 根无既有 review.md——plan/review.md 为 /confirm 计划评审、refs/prev-review.md 为已废弃设计评审，均保持不动）

**Verdict**: READY — 可进入 Step 25 提交。

## 遗留记录（不阻塞，供协调者决策）

1. `cargo fmt --check` 3 处 drift（src/proxy.rs:493/543/550）——是否提交前统一格式化
2. AC10/AC13/AC15 三个 9/10 次要偏离——审计已文档化接受理由，无需修复
