# Verifier Report: Test Depth (Angle 6/7) — Round 3

## Score: 9/10
## Verdict: PASS

## Findings

### ADVISORY: AC 4-6 的 resume 腿仍写"以 profile B 等价 --config 旗标运行 codex exec resume --last"，可读为手工复刻 6 旗标
- Location: spec.md:AC 4/5/6 when 子句
- Fix: AC 4-6 when 子句改为与 AC 3 同构——经 `cct run <profile>`（extra_args 嵌入 `exec resume --last -o <out>`）运行，或注明"等价 --config 旗标"由 cct run 真实函数生成、禁止手工复刻

### ADVISORY: 冒烟 profile 的 full_auto 取值未指定，与 extra_args 内嵌 bypass 旗标存在重复风险
- Location: spec.md:Solution Summary Part B 与 AC 3
- Evidence: build_shared_codex_args 先追加 full_auto 批准旗标再追加 extra_args；若冒烟 profile 设 full_auto=danger 会重复出现 bypass 旗标
- Fix: 冒烟 profile 不设 full_auto，非交互批准由 extra_args 内嵌 bypass 旗标单点承担

## 闭合核查（round-2 → rev 3）
- BLOCKER（无 tty 裸 codex）→ 闭合（代码证据：extra_args 追加位置 launch.rs:240-241/199 在 --config 之后；cct run 既有子命令；exec 无 tty 要求；SSE stub 确定性化）✓
- ADVISORY 1（fake 目标绑定临时 socket）→ 闭合（CCT_PROXY_SOCKET 入 Terminology + Solution Summary + AC）✓
- ADVISORY 2（旗标 drift）→ 部分闭合（AC 3 已落实，AC 4-6 措辞残留 → 本轮修复）✓
- ADVISORY 3（smoke 前置条件）→ 闭合 ✓

## L1/L2 分层复核
- L1：契约测试 9 场景 + 配置快照对比，mock 边界显式 ✓
- L2：临时 CODEX_HOME/profiles.toml/stub 上游/真实 codex 0.146.0，AC 3 全链路 + AC 4-6 真实 resume 场景与负例 + 分层诊断 ✓

无 INTERVIEW_NEEDED。
