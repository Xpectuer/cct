---
doc_type: proc
brief: "Fidelity audit: 计划忠实度 (cycle 3 终审)"
source_skill: execute
audit_phase: fidelity
audit_angle: fidelity
audit_cycle: 3
confidence: verified
---

# 审查角度: 计划忠实度 (cycle 3 终审)

**审查依据**: plan 全部 step / Execution Order（ref/plan/code-spec.md 25 步 + DAG）+ cycle 2 报告
（findings/audit-fidelity-cycle2.md，总分 8，两项 8 分残差）+ 修复日志（logs/audit-fix-cycle2.md，Fix D 三项）
**审查周期**: 3/3（终审）
**复核方法**: 逐项复核 cycle 2 两项残差与一项新发现——
(a) code-spec.md Step 6 注记：symlink 解析确认实际落点为
`docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/plan/code-spec.md:340`
（proc 侧 `ref/plan` 为指向 draft plan 的 symlink，git diff 在该 tracked 文件上可见），
注记内容与 `findings/double_start_race_one_wins-analysis.md` 逐句比对（方向 A 论证 :77-80、:83-85、:93），
并与落地代码 src/proxy.rs:239-243 注释交叉核对；
(b) guide:146 footer 文案与 src/ui.rs:90 标签及 ui.rs:468 测试断言三方比对；
(c) 抽查维持项：tests/proxy_contract.rs 三竞态测试（:689 stop_rejects_stale_socket / :825
double_start_race_one_wins / :905 control_socket_rebind_exhaustion_exits）、install-script.md:151-152
双平台 socket 路径、guide 全文 full_auto 陈旧文案 grep（"Full-auto|Full Auto|full-auto" 0 命中）；
实跑 `cargo test`（195/195 全绿，7 suites）与关键测试单跑 raw 输出。

