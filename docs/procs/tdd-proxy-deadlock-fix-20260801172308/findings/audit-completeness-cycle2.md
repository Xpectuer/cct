---
doc_type: proc
brief: "Fidelity audit: 需求完整性 (cycle 2)"
source_skill: execute
audit_phase: fidelity
audit_angle: completeness
audit_cycle: 2
confidence: verified
---

# 审查角度: 需求完整性

**审查依据**: AC1-AC15 / Step 23 Cross-Check
**审查周期**: 2/3（复核 cycle 1 修复）
**审查方式**: 独立复跑 B006/B007/B011/B013 + 复跑可证伪性实验 + 逐项复核证据

## 评分明细

| # | 检查项 | 评分 | 证据 | 严重程度 |
|---|--------|------|------|----------|
| 1 | AC-1 死锁回归：真实二进制 + ≥20 次并发 status + HTTP GET + 3s 时间界 | 10/10 | 同 cycle 1（tests/proxy_contract.rs:390-443 + src/proxy.rs:504-531）；修复周期未触碰 src/tests，归档 run-all 15/15 复证 | — |
| 2 | AC-2 僵尸自愈：三层证据 + SIGKILL 残留 + 耗尽报错 | 10/10 | 同 cycle 1（launch.rs:134-174 / proxy.rs:263-285 / 三个契约测试）；未变 | — |
| 3 | AC-3 占端口报错：非 0 + lsof/降级文本 + 不 panic + 占用者存活 + 动态端口 | 10/10 | 同 cycle 1（proxy_contract.rs:719-766 + launch_proxy_contract.rs:306-330）；未变 | — |
| 4 | AC-4 stub 转发：Bearer 注入 + SSE DELTA + 事件顺序；responses-API SSE 契约 | 10/10 | 同 cycle 1（proxy_contract.rs:451-513 + stub-sse-upstream.py:37-51）；未变 | — |
| 5 | AC-5 日志脱敏：stderr 无明文 + 两路径 + sk-/custom 双形态 | 10/10 | 同 cycle 1（proxy_contract.rs:521-597 + proxy.rs:628-650 + 单测 939-1010）；未变 | — |
| 6 | AC-6 同 provider 可见：**session-id 对比 + rollout 复用计数**；6 旗标真实函数 | **10/10** | 偏离 1 已修复并独立复跑验证（见下）；`cct run smoke-b` 的 6 旗标经 build_codex_proxy_config_args 真实生成（launch.rs:251） | 已修复 |
| 7 | AC-7 跨 provider 不可见 + 显式恢复：rollout 计数 + **id 级核对** + 真实函数旗标 | **10/10** | 偏离 2 已修复并独立复跑验证（见下）；smoke-sub 经 build_codex_subscription_args、smoke-explicit 经 build_codex_proxy_config_args 真实生成 | 已修复 |
| 8 | AC-8 cwd 过滤 + --all：真实切换 cwd/仓库目录验证 | 10/10 | 同 cycle 1（verify-B008-cwd-filter.sh:15-53 + 协议修正记录）；未变 | — |
| 9 | AC-9 活 proxy 报错 + 复用：恰一存活 + 进程数不变 + 不删 socket | 10/10 | 同 cycle 1（proxy_contract.rs:777-839 + launch_proxy_contract.rs:178-215）；未变 | — |
| 10 | AC-10 契约覆盖 7 场景 + 隔离 | 9/10 | **未修复（维持）**：控制 socket EADDRINUSE 重探测耗尽子分支（proxy.rs:263-285）仍无直接测试（TCP-first 仲裁下双启动不可达）；EEXIST→探测→删→重绑由僵尸契约覆盖；proxy.rs:262 注释已注明防御性定位 | 次要 |
| 11 | AC-11 迁移说明三要素 + **双平台 socket 路径** | **10/10** | 偏离 3 已修复：install-script.md:149-151 现写 Linux + macOS 双路径；三要素齐备；独立复跑 verify-B011 PASS（exit 0） | 已修复 |
| 12 | AC-12 L2 前置：B012 预检 + 基线补录 + **ps -p 确认记录** | **10/10** | 偏离 4 已修复：poc.md 修复前基线行 Notes 追加 ps -p 29182 确认记录（已不存在 → 无需 kill，用户已确认）；事实依据核对：run_all_full_pass-green.md:16（29182 已终止 + 端口空闲）与 refactor-verify.md:10（29182 = ~/.local/bin/cct 实例）一致 | 已修复 |
| 13 | AC-13 五文档清理 + resume 语义段落 + 历史快照零改动 | 9/10 | **未修复（维持）**：五文档独立 grep 零命中；resume 语义两处齐备（guide:215-223 + layout:174-182）；codex-configuration-reference.md:324 "（待更新）"指针仍残留（范围外） | 次要 |
| 14 | AC-14 不写配置快照对比 + 接口冻结 | 10/10 | 同 cycle 1（proxy_contract.rs:916-971 + verify-B014 + clap 路由测试）；未变 | — |
| 15 | AC-15 分层诊断：curl --noproxy '*' 先行 + Results Log 当日行 | 9/10 | **未修复（维持）**：B015 仍诊断自起实例（脚本注释已注明门约束下的等价路径）；分层顺序与 3s 时间界不变 | 次要 |

