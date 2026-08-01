# Verifier Report: Consistency (Angle 2/7)

## Score: 7/10
## Verdict: PASS

## Findings

### ADVISORY: AC7 目标文档清单在 spec 内部自相矛盾（Terminology 含 README，AC7 准则含 ARCHITECTURE.md）
- Location: spec.md:Terminology AC7 行 vs AC 7
- Evidence: 两份清单互不包含对方文件；session-log 访谈盘点为权威（launch.md/CLAUDE.md/ARCHITECTURE.md/codex-home-storage-layout.md/codex-backend-development-guide.md，无 README）；实仓核查 README 0 处 CODEX_HOME、ARCHITECTURE.md 有 1 处
- Fix: 以 session-log 为准统一；修正 requirements.md §5 的 README 表述

### ADVISORY: AC6 "报错/复用退出" 与 Solution Summary、访谈记录"报错退出"不一致
- Location: spec.md:AC 6
- Fix: 按两条路径拆分——手动 `cct proxy start` → 报错退出；`ensure_proxy_running` 自动路径 → 探测成功即复用返回成功

### ADVISORY: AC8 后半句"不改变 CCT_PROXY_PORT/CCT_PROXY_LOG/start|stop 接口"为孤儿约束（无决策出处）
- Location: spec.md:AC 8
- Fix: 补记 decisions.md 或降级为 Solution Summary 叙述

### ADVISORY: spec Decisions 摘要声称"完整决策记录见 decisions.md"，但 decisions.md 仅收录 4/7 条
- Location: spec.md:Decisions vs decisions.md
- Fix: 补 3 条缺失决策入 decisions.md（与 completeness ADVISORY 1 相同）

### ADVISORY: Terminology 来源标注 "[interview] Step 4" 在 session-log.md 中无对应章节
- Location: spec.md:Terminology
- Fix: 改为可定位引用（如 [debate] [coverage] [ux]）

### ADVISORY: "codex exec 链路"称与 TUI "共用同一过滤逻辑"表述不精确
- Location: spec.md:Terminology
- Evidence: 两条独立代码路径——TUI 用 ProviderFilter::MatchDefault（resume_picker.rs），exec 用 resume_lookup_model_providers（exec/src/lib.rs），语义等价非同一代码
- Fix: 改为"与 TUI picker 的过滤语义一致（源码取证：exec 用 resume_lookup_model_providers，TUI 用 ProviderFilter::MatchDefault）"

## 已验证通过项
- C1 内部矛盾：Part A 机制、可见性公式、端口/函数名/环境变量均与代码一致 ✓
- C2 孤儿决策：spec 7 条决策全部可追溯；decisions.md 4 条全部在 spec 中 ✓
- C3 范围漂移：无 requirements Out of Scope 项 ✓
- C4 yields_from：术语与不变量一致，未违反硬边界 ✓
- C5 AC↔US 对应完整 ✓
