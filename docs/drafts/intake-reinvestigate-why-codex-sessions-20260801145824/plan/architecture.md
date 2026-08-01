---
title: "Architecture: cct proxy 死锁修复 + Codex 会话可见性验证与文档收尾"
doc_type: proc
brief: "File layout, dependency DAG, and execution order"
confidence: verified
yields_from:
  - spec.md
created: 2026-08-01
updated: 2026-08-01
revision: 1
---

# Architecture

**Strategy**: 4 组依赖式实施 — Proxy 修复（代码）→ 契约测试（防护网）→ L2 实测（行为证据）→ 文档收尾（平行于测试）。每组按锁定的 constraints 独立生成，组间只传产物清单。

## Files Changed

| File | Change Type | Group |
|------|-------------|-------|
| src/proxy.rs | Major edit（异步 accept、探测/重启/报错逻辑、脱敏、CCT_PROXY_SOCKET、shutdown 清理） | G1 Proxy 修复 |
| src/launch.rs | Major edit（ensure_proxy_running 重写：探测→bind 判定→spawn→就绪；CCT_PROXY_BIN 注入；check_proxy_running 应用层化） | G1 Proxy 修复 |
| src/main.rs | Minor edit（proxy 相关命令路径若引用 check_proxy_running 签名变化则适配） | G1 Proxy 修复 |
| src/ui.rs | 无改动（脱敏复用 ui::mask_value 或仅内部引用，不跨模块） | — |
| tests/proxy_contract.rs（新）或 src/proxy.rs 内 #[cfg(test)] | New file（契约测试：并发/僵尸/占端口/双启动/stub 转发/脱敏/stop 超时） | G2 契约测试 |
| tests/launch_proxy_contract.rs（新） | New file（launch 层重启契约：CCT_PROXY_BIN fake + CCT_PROXY_SOCKET） | G2 契约测试 |
| tests/ 或 poc/scripts/ 下 stub 工具 | New file（契约测试用 stub 上游；L2 smoke 用 SSE stub 已存在于 poc/scripts/stub-sse-upstream.py） | G2 契约测试 |
| poc/scripts/setup-smoke.sh（已存在） | Minor edit（若契约测试先行后 L2 链路需微调） | G3 L2 实测 |
| docs/references/install-script.md | Minor edit（AC11 迁移说明段落） | G4 文档收尾 |
| CLAUDE.md / ARCHITECTURE.md / docs/modules/launch.md / docs/references/codex-home-storage-layout.md / docs/references/codex-backend-development-guide.md | Major edit（AC13：消除陈旧叙述 + 新增 resume 过滤语义） | G4 文档收尾 |
| 配置快照回归测试（tests/ 内） | New file（AC14：断言不写 Codex 配置文件） | G2 契约测试 |

## Execution Order (overview)

```
G1 Proxy 修复 ──> G2 契约测试 ──> G3 L2 实测
                                    │
G4 文档收尾 ◄──（可平行于 G2/G3 启动；仅依赖 G1 完成后的行为事实）──┘
```

- **G1 → G2**：契约测试断言 G1 的行为（并发响应、自愈、报错、脱敏、stop 超时、重启注入）。
- **G2 → G3**：L2 smoke 依赖契约测试证明的行为基线；smoke 脚本已在 poc/ 存在，G3 是执行 + 结果记录 + 按需微调。
- **G4 平行**：文档改动只依赖 G1 的实现事实（CCT_PROXY_SOCKET 行为、shutdown 清理、探测语义），不依赖测试结果；可与 G2/G3 并行，但在 G3 结果落定后收尾（若实测发现 cct bug 追加修复，文档需同步）。

## Grouping Strategy

按**依赖链**（测试/文档依赖修复后的行为）而非架构层划分：每组产出是下一个组的输入，组内上下文隔离。
