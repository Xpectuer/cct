---
title: "Plan: Codex conversation history shared across profiles"
doc_type: proc
brief: "Pointer to plan/ directory — revision 2 design (shared CODEX_HOME + official --profile)"
confidence: verified
created: 2026-04-02
updated: 2026-08-01
revision: 2
---

# Plan

本文件历史上是 revision 1 设计（per-profile `CODEX_HOME` + 符号链接共享历史 artifact），
已在 2026-08-01 被 revision 2 取代并否决（依据：`state_5.sqlite.threads.rollout_path`
绝对路径与符号链接不兼容、SQLite WAL 多 home 不安全、artifact 列表不可穷尽、
Windows 符号链接权限；详见 [spec.md](spec.md) 的 Alternatives 部分）。

当前权威实现计划在 **`plan/` 目录**：

- [plan/index.md](plan/index.md) — 入口，索引全部 aspect 文件
- [plan/constraints.md](plan/constraints.md) — 验收标准 + 硬约束
- [plan/code-spec.md](plan/code-spec.md) — 18 步实现指令与执行 DAG

历史 revision 1 内容保留在 git 历史中。
