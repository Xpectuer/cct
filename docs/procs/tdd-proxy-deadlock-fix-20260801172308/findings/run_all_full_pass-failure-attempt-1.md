---
title: "run_all_full_pass — FAILURE (attempt 1)"
brief: "TC-20 run-all.sh 门未过：脚本 harness bug（wait/trap）+ stub↔codex SSE 契约不匹配"
doc_type: finding
created: 2026-08-02
step: 15
attempt: 1
---
## run_all_full_pass FAILURE (attempt 1)

**Timestamp**: 2026-08-02
**Phase**: TC-20 Green（e2e 执行门）

**门要求**：run-all.sh 输出 `Total: 15 | Pass: 15 | Fail: 0 | Skip: 0`

**实际**：
- run 1: 15/3/11/1 —— config.env 缺失（gitignored setup 文件，新 worktree 无）→ B001-B009 死于 `CCT_BIN/TEST_API_KEY: Must be set in config.env`（环境准备问题，非产品缺陷）
- run 2（创建 config.env 后）: B001 卡死 —— `verify-B001-concurrent-http.sh:36` 无参 `wait` 等待全部后台任务（含长驻 proxy daemon）→ 无限挂起（bash 3.2.57 实测复现）
- 逐脚本有界复跑：
  - B001 PASS（人工探针：并发 5×status + curl 3s 内响应）—— 死锁修复端到端确认 ✅
  - B002 PASS、B009 PASS（首次 FAIL 为 B004 遗留 daemon 占端口级联假象）
  - B003/B005 行为正确但退出码被 harness 覆写（`set -e` + EXIT trap `kill ""` 失败覆写为 1）
  - B004/B006/B007/B008 FAIL 根因同一：**codex 0.146 需 item-based SSE**（缺 `response.output_item.added`）→ `OutputTextDelta without active item` → `-o` 为空。**stub↔codex 契约不匹配（PoC 设计验证目标），非代理修复回归**（代理层转发已由契约测试 stub_forwarding_with_bearer 证明）
  - B011/B013 真实文档缺口（TC-23 Step 19-21 范围，TC-20 预期内不通过）
  - B010/B012/B014 PASS、B015 PASS

**关键判定**：产品修复（Part A）有效——全部契约测试绿（TC-6..19）+ 人工端到端探针 PASS。门未过为三类问题：① 脚本 harness（wait 无 PID / trap 空 PID / 退出码覆写）；② stub SSE 协议不匹配 codex 0.146（缺 output_item.added）；③ B011/B013 属 TC-23。

**关联**：logs/run_all_full_pass-green.md（如实标 blocked）
