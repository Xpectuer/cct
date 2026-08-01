---
title: "run_all_full_pass — Green Phase"
brief: "run_all_full_pass — blocked（门未过：run-all 卡死 B001 + 脚本 harness bug + codex↔stub 契约不匹配）"
doc_type: proc
created: 2026-08-02T01:00:52+0800
case: "run_all_full_pass"
phase: blocked
---

# run_all_full_pass — 执行记录（门未过，详见下）

## 结论速览

- Exit code: 1（run-all.sh run 1）；run 2 卡死 B001（SIGTERM 143，未产出计数）
- clippy: 已修复 `clippy::trim_split_whitespace`（`tests/proxy_contract.rs:82` 删去冗余 `.trim()`），`cargo clippy --all-targets` exit 0，无新增警告
- B012: PASS（29182 已终止 + 端口 19191 空闲；首次运行即 PASS）
- run-all: run 1 = 15/3/11/1；run 2 = 卡死未完成
- B001/B002/B003/B005/B009 转 PASS 确认: **否**（run-all 层面未实现；见下方逐项证据）
- poc.md Results Log: 已追加 4 行（基线 + run 1 + run 2 + 单脚本/人工探针），surgical 追加未动其它内容

## 门判定

`run-all.sh` 未输出 `Total: 15 | Pass: 15 | Fail: 0 | Skip: 0`，TC-20 门**未通过**。
按要求未修复任何门失败项（仅执行了任务指定的 clippy 修复与文档化 setup：创建 gitignore 的 `config.env`）。

## 迁移前置（任务 2）

`bash poc/scripts/verify-B012-l2-prereqs.sh` → PASS：
- [OK] 旧实例 PID 29182 未在运行
- [OK] 端口 19191 空闲

## run 1（无 config.env，2026-08-01 21:31）

`./run-all.sh` 输出：`Total: 15 | Pass: 3 | Fail: 11 | Skip: 1`（exit 1）。
- B001-B009 FAIL 均为环境性：`CCT_BIN: Must be set in config.env` / `TEST_API_KEY: Must be set in config.env`
  —— poc.md "How to Use" 第 1 步（复制 config.env.example → config.env，gitignore 文件）未执行，属文档化 setup 缺失，非产品失败。
- B010 PASS、B012 PASS、B014 PASS（真实 exit 0）。
- B011 FAIL、B013 FAIL：真实文档缺口（install-script.md 迁移说明缺失 / 5 文档陈旧叙述 + resume 过滤语义）——按 tdd.md 计划属 TC-23（Step 19-21）范围，TC-20 预期内不通过。
- B015 SKIP（无 proxy 监听）——按脚本设计（须先启动 proxy）在 run-all 单次序列中恒 SKIP。

## run 2（config.env 已建，2026-08-01 21:33）

卡死 B001，10+ 分钟无输出，SIGTERM 终止（exit 143）。进程树证据：
`run-all.sh → verify-B001-concurrent-http.sh → cct proxy start`，nc/curl 均已结束（各自有界），脚本卡在 `wait`（第 36 行）。
**根因（脚本 harness bug，与修复无关）**：`verify-B001-concurrent-http.sh:36` 无参 `wait` 等待全部后台任务，含 `&` 启动的长驻 proxy daemon（永不退出）→ 无限挂起。修复前同样会挂（curl 3s 超时后 `wait` 仍等 daemon）。

## 单脚本 + 人工等价探针（config.env 已建，有界运行）

