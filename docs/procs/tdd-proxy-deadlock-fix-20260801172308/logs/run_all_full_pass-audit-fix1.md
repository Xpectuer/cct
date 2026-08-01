---
title: "run_all_full_pass — Audit Fix (cycle 1)"
brief: "run_all_full_pass — 审计修复后 15/15 全量原始输出归档（B006/B007 断言可证伪化后 run-all 复跑）"
doc_type: proc
created: 2026-08-02T02:07:50+0800
case: run_all_full_pass
phase: audit-fix
---

# run_all_full_pass — 审计修复（cycle 1）后全量原始输出

**背景**: 保真度审计 cycle 1（findings/audit-completeness-cycle1.md 偏离 1/2）判定
B006/B007 断言不可证伪（标记文本断言在"复用/新建"两路径下恒 PASS；smoke-sub 直连
OpenAI 结构性空转；显式恢复 6 旗标手工复刻）。本日志为修复两脚本后 `run-all.sh`
的**完整原始输出**（终态 15/15 转录证据，对应审计 fidelity 偏离 3"最终 15/15
原始输出未归档"的补足）。

**修复内容**（详见 `logs/audit-fix-completeness-cycle1.md`）:
- B006: 断言改为 session-id 对比 + rollout 复用计数（spec AC-6 明文要求的证据）
- B007: 跨 provider 不可见改为 rollout 计数 + id 级核对；显式恢复改经 `cct run`
  临时 profile（6 旗标由真实函数 `build_codex_proxy_config_args` 生成，不再手工复刻）

**执行**: `cd docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc && ./run-all.sh`
（2026-08-02 02:06:11 起，退出码 0 = "All checks passed."，无 Failures 列表）

## 完整原始输出（tee 2>&1 捕获）

```
=== PoC Runner ===
Draft: intake-reinvestigate-why-codex-sessions-20260801145824
Started: 2026-08-02 02:06:11

--- verify-B001-concurrent-http.sh ---
[PoC B001] 并发控制命令 + HTTP 请求不挂起（死锁回归）
[PASS] B001: HTTP 在 3s 内得到响应 (HTTP 502)
/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605/docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/scripts/verify-B001-concurrent-http.sh: line 20:   774 Terminated: 15          "$CCT_BIN" proxy start > /dev/null 2>&1

--- verify-B002-zombie-recovery.sh ---
[PoC B002] 僵尸死 proxy 自愈重启
[PASS] B002: 死 socket 被清理, proxy 自愈并响应
/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605/docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/scripts/verify-B002-zombie-recovery.sh: line 18:   900 Terminated: 15          "$CCT_BIN" proxy start > /dev/null 2>&1

--- verify-B003-port-occupied.sh ---
[PoC B003] 端口被占时明确报错, 不 panic 不自动终止
[PASS] B003: 端口被占时明确报错退出 (RC=1), 占用进程存活, 未 panic
/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605/docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/scripts/verify-B003-port-occupied.sh: line 18:   964 Terminated: 15          python3 -c "import socket,time; s=socket.socket(); s.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1); s.bind(('127.0.0.1',$CCT_PROXY_PORT)); s.listen(1); time.sleep(30)"

--- verify-B004-stub-forwarding.sh ---
[PoC B004] proxy 指向 stub 上游: 转发带 Bearer key, SSE 流式返回
[PASS] B004: cct→proxy→stub 链路正常 — Bearer key 转发 + SSE 流式返回
/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605/docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/scripts/setup-smoke.sh: line 22:  1053 Terminated: 15          python3 "$SCRIPT_DIR/stub-sse-upstream.py" "$STUB_PORT" "$STUB_LOG"

--- verify-B005-log-masking.sh ---
[PoC B005] 日志打印不含 api_key 明文（脱敏）
/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605/docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/scripts/verify-B005-log-masking.sh: line 38:  2021 Terminated: 15          "$CCT_BIN" proxy start > "$LOG" 2>&1
[PASS] B005: 日志不含 api_key 明文

--- verify-B006-same-provider-visible.sh ---
[PoC B006] 同 provider 会话在 resume --last 中互相可见
[PASS] B006: profile B 经 resume --last 复用 profile A 的会话（session-id 019fbe81-618a-79e0-9c14-dc20277124a7 一致, rollout 数不变）
/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605/docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/scripts/setup-smoke.sh: line 22:  2077 Terminated: 15          python3 "$SCRIPT_DIR/stub-sse-upstream.py" "$STUB_PORT" "$STUB_LOG"

--- verify-B007-cross-provider-invisible.sh ---
[PoC B007] 跨 provider 不可见 + 显式 resume <session-id> 可恢复
[PASS] B007: 跨 provider 不可见（新会话 019fbe81-6e9e-74f3-9813-34cb2307b9d7 与 A 的 019fbe81-6b33-73e1-b69f-184e7c01fe79 不同）; 显式 resume 019fbe81-6b33-73e1-b69f-184e7c01fe79 可恢复
/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605/docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/scripts/setup-smoke.sh: line 22:  3420 Terminated: 15          python3 "$SCRIPT_DIR/stub-sse-upstream.py" "$STUB_PORT" "$STUB_LOG"

--- verify-B008-cwd-filter.sh ---
[PoC B008] cwd 过滤: 跨仓库不可见, --all 可见
[PASS] B008: 默认跨仓库不可见, --all 后可见（cwd 过滤维度）
/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605/docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/scripts/setup-smoke.sh: line 22:  5370 Terminated: 15          python3 "$SCRIPT_DIR/stub-sse-upstream.py" "$STUB_PORT" "$STUB_LOG"

--- verify-B009-double-start.sh ---
[PoC B009] 双启动防护: 报错退出, 不删活 proxy 的 socket
[PASS] B009: 双启动报错退出, 原 proxy socket 与响应完好
/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605/docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/scripts/verify-B009-double-start.sh: line 18:  7312 Terminated: 15          "$CCT_BIN" proxy start > /dev/null 2>&1

--- verify-B010-contract-tests.sh ---
[PoC B010] 契约测试全部通过
[PASS] B010: proxy 单元契约 + integration 测试全部通过

--- verify-B011-migration-docs.sh ---
[PoC B011] 迁移说明已写入 install-script.md
[PASS] B011: install-script.md 含旧实例迁移说明

--- verify-B012-l2-prereqs.sh ---
[PoC B012] L2 冒烟前置条件检查
[OK] B012: 旧实例 PID 29182 未在运行
[OK] B012: 端口 19191 空闲
[PASS] B012: 前置条件满足（旧实例已终止 + 端口空闲）

--- verify-B013-doc-cleanup.sh ---
[PoC B013] 文档收尾检查（陈旧叙述 + resume 过滤语义）
[PASS] B013: 5 份文档无陈旧叙述, resume 过滤语义已说明

--- verify-B014-interface-frozen.sh ---
[PoC B014] 配置快照回归 + 接口冻结
[PASS] B014: 回归测试通过, 既有接口未变

--- verify-B015-layered-diag.sh ---
[PoC B015] proxy 层存活诊断（无上游时 502/404 亦为存活证据）
[PASS] B015: proxy 层存活 (HTTP 502) — 可进入上游层诊断
/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605/docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/scripts/verify-B015-layered-diag.sh: line 19: 11678 Terminated: 15          "$CCT_BIN" proxy start > /dev/null 2>&1

=== Results ===
Total: 15 | Pass: 15 | Fail: 0 | Skip: 0
All checks passed.
```

