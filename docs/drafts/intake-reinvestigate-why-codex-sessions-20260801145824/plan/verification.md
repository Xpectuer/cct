---
title: "Verification: cct proxy 死锁修复 + Codex 会话可见性验证与文档收尾"
doc_type: proc
brief: "Verification strategy, test plan, and self-review checklist"
confidence: verified
yields_from:
  - spec.md
created: 2026-08-01
updated: 2026-08-01
revision: 1
---

# Verification

## Per-Step Verify Commands

| Step | Verify Command | Expected Result |
|------|---------------|-----------------|
| 1 | `cargo test proxy_socket_path` | 新测试 `proxy_socket_path_override` 通过 |
| 2 | `cargo build && cargo test` | 编译通过；现有 7 个 proxy 单测 + 全量绿 |
| 3 | `cargo test`（含新增探测单测） | 不存在路径 → false；应答线程 → true |
| 4 | `cargo build && cargo test` | 编译通过（G2 Step 12 契约后续覆盖） |
| 5 | `cargo test tcp_port_owner` | lsof 缺失场景 → None + 降级建议文本含 "lsof -iTCP" |
| 6 | `cargo build` | 编译通过（G2 Step 11 占端口/双启动契约覆盖） |
| 7 | G2 Step 11 契约 | shutdown 后 socket 文件不存在 |
| 8 | `cargo test mask_sensitive` | switch JSON 与查询串均脱敏为 `sk-***` |
| 9 | G2 Step 11 stop-超时契约 | 无响应 socket → 2s 内 Err，不挂起 |
| 10 | `cargo test --test proxy_contract` | 编译通过 + smoke 测试（stub 收到请求） |
| 11 | `cargo test --test proxy_contract` | 7 个行为契约全绿 |
| 12 | `cargo test --test launch_proxy_contract` | 5 个重启契约全绿（各 ≤2s） |
| 13 | `cargo test --test proxy_contract launch_path_writes_no_codex_config` | 临时 CODEX_HOME 无 config.toml/auth.json/profile-*.config.toml |
| 14 | `cargo test`；`cargo clippy --all-targets`；`bash poc/scripts/verify-B014-interface-frozen.sh`；`bash poc/scripts/verify-B010-contract-tests.sh` | 全绿；无新增警告；B014/B010 PASS |
| 15 | `bash poc/scripts/verify-B012-l2-prereqs.sh`；`cd poc && ./run-all.sh` | B012 PASS；`Total: 15 | Pass: 15 | Fail: 0 | Skip: 0` |
| 16 | `bash poc/scripts/verify-B006-*.sh && verify-B007-*.sh && verify-B008-*.sh` | 三者 PASS（任一 FAIL → 定义 cct bug 追加修复） |
| 17 | `bash poc/scripts/verify-B015-layered-diag.sh`；读 poc.md Results Log | PASS；Results Log 有当日记录 |
| 18 | （可选 MANUAL）TUI 可视化确认 | 用户视觉确认，不阻塞 |
| 19 | `bash poc/scripts/verify-B011-migration-docs.sh` | PASS（迁移说明存在） |
| 20 | `bash poc/scripts/verify-B013-doc-cleanup.sh` | PASS（5 文档零陈旧叙述 + resume 语义说明） |
| 21 | `bash poc/scripts/verify-B011-*.sh && verify-B013-*.sh && verify-B014-*.sh` | 三者全 PASS |

## Test Strategy

**分层**（自底向上，每层防护下一层）：

1. **单元测试**（src/proxy.rs `#[cfg(test)]`，Step 1/3/5/8）：CCT_PROXY_SOCKET 覆盖、探测语义、lsof 降级、脱敏 helper——纯函数级，快速反馈。
2. **契约测试**（tests/proxy_contract.rs + tests/launch_proxy_contract.rs，Step 10-13）：真实 `CARGO_BIN_EXE_cct` 二进制 + 临时 socket（CCT_PROXY_SOCKET）+ 动态端口（CCT_PROXY_PORT）——与用户实例完全隔离；覆盖约束 #1-5、#9、#10、#14 的 12 个行为。env 隔离用 `serial_test`。
3. **L2 实测**（poc/ 脚本，Step 15-17）：真实系统上经 codex exec 非交互链路验证会话可见性（临时 CODEX_HOME + 临时 profiles.toml + stub SSE 上游）；分层诊断（curl --noproxy '*' → codex 对话）定位故障层。
4. **文档断言**（poc/verify-B011/B013/B014，Step 19-21）：grep 驱动，契约语义（无陈旧模式 + 语义说明存在）而非逐字匹配。

**Mock 策略**：外部命令 stub——CCT_PROXY_BIN 注入 fake spawn 目标（仿 CCT_CLAUDE_BIN 先例）；stub 上游（协议无关 HTTP）解耦 proxy 转发正确性与真实上游连通性。codex CLI 本身不 mock（L2 层真实调用，非交互 exec 链路）。

**快照回归**：Step 13 断言启动链路不写任何 Codex 配置文件（临时 CODEX_HOME 前后文件集合一致）——约束 #14。

**基线对比**（约束 #15）：修复前证据在 [refs/proxy-deadlock-diagnosis.md](../refs/proxy-deadlock-diagnosis.md)（curl 超时死锁复现 + 采样）与 session-log（PID 29182 占端口、AC7 文档残留盘点）；4 个只读脚本（B011/B012/B013/B015）修复前 FAIL 已实测，Step 15 补录进 poc.md Results Log；修复后全部转 PASS 即问题闭合证据。

## Self-Review Checklist

| Check | Pass Condition |
|-------|---------------|
| All acceptance criteria covered | 15/15 AC 映射到 Step（见 code-spec.md Step 23 表） |
| Every step has executable Verify | 24 个实现步骤均有命令级 Verify（上表） |
| 契约测试与用户实例隔离 | 全部经 CCT_PROXY_SOCKET 临时路径 + 动态端口；无真实 ~/.codex / 真实 socket 触碰 |
| 接口冻结保持 | CCT_PROXY_PORT / CCT_PROXY_LOG / proxy start\|stop / run 签名与命令不变（B014 断言） |
| 脱敏覆盖所有显示路径 | ctl 命令日志按 api_key 字段名脱敏（任意值形态）+ 请求日志 sk- 值扫描（Step 8，约束 #7）；契约测试 grep 无明文 |
| 无新增 panic 路径 | run_proxy 的 bind/accept 失败均报错退出（Step 6）；契约测试断言 stderr 无 "panic" |
| 超时均收敛 | 探测 500ms×3、stop 2s——无无限等待路径（契约测试断言 ≤2.5s） |
| 文档语义准确 | 5 文档零 per-profile CODEX_HOME / generate_codex_config；resume 过滤语义（model_provider ∩ cwd）说明存在 |
| 历史快照不动 | session-cards / procs / context-* 未列入改动文件（约束 #14 scope） |
| MANUAL 步骤显式 | Step 18（OQ3 TUI 确认，可选）🖐️ MANUAL；Step 15 kill 动作执行前向用户确认 |
| 测试可并行性 | G3（15-17）与 G4（19-20）无 shared files——并行安全 |
| 条件分支有兜底 | B006 实测不符 → 定义为 cct bug 追加修复（Step 16 分支） |
