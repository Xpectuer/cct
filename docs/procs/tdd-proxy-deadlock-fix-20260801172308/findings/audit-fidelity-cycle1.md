---
doc_type: proc
brief: "Fidelity audit: 计划忠实度 (cycle 1)"
source_skill: execute
audit_phase: fidelity
audit_angle: fidelity
audit_cycle: 1
confidence: verified
---

# 审查角度: 计划忠实度

**审查依据**: plan 全部 step / Execution Order（ref/plan/code-spec.md 25 步 + DAG）
**审查周期**: 1/3
**复核方法**: 源文件逐行对照（src/proxy.rs 1011 行、src/launch.rs ensure_proxy_running、src/main.rs stop_proxy、tests/proxy_contract.rs 971 行、tests/launch_proxy_contract.rs 330 行）+ 67 个 RGR 日志按用例核查 + findings 两份分析 + 文档 diff + 实跑 `cargo test`（193/193 复验通过）+ /tmp/run-all-fix1.log 原始输出核验。

## 评分明细
| # | 检查项 | 评分 | 证据 | 严重程度 |
|---|--------|------|------|----------|
| 1 | TC-1（Step 1）：CCT_PROXY_SOCKET env 优先；单测 set_var/remove_var；Red 真失败 | 10/10 | src/proxy.rs:67-75（`if let Ok(p) = env::var` 提前返回）；src/proxy.rs:742-749（set_var→断言→remove_var→回退断言）；logs/proxy_socket_path_override-red.md（exit 101，左值=默认路径右值=临时路径） | — |
| 2 | TC-2（Step 3）：应用层探测；三常量；send_control 签名保持（#9） | 10/10 | src/proxy.rs:94-96（PROBE_TIMEOUT/500ms、PROBE_RETRIES/3、STOP_TIMEOUT/2s）；:101-109（status 命令探测）；:112-114（send_control 原签名转调）；red 日志 2 测试真失败（silent 误报 true + responder 收 EOF） | — |
| 3 | TC-3（Step 5）：单次 lsof + 失败/空/非 UTF-8 → None；降级文本含 "lsof -iTCP"；单测真隔离 lsof | 10/10 | src/proxy.rs:472-487（`.ok()?`/`status.success()`/`from_utf8().ok()?`/`filter(!empty)` 全降级链）；:490-500（降级文本）；:899-914（PATH=/nonexistent 真隔离 + 恢复）；:918-933（lsof 可用分支） | — |
| 4 | TC-4（Step 8）：两 helper 集中 + 两应用点；outbound 泄漏备注在 TC-8 覆盖 | 10/10 | src/proxy.rs:628-650（mask_ctl_line/mask_request_path 集中）；应用点 :551（ctl）、:338（inbound）；**超集** :371（outbound URL）、:443（upstream error）——TC-4 refactor 日志"潜在泄漏观察"已闭环；tests/proxy_contract.rs:521-597 两条脱敏契约（log_masks_api_key + log_masks_api_key_upstream_error）覆盖 ctl/inbound/outbound/error 四路径 | — |
| 5 | TC-5（Step 9）：shutdown_proxy STOP_TIMEOUT 传播错误；stop_proxy 区分两态；Refactor status_to_result 只移代码 | 10/10 | src/proxy.rs:172-181（STOP_TIMEOUT + status_to_result，不再吞错）；src/main.rs:235-246（`!exists()` 快速 Ok vs shutdown_proxy 传播）；logs/shutdown_proxy_timeout-refactor.md（"行为零变化：同一 status 比较、同一错误构造、同一超时窗口"）；main.rs:845-888 两单测 | — |
| 6 | TC-6（Step 11）：与 plan 骨架一致（≥20 status + GET + 3s 界）；依赖链无跳步 | 10/10 | tests/proxy_contract.rs:388-443（ITERATIONS=20、BUDGET=3s、recv_timeout(3s) + http_elapsed<3s）；red 日志 exit 101 真死锁（修复前 HTTP 挂起）→ green 0.11s；依赖链 1→2→3→4/5→6 与 DAG 一致 | — |
| 7 | TC-7（Step 11）：vacuous Red 判定理由可信；Green 断言真实验证转发/Bearer/SSE | 10/10 | logs/stub_forwarding_with_bearer-red.md（exit 0，逐项核实三项失败假设均不成立：switch 分支/HEAD 基线:423 存在、Bearer 注入、SSE 流式；断言加强到完整契约面 + 手动 E2E 交叉验证）；tests:451-513（stub 记录恰 1 条 (POST,/v1/chat,Bearer sk-contract-key) + DELTA + chunked + 事件顺序 created→delta→completed） | — |
| 8 | TC-8（Step 11）：stderr piped 捕获；覆盖 ctl + 请求两路径 | 10/10 | tests:521-559（switch sk-contract-key 走 ctl 路径 + `?key=sk-xyz-query` 走 inbound/outbound 路径；stderr 全文无明文 + `sk-***` 反真空守卫）；tests:563-597（upstream error 路径扩展，超出 plan 7 契约清单的加固） | — |
| 9 | TC-9（Step 11）：①无响应 ≤2.5s 非 0 ②无 socket 快速 exit 0 **③stale connect-refused 快速非 0——未实现** | 7/10 | ①② 实现于 tests/proxy_contract.rs:604-677（elapsed≤2.5s + 非 0 + stderr 含 Error + 不误报 not running；exit 0 <1s + "Proxy is not running."）；**③ 全仓无自动化断言**——tdd.md:50 明示 "TC-9 case 3（stale socket 快错误路径）…快速非 0 退出" 属本用例范围；行为经代码语义成立（socket 存在 → shutdown_proxy → connect ECONNREFUSED → Err 传播 → 快速非 0），但无测试断言，TC-9 green/refactor 日志亦未记录"case 3 未落地" | 次要 |
| 10 | TC-10（Step 11）：SIGKILL→残留→ensure_proxy_running Ok；CCT_PROXY_BIN 注入正确 | 10/10 | tests:686-710（kill_and_wait + `sock.exists()` 前置 + `!check_proxy_running` 前置 + Ok + 恢复健康）；RestartEnvGuard:192-229（CCT_PROXY_BIN=CARGO_BIN_EXE_cct 注入 + Drop 清理）；red 日志 vacuous 论证含注入生效证据（0.63s 未耗尽） | — |
| 11 | TC-11（Step 11）：占用者存活断言 | 10/10 | tests/proxy_contract.rs:758-765（同端口再 bind 必须失败 + listener handle 存活） | — |
| 12 | TC-12（Step 11）：**偏差 1** bind 顺序（先 TCP 后控制） | 8/10 | (a) analysis 证据充分：确定性复现 SIGSTOP 24/24（修复前）→ 10/10（修复后，stderr 无 "control socket bound" 证明败者在 TCP 处先退）；统计样本 60+200+160+40；2/40 契约失败逐字段匹配；机制分类 (a)/(b)/(c) + 方向 A/B/C/D 论证完整（findings/double_start_race_one_wins-analysis.md）。(b) 收敛契约保持：fix-attempt-1 记录 20/20 无 flake + 全量 192/192 + SIGSTOP 10/10；refactor-verify 3/3 suite 稳定性。(c) EADDRINUSE 控制分支**非死代码**：delete-on-conflict 仍是僵尸文件路径（探测 false → 删 → 重绑），重绑冲突重探测 3×500ms 保留（src/proxy.rs:261-286）；`exit_socket_owned` 保留作防御；AC10 收敛契约迁移至 TCP 层仲裁（tests:777-839 断言未改一字且 20/20 过）。残差：analysis 建议"实施前在 plan 中确认"未形成 plan 修订（仅 tdd.md rev2 与 fix-attempt 日志记录） | 次要 |
| 13 | TC-13（Step 7）：shutdown 清理 socket；handle_control 签名变更 | 10/10 | src/proxy.rs:533（`fn handle_control(stream, state, socket_path: PathBuf)`）；:611-614（写响应后 remove_file + exit(0)）；tests:849-871（stop exit 0 后 `!sock.exists()`）；red 日志 exit 101 真失败（Step 7 未做时） | — |
| 14 | TC-14（Step 13）：临时 CODEX_HOME + 快照对比断言 | 10/10 | tests:916-971（CODEX_HOME/CCT_CONFIG 注入 + snapshot_codex_home 前后集合断言 + is_codex_config_file 禁止名单；ENOTCONN 瞬态经 3 次连接级重试加固 :941-954） | — |
| 15 | TC-15..19（Step 12）：fake 脚本真实应答；5 契约断言 | 10/10 | tests/launch_proxy_contract.rs:27-71（fake 真 `rm -f $SOCK` + python3 accept + 应答 `{"status":"ok"}`；probe_exhaustion fake `exit 0` 不监听）；5 测试断言：READY 标记（spawns:162-165）、PID 存活 + mtime 未变（reuses:199-211）、READY 重新 touch（zombie:253-256）、"did not become healthy" + ≤2s（probe:289-297）、Err 含 "port {port} already in use" + READY 不存在（bails:315-329） | — |
| 16 | TC-20（Step 15）：**偏差 2 harness** wait/trap/SSE 修复真实；15/15 来自真实输出；基线补录存在 | 8/10 | wait 修复真实（verify-B001 diff：无参 `wait` → `wait "${NCS[@]}"`）；trap 修复真实（B001/B003 cleanup 加 `[ -n ] && ... || true`，消除 set -e 下 `kill ""` 退出码覆写）；SSE 修复真实（stub-sse-upstream.py 补 `response.output_item.added/done` item-based 事件，对齐 codex 0.146）；基线补录行存在（poc.md:73 "修复前基线 15/11/4/0"）；13/15 原始输出已归档（/tmp/run-all-fix1.log，逐脚本 PASS/FAIL 行 + `Total: 15 | Pass: 13 | Fail: 2 | Skip: 0` 真实聚合）；**最终 15/15 运行原始输出未归档**（仅 doc_cleanup_final-green.md:53 与 poc.md:79 两处声明）；plan Step 15 门（15/15）与 B011/B013 属 Step 19-21 的 plan 内部时序张力被诚实记录（run 1/run 2/逐脚本三行留痕） | 次要 |
| 17 | TC-21（Step 16）：B006/B007/B008 全 PASS 判定成立；B007/B008 断言重写仍断言原契约（id 级） | 10/10 | logs/visibility_three_checks-green.md（B006：out-a/out-b 均含 POC_STUB_LAST_MESSAGE；B007：rollout 文件名提取 session-id 锁定 → resume --last 不出现 → 显式 `resume <session-id>` 恢复成功；B008：repo1/repo2 --all 复用 rollout 数不变 / repo3 默认过滤 +1 新建）——id 级核对非计数 | — |
| 18 | TC-22（Step 17）：B015 PASS + Results Log 当日行 | 10/10 | logs/layered_diag_and_log-green.md（`[PASS] B015: proxy 层存活 (HTTP 502)`，curl --noproxy '*' 先行）；poc.md:73-79 共 7 行（基线/run1/run2/单脚本/harness 修复后/逐脚本/终态）surgical 追加 | — |
| 19 | TC-23（Step 19-21）：迁移小节 + 五文档清理 + 三脚本 PASS；**偏差 3 判定** | 7/10 | install-script.md:+12 行与 plan Step 19 原文逐字一致；5 文档 grep 零陈旧叙述（本次复核 CLAUDE.md/ARCHITECTURE.md/docs/modules/launch.md/两 references 均 0 命中）；resume 语义段落在 codex-home-storage-layout.md:174-179 与 codex-backend-development-guide.md:211-219（provider ∩ cwd / --all 关不掉 provider / 显式 id 绕过）；B011/B013/B014 PASS + run-all 15/15。**偏差 3 判定：保留 plan 原文 `~/.config/cc-tui/proxy.sock` 忠实，但内容在 macOS 上事实错误**（代码为 `dirs::config_dir()/cc-tui/proxy.sock`，macOS 解析为 `~/Library/Application Support/cc-tui/proxy.sock`；本机即 macOS，迁移文档读者按文查找将找不到该路径）——green 日志已记录该 drift 但未修正 | 次要 |
| 20 | TC-23 范围核查（**偏差 4**）：full_auto 布尔叙述 | 8/10 | (a) 不在 AC13 范围成立（AC13 = per-profile CODEX_HOME/generate_codex_config/resume 语义）；(b) 预存 drift 证实：`Option<ApprovalLevel>`（src/config.rs:117）由 commit a67aeeb 引入，早于本任务全部提交（7dfa9de/a64362e/342428e/69c17b0），src/config.rs 本次零改动；(c) 风险记录存在（doc_cleanup_final-green.md 备注"属任务范围外预存 drift"）。**残差**：字段表行被"顺带更新"为 ApprovalLevel 语义（guide:39），而正文 6+ 处仍描述布尔 full_auto（guide:66/82/113/125/140-141/202/206/236）——文档内部新旧不一致，比"完全未动"略增歧义 | 次要 |
| 21 | 终端步骤（22-25）：Proof-Read 记录、Cross-Check 表、review.md、提交信息 | 8/10 | Proof-Read 与 Cross-Check AC 无执行记录（logs/ 无对应文件）；review.md 为 plan 阶段产物（draft/plan/review.md mtime Aug 1 17:36，非本次执行写入）；无提交（worktree 全量未提交）。tdd.md:68 明示"待回归门 + 保真度审计"——终端步骤设计上在审计循环之后，当前缺失属预期状态；回归门已实际执行（logs/regression-gate.md 原始输出 193/193 exit 0，本次复核 `cargo test` 复验 193/193 一致） | 次要 |
| 22 | 执行顺序：DAG 与 tdd.md 依赖列一致，无跳步 | 9/10 | 依赖列与 DAG 主线一致（TC-20 依赖 6..19、TC-21 [20]、TC-22 [20,21]、TC-23 [22]）；并行组合法交错（TC-15 red 10:12 先于 TC-6 red 10:15，即 launch 契约先于 proxy_contract infra 完成——tdd.md 依赖列 [1,2,3,4,5] 与 plan step 12 的 depends_on [4,10] 简化差异，因两测试文件互不依赖属良性）；无跳步、无并行冲突（zombie_recovery red 日志记录的并行 agent 重构系同组内部交错，最终 5/5 全绿） | 极轻 |

