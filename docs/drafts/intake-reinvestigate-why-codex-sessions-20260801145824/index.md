---
title: "Intake: Codex conversation history isolation re-evaluation"
doc_type: reference
brief: "Directory index for intake-20260801145824"
confidence: verified
created: 2026-08-01
updated: 2026-08-01
revision: 1
---

# Intake Directory Index

## Mandatory (downstream skills MUST load)

| File | Purpose | When to Read |
|------|---------|--------------|
| [requirements.md](./requirements.md) | Problem statement, user stories, acceptance criteria | Start here for the problem/solution summary |
| [terminology.md](./terminology.md) | Canonical domain glossary — terms, definitions, and _Avoid_ lists | Before any design discussion to avoid synonym confusion |
| [domain-model.md](./domain-model.md) | Entities, relationships, and domain invariants | To understand the problem space structure before designing solutions |

## Optional (load when relevant)

| File | Purpose | When to Read |
|------|---------|--------------|
| [constraints.md](./constraints.md) | Expanded tech stack, hard boundaries, relevant rules | When you need detailed constraints beyond the requirements summary |
| [session-log.md](./session-log.md) | Audit trail of how this intake was produced — **含 [UNCERTAIN] 清单（隔离现象场景待用户确认）** | 进入 debate 前必读；确认 UNCERTAIN 项 |
| [refs/](./refs/) | issues.md（官方文档取证）、**codex-resume-filtering-source.md（源码取证：provider 过滤根因）**、git-history.md、prev-* 先前流程、rules/references 符号链接 | 需要官方机制原文、源码证据或先前 spec/review 深层上下文时 |

## Downstream Consumption

1. **Read this index** to discover available context
2. **Load all mandatory files** — these establish the shared vocabulary and problem understanding
3. **Optionally load context files** — pull in constraints, session logs, or refs as needed for deeper context
