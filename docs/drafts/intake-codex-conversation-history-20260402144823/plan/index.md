---
title: "Plan Index: Codex history shared across profiles"
doc_type: proc
brief: "Implementation plan: shared CODEX_HOME + --profile overlay + two-way binding + migration"
confidence: verified
yields_from:
  - spec.md
created: 2026-08-01
updated: 2026-08-01
revision: 1
---

# Plan Index

## How to Use

Each aspect file covers one dimension of the plan. Load only what you need
for the current task — consult the **When to Use** column to decide which
files to load.

**Typical loading patterns**:
- **Before coding**: load [architecture.md](architecture.md) + [constraints.md](constraints.md)
- **During coding**: load [code-spec.md](code-spec.md), reference others as needed
- **Domain confusion**: load [domain-knowledge.md](domain-knowledge.md)
- **After coding**: load [verification.md](verification.md)

## Aspect Files

| File | Description | When to Use |
|------|-------------|-------------|
| [constraints.md](constraints.md) | Acceptance criteria（7 条，可验证检查）+ hard constraints（14 条：scope/tech/compat/behavior） | **Always** — 所有实现决策的权威依据。先读。 |
| [domain-knowledge.md](domain-knowledge.md) | 领域实体（CodexLayout、overlay、KeyDiff…）、术语表（避免 `[profiles.*]` 混淆）、业务规则（双向绑定、迁移、exec 无返回） | **领域概念不清楚时** — 查实体定义、术语、业务不变量；代码评审时校验术语一致。 |
| [architecture.md](architecture.md) | 策略 + Files Changed（10 文件）+ 依赖 DAG（A→B→C 三组）+ 执行顺序 | **编码前** — 理解受影响文件、分组关系与工作顺序。 |
| [code-spec.md](code-spec.md) | 18 步实现指令（old/new/verify 锚点）、终态步骤（proof-read/cross-check/review/commit）、Execution Order YAML | **编码中** — 按执行顺序逐步执行；每步有可执行 Verify。 |
| [verification.md](verification.md) | 每步 verify 命令表、测试策略（纯/tempdir/契约/TUI）、自审清单 10 项 | **编码后** — 跑验证命令、执行测试策略、完成自审清单后提交。 |