## 偏离详情
（仅列出评分 < 10 的检查项）

### 偏离 1: TC-9 case ③（stale socket 快速非 0 退出）无自动化断言
- **关联检查项**: #9
- **评分**: 7/10
- **证据**: tests/proxy_contract.rs:604-677 仅 ①（无响应 ≤2.5s 非 0 + 不误报）与 ②（无 socket 快速 exit 0）；tdd.md:50 明示 "TC-9 case 3（stale socket 快错误路径）：socket 文件存在但 connect 立即拒绝（旧版遗留死 socket）→ `cct proxy stop` 快速非 0 退出" 属 TC-9 范围；grep 全仓无 stale-connect-refused 的 stop 断言；TC-9 green/refactor 日志仅记录 ①②。
- **期望**: tdd.md 承诺的三 case 全部实现（① ② ③ 均有自动化断言）
- **实际**: ③ 未实现；行为经代码语义成立（src/main.rs:239-243：socket 存在 → shutdown_proxy → send_control_timeout connect ECONNREFUSED → Err 传播 → 快速非 0），仅缺断言
- **严重程度**: 次要
- **修复建议**: 在 tests/proxy_contract.rs stop_times_out_on_unresponsive_socket 追加 ③：`UnixListener::bind` 后让持有线程退出（或直接创建 socket 文件后模拟 connect 拒绝——bind 后进程内不 accept 且 listener 被 drop 使文件残留），spawn `cct proxy stop` → 断言快速（≤1s）非 0 退出且 stderr 含错误、stdout 不含 "Proxy is not running."；并补一行日志记录

