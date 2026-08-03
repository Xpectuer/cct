---
title: "run_all_full_pass — Fix (attempt 1)"
brief: "PoC 脚本 harness + stub SSE 修复"
doc_type: proc
created: 2026-08-02T01:26:48+0800
case: "run_all_full_pass"
phase: fix
---

# run_all_full_pass — 修复记录（attempt 1）

## 结论速览

- run-all.sh 不再挂起，完整跑完（B001 无参 wait 已修；codex 链路 `</dev/null` 防 stdin 挂起）
- 最终输出：`Total: 15 | Pass: 13 | Fail: 2 | Skip: 0`（exit 1）
- Fail 2 项 = B011/B013（TC-23 文档收尾缺口，预期内，按要求不强行修，见 Notes）
- B001-B010、B012、B014、B015 全部 PASS；Skip: 0（B005 断言真实生效、B015 自起实例）
- 产品修复（Part A）无回归：契约测试（B010/B014）全绿；无残留 daemon（19191/19200 空闲）
- 完整 run-all 输出留存：`/tmp/run-all-fix1.log`

## Changes made

改动范围仅 `docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/`（scripts/ 下 10 个文件）。未动 `src/`、`tests/`、`docs/procs/`。

1. **verify-B001-concurrent-http.sh**（修复 1 — 卡死根因）
   - :36 无参 `wait` → 收集 5 个 nc 后台任务 PID 后 `wait "${NCS[@]}" 2>/dev/null || true`（无参 wait 会连带等待长驻 proxy daemon → 无限挂起）
   - cleanup 统一判空守卫（见 2）

2. **verify-B002 / verify-B003 / verify-B005 / verify-B009**（修复 2 — 退出码覆写）
   - EXIT trap 内 `kill "${PID:-}" 2>/dev/null`（空 PID 时失败）→ `[ -n "${PID:-}" ] && kill "$PID" 2>/dev/null || true`
   - 消除 `set -e` + trap 下 `kill ""` 失败把 exit 0/1/77 覆写为 1 的 harness bug

3. **setup-smoke.sh**（修复 4 + 两个 fixture 契约 bug）
   - cleanup_smoke 增加 `"$CCT_BIN" proxy stop`（经控制 socket 停掉 `cct run` 内部 `ensure_proxy_running` 拉起的 proxy daemon）→ 修复 B002/B009 首跑级联失败；STUB_PID kill 加判空守卫
   - profiles.toml 顶层 `api_key = ...` 改为 `[profiles.env] OPENAI_API_KEY = ...`——Profile schema 无顶层 api_key 字段（serde 静默忽略），`exec_codex_proxy` 只从 `profile.env["OPENAI_API_KEY"]` 读取 key → Bearer 转发断言才成立（与 `cct add` 写入形态一致）
   - resume 类 profile（smoke-b/c/sub）补 prompt 参数——codex 0.146 `exec resume` 无 prompt 报 "No prompt provided via stdin"

4. **stub-sse-upstream.py**（修复 3 — B004-B008 根因）
   - SSE 事件流改为 item-based：`response.created → response.output_item.added → response.output_text.delta → response.output_item.done → response.completed`（原为 created → delta → completed）
   - 依据 codex 0.146 源码：`turn.rs` 的 `OutputTextDelta` 需要 active item（缺 output_item.added 报 "OutputTextDelta without active item"，`-o` 为空）；`output_item.done` 的 message content 文本即 `last_agent_message`（`-o` 内容）；形状对齐 codex 自身测试 fixture（codex-api/sse/responses.rs）
   - `Authorization` 头日志格式 `auth=<v>` → `Authorization: <v>`（匹配 B004 断言；代理注入的 Bearer 本就存在）

5. **verify-B004 / verify-B006 / verify-B007 / verify-B008**
   - 所有 codex 调用加 `</dev/null`——`codex exec` 非 tty stdin 时读输入直到 EOF（"Reading additional input from stdin..."），挂起风险消除

6. **verify-B007**
   - session-id 提取：扁平 glob `sessions/*.jsonl`（codex 0.146 实际为 `sessions/<Y>/<M>/<D>/rollout-<ts>-<id>.jsonl`，glob 永不匹配且 pipefail+set -e 静默退出）→ `find` + rollout 文件名 UUID 正则
   - SESSION_ID 锁定提前到 smoke-sub（openai 无匹配会话会新建会话）之前
   - 显式 resume 补 6 个 `--config` custom provider 旗标（镜像 `build_codex_proxy_config_args`，`--config` 置于 `exec` 之前），指向 proxy→stub，`-o` 才能取到标记文本

