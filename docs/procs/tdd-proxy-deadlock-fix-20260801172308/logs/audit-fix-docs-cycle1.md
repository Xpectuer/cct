---
title: "audit-fix-docs — Cycle 1"
brief: "文档组保真度审计 cycle 1 修复：install-script.md socket 路径双平台措辞 + codex guide full_auto 正文与表对齐 + poc.md kill 29182 确认记录"
doc_type: proc
created: 2026-08-02T00:00:00+0800
case: audit-fix-docs
phase: audit-fix
---

## 修复日志: 文档组 — Cycle 1

依据: `findings/audit-fidelity-cycle1.md` 偏离 3（AC-11 install-script.md socket 路径
macOS 误导）、偏离 5（AC-13 范围内 full_auto 表/正文不一致残余）与偏离 4（AC-12 kill
29182 身份确认记录缺失）。修复范围: 仅 3 个文档文件——`docs/references/install-script.md`、
`docs/references/codex-backend-development-guide.md`、
`docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/poc.md`。
不动 src/、tests/、poc/scripts/。

### 偏离 C1: install-script.md 迁移小节 socket 路径 macOS 误导

- **修复文件**: `docs/references/install-script.md`（迁移小节第 2 项）
- **修复内容**:
  - **before**: `遗留 socket 文件（`~/.config/cc-tui/proxy.sock`）→ 可手动删除兜底；…`
    ——只写 Linux 路径，本机 macOS 用户（`dirs::config_dir()` → `~/Library/Application
    Support`）按此路径找不到文件。
  - **after**: `遗留 socket 文件（`~/.config/cc-tui/proxy.sock`，Linux；
    `~/Library/Application Support/cc-tui/proxy.sock`，macOS）→ 可手动删除兜底；…`
    ——双平台措辞，保留 plan 语义（遗留 socket 文件位置）与迁移三要素
    （29182/手动终止/遗留 socket）不变。先核对了 verify-B011-migration-docs.sh 的断言
    （仅 `29182|手动终止|遗留.*socket|死锁实例`，不含路径），改后断言仍命中。
- **验证**: `bash scripts/verify-B011-migration-docs.sh` → `[PASS] B011:
  install-script.md 含旧实例迁移说明`，exit 0。

### 偏离 C2: codex-backend-development-guide.md full_auto 表/正文不一致

- **修复文件**: `docs/references/codex-backend-development-guide.md`
- **修复内容**: 字段表（guide:39）已更新为 `ApprovalLevel`（`untrusted`/`never`/`danger`
  循环语义），正文 6 处仍描述 bool 时代 full_auto —— 表格新、正文旧，歧义最大。按修复
  建议选项 (b) 将正文与表对齐为当前语义（以 src/config.rs `ApprovalLevel` 循环 +
  `deserialize_approval` 向后兼容、src/app.rs `field_labels`/表单映射、src/ui.rs 渲染、
  src/launch.rs 旗标映射为据）：
  - **:66** before: `full_auto` is written as a profile-level **boolean** when present
    → after: 写为 profile-level **string**（`untrusted`/`never`/`danger`），并注明遗留
    bool 仍可反序列化（`true` → `danger`，`false` → unset）。
  - **:82 示例** before: `full_auto = true` → after: `full_auto = "never"`。
  - **:101-103 表单标签** before: `["Name *", "Base URL", "API Key", "Model",
    "Full Auto (y/n)"]` → after: `["Name *", "Base URL", "API Key", "Model",
    "Approval"]`（与 src/app.rs `field_labels` 一致）。
  - **:113 字段索引表** before: `| 4 | Full Auto (y/n) | full_auto |` → after:
    `| 4 | Approval | full_auto |`。
  - **:118-119 表单取值** before: `"y"`/`"yes"` → `Some(true)`，其他 → `Some(false)`
    → after: `"untrusted"`/`"never"`/`"danger"` 映射对应级别；`"y"`/`"yes"` 映射
    `danger`（向后兼容，与 src/app.rs 一致）；其他 → `None`。
  - **:140-141 TUI 行为** before: `full_auto = true` 行渲染为黄色、详情面板显示
    `full_auto: ✓` → after: 行按级别着色 `untrusted` 绿 → `never` 黄 → `danger` 红
    （unset 白）；详情面板显示 `approval: <level>`（经 `approval_label`，unset 为
    `approval: on-request`，与 src/config.rs `approval_label` 一致）。
  - 未动的准确叙述（保持不动）: 字段表 :39、`cct add` CLI 恒 `full_auto = None` :127、
    :176 旗标映射、:202/:206/:210 `s` 键 toggle 与 `toml_edit` 持久化、:240 测试边界。
    范围限定为表/正文内部一致性，未做 AC13 范围外的扩展清理（其余预存 drift 由审计
    接受为范围外）。
- **验证**: `bash scripts/verify-B013-doc-cleanup.sh` → `[PASS] B013: 5 份文档无陈旧
  叙述, resume 过滤语义已说明`，exit 0（改动未引入 `generate_codex_config` /
  per-profile CODEX_HOME 陈旧叙述，resume 过滤语义行未受影响）。

### 偏离 C3: kill 29182 身份确认记录缺失

- **修复文件**: `docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/poc.md`
  （Results Log 基线行 Notes）
- **修复内容**: plan Step 15 要求"先 `ps -p 29182` 确认仍是旧版 cct proxy 实例再执行"；
  迁移时 29182 已不存在（无需 kill），但执行记录未说明该确认步骤。surgical 追加到基线行
  Notes：
  - **before**: `修复前基线（只读脚本 B011/B012/B013/B015 FAIL；证据
    refs/proxy-deadlock-diagnosis.md + session-log）`
  - **after**: 上述文字后追加 `。迁移前置（plan Step 15）确认记录：`ps -p 29182`
    显示旧版 cct proxy 实例 29182 已不存在、端口 19191 空闲 → 无需 kill，直接继续迁移
    （用户已确认）`。事实核对自 logs/run_all_full_pass-green.md（B012: 29182 已终止 +
    端口 19191 空闲）与 logs/double_start_race_one_wins-refactor-verify.md（29182 为
    用户 ~/.local/bin/cct 实例）。
- **验证**: 文档改动不触碰任何 verify 脚本断言文本；B011/B013 复跑均 PASS（exit 0）。

### 结果

三处偏离全部修复；B011、B013 各自独立运行 PASS（exit 0），输出见上。无 src/、
tests/、poc/scripts/ 改动。