### 偏离 2: TC-12 偏差 1（bind 顺序先 TCP 后控制）——判定：决策合理，记录充分
- **关联检查项**: #12
- **评分**: 8/10
- **证据**: findings/double_start_race_one_wins-analysis.md（SIGSTOP 确定性复现 24/24 → 修复后 10/10；统计样本 60+200+160+40；2/40 契约失败与 failure-attempt-1 逐字段一致；机制分类 (a)/(b)/(c)；方向 A 论证含约束 #3/#4/#5 合规性）；logs/double_start_race_one_wins-fix-attempt-1.md（20/20 无 flake + SIGSTOP 10/10 + 消息文本逐字未动）；src/proxy.rs:239-286（TCP 先行 + delete-on-conflict + 重探测保留）
- **期望**: plan Step 6 的控制 socket 先绑定顺序
- **实际**: TCP bind 前置为唯一仲裁者；控制段 EADDRINUSE 重探测保留于重绑冲突分支（僵尸安全网），`exit_socket_owned` 保留作防御；AC10 收敛契约在 TCP 层强制执行，`double_start_race_one_wins` 断言零改动且 20/20 过
- **严重程度**: 次要
- **修复建议**: 无需代码修复。建议在 code-spec.md Step 6 或约束注释中补一行顺序决策记录（analysis 已建议"在 plan 中确认"，当前仅 tdd.md rev2 与日志记录），供后续读者回溯