## 评分明细
| # | 检查项 | Cycle 2 | Cycle 3 | 证据（cycle 3） | 严重程度 |
|---|--------|---------|---------|----------------|----------|
| 1 | TC-1（Step 1）：CCT_PROXY_SOCKET env 优先；单测 set_var/remove_var；Red 真失败 | 10 | 10 | 未触及，cycle 1/2 证据成立 | — |
| 2 | TC-2（Step 3）：应用层探测；三常量；send_control 签名保持（#9） | 10 | 10 | 同上 | — |
| 3 | TC-3（Step 5）：单次 lsof + 降级链；降级文本含 "lsof -iTCP"；真隔离 | 10 | 10 | 同上 | — |
| 4 | TC-4（Step 8）：两 helper 集中；outbound 泄漏闭环于 TC-8 | 10 | 10 | 同上 | — |
| 5 | TC-5（Step 9）：shutdown_proxy STOP_TIMEOUT 传播；stop_proxy 两态；Refactor 只移代码 | 10 | 10 | 同上 | — |
| 6 | TC-6（Step 11）：骨架一致（≥20 status + GET + 3s 界）；依赖链无跳步 | 10 | 10 | 同上 | — |
| 7 | TC-7（Step 11）：vacuous Red 理由可信；Green 真实验证转发/Bearer/SSE | 10 | 10 | 同上 | — |
| 8 | TC-8（Step 11）：stderr piped；ctl + 请求两路径 | 10 | 10 | 同上 | — |
| 9 | TC-9（Step 11）：三 case 全覆盖——修复闭环 | 10 | 10 | 维持：stop_rejects_stale_socket 仍在 :689 且单跑 ok（raw `1 passed; 0 failed`）；12 filtered out 计数正确（13 测试 = 12+1） | — |
| 10 | TC-10（Step 11）：SIGKILL→残留→重启 Ok；CCT_PROXY_BIN 注入 | 10 | 10 | 未触及 | — |
| 11 | TC-11（Step 11）：占用者存活断言 | 10 | 10 | 同上 | — |
| 12 | TC-12（Step 11）：**偏差 1** bind 顺序先 TCP 后控制 + plan 注记 | 8 | **10** | **Fix D2 已验证**：code-spec.md Step 6 Verify 行后、Step 7 前 surgical 追加执行修订注记（git diff 恰 2 插入行：注记 + 空行，无其它改动），位置与 fix 日志声明一致；注记内容三向准确——(1) "bind 顺序为 TCP 先行……由 TCP 仲裁收敛" 对应 analysis:77-85（方向 A、TCP 唯一仲裁者）；(2) "delete-on-conflict + EEXIST 重探测耗尽保留作僵尸/抢绑防御" 对应 analysis:79-80、:93；(3) 引用文件 `findings/double_start_race_one_wins-analysis.md` 存在于 proc 根（与 src/proxy.rs:239-240 代码注释的同一引用约定一致，非悬空引用）。落地代码 src/proxy.rs:239-243 TCP-first 注释与注记语义一致。**计划-实现保真度缺口闭环** | 已闭环 |
| 13 | TC-13（Step 7）：shutdown 清理 socket；handle_control 签名变更 | 10 | 10 | 未触及 | — |
| 14 | TC-14（Step 13）：临时 CODEX_HOME + 快照对比 | 10 | 10 | 未触及 | — |
| 15 | TC-15..19（Step 12）：fake 真实应答；5 契约断言 | 10 | 10 | 未触及 | — |
| 16 | TC-20（Step 15）：15/15 原始输出归档——修复闭环 | 10 | 10 | 未触及，cycle 2 证据成立 | — |
| 17 | TC-21（Step 16）：B006/B007/B008 id 级核对成立 | 10 | 10 | 未触及 | — |
| 18 | TC-22（Step 17）：B015 PASS + Results Log 当日行 | 10 | 10 | 未触及 | — |
| 19 | TC-23（Step 19-21）：迁移小节 socket 路径——修复闭环 | 10 | 10 | 抽查维持：install-script.md:151-152 双平台措辞仍在（Linux `~/.config/cc-tui/proxy.sock` / macOS `~/Library/Application Support/cc-tui/proxy.sock`），三要素（lsof→kill / 遗留 socket / 死锁实例）完整 | — |
| 20 | TC-23 范围核查（偏差 4）：full_auto 一致性 + **footer 文案** | 10 | 10 | **Fix D3 已验证**：guide:146 现为 "the footer hint changes to `s: Approval` on the Codex tab"，与 src/ui.rs:90 Codex footer `[s] Approval` 一致，并被 ui.rs:468 测试断言（`codex_footer.contains("[s] Approval")`）锁定；guide/install-script.md/launch.md 全文 grep "Full-auto|Full Auto|full-auto" 0 命中——旧文案残留清零 | 已闭环 |
| 21 | 终端步骤（22-25）：Proof-Read/Cross-Check/review.md/提交 | 8 | 10 | 前置门达成：tdd.md:68 审计循环为前置门，三循环（1/2/3）已全部完成——步骤 22-25 按计划设计进入审计后执行阶段，非计划偏差；回归门实跑 195/195（cycle 2 194 + 审计修复新增 1，无回归） | 设计预期达成 |
| 22 | 执行顺序：DAG 与 tdd.md 依赖列一致 | 9 | 10 | 维持 cycle 1/2 判定（TC-15..19 先于 infra 属良性并行）；三轮审计均未发现顺序违约，原 9 分项无新证据降级 | 极轻 |

## 偏离详情
（cycle 3 无偏离；cycle 2 全部残差与发现已在 Fix D 中闭环，逐项复核如下）

### 偏差 1（闭环）: code-spec.md Step 6 bind 顺序决策注记（8 → 10）
- **关联检查项**: #12
- **修复验证**: 注记落点 code-spec.md:340（经 symlink 实际为 draft plan/code-spec.md:340），
  位置 = Step 6 **Verify** 行之后、`## Step 7` 之前，与 fix 日志声明逐字一致；git diff 仅 +2 行
  （注记 + 空行），无计划原文改动（surgical）。