## 逐脚本 PASS/FAIL 汇总

| # | 脚本 | 结果 | 关键证据 |
|---|------|------|----------|
| B001 | verify-B001-concurrent-http.sh | PASS | HTTP 3s 内响应 (502) |
| B002 | verify-B002-zombie-recovery.sh | PASS | 死 socket 清理 + 自愈 |
| B003 | verify-B003-port-occupied.sh | PASS | RC=1 报错 + 占用者存活 |
| B004 | verify-B004-stub-forwarding.sh | PASS | Bearer 转发 + SSE 流式 |
| B005 | verify-B005-log-masking.sh | PASS | 日志无 api_key 明文 |
| B006 | verify-B006-same-provider-visible.sh | PASS | **session-id 019fbe81-618a-79e0-9c14-dc20277124a7 复用一致, rollout 数不变** |
| B007 | verify-B007-cross-provider-invisible.sh | PASS | **新会话 019fbe81-6e9e-74f3-9813-34cb2307b9d7 ≠ A 的 019fbe81-6b33-73e1-b69f-184e7c01fe79; 显式 resume A 恢复成功** |
| B008 | verify-B008-cwd-filter.sh | PASS | --all 复用 / 默认过滤 +1 |
| B009 | verify-B009-double-start.sh | PASS | 报错 + socket 完好 |
| B010 | verify-B010-contract-tests.sh | PASS | 契约测试全绿 |
| B011 | verify-B011-migration-docs.sh | PASS | 迁移说明存在 |
| B012 | verify-B012-l2-prereqs.sh | PASS | 29182 已终止 + 端口空闲 |
| B013 | verify-B013-doc-cleanup.sh | PASS | 五文档净 + resume 语义 |
| B014 | verify-B014-interface-frozen.sh | PASS | 快照回归 + 接口冻结 |
| B015 | verify-B015-layered-diag.sh | PASS | proxy 层存活 (502) |

**Total: 15 | Pass: 15 | Fail: 0 | Skip: 0** — 门通过。

> 注: 各脚本输出中 `Terminated: 15 python3/stub/cct proxy ...` 行是 EXIT trap 正常清理
> stub/占用进程/daemon 的噪音（先前日志已确认不影响退出码；run-all 无 Failures 列表、
> 退出码 0）。
