---
title: "PoC Matrix: cct proxy 死锁修复 + Codex 会话可见性验证与文档收尾"
doc_type: proc
brief: "Verification matrix mapping spec behaviors to PoC scripts"
confidence: speculative
yields_from:
  - spec.md
created: 2026-08-01
updated: 2026-08-01
revision: 1
---

# PoC Verification Matrix

**Spec**: [../spec.md](../spec.md)
**Generated**: 2026-08-01

## How to Use

1. Copy `config.env.example` to `config.env` and fill in connection details
2. Run `./run-all.sh` to execute all PoC scripts
3. Review this matrix for the behavior-to-script mapping

> **修复前运行说明**：本 spec 的 Part A（proxy 死锁修复）尚未实现。多数脚本按
> 修复后语义断言——修复前运行预期 FAIL，这些 FAIL 正是问题基线的证据
> （死锁、panic、日志明文、文档陈旧叙述均可复现）。B012 前置条件不满足时
> 会阻塞冒烟类脚本（B001-B009 与真实端口/socket 交互）。

## Verification Matrix

| # | Behavior | AC # | [Smoke] | System | Script | Expected |
|---|----------|------|---------|--------|--------|----------|
| B001 | 并发控制命令 + HTTP 请求不挂起（死锁回归） | 1 | Y | api | verify-B001-concurrent-http.sh | curl 3s 内得到响应（修复前: 超时 FAIL = 死锁复现） |
| B002 | 僵尸死 proxy（进程退出/socket 残留/端口空闲）自愈重启 | 2 | Y | process | verify-B002-zombie-recovery.sh | 重新启动后应用层探测成功（修复前: 10s 无响应 FAIL） |
| B003 | 进程存活占端口时启动报错 + lsof 诊断, 不 panic 不 kill | 3 | N | process | verify-B003-port-occupied.sh | 非 0 退出 + 报错含占用信息 + 占用者存活（修复前: panic FAIL） |
| B004 | cct→proxy→stub 整条链路: SSE 流式返回 + Bearer key 转发 | 4 | Y | api | verify-B004-stub-forwarding.sh | -o 文件含 stub 末尾文本 + stub.log 含 Bearer key |
| B005 | CCT_PROXY_LOG 日志不含 api_key 明文 | 5 | N | filesystem | verify-B005-log-masking.sh | 日志 grep 无测试 key 明文（修复前: 明文 FAIL） |
| B006 | 同 provider（proxy/custom）会话 resume --last 互相可见 | 6 | Y | cli | verify-B006-same-provider-visible.sh | profile B 的 -o 输出含 profile A 会话的末尾文本 |
| B007 | 跨 provider 不可见; 显式 `codex exec resume <id>` 可恢复 | 7 | Y | cli | verify-B007-cross-provider-invisible.sh | 跨 provider -o 不含标记; 显式 resume 含标记 |
| B008 | cwd 过滤: 跨仓库默认不可见, 追加 --all 可见 | 8 | Y | cli | verify-B008-cwd-filter.sh | repo2 默认无标记; --all 后含标记 |
| B009 | 活 proxy 时手动 `cct proxy start` 报错退出, 不删 socket 不 panic | 9 | N | process | verify-B009-double-start.sh | 非 0 退出 + socket 完好 + 原 proxy 仍响应 |
| B010 | 契约测试覆盖（并发/僵尸/占端口/双启动竞态/转发/脱敏/stop 超时） | 10 | N | cli | verify-B010-contract-tests.sh | cargo test proxy + integration 全绿 |
| B011 | install-script.md 含旧实例迁移说明（一次性升级指引） | 11 | N | filesystem | verify-B011-migration-docs.sh | 文档含 29182/手动终止/遗留 socket 说明（当前: PASS，TC-23） |
| B012 | L2 冒烟前置: 旧实例 29182 已终止 + 端口空闲 | 12 | N | process | verify-B012-l2-prereqs.sh | 29182 不运行 + 端口无占用（当前: 可能 FAIL = 提示先迁移） |
| B013 | 5 份文档无 per-profile CODEX_HOME / generate_codex_config 陈旧叙述 + resume 过滤语义说明 | 13 | N | filesystem | verify-B013-doc-cleanup.sh | 5 文档零陈旧叙述 + 语义说明存在（当前: PASS，TC-23） |
| B014 | 不写 Codex 配置（快照回归）+ 既有接口不变 | 14 | N | cli | verify-B014-interface-frozen.sh | 回归测试绿 + CCT_PROXY_PORT/LOG + 命令接口仍在 |
| B015 | 分层诊断: curl --noproxy '*' 验证 proxy 层存活 | 15 | N | network | verify-B015-layered-diag.sh | 任意响应码（502/404 亦存活）; 超时 = 死锁 FAIL; 无监听 SKIP |

## Connection Requirements

