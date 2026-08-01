---
title: "visibility_three_checks — Green Phase"
brief: "visibility_three_checks — Green: exit 0"
doc_type: proc
created: 2026-08-02T01:29:08+0800
case: "visibility_three_checks"
phase: green
---

# visibility_three_checks — B006/B007/B008 会话可见性结果判定

Exit code: 0（三个脚本均独立运行，exit code 分别为 0 / 0 / 0）

B006: PASS — 同 provider（proxy/custom）可见。profile A（smoke-a, `exec -o out-a.txt hello`）创建会话且 out-a.txt 含 stub 末尾文本 `POC_STUB_LAST_MESSAGE`；同 provider profile B（smoke-b, `exec resume --last -o out-b.txt hello`）经 `resume --last` 复用 A 的会话，out-b.txt 含 `POC_STUB_LAST_MESSAGE`。AC #6 语义成立。

B007: PASS — 跨 provider 不可见 + 显式恢复。profile A（custom provider）创建会话，锁定 session-id（`sessions/<Y>/<M>/<D>/rollout-<ts>-<session-id>.jsonl` 文件名提取）；切换 subscription/openai provider 后 `resume --last`（smoke-sub）不显示该会话（out-sub.txt 无 `POC_STUB_LAST_MESSAGE`）；显式 `codex exec resume <session-id>`（同 custom provider 旗标，经 proxy→stub 链路）成功恢复会话内容（out-explicit.txt 含标记）。AC #7 语义成立。

B008: PASS — cwd 过滤。repo1 中创建会话（rollout 文件数 = 1）；repo2 中 `resume --last --all`（smoke-c）跨仓库复用既有会话：out-c.txt 含标记且 rollout 文件数保持 1（复用未新建）；repo3 中默认 `resume --last`（smoke-b）cwd 过滤生效：A 不可见 → 新建会话，rollout 文件数 +1。AC #8 语义成立。

判定: 全 PASS → 官方（codex 0.146.0）会话可见性语义（同 provider 共享、跨 provider 过滤 + 显式 session-id 可绕过、cwd 过滤 + `--all` 放开）与 cct 链路（cct run → proxy → stub，含 `resume --last` / `--all` 参数透传与 CODEX_HOME 隔离）实测一致，**无 cct 层 bug**。spec AC6 兜底条款（任一 FAIL → 定义为 cct 层 bug → 追加修复任务）未触发，无需追加修复任务。

执行细节：
- 脚本路径：`docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/scripts/verify-B006/B007/B008-*.sh`，串行逐个运行，未用 run-all.sh
- 前置：cct binary 已 build（target/debug/cct）、codex-cli 0.146.0、无遗留 proxy 进程、config.env 就位（PROXY_PORT=19191, STUB_PORT=19200, TEST_API_KEY=sk-poc-test-0123456789abcdef）
- 各脚本自动 mktemp 临时 CODEX_HOME / profiles.toml（smoke-a/b/c/sub），不触碰真实 `~/.codex`；退出时 EXIT trap 停 proxy daemon + 杀 stub 并清理临时目录
- 输出中的 `Terminated: 15 python3 ... stub-sse-upstream.py` 为 EXIT trap 正常杀 stub 的清理噪音，不影响退出码（实测 exit 0）
- 修复后链路复用 TC-20 的 stub 契约修复（item-based SSE：`response.output_item.added`），与 codex 0.146 匹配

证据（本轮运行）：
- B006: `[PASS] B006: profile B 经 resume --last 看到 profile A 的会话`，exit 0
- B007: `[PASS] B007: 跨 provider 不可见; 显式 resume <session-id> 可恢复`，exit 0
- B008: `[PASS] B008: 默认跨仓库不可见, --all 后可见（cwd 过滤维度）`，exit 0