| 脚本 | run-all 计数 | 真实结果 | 说明 |
|------|-------------|---------|------|
| B001 | （挂起） | 行为 PASS | 脚本挂起（见上）；人工等价探针（同步骤、去 `wait`）：并发 5×status + curl `--max-time 3` → RC=0、HTTP=502（无上游时为预期存活信号）→ **死锁修复端到端确认** |
| B002 | — | PASS (rc=0) | 僵尸自愈：死 socket 被清理、proxy 自愈并响应 |
| B003 | — | 行为 PASS / rc=1 | 打印 PASS 但退出码被 EXIT trap 覆写为 1（见"harness bug 说明"）→ run-all 误计 FAIL。行为断言全部满足：非 0 退出 + 占用诊断 + 占用者存活 + 无 panic |
| B004 | — | FAIL | 根因：codex 0.146 需 item-based SSE，stub 缺 `response.output_item.added` → codex `OutputTextDelta without active item`，delta 被丢弃 → `-o` 为空（exit 0）。证据 `/tmp/codex-direct/codex.log`（codex_http_client 显示 200 + sse_event 三连 + ERROR line 94）。**代理层转发已由契约测试 stub_forwarding_with_bearer 证明** |
| B005 | — | 行为 PASS / rc=1 | 打印 SKIP 但 rc=1（trap 覆写，见下）；且脚本断言 CCT_PROXY_LOG 指向的**日志文件**生成，实现为 CCT_PROXY_LOG 存在即写 **stderr**（src/proxy.rs:88）→ 脚本期望与实现契约不符。脱敏本身已由契约测试 log_masks_api_key（stderr 捕获）证明 |
| B006 | — | FAIL | 同 B004 根因：smoke-a 会话创建失败（`-o` 无 stub 末尾文本）→ resume 可见性前置不成立 |
| B007 | — | FAIL | 同 B006（smoke-a 会话未创建成功） |
| B008 | — | FAIL | 同 B006（smoke-a 会话未创建成功） |
| B009 | — | PASS（隔离，rc=0） | 首次批量运行 FAIL 为级联假象：B004 的 `cct run` 按设计留下共享 proxy daemon 占 19191 → B009 proxy A bind 失败。隔离重跑 PASS：双启动报错退出、socket 完好、原 proxy 仍响应。契约测试 double_start_race_one_wins 20/20 佐证 |
| B010 | PASS | PASS | cargo test proxy + integration 全绿（Step 14 门已验） |
| B011 | FAIL | FAIL | 真实缺口：install-script.md 无迁移说明（TC-23 范围） |
| B012 | PASS | PASS | 预检通过 |
| B013 | FAIL | FAIL | 真实缺口：5 文档陈旧叙述 + resume 过滤语义（TC-23 范围） |
| B014 | PASS | PASS | 快照回归 + 接口冻结 |
| B015 | SKIP（run 1） | PASS（级联） | 依赖 19191 已有监听；run 1 无 proxy → SKIP；后随 B009 遗留 daemon 运行 → PASS (HTTP 502)。单序列 run-all 内恒 SKIP（脚本设计：须先 `cct proxy start`） |

## harness bug 说明（脚本缺陷，未修复）

1. **B001 `wait` 挂起**：`verify-B001-concurrent-http.sh:36` 无参 `wait` 含长驻 proxy daemon → run-all 永不完成。
2. **EXIT trap 退出码覆写**：bash 3.2.57 下 `set -e` 对 EXIT trap 生效；`kill "${PROXY_PID:-}"` 在 PROXY_PID 为空时返回 1 → 覆写脚本退出码为 1（最小复现：`bash -c 'set -e; trap "kill \"\" 2>/dev/null" EXIT; exit 77'; echo $?` → 1）。影响 B003（PASS 打成了 rc=1）、B005（SKIP 打成 rc=1，非 77 → run-all 误计 FAIL 而非 SKIP）。
3. **B005 日志文件期望**：脚本断言 CCT_PROXY_LOG 路径的日志文件生成，实现是 stderr（布尔开关语义）。

## 产品修复本身（Part A）的确认状态

- 死锁（B001 契约）、僵尸自愈（B002）、占端口诊断（B003 契约）、脱敏（B005 契约）、双启动收敛（B009 契约）在契约测试层（tests/proxy_contract.rs + launch_proxy_contract.rs，真实二进制 + 临时 socket/动态端口）**全部绿**（TC-6..19），人工探针端到端亦 PASS——**修复本身有效**。
- TC-20 门未过的原因分三类：脚本 harness bug（B001/B003/B005 计数层）、codex↔stub 契约不匹配（B004/B006/B007/B008，L2 实测揭示，正是 PoC 验证目标）、文档缺口按计划属 TC-23（B011/B013）。

## 遗留证据路径

- `/tmp/codex-direct/codex.log`（codex 0.146 SSE 解析错误证据，line 94）
- `/tmp/codex-direct/stub.log`（codex 实际 POST /v1/responses 记录）
- `/var/folders/8t/.../T/tmp.3xMzHFyYma/run-a.log`（`cct run smoke-a` stdin 挂起 + 遥测证据）
- poc.md Results Log 已追加 4 行记录以上全部结果