### 偏离 3: TC-20 最终 15/15 运行原始输出未归档
- **关联检查项**: #16
- **评分**: 8/10
- **证据**: /tmp/run-all-fix1.log 存在（13/15 原始输出，含逐脚本 PASS/FAIL 行与真实聚合计数）；最终 15/15 仅 doc_cleanup_final-green.md:53 与 poc.md:79 声明，无原始输出留存
- **期望**: 与 13/15 同等的原始输出归档（plan Step 15 Verify "run-all.sh 输出 Total: 15 | Pass: 15 | Fail: 0 | Skip: 0" 应附证据）
- **实际**: 声明一致（两处文档互证）但原始输出未归档
- **严重程度**: 次要
- **修复建议**: 在 doc_cleanup_final-green.md 或 poc.md 附上最终 run-all 原始输出（或注明归档路径）；亦可在审计周期内补跑一次 run-all.sh 留存输出

### 偏离 4: 偏差 3——install-script.md 迁移文档 socket 路径（判定：建议修正）
- **关联检查项**: #19
- **评分**: 7/10
- **证据**: docs/references/install-script.md:144-147（"遗留 socket 文件（`~/.config/cc-tui/proxy.sock`）"）；代码实际路径 = `dirs::config_dir().join("cc-tui").join("proxy.sock")`（src/proxy.rs:71-74），macOS 解析为 `~/Library/Application Support/cc-tui/proxy.sock`；本机为 macOS（Darwin 25.5.0），PID 29182 迁移场景即本机
- **期望**: 迁移文档引导用户找到实际 socket 路径
- **实际**: plan 原文 `~/.config/...` 逐字保留（对 Linux 正确，对 macOS 事实错误）；green 日志已记录 drift 但未修正。保留的论据：plan 忠实 + Linux 正确；修正的论据：文档读者是本机 macOS 用户，按文查找将落空
- **严重程度**: 次要
- **修复建议**: 改为双平台表述或引用代码语义，如"遗留 socket 文件（默认位于配置目录下的 `cc-tui/proxy.sock`；macOS 为 `~/Library/Application Support/cc-tui/proxy.sock`，Linux 为 `~/.config/cc-tui/proxy.sock`）"。verify-B011 断言（grep "遗留.*socket"）不受影响，无需改脚本

