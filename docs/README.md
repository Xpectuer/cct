---
title: Documentation index
doc_type: index
brief: Canonical navigation map for active and historical project documentation.
updated: 2026-08-20
---
# Documentation Index

Use this page to choose the smallest relevant context set. Current product behavior is defined by the code, [README](../README.md), [architecture](../ARCHITECTURE.md), and active module guides. Timestamped material is historical unless a current document links to it as evidence.

## Active Documentation

| Need | Read |
|------|------|
| Agent workflow and repository map | [Agent guide](../AGENTS.md) |
| User-facing behavior and examples | [README](../README.md) |
| System design and data flow | [Architecture](../ARCHITECTURE.md) |
| Installation | [Install guide](../INSTALL.md) and [agent install prompt](../INSTALL_PROMPT.md) |
| Module boundaries | [Module index](modules/index.md), then the relevant file in `modules/` |
| Engineering constraints | The relevant file in `rules/` |
| Known reusable pitfalls | The relevant file in `lessons/` |
| Project status | [Dashboard](dashboard.md) |
| Quality assessment | [Codebase review](quality/codebase-review.md) |

## Collections

| Directory | Status | Purpose |
|-----------|--------|---------|
| `modules/` | Active | Contracts and ownership for stable source modules |
| `rules/` | Active | Required project-specific implementation rules |
| `sops/` | Active when populated | Repeatable operational procedures; currently contains a template |
| `issues/` | Active when populated | Structured issue records; currently contains a template |
| `lessons/` | Reference | Durable findings that should influence new work |
| `quality/` | Reference | Audits and quality reviews |
| `presentations/` | Reference | Human-facing project presentations |
| `references/` | Reference | External source snapshots, technical notes, patches, and context captures |
| `drafts/` | Historical workflow | Requirements, plans, and design sessions |
| `procs/` | Historical workflow | Execution logs and progress records |
| `handoff/` | Historical | Point-in-time handoff notes |
| `failures/` | Historical | Recorded failed runs and diagnoses |
| `session-cards/` | Archive | Generated session summaries; do not use as current specification |

## High-Value References

- [Codex backend development guide](references/codex-backend-development-guide.md)
- [Codex storage layout](references/codex-home-storage-layout.md)
- [Install script reference](references/install-script.md)
- [Context snapshot index](references/index.md)
- [Legacy Kimi backend patch](references/kimi-backend.patch)
- [Internal sharing slides](presentations/cct-sharing-slides.html)

`references/cc-switch/` is an imported external codebase used only for comparison. Search it deliberately; do not treat its conventions or generated files as cct project rules.

## Maintenance

- Add new docs to the narrowest matching collection and link them here.
- Keep active guidance timeless where possible; put session-specific investigation in `drafts/` or `procs/`.
- Prefer one canonical explanation with links over copying the same instructions into several files.
- Do not rewrite historical records merely to match current behavior.