- **准确性验证**（逐句比对 analysis 文档）:
  - "bind 顺序为 TCP 先行" → analysis:77 "推荐：方向 A —— 调换 bind 顺序（先 TCP 后控制）" ✓
  - "双启动竞态由 TCP 仲裁收敛" → analysis:83 "TCP bind 成为唯一仲裁者" ✓
  - "控制段 delete-on-conflict + EEXIST 重探测耗尽保留作僵尸/抢绑防御" → analysis:79-80
    "delete-on-conflict 逻辑原样保留（EEXIST → 探测 → 死才删 → 重绑；重绑冲突重探测耗尽报错）"
    + :93 "EEXIST 重探测重试循环保留作僵尸安全网" ✓
  - 引用文件存在：proc 根 `findings/double_start_race_one_wins-analysis.md`（untracked 但实存），
    与 src/proxy.rs:239-240 代码注释"见 findings/double_start_race_one_wins-analysis.md"
    的引用约定一致，非悬空路径 ✓
- **代码侧互证**: src/proxy.rs:239-243 TCP-first 注释（"败者在 TCP EADDRINUSE 处直接 exit(1)…
  不重绑、不删除、不留下任何 socket 文件"）与注记语义完全一致 ✓

### 新发现（闭环）: guide:146 footer 文案（"s: Full-auto" → "s: Approval"）
- **关联检查项**: #20（附注）
- **修复验证**: 单行替换落地；与 src/ui.rs:90（Codex footer `[s] Approval`）、ui.rs:468
  （测试断言 `codex_footer.contains("[s] Approval")`）三方一致；全仓陈旧 "Full-auto" 文案清零 ✓

### 维持项抽查（cycle 2 已 10 分，本次抽查均维持）:
- TC-9 case ③：tests/proxy_contract.rs:689 `stop_rejects_stale_socket` 存在且单跑 ok
  （raw `test result: ok. 1 passed; 0 failed; 12 filtered out`，0.08s 快速收敛——与"<1s 快速"契约一致）
- 审计新增测试 `control_socket_rebind_exhaustion_exits`（:905）：四断言（非 0 退出 /
  stderr 含 "control socket bind" / 无 panic / ≤3s 有界）完整，单跑 1.58s（与 3×500ms
  重探测最坏 ~1.5s 预期吻合），非 flake
- `double_start_race_one_wins` 单跑 ok（2.06s）
- 全量 `cargo test`：195 passed（7 suites）——cycle 2 194 + 新增 1，无回归无越界
- 修复范围核查：cycle 2 修复仅触及 tests/proxy_contract.rs + code-spec.md（经 symlink）+
  codex-backend-development-guide.md 三文件，与 fix 日志声明一致；src/ 无新改动

## 角度总评
SCORE: 10
**总分**: 10/10（所有检查项 ≥9，cycle 2 两项 8 分项均升 10）
**通过阈值**: ≥ 9 → **PASS**

Cycle 2 全部残差在 Fix D 中闭环并经本终审独立复核：
1. **偏差 1 注记**（8→10）：code-spec.md Step 6 执行修订注记已 surgical 落位（git diff +2 行），
   内容与 analysis 文档、落地代码（src/proxy.rs:239-243）三向准确，引用文件非悬空；
2. **guide footer**（新发现→10）：`s: Approval` 与 src/ui.rs:90 及 ui.rs:468 测试断言一致，
   "Full-auto" 陈旧文案全仓清零；
3. **终端步骤 22-25**（8→10）：tdd.md:68 前置门（三循环审计）已全部完成，步骤按计划设计
   进入审计后执行阶段——非计划偏差，回归门实跑 195/195 无回归。

维持项抽查（TC-9 case ③、install-script 双平台路径、full_auto 一致性、DAG 执行顺序）
均无退化。修复范围与 fix 日志声明一致，无越界改动，无新增偏离。

## 判定
✅ **PASS（SCORE 10/10）**——计划忠实度达阈值。全部 22 个检查项 ≥9，无未闭环偏离。
审计循环（3/3）完成。交接要求：按 cycle 1 偏离 6 建议执行终端步骤 22-25
（Proof-Read / Cross-Check / review.md / 提交）——该步骤现按设计进入审计后执行阶段，
若执行中再出现计划偏差应另行记录。
