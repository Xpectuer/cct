---
title: "Plan Index: cct proxy 死锁修复 + Codex 会话可见性验证与文档收尾"
doc_type: proc
brief: "Plan for Part A proxy 死锁修复 + Part B 会话可见性验证与文档收尾"
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
| [constraints.md](constraints.md) | 15 条 Acceptance Criteria（可验证检查）+ 15 条硬约束（scope/tech/compat/security/baseline），从 spec 提取 | **Always** — 所有实现决策的权威依据。任何编码前先加载；约束锁定后不再回看 spec 重推 |
| [domain-knowledge.md](domain-knowledge.md) | 领域实体（cct proxy / 僵尸 / model_provider / resume / stub 上游）、术语表（含 Avoid 词）、业务规则（生命周期/启动顺序/socket 清理责任/测试隔离） | **领域概念不清时** — 查正确术语、实体定义、业务逻辑；代码审查时对照术语一致性 |
| [architecture.md](architecture.md) | 4 组依赖式策略（G1 修复→G2 契约→G3 实测→G4 文档）、Files Changed 表（9 文件）、执行顺序 DAG 概览 | **Before coding** — 理解改动文件、组间关系与顺序 |
| [code-spec.md](code-spec.md) | 24 步实现（4 组 + 终端步骤），每步 old/new/verify 锚点；Execution Order YAML（authoritative DAG，25 节点）；MANUAL 标记（Step 15 kill 授权、Step 18 OQ3 可选） | **During coding** — 按执行顺序逐步骤实现，每步以 Verify 命令确认 |
| [verification.md](verification.md) | 每步 Verify 命令表、4 层测试策略（单元/契约/L2 实测/文档断言）、快照回归、12 项自审清单 | **After coding** — 跑验证命令、执行测试策略、完成审查清单后提交 |
| [poc/](../poc/poc.md) | PoC 验证矩阵（15 行为→15 脚本）+ 修复前基线 FAIL 证据 + 连接需求 | **实现前与实现后** — 基线（修复前 FAIL）与闭合（修复后 PASS）对比；修复前已复现死锁（B015） |

## Custom Aspect Files

| File | Description | When to Use |
|------|-------------|-------------|
| [poc/](../poc/poc.md) | 由 lb-dev:poc 技能生成的验证矩阵与 15 个真实系统脚本（B001-B015），映射 spec 行为 | **运行验证时** — `cd poc && ./run-all.sh`；实现前基线、实现后闭合证据 |

## Spec Status Reference

- Spec revision 4（session-log.md status: active；spec.md 无 status 字段，以 session-log 为准）→ 本 plan 生成后待 `/lb-dev:confirm` 收尾置 ready
- 4 个 Open Questions：issue #9 关闭时机（用户决定）、`--last` 输出（语义契约断言）、TUI picker（OQ3，已入 plan Step 18 可选 MANUAL）、Clash 代理（非本 spec 范围）
