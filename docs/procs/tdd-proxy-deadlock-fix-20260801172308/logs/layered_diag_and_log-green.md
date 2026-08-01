---
title: "layered_diag_and_log — Green Phase"
brief: "layered_diag_and_log — Green: exit 0"
doc_type: proc
created: 2026-08-02T01:31:29+0800
case: "layered_diag_and_log"
phase: green
---

Exit code: 0

B015: PASS — 输出 `[PASS] B015: proxy 层存活 (HTTP 502) — 可进入上游层诊断`（`bash poc/scripts/verify-B015-layered-diag.sh`，脚本自起 proxy 实例后 curl `--noproxy '*'` 直连 19191 得 HTTP 502 = 无上游时的存活证据）。修复前基线该脚本预期 FAIL（死锁复现：curl 3s 超时）；修复后转 PASS 即分层诊断第一层证据。脚本退出时的 `Terminated: 15` 为 cleanup trap 正常回收自起 proxy，非失败。

poc.md Results Log（`docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/poc.md`）终态摘要：
- 基线行：`2026-08-01（修复前）15/11/4/0`（只读脚本 B011/B012/B013/B015 FAIL）
- 修复后行（harness 修复后 run-all）：`2026-08-02 15/13/2/0`，Notes 注明"修复后全量"——B001-B010、B012、B014、B015 全 PASS，Skip 0；Fail 2 = B011/B013
- 修复后逐脚本单独记录：`2026-08-02 15/13/2/0`——B001-B010/B012/B014/B015 各自独立运行 PASS（exit 0），B011/B013 FAIL
- 历史行保留：run 1（15/3/11/1，config.env 未建）、run 2（卡死 B001）、修复前单脚本 + 人工等价探针行——surgical 追加，未改动既有行

Notes: 13/15 中的 2 项 Fail（B011 install-script.md 旧实例迁移说明缺失、B013 5 份文档陈旧叙述 + resume 过滤语义缺失）为真实文档缺口，按计划归属 TC-23（Step 19-21 doc_cleanup_final）范围，本用例不修复。TC-23 完成后 run-all 应可达 15/15/0/0。
