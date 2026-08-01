---
title: "audit-fix-completeness — Cycle 1"
brief: "需求完整性审计 cycle 1 修复：B006/B007 断言可证伪化（session-id/rollout 语义 + 真实函数旗标）"
doc_type: proc
created: 2026-08-02T02:07:50+0800
case: audit-fix-completeness
phase: audit-fix
---

## 修复日志: 需求完整性 — Cycle 1

依据: `findings/audit-completeness-cycle1.md` 偏离 1（AC-6 B006 不可证伪）与偏离 2
（AC-7 B007 结构性空转 + 手工复刻 6 旗标）。修复范围: 仅 `poc/scripts/` 与 `poc.md`。

### 偏离 A1: B006 同 provider 可见性断言不可证伪 → 改为 session-id 对比 + rollout 复用计数

- **修复文件**: `docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/scripts/verify-B006-same-provider-visible.sh`
- **修复内容**:
  - **before**: 唯一判别断言是 `grep -q POC_STUB_LAST_MESSAGE out-b.txt`。stub 对所有
    请求返回同一 DELTA，`resume --last` 无论复用 A 的会话还是新建会话都产生该标记
    → 断言恒 PASS（执行团队在 B008 已发现同构问题并修正，但未回查 B006）。
  - **after**:
    1. 新增 `rollout_count()` 与 `session_id_of()` 两个 helper（session-id 从
       `sessions/<Y>/<M>/<D>/rollout-<ts>-<session-id>.jsonl` 文件名提取；
       原 B007 正则与真实文件名不匹配，改用尾部 UUID 捕获的稳健正则）。
    2. smoke-a 后: 断言 rollout 数 == 1 并记录 `SESSION_ID_A`。
    3. smoke-b（同 provider `resume --last`）后: 断言 **rollout 数仍 == 1**
       （复用 A 的会话 → 文件数不变；新建会话 → +1 = FAIL）且 **session-id 与 A
       一致**（spec AC-6 明文要求 "输出中出现 profile A 会话的 session-id"）。
       `-o` 标记文本降级为"resume 确实产生了输出"的辅助检查。
- **验证**: 单跑 `bash scripts/verify-B006-same-provider-visible.sh` → exit 0:
  `[PASS] B006: profile B 经 resume --last 复用 profile A 的会话（session-id
  019fbe7e-1bbd-7fe2-9a59-7b39d3ff7691 一致, rollout 数不变）`。
  run-all 全量中 B006 PASS（session-id 019fbe81-618a-79e0-9c14-dc20277124a7 一致）。

### 偏离 A2: B007 跨 provider 不可见断言结构性空转 + 显式恢复 6 旗标手工复刻

- **修复文件**: `docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/scripts/verify-B007-cross-provider-invisible.sh`
- **修复内容**:
  - **不可见半项**（before: smoke-sub 直连 api.openai.com 不经 stub，假 key 使
    续接必然 401 → out-sub.txt 永不生成 → `[ -f out-sub.txt ] && grep 标记` 恒
    PASS，任何路径无法 FAIL）:
    - 保留 smoke-sub 经 `cct run` 的 subscription 路径（provider 身份旗标
      `model_provider=openai` 由 cct 真实函数 `build_codex_subscription_args`
      生成，零手工复刻），但判别断言改为 rollout 语义:
      ① 断言 rollout 数 == 2——跨 provider 不可见 → 新建会话（+1）；若错误复用
      A 的会话 → 数仍 1 → FAIL；
      ② id 级核对——新会话 session-id 与 `SESSION_ID_A` 无交集（spec AC-7
      "输出中不包含另一 provider 的任何 session-id"）；
      ③ 原"out-sub.txt 含标记即 FAIL"检查移除（若直连意外成功，新建会话自身
      输出也含标记，该检查会误报——判别力已由 ①② 承担）。
      经验前提（本机实测）: codex 0.146 在会话创建时即写 rollout 文件，续接
      API 失败不影响 ①② 可观测量。
  - **显式恢复半项**（before: 6 个 `--config` 旗标手工复刻，注释自认"镜像
    build_codex_proxy_config_args"）:
    - 改为向 `$CCT_CONFIG` 追加临时 profile `smoke-explicit`
      （`extra_args = ["exec", "resume", "<SESSION_ID_A>", "-o", "<out-explicit>", "hello"]`），
      经 `"$CCT_BIN" run smoke-explicit` 启动——6 个 `--config` 旗标由 cct 真实
      函数 `build_codex_proxy_config_args` 生成（spec: "6 旗标由真实函数生成、
      禁止手工复刻" + single-source-of-truth）。
    - 断言: out-explicit.txt 含标记（会话内容恢复）+ rollout 数仍 == 2（显式
      resume 复用既有会话；新建会话 → 3 → FAIL）。
- **验证**: 单跑 `bash scripts/verify-B007-cross-provider-invisible.sh` → exit 0:
  `[PASS] B007: 跨 provider 不可见（新会话 019fbe80-cb12-78a0-aef0-88c6fb2c2c47
  与 A 的 019fbe80-c789-7803-866e-babc248c0b1b 不同）; 显式 resume
  019fbe80-c789-7803-866e-babc248c0b1b 可恢复`。run-all 全量中 B007 PASS。

### 可证伪性说明（修复后 B006/B007 在错误实现下应 FAIL）

- **B006 判别力**: 若同 provider 可见性失效（`resume --last` 未复用 A 的会话），
  codex 会新建会话 → rollout 数 1→2 → 断言 ① FAIL。经验验证:
  `/tmp/b007-exp.sh` 实测"无匹配会话 → 新建 rollout（1→2）"（该场景即 B006
  的错误实现路径）; `/tmp/b006-exp.sh` 实测正确路径 rollout 数保持 1 且
  session-id 不变。
- **B007 判别力**: 若跨 provider 过滤失效（`resume --last` 错误复用 A 的会话），
  rollout 数保持 1 → 断言 ① FAIL（可证伪方向与 B006 相反但同观测量）; 若
  新会话 id 与 A 相同 → 断言 ② FAIL; 若显式 resume 新建会话 → 断言 ③ FAIL。
  修复前恒 PASS 的空转路径（out-sub.txt 永不生成）已不再参与判别。
- 未真正篡改断言观察 FAIL（按任务要求仅说明）——判别可观测量均已通过上述
  独立实验实证。

### 附注: 修复中发现的 bash 3.2 解析陷阱

- PASS echo 行内 `$SESSION_ID_B ≠ $SESSION_ID_A）` 中多字节字符（`≠`/`）`）
  紧邻 `$VAR` 时，macOS bash 3.2.57 报 `SESSION_ID_A�: unbound variable`
  （脚本文件模式复现; 交互模式无此问题）。修复: 变量后只用 ASCII/空格分隔
  （`新会话 $SESSION_ID_B 与 A 的 $SESSION_ID_A 不同`）。两脚本其余 `$VAR` 处
  已逐一核对无多字节紧邻。

### 结果

修复后单独运行 B006/B007 均 PASS（exit 0）; `./run-all.sh` 全量
**Total: 15 | Pass: 15 | Fail: 0 | Skip: 0**，完整原始输出
见 `logs/run_all_full_pass-audit-fix1.md`; poc.md Results Log 已追加
"审计修复后确认"一行。