| 系统 | 说明 | 脚本 |
|------|------|------|
| cct binary | `cargo build` 产出（config.env: `CCT_BIN`） | 全部 |
| 本地 TCP 端口 | proxy 监听（config.env: `PROXY_PORT`, 默认 19191） | B001-B009, B012, B015 |
| Unix 控制 socket | `CCT_PROXY_SOCKET` 覆盖（修复后生效）; 当前版本固定真实路径, 冒烟前 B012 预检 | B001-B005, B009 |
| stub 上游 | 本地 python3 HTTP 服务（config.env: `STUB_PORT`, 默认 19200） | B004, B006-B008 |
| codex CLI | 非交互 `exec` 链路（无 tty 可用） | B004, B006-B008 |
| 临时 CODEX_HOME | 脚本自动 mktemp; 不碰真实 `~/.codex` | B006-B008 |
| 真实系统旧实例 | 用户本机旧版死锁实例 PID 29182 | B012（预检） |

## Manual Checks

| Behavior | AC # | Reason |
|----------|------|--------|
| 旧版死锁实例若控制 socket 仍响应, 新版本探测视为健康并复用（HTTP 仍挂起）→ 唯一修复路径是用户手动终止 | 11 | 需真实旧版本进程; 脚本无法制造此状态。B012 的前置检查负责提示迁移 |
| TUI picker 手动 `codex resume` 可视化确认 | OQ3 | agent 无法操作 TUI; 用户自愿, 不阻塞 AC |

## Results Log

| Date | Total | Pass | Fail | Skip | Notes |
|------|-------|------|------|------|-------|
| — | — | — | — | — | Run `./run-all.sh` to populate |
| 2026-08-01（修复前） | 15 | 11 | 4 | 0 | 修复前基线（只读脚本 B011/B012/B013/B015 FAIL；证据 refs/proxy-deadlock-diagnosis.md + session-log）。迁移前置（plan Step 15）确认记录：`ps -p 29182` 显示旧版 cct proxy 实例 29182 已不存在、端口 19191 空闲 → 无需 kill，直接继续迁移（用户已确认） |
| 2026-08-01 21:31（修复后全量，run 1） | 15 | 3 | 11 | 1 | config.env 未创建 → B001-B009 环境性 FAIL（CCT_BIN/TEST_API_KEY 缺失）；B010/B012/B014 PASS；B011/B013 文档缺口（TC-23 范围）；B015 SKIP（无 proxy 监听）。run-all.sh 首个完整输出 |
| 2026-08-01 21:33（修复后全量，run 2，config.env 已建） | 15 | — | — | — | run-all.sh 卡死 B001 未完成：verify-B001 第 36 行 `wait`（无参）等待全部后台任务含长驻 proxy daemon → 无限挂起（脚本 harness bug，与修复无关）；已 SIGTERM 终止 |
| 2026-08-02（修复后单脚本 + 人工等价探针） | 15 | — | — | — | B001 人工探针 PASS（<3s 响应）；B002/B009(隔离)/B010/B012/B014/B015 PASS；B003/B005 行为正确但退出码被 EXIT trap `kill ""`（set -e 下）覆写为 1；B004/B006/B007/B008 FAIL 根因：codex 0.146 需 item-based SSE（`response.output_item.added`），stub 缺该事件 → "OutputTextDelta without active item"（证据 /tmp/codex-direct/codex.log）；B011/B013 文档缺口（TC-23） |
| 2026-08-02（修复后全量，harness 修复后 run-all） | 15 | 13 | 2 | 0 | 修复后全量：4 类 harness 修复（B001 无参 `wait` 挂起 / EXIT trap `kill ""` 退出码覆写 / stub SSE 改 item-based / daemon 生命周期清理）后 run-all.sh 完整跑完不再挂起；B001-B010、B012、B014、B015 全 PASS，Skip 0；Fail 2 = B011/B013（TC-23 文档收尾范围，预期内）。输出留存 /tmp/run-all-fix1.log |
| 2026-08-02（修复后逐脚本单独运行） | 15 | 13 | 2 | 0 | 逐脚本串行各自独立运行 PASS（exit 0）：B001 0.18s HTTP 502 不挂起；B002 死 socket 自愈；B003 报错诊断且占用者存活；B004 codex `-o` 非空 + Bearer 转发；B005 脱敏断言真实生效；B006/B007/B008 可见性三查；B009 双启动报错且原 proxy 完好；B010/B014 契约测试全绿；B012 预检通过；B015 自起实例 proxy 层存活 HTTP 502；B011/B013 FAIL（TC-23）。详见 docs/procs/tdd-proxy-deadlock-fix-20260801172308/logs/run_all_full_pass-fix-attempt-1.md |
| 2026-08-02（doc_cleanup_final 后全量 run-all） | 15 | 15 | 0 | 0 | TC-23 文档收尾完成（B011 迁移说明 + B013 五文档清理 + resume 语义）后 run-all.sh 全量闭环：B001-B015 全 PASS、Skip 0、Fail 0。证据 logs/doc_cleanup_final-green.md |
| 2026-08-02（审计修复后全量 run-all） | 15 | 15 | 0 | 0 | 审计修复后确认：B006 断言改为 session-id 对比 + rollout 复用计数（spec AC-6），B007 跨 provider 不可见改 rollout 计数 + id 级核对、显式恢复 6 旗标改经 `cct run` 真实函数生成（spec AC-7）——两断言在错误实现下可 FAIL（可证伪）。run-all.sh 15/15/0/0。完整原始输出 logs/run_all_full_pass-audit-fix1.md |
