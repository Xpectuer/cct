---
title: "doc_cleanup_final — Green Phase"
brief: "doc_cleanup_final — Green: exit 0"
doc_type: proc
created: 2026-08-02T01:39:08+0800
case: "doc_cleanup_final"
phase: green
---

Exit code: 0
B011: PASS
B013: PASS
B014: PASS
run-all 终态: 15/15/0/0

## 改动摘要

### Step 19 — install-script.md 迁移说明（AC11，B011 PASS）

- `docs/references/install-script.md` — 追加 `## Upgrading from pre-fix versions (deadlock) 迁移说明` 小节（`### 旧版死锁实例迁移（一次性）`）：
  1. `lsof -iTCP:19191` 发现旧 PID 存活 → `kill <PID>`（新版探测会视为健康复用，唯一修复路径是手动终止）；
  2. 遗留 socket 文件（`~/.config/cc-tui/proxy.sock`）可手动删除兜底，新版启动探测失败会自动清理，删除顺序无关均安全；
  3. 新版本不再产生死锁进程，迁移一次性。
- 内容按 plan code-spec.md Step 19 原文追加（surgical 追加，未动原有章节）。

### Step 20 — 五文档 AC13 清理 + resume 语义（B013 PASS）

- `CLAUDE.md`（4 处）：
  - launch 模块表行：`generate_codex_config` → `build_codex_proxy_config_args`；
  - Codex launch 设计要点：改写为 CODEX_HOME 不设置（所有 profile 共享默认 `~/.codex`）、proxy 模式 6 个 `--config` 旗标注入（不再写 config.toml）、subscription 模式 `--config model_provider=openai`；
  - 模块文档索引行与 CODEX_HOME storage layout 索引 brief（per-profile → shared）同步。
- `ARCHITECTURE.md`：grep 确认无陈旧叙述（第 95/297 行已为新叙述），零改动。
- `docs/modules/launch.md`（revision 3 → 4）：
  - Purpose 改写（"generates Codex config files" → "builds inline `--config` flags"）；
  - `generate_codex_config` / `write_codex_auth` 条目删除，替换为 `build_codex_proxy_config_args`（6 个 `--config` 旗标明细）+ "不写 config.toml/auth.json（key 经 control socket 入 proxy）" 说明；
  - `build_codex_args`（approval 旗标 + extra_args，不含 --model/--config）、`exec_codex`（proxy/subscription 双模式步骤、CODEX_HOME 永不设置）条目改写；
  - 依赖图 fs/dirs 条目、state management `exec_codex` 条目改写；frontmatter + footer 更新。
- `docs/references/codex-home-storage-layout.md`（revision 1 → 2）：
  - brief / Purpose 改写（shared `~/.codex`）；Verified Launch Boundary 重写（proxy 启动 → switch_profile → 6 个 `--config` 旗标 → exec，有效路径 `~/.codex`）；
  - "Observed Per-Profile Files" → "Observed Files Under ~/.codex"；约束条目 "Let launch derive config.toml/auth.json" → "不写 Codex 配置文件"；
  - 新增 `## Session Visibility (resume)` 小节（resume 按 model_provider_id ∩ cwd 过滤；`--all` 绕过 cwd 但关不掉 provider；显式 `resume <session-id>` 绕过全部；同 provider 会话跨 profile 可见）。
- `docs/references/codex-backend-development-guide.md`（revision 1 → 2）：
  - 字段表 `base_url`/`model`/`full_auto` 行更新（--config 旗标 / approval level 语义）；
  - `base_url` env 说明改写（control socket + `model_providers.custom.base_url` 旗标）；
  - Runtime Launch Flow 重写（proxy / subscription 双模式 6 步）；"Generated Codex Config" → "Codex Provider Configuration (inline --config)"（6 旗标明细 + 不再写 config.toml/auth.json）；
  - 新增 `## Session Visibility (resume)` 小节；测试覆盖行 `generate_codex_config()` → `build_codex_proxy_config_args()`。

### Step 21 — 终审（三脚本全 PASS）

- `verify-B011-migration-docs.sh` → PASS
- `verify-B013-doc-cleanup.sh` → PASS
- `verify-B014-interface-frozen.sh` → PASS（`cargo test` 全绿 + CCT_PROXY_PORT/CCT_PROXY_LOG/proxy start|stop/run 接口冻结）
- 补跑 `docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/run-all.sh` → **Total: 15 | Pass: 15 | Fail: 0 | Skip: 0**（B011/B013 从 FAIL 转 PASS 后全量闭环）
- `poc/poc.md` Results Log：B011/B013 状态列更新为 PASS，新增 2026-08-02 15/15/0/0 终态行。

## 备注（未动，预存 drift）

- `docs/references/codex-backend-development-guide.md` "Persisted Full-Auto Toggle" 与 add-form "y/yes → Some(true)" 段描述 bool 时代 full_auto（现为 `ApprovalLevel` 循环，commit a67aeeb 引入，早于本任务）：字段表行已顺带更新，但 TUI/持久化小节未改写，属任务范围外预存 drift。
- install-script.md 迁移小节中 socket 路径按 plan 原文写 `~/.config/cc-tui/proxy.sock`（代码为 `dirs::config_dir()/cc-tui/proxy.sock`，macOS 上解析为 `~/Library/Application Support`）。
- 历史快照（session-cards / procs / context-*）未动。
