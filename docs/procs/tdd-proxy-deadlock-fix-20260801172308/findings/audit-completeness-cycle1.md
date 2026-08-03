---
doc_type: proc
brief: "Fidelity audit: 需求完整性 (cycle 1)"
source_skill: execute
audit_phase: fidelity
audit_angle: completeness
audit_cycle: 1
confidence: verified
---

# 审查角度: 需求完整性

**审查依据**: AC1-AC15 / Step 23 Cross-Check
**审查周期**: 1/3

## 评分明细

| # | 检查项 | 评分 | 证据 | 严重程度 |
|---|--------|------|------|----------|
| 1 | AC-1 死锁回归：真实二进制 + ≥20 次并发 status + HTTP GET + 3s 时间界 | 10/10 | tests/proxy_contract.rs:390-443（20 次 send_control + 并发 HTTP GET，recv_timeout(3s) + http_elapsed<3s 双界）；src/proxy.rs:504-531（tokio 异步 accept + spawn_blocking）；verify-B001-concurrent-http.sh:31-38 | — |
| 2 | AC-2 僵尸自愈：三层证据 + SIGKILL 残留 + 耗尽报错 | 10/10 | src/launch.rs:134-174（探测→试探 bind→CCT_PROXY_BIN spawn→就绪 500ms×3）；src/proxy.rs:263-285（先探测再删+重绑）；tests/proxy_contract.rs:686-710；tests/launch_proxy_contract.rs:221-261；probe_exhaustion_reports_error（launch_proxy_contract.rs:268-298，≤2s Err + "did not become healthy"）；verify-B002-zombie-recovery.sh | — |
| 3 | AC-3 占端口报错：非 0 + lsof/降级文本 + 不 panic + 占用者存活 + 端口取实际绑定值 | 10/10 | tests/proxy_contract.rs:719-766（动态端口 + 4 断言）；tests/launch_proxy_contract.rs:306-330（Err + READY 标记证明未 spawn）；src/proxy.rs:472-500（tcp_port_owner/port_conflict_message）；verify-B003-port-occupied.sh:37-39（lsof 可用时断言 PID） | — |
| 4 | AC-4 stub 转发：Bearer 注入 + SSE DELTA + 事件顺序；stub 为 responses-API SSE 契约 | 10/10 | tests/proxy_contract.rs:451-513（POST /v1/chat 无客户端 Auth 头 → stub 记录 Bearer sk-contract-key + DELTA + created→delta→completed 顺序）；stub-sse-upstream.py:37-51（item-based SSE：output_item.added/done，对齐 codex 0.146）；verify-B004-stub-forwarding.sh | — |
| 5 | AC-5 日志脱敏：stderr 捕获无明文 + 两路径 + sk-/custom 双形态 | 10/10 | tests/proxy_contract.rs:521-559 + 565-597（ctl + inbound + outbound 错误路径，反真空 sk-*** 守卫）；src/proxy.rs:628-650（mask_ctl_line 按字段名 + mask_request_path sk- 扫描）；单测 proxy.rs:939-1010 | — |
| 6 | AC-6 同 provider 可见：**B006 输出 session-id 对比**；6 旗标真实函数生成 | **5/10** | verify-B006-same-provider-visible.sh:28-34 仅断言 out-b.txt 含固定标记文本——不可证伪（见偏离 1）；旗标经 cct run → build_codex_proxy_config_args 真实生成（该半项满足） | 主要 |
| 7 | AC-7 跨 provider 不可见 + 显式恢复：**id 级核对** | **6/10** | verify-B007-cross-provider-invisible.sh:31-35 不可见断言结构性空转（smoke-sub 不经过 stub）；:40-46 显式 resume 手工复刻 6 旗标（见偏离 2）；session-id 提取（:23-27）+ 显式恢复（:51）为真实证据 | 主要 |
| 8 | AC-8 cwd 过滤 + --all：真实切换 cwd/仓库目录验证 | 10/10 | verify-B008-cwd-filter.sh:15-53（git init 三个 repo；(cd repo*) 真切换；rollout 文件数语义：--all 复用保持 1、默认过滤新建 +1——经 run_all_full_pass-fix-attempt-1.md:52 协议发现修正为可证伪断言） | — |
| 9 | AC-9 活 proxy 报错 + 复用：恰一存活 + 进程数不变 + 不删 socket | 10/10 | tests/proxy_contract.rs:777-839（恰一存活 + 败者 ≤2s 非 0 + 无 panic）；tests/launch_proxy_contract.rs:178-215（原进程存活 + READY mtime 不变）；verify-B009-double-start.sh:37-39（socket 完好 + 原 proxy 响应） | — |
| 10 | AC-10 契约覆盖 7 场景 + 隔离（临时 socket/动态端口/serial） | 9/10 | 7 行为契约逐一存在（tests/proxy_contract.rs 7 个 #[serial]）+ shutdown 清理 + 快照 + 上游错误；隔离经 CCT_PROXY_SOCKET 临时路径 + free_port + NO_PROXY；**控制 socket EADDRINUSE 重探测耗尽子分支（src/proxy.rs:263-285）不再被直接测试**（偏差 1：TCP-first 仲裁；EEXIST→探测→删→重绑路径由僵尸契约覆盖） | 次要 |
| 11 | AC-11 迁移说明三要素（健康复用→手动 kill；删遗留 socket 兜底；新版不再死锁） | 8/10 | docs/references/install-script.md:143-153 三要素齐备（B011 PASS）；**偏差 3：:151 socket 路径 `~/.config/cc-tui/proxy.sock` 在 macOS 上不存在**（dirs::config_dir() → ~/Library/Application Support），用户本机即 macOS | 主要 |
| 12 | AC-12 L2 前置：B012 预检 + 基线补录 + kill 前确认记录 | 8/10 | verify-B012-l2-prereqs.sh（29182 + 端口预检）PASS；poc.md Results Log:73 基线行存在；**kill 29182 无显式 ps -p 确认/用户确认日志**——身份确认仅见于 double_start_race_one_wins-refactor-verify.md:10（pgrep -fl 记录 29182 为 ~/.local/bin/cct 实例） | 次要 |
| 13 | AC-13 五文档清理 + resume 语义段落 + 历史快照零改动 | 9/10 | 五文档 grep 零命中陈旧叙述（实测）；resume 语义两处（codex-backend-development-guide.md:211-219 + codex-home-storage-layout.md:174-182）；context-*/procs 快照未动（git status 验证）；**残留：docs/references/codex-configuration-reference.md:324 "当前 per-profile CODEX_HOME 布局（待更新）"已事实过时**（布局文档已更新，属五文档范围外） | 次要 |
| 14 | AC-14 不写配置快照对比 + 接口冻结 | 10/10 | tests/proxy_contract.rs:916-971（CODEX_HOME 快照前后一致 + 禁止名单）；verify-B014-interface-frozen.sh（全量 cargo test + CCT_PROXY_PORT/LOG/proxy start\|stop/run 存在性）；clap 路由测试（main.rs:611-641） | — |
| 15 | AC-15 分层诊断：curl --noproxy '*' 先行 + Results Log 当日行 | 9/10 | verify-B015-layered-diag.sh（自起实例 + curl --noproxy '*' 3s 界，502=存活）PASS；poc.md Results Log 含 2026-08-02 行（15/15/0/0）；**自起实例替代"用户运行中实例"为脚本原设计偏差**（fix-attempt-1.md:61 解释 Skip:0 门） | 次要 |