### 偏离 5: 偏差 4——full_auto 字段表行被顺带更新造成文档内部新旧不一致
- **关联检查项**: #20
- **评分**: 8/10
- **证据**: docs/references/codex-backend-development-guide.md:39（已更新为 ApprovalLevel 语义）vs :66/:82/:113/:125/:140-141/:202/:206/:236（仍为布尔 full_auto 叙述）；commit a67aeeb 引入 ApprovalLevel 早于本任务
- **期望**: 范围外叙述完全不动（避免新旧混写）
- **实际**: 字段表行顺带更新（内容正确，与代码一致），但正文仍为旧叙述——读者按表格理解与按正文理解冲突
- **严重程度**: 次要
- **修复建议**: 二选一——(a) 回退字段表行改动，与其余布尔叙述保持一致（整体留给后续 full_auto 专项）；或 (b) 在同次收尾中把正文 6 处布尔叙述一并更新为 ApprovalLevel 语义（超出 AC13 范围，需单独确认）。当前状态（表格新、正文旧）歧义最大

### 偏离 6: 终端步骤 22-25 尚未执行（设计预期状态，需审计后闭环）
- **关联检查项**: #21
- **评分**: 8/10
- **证据**: tdd.md:68 "待回归门 + 保真度审计"；logs/ 无 proof-read/cross-check/review/commit 记录；draft/plan/review.md 为 plan 阶段产物（Aug 1 17:36）；git status 全量未提交
- **期望**: Step 22 Proof-Read、Step 23 Cross-Check 表、Step 24 review.md、Step 25 Commit 在全部用例完成后执行
- **实际**: 未执行——但 tdd.md 明示审计循环为前置门，属设计中的待办而非漏做
- **严重程度**: 次要
- **修复建议**: 三循环审计完成后：执行 Step 22/23 并留记录（Cross-Check AC 表 15 行逐项对照）；Step 24 更新 draft 下 review.md（含本次审计结论）；Step 25 提交（建议信息已在 plan Step 25 给定）