## 偏离复核

### 偏离 1（AC-6 B006 断言不可证伪）→ 已修复，独立复跑验证 PASS
- **修复内容核对**: verify-B006-same-provider-visible.sh 现含 rollout_count() 与 session_id_of() 两个 helper（:19-21）；smoke-a 后断言 rollout 数==1 并提取 SESSION_ID_A（:32-34）；smoke-b 后断言 **rollout 数仍==1**（新建会话 → +1 → FAIL，:46-49）且 **SESSION_ID_B == SESSION_ID_A**（:51-54）；标记文本降级为辅助检查。修复内容与 fix 日志描述逐行一致，无夹带。
- **可证伪性独立实证**: 复跑 `/tmp/b007-exp.sh`（无匹配会话场景 = B006 错误实现路径）实测 **rollout 数 1→2**（smoke-a 后 1 个文件、smoke-sub 新建会话后 2 个文件，session-id 互异）——证明"若 resume --last 未复用 A 的会话，断言①必然 FAIL"的判别链成立。
- **独立复跑**: `bash scripts/verify-B006-same-provider-visible.sh` → `[PASS] B006: ... session-id 019fbe8b-ed2a-73e2-aa6a-7da42ac3998f 一致, rollout 数不变`，exit 0。
- **结论**: 偏离消除。spec AC-6 明文要求的 session-id 对比已落地（rollout 文件名 id 级）；OQ2 语义契约口径满足。**5/10 → 10/10**。

### 偏离 2（AC-7 B007 结构性空转 + 手工复刻 6 旗标）→ 已修复，独立复跑验证 PASS
- **不可见半项**: 判别断言改为 ① rollout 数==2（跨 provider 新建会话；错误复用 → 1 → FAIL，:44-47）② 新会话 session-id 与 SESSION_ID_A 无交集（id 级核对，:49-54）；原 `[ -f out-sub.txt ] && grep` 空转检查已移除。经验前提（codex 0.146 会话创建即写 rollout、API 失败不影响观测量）经我复跑 b007-exp.sh 证实（count 1→2，即使两跑均 API 失败）。
- **显式恢复半项**: 6 旗标不再手工复刻——临时 profile `smoke-explicit` 追加至 CCT_CONFIG 并经 `cct run smoke-explicit` 启动（:61-71）；launch.rs:251 确认 proxy 路径旗标由 `build_codex_proxy_config_args` 真实生成；smoke-sub 的 model_provider=openai 由 `build_codex_subscription_args` 真实生成（launch.rs:183-190）。断言：out-explicit 含标记 + rollout 数仍==2（新建 → 3 → FAIL）。
- **独立复跑**: `bash scripts/verify-B007-cross-provider-invisible.sh` → `[PASS] B007: 跨 provider 不可见（新会话 019fbe8c-0132-7912-964a-7cb66e52dd71 与 A 的 019fbe8b-fdc1-7042-a173-fadd34f3aa74 不同）; 显式 resume ... 可恢复`，exit 0。
- **结论**: 偏离消除。B006 与 B007 形成互证对：同 provider 复用（count 不变 + id 相同）∧ 跨 provider 新建（count +1 + id 互异），恰为 AC-6/AC-7 契约的判别组合。**6/10 → 10/10**。