## 偏离详情

### 偏离 1: B006 同 provider 可见性断言不可证伪——AC-6 证据不能证明 AC
- **关联检查项**: #6（AC-6）
- **评分**: 5/10
- **证据**:
  - `docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/scripts/verify-B006-same-provider-visible.sh:28-34` — 唯一断言：`grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-b.txt"`。
  - `docs/procs/tdd-proxy-deadlock-fix-20260801172308/logs/run_all_full_pass-fix-attempt-1.md:52` — 执行团队自己的协议发现："实测 codex `exec resume --last` 无匹配会话时**新建会话并运行（标记必然出现）**，原断言在真实协议下不可能成立"（该修正仅应用于 B008，未应用到 B006 的正向断言）。
  - `stub-sse-upstream.py:23` — stub 对所有请求返回固定 DELTA 文本 `POC_STUB_LAST_MESSAGE`，新会话与复用会话的 -o 输出相同。
  - `spec.md:70` — AC6 要求"输出中出现 profile A 会话的 **session-id**（-o 捕获 + rollout session_meta 对比）"；OQ2 放宽为语义契约断言，但实现连语义级可证伪断言也缺失。
- **期望**: 有可证伪的会话级证据（session-id 对比，或 rollout 复用计数——B008 已验证该法可行）证明 resume --last 复用了 profile A 的会话。
- **实际**: 标记文本断言在"复用 A 会话"与"新建会话"两条路径下均 PASS——断言语义上空转。spec 明文要求的 session-id 对比完全缺失。
- **严重程度**: 主要（验收证据缺陷；底层语义可能为真——B008 repo2 --all 复用计数与 B007 显式恢复为间接佐证——但 AC6 专属证据无法证明该 AC。若以可证伪断言重测后 FAIL，升级为严重）。
- **修复建议**: 在 B006 中加入与 B008 相同的 rollout 计数断言（smoke-a 后计数=1；smoke-b resume --last 后计数仍=1 证明复用，若新建会话计数=2 则 FAIL）；或在 run-b.log 中断言 codex 打印的 resumed session-id 与 A 的 rollout 文件名 id 一致。