### 偏离 7: 执行顺序——TC-15..19 先于 Step 10 infra 完成（良性）
- **关联检查项**: #22
- **评分**: 9/10
- **证据**: TC-15 red 10:12:49Z 早于 TC-6 red 10:15:03Z（launch 契约组先于 proxy_contract 契约组）；tdd.md 依赖列 TC-15 = [1,2,3,4,5] vs plan step 12 depends_on [4,10]
- **期望**: plan DAG 中 step 12 依赖 step 10
- **实际**: launch_proxy_contract.rs 与 proxy_contract.rs 无共享代码/环境（serial 各自域、env 各自守卫），TC-15..19 不依赖 step 10 基础设施；tdd.md 依赖列已显式反映 [1,2,3,4,5]，执行按其落地，两组并行无冲突
- **严重程度**: 极轻
- **修复建议**: 无需修复；如严格化可将 plan step 12 的 depends_on 改为 [4]（或保留保守 DAG，二者皆合规）

## 角度总评
SCORE: 7
**总分**: 7/10（所有检查项最低分）
**通过阈值**: ≥ 9

23 用例中 20 项满分布证充分（RGR 日志、测试断言、源文件三方互证），4 个已知偏差中偏差 1（bind 顺序）与偏差 2（harness 修复）判定为决策合理、记录完整、契约保持；偏差 3/4 判定为内容层面的次要缺陷。核心偏离不存在——plan 的核心实现（异步 accept、应用层探测、僵尸自愈、lsof 诊断、脱敏、stop 2s 超时、shutdown 清理、CCT_PROXY_SOCKET/BIN 注入、7+1+5 契约、15/15 闭环、五文档清理）全部落地且有测试证据。

## 判定
❌ NEEDS_REWORK — 共 2 个偏离需修复（偏离 1：TC-9 case ③ 补自动化断言；偏离 4：install-script.md socket 路径双平台表述）；另 3 项建议性修复（偏离 3 最终 15/15 原始输出归档、偏离 5 full_auto 新旧一致化、偏离 6 终端步骤审计后闭环）