### 偏离 3（AC-11 socket 路径 macOS 误导）→ 已修复
- install-script.md:149-151 现为 `遗留 socket 文件（`~/.config/cc-tui/proxy.sock`，Linux；`~/Library/Application Support/cc-tui/proxy.sock`，macOS）`——双平台措辞，与 dirs::config_dir() 实际解析一致（macOS → ~/Library/Application Support）；迁移三要素（健康复用→手动 kill / 删遗留 socket 兜底 / 新版不再死锁）齐备。
- 独立复跑 verify-B011-migration-docs.sh → PASS（exit 0）。**8/10 → 10/10**。

### 偏离 4（AC-12 kill 29182 确认记录缺失）→ 已修复
- poc.md 修复前基线行 Notes 追加：`迁移前置（plan Step 15）确认记录：ps -p 29182 显示旧版 cct proxy 实例 29182 已不存在、端口 19191 空闲 → 无需 kill，直接继续迁移（用户已确认）`。
- 事实基础核对：verify-B012-l2-prereqs.sh 用 `kill -0 29182` + `nc -z` 端口预检；run_all_full_pass-green.md:16（29182 已终止 + 端口空闲）、double_start_race_one_wins-refactor-verify.md:10（29182 为 `~/.local/bin/cct proxy start` 实例）——记录与日志证据一致，无虚构。**8/10 → 10/10**。

### 偏离 5（AC-10 控制 socket EADDRINUSE 分支无直接测试）→ 未修复，维持 9/10
- proxy.rs:263-285 复核：分支仍在（bind 冲突 → check_proxy_running → 删 → 重绑 → 3×500ms 重探测 → exit_bind_failed）；TCP-first 仲裁（:244-254）下双启动败者在 TCP 层退出，该分支对双启动不可达；僵尸场景（EEXIST→探测失败→删→重绑）仍由 zombie 契约覆盖。proxy.rs:262 注释已注明"本分支只对僵尸文件/异例触发——exit_socket_owned 保留作防御"（注释源自 double_start refactor，先于审计周期）。行为级覆盖成立、机制级缺口已文档化，归属偏差 1 判定范畴（fidelity 角度专审）。维持 9/10。

### 偏离 6（AC-13 codex-configuration-reference.md:324 过时指针）→ 未修复，维持 9/10
- :324 "当前 per-profile CODEX_HOME 布局（**待更新**）"复核仍存在——布局文档已更新为 shared ~/.codex（layout:184 "no per-profile CODEX_HOME"），"待更新"标注已事实过时。AC-13 范围限五份文档（独立 grep 全部零命中陈旧叙述；resume 语义段落 guide:215-223 与 layout:174-182 齐备：provider ∩ cwd 过滤、--all 不能关 provider 过滤、显式 id 绕过），此指针在范围外。维持 9/10。

### 偏离 7（AC-15 B015 自起实例）→ 未修复，维持 9/10
- verify-B015-layered-diag.sh:24-28 复核仍为自起 `cct proxy start` 后诊断（门约束下原"无监听→SKIP"设计恒 SKIP，破坏 Skip:0 门）；分层方法学（curl --noproxy '*' 3s 界、502=存活）完整保留；脚本头注释已注明等价性理由。修复前基线（真实例死锁 curl 超时）由 refs/proxy-deadlock-diagnosis.md 记录。维持 9/10。

## 角度总评
SCORE: 9
**总分**: 9/10（所有检查项最低分；cycle 1 为 5）
**通过阈值**: ≥ 9

## 判定
✅ PASS — 4 个偏离修复全部真实落地且经独立复跑/复实验验证；3 个未修复偏离均为次要（9/10），
各自有文档化接受理由，不构成验收证据缺陷

主要结论：cycle 1 的两个主要偏离（AC-6/AC-7 验收证据不可证伪）已彻底修复——B006/B007 的判别断言
现为 rollout 计数 + session-id 级对比（spec 明文要求的证据形态），且我独立复跑了修复后的脚本（均
PASS，exit 0）与可证伪性实验（无匹配会话 → rollout 1→2，证明断言在错误实现下会 FAIL）。B006 与
B007 构成互证对：同 provider 复用（count 不变 + id 一致）∧ 跨 provider 新建（count +1 + id 互异）。
文档侧修复（AC-11 双平台路径、AC-12 ps -p 记录）均与实际日志证据吻合，B011/B013 独立复跑 PASS。
遗留三项（AC-10 防御分支无直接测试、AC-13 范围外过时指针、AC-15 自起实例）为已文档化的次要偏离，
不阻塞。总评自 cycle 1 的 5/10 提升至 9/10，达通过阈值。