### 偏离 2: B007 跨 provider 不可见断言结构性空转 + 显式恢复手工复刻 6 旗标
- **关联检查项**: #7（AC-7）
- **评分**: 6/10
- **证据**:
  - `verify-B007-cross-provider-invisible.sh:31-35` — 不可见断言为 `if [ -f out-sub.txt ] && grep -q 标记; then FAIL`。smoke-sub 是 subscription 模式（`setup-smoke.sh:72-77`），codex 直连 api.openai.com（不经 proxy/stub），TEST_API_KEY 为假 key（visibility_three_checks-green.md:24）→ 续接必然 401 失败 → out-sub.txt 永不生成 → 断言恒 PASS。**无论 provider 过滤是否生效，该断言结果相同**——结构性不可证伪。
  - `verify-B007-cross-provider-invisible.sh:40-46` — 显式 resume 的 6 个 `--config` 旗标为手工复刻（注释自认"镜像 build_codex_proxy_config_args"，run_all_full_pass-fix-attempt-1.md:49），违反 spec 的"6 旗标由真实函数生成、禁止手工复刻"（spec.md:70）与 single-source-of-truth 规则——旗标一旦 drift，显式恢复证据即失真。
  - 正向半项为真：session-id 自 rollout 文件名提取（:23-27）+ 显式 `resume <session-id>` 恢复成功（:51）。
- **期望**: 不可见性有 id 级可证伪核对（审计项要求"另一 provider id 不出现（id 级核对，非仅计数）"）；显式恢复旗标由真实函数生成。
- **实际**: 不可见断言无法在任何可执行路径上 FAIL；无 id 级核对；显式恢复旗标为手写镜像。
- **严重程度**: 主要
- **修复建议**: ① 不可见半项改用 rollout 计数语义（smoke-sub 运行后对比 A 的 session-id 不存在于新 rollout 文件，或断言 openai 新建会话的 rollout 数与 A 的 session-id 无交集）；② 显式恢复改为经真实函数——在 `cct run` 路径上增加一个 extra_args 嵌入 `exec resume <id>` 的临时 profile（旗标由 build_codex_proxy_config_args 生成），或至少以契约测试锁定手工旗标与 `build_codex_proxy_config_args("gpt-4.1", $CCT_PROXY_PORT)` 输出一致。