7. **verify-B008**
   - "不可见"断言从"out-b.txt 无标记"改为 rollout 文件数语义：实测 codex `exec resume --last` 无匹配会话时**新建会话并运行**（标记必然出现），原断言在真实协议下不可能成立
   - 正确契约（源码 `resolve_resume_thread_id` / `latest_thread_cwd`）：resume 复用既有会话 → rollout 文件数不变；被过滤 → 新建会话 +1；且 resume 会把最新 turn 的 cwd 写回会话
   - 用 repo3 隔离 --all（repo2）与默认（repo3）两个方向的干扰：`--all` 从 repo2 唯一候选 A 被复用（数不变）；默认从 repo3 时 A 的最新 cwd≠repo3 被过滤（新建会话，数 +1）

8. **verify-B005**
   - `CCT_PROXY_LOG` 语义修正：它是开关（置位后 `log_proxy!` 写 stderr），值不是日志路径；proxy stderr 重定向到 `$LOG` 捕获，使脱敏断言真实生效（原实现日志从未生成 → 恒 SKIP，断言从未执行）

9. **verify-B015**
   - 自起 proxy 实例再执行分层诊断（原"无监听 → SKIP"在 daemon 清理干净后恒 SKIP，破坏 Skip: 0 门）；断言不变（502/404 亦为存活证据，3s 超时 = 死锁 FAIL）

## 单独跑结果（各自独立、串行执行）

| 脚本 | 结果 | 说明 |
|------|------|------|
| verify-B001 | PASS (exit 0) | 0.18s 完成，HTTP 502，不再挂起 |
| verify-B002 | PASS (exit 0) | 死 socket 清理 + 自愈 |
| verify-B003 | PASS (exit 0) | RC=1 明确报错、占用者存活、未 panic |
| verify-B004 | PASS (exit 0) | **codex `-o` = POC_STUB_LAST_MESSAGE（非空）**；stub.log 含 `Authorization: Bearer <key>` |
| verify-B005 | PASS (exit 0) | 日志无 api_key 明文（断言真实生效） |
| verify-B006 | PASS (exit 0) | 同 provider resume --last 可见 |
| verify-B007 | PASS (exit 0) | 跨 provider 不可见 + 显式 resume 可恢复 |
| verify-B008 | PASS (exit 0) | 默认跨仓库不可见，--all 后可见（rollout 数语义） |
| verify-B009 | PASS (exit 0) | 双启动报错退出、原 proxy 完好 |
| verify-B010 | PASS (exit 0) | 契约测试全绿 |
| verify-B011 | FAIL (exit 1) | 见 Notes（TC-23） |
| verify-B012 | PASS (exit 0) | 29182 已终止 + 端口空闲 |
| verify-B013 | FAIL (exit 1) | 见 Notes（TC-23） |
| verify-B014 | PASS (exit 0) | 回归全绿 + 接口冻结 |
| verify-B015 | PASS (exit 0) | proxy 层存活（自起实例，HTTP 502） |

**B004 验证标准确认**：B004 场景中 codex `-o` 输出非空（`POC_STUB_LAST_MESSAGE`）；复现链路日志中不再出现 `OutputTextDelta without active item`（item-based SSE 生效）。

## run-all 最终输出

```
=== Results ===
Total: 15 | Pass: 13 | Fail: 2 | Skip: 0
Failures:
  - verify-B011-migration-docs.sh
  - verify-B013-doc-cleanup.sh
```

- 无挂起：15 个脚本全部有界完成（B001-B009 秒级；B010/B014 为 cargo 测试，约 2-3 分钟）
- run-all 退出码 1（因存在 2 项 Fail；修复目标项 B001-B010/B012/B014/B015 全 PASS，Skip: 0）

## Notes

- **B011/B013 状态**：run-all.sh 按 `verify-*.sh` glob 计入两者，按"文档断言"执行。当前 FAIL 为真实文档缺口——B011：install-script.md 缺旧死锁实例（PID 29182）迁移说明；B013：CLAUDE.md / docs/modules/launch.md / docs/references/codex-home-storage-layout.md / codex-backend-development-guide.md 仍含 per-profile CODEX_HOME / generate_codex_config 陈旧叙述，且缺 "resume 按 model_provider ∩ cwd 过滤" 语义说明。这些属 TC-23（Step 19-21）范围，按要求**不强行修**，记录为"预期内（TC-23 后补）"。TC-23 完成后 run-all 应可达 `Total: 15 | Pass: 15 | Fail: 0 | Skip: 0`。
- 稳定性加固：setup-smoke 清理 proxy daemon 后，逐脚本与 run-all 全程端口 19191/19200 无残留监听、无孤儿 `cct proxy` 进程（修复 4 生效）。
- 修复过程中发现的额外 fixture 契约问题（非任务清单内，但均阻塞门项）：api_key 写入形态、resume prompt 缺失、session 存储路径、`--config` 旗标位置、B005 日志开关语义、B015 恒 SKIP、resume --last 无匹配新建会话语义——均已在 Changes made 中说明并修正。