### 偏离 3: 迁移说明 socket 路径在 macOS 上不存在（忠实 plan 但误导用户）
- **关联检查项**: #11（AC-11）
- **评分**: 8/10
- **证据**: `docs/references/install-script.md:151` — "遗留 socket 文件（`~/.config/cc-tui/proxy.sock`）→ 可手动删除兜底"。`src/proxy.rs:71-75` — `proxy_socket_path()` 用 `dirs::config_dir()`，macOS 解析为 `~/Library/Application Support`；用户本机为 Darwin（本仓库 CLAUDE.md 亦写明 macOS 配置路径为 `~/Library/Application Support/cc-tui/`）。旧版（plan code-spec.md Step 1 "Old" 片段）同样用 `dirs::config_dir()`，遗留 socket 实际在 `~/Library/Application Support/cc-tui/proxy.sock`。路径系 plan Step 19 模板原文照抄（doc_cleanup_final-green.md:59 已自注）。
- **期望**: 迁移文档给出的路径在用户实际平台可定位。
- **实际**: macOS 用户按文档找不到该文件；幸而第 2 步为"可手动删除兜底"且附带"新版本启动时探测失败会自动清理，删除顺序无关均安全"，不阻塞迁移（步骤 1 的 lsof -iTCP:19191 正确）。
- **严重程度**: 主要（用户面事实错误；文档收尾的产物本身不准确）
- **修复建议**: 改为双平台表述，如"`<config_dir>/cc-tui/proxy.sock`（macOS 为 `~/Library/Application Support/cc-tui/proxy.sock`，Linux 为 `~/.config/cc-tui/proxy.sock`）"。

### 偏离 4: kill 29182 的 ps -p 确认与用户确认记录缺失
- **关联检查项**: #12（AC-12）
- **评分**: 8/10
- **证据**: `logs/run_all_full_pass-fix-attempt-1.md:77` 与 `logs/run_all_full_pass-green.md:16` 均只记"29182 已终止"（21:31 首跑时已不在）；kill 动作本身、kill 前的 `ps -p 29182` 确认、执行前用户确认，在 proc logs 中无显式记录。身份确认的最接近证据是 `logs/double_start_race_one_wins-refactor-verify.md:10`（pgrep -fl 确认 29182 为 `~/.local/bin/cct proxy start`，且注明"未 kill"）——确认存在但先于 kill，且无专门日志行。
- **期望**: plan Step 15 要求"先 `ps -p 29182` 确认仍是旧版 cct proxy 实例再执行"且"kill 前向用户确认"，应有记录。
- **实际**: 无显式 ps -p/kill/用户确认日志行；B012 预检结果反证迁移已发生。
- **严重程度**: 次要（证据链缺口，行为结果正确）
- **修复建议**: 补记一行迁移执行记录（ps -p 29182 输出 + kill + 时间戳）到 tdd.md 或 poc.md Results Log Notes。

### 偏离 5: 控制 socket EADDRINUSE 重探测耗尽子分支不再被契约直接测试
- **关联检查项**: #10（AC-10）
- **评分**: 9/10
- **证据**: `src/proxy.rs:263-285` — is_bind_conflict → 探测 → 删 → 重绑 → 重绑仍冲突则 3×500ms 重探测 → exit_bind_failed。但 `src/proxy.rs:244-254` TCP bind 先行的仲裁（偏差 1）使双启动败者在 TCP EADDRINUSE 处直接退出，控制段 EADDRINUSE 重探测分支在双启动场景不可达；`double_start_race_one_wins`（tests/proxy_contract.rs:777-839）实际只测试 TCP 层收敛。EEXIST→探测失败→删→重绑路径仍被僵尸契约（tests/proxy_contract.rs:686-710）覆盖。
- **期望**: AC10 列举"控制 socket bind EADDRINUSE → 重新探测而非 panic"为契约覆盖项。
- **实际**: 行为契约（收敛 + 不 panic + 不双活）成立且有测试；spec 点名的控制-socket EADDRINUSE 机制分支变为防御性死代码、无直接测试。
- **严重程度**: 次要（行为覆盖，机制级覆盖缺口——归属偏差 1 的判定范畴，fidelity 角度另有专审）
- **修复建议**: 接受为防御性代码并在注释/文档注明测试边界，或在单测中构造"TCP 空闲 + 控制路径被占"场景直接测试该分支。

### 偏离 6: codex-configuration-reference.md 残留过时"待更新"指针
- **关联检查项**: #13（AC-13）
- **评分**: 9/10
- **证据**: `docs/references/codex-configuration-reference.md:324` — "`docs/references/codex-home-storage-layout.md` — 当前 per-profile CODEX_HOME 布局（**待更新**）"。该布局文档已在本次收尾中更新为 shared `~/.codex`（codex-home-storage-layout.md revision 1→2），"待更新"标注已事实过时。AC13 范围限五份文档，此文件不在列。
- **期望**: 文档树内无与已落地改动矛盾的文字（该指针会让读者误以为布局文档仍未更新）。
- **实际**: 五份目标文档全净（grep 零命中）；范围外参考文档残留一条过时指针。历史快照（context-*/procs）按要求未动——正确。
- **严重程度**: 次要
- **修复建议**: 将 :324 的 brief 更新为 shared `~/.codex` 布局（一行 surgical 改动），或列入后续 doc 维护。

### 偏离 7: B015 自起实例替代用户运行中实例
- **关联检查项**: #15（AC-15）
- **评分**: 9/10
- **证据**: `verify-B015-layered-diag.sh:24-28` — 脚本自行 `cct proxy start` 后诊断（`run_all_full_pass-fix-attempt-1.md:61` 注明原"无监听→SKIP"设计在 daemon 清理后恒 SKIP，破坏 Skip:0 门）；分层顺序（curl --noproxy '*' 3s 界，502/404=存活）与断言不变；B004 承担上游层（codex 对话）。Results Log 当日行齐备（poc.md:79，15/15/0/0）。
- **期望**: 对"用户运行中实例"做分层诊断（spec AC15 场景）。
- **实际**: 改为对自起实例诊断——分层诊断方法学（proxy 层先行、超时=死锁）完整保留，但对象与用户实跑实例不同；修复前基线（真实例死锁 curl 超时）已由 refs/proxy-deadlock-diagnosis.md 记录。
- **严重程度**: 次要（等价验证路径，语义保留）
- **修复建议**: 无需修复；建议在脚本注释中注明"自起实例 + 修复前基线证据"双轨。

## 角度总评
SCORE: 5
**总分**: 5/10（所有检查项最低分）
**通过阈值**: ≥ 9

## 判定
❌ NEEDS_REWORK — 共 7 个偏离需修复（其中 2 个为主要：AC-6 与 AC-7 的验收证据不可证伪；5 个为次要）

主要结论：Part A（AC-1~5、AC-9、AC-10、AC-14）实现与证据扎实——契约测试真实二进制 + 临时 socket/动态端口 + 时间界 + 反真空断言，逐一核对无空转。Part B 的文档侧（AC-11/13）与前置（AC-12/15）基本到位，但**会话可见性三查中 B006/B007 的判别性断言存在结构性缺陷**：stub 固定输出使标记文本断言在"复用/新建"两路径下同结果，执行团队已在 B008 上发现并修正同一问题（fix-attempt-1.md:52），却未回查同构的 B006/B007 断言——AC-6/AC-7 的通过证据不能证明其 AC。修复建议均已给出可执行步骤（rollout 计数语义 / session-id 核对 / 真实函数旗标）。
