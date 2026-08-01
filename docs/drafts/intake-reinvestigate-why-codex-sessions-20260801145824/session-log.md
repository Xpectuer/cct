---
title: "Intake Session Log"
doc_type: proc
status: active
brief: "Re-investigate why Codex sessions still appear isolated across cct profiles after shared CODEX_HOME"
confidence: speculative
created: 2026-08-01
updated: 2026-08-01
revision: 1
context_ref: "refs/"
---

# Intake Session Log

**Session**: intake-20260801145824
**Status**: active

## User Input

> /lb-dev:intake https://github.com/Xpectuer/cct/issues/9
> 事实上当前HEAD还存在被隔离的codex会话
>
> https://learn.chatgpt.com/docs/config-file/
> https://learn.chatgpt.com/docs/config-file/config-basic
> use /browser-harness to investigate official doc

## Context Gathering Summary

- **issue #9**（OPEN）：标题 "codex conversation history isolated by profiles"；正文描述旧机制（per-profile CODEX_HOME）；owner 评论 "v0.4.0 仍然有同样问题 记得看下"（2026-07-31）。
- **关键提交链**：
  - `afc1d11`（07-13）：删除 `generate_codex_config` / `write_codex_auth` / per-profile CODEX_HOME，改 `--config` flags + 共享 `~/.codex`；
  - `e10ba18` / `c8a7d29`（08-01）：上轮 re-review 判定"核心目标已达成"，abandon 0/25 TDD proc，关闭 draft（#9）；唯一遗留 AC7 文档陈旧。
  - **用户本次输入否定该结论**：HEAD 实际运行仍可观察到被隔离的会话 → 本轮重新调查。
- **先前 draft**：`docs/drafts/intake-codex-conversation-history-20260402144823/`（status: activated，含 plan/、spec.md rev3、review.md rev3）——上轮完整流程产物，已关闭，作为历史参考符号链接进 refs/prev-*。
- **官方文档调查**（browser-harness，learn.chatgpt.com 2026-08 版）：
  - 用户给的 `docs/config-file/` → **404**（官方改版）；有效页为 `config-basic` / `config-advanced#profiles` / `config-reference` / `developer-commands#codex-resume`；
  - 配置优先级：CLI flags > 项目 `.codex/config.toml`（受信任）> `--profile` 文件 > 用户 config > 系统 config > 内置；
  - `--profile` 是配置叠加层（`~/.codex/profile-name.config.toml`，0.134.0+），**不是状态隔离层**；
  - CODEX_HOME 承载全部本地状态；`history.jsonl` 仅在启用 history persistence 时存在；
  - **`codex resume` 默认按当前仓库过滤**（"Reopen a recent chat from the current repository"），`--last` 取当前 cwd 最近会话，`--all` 跨目录。
- **本地实测**（codex-cli 0.146.0，只读）：
  - `~/.codex/` 全局共享：sessions/（年/月/日分目录 rollout jsonl）、state_5.sqlite、history.jsonl、session_index.jsonl；
  - threads 表 268 行，含 `model_provider`（索引）、`cwd`、`git_origin_url`、`history_mode=legacy`；**无 profile 字段**；本机 4 种 provider 共存（openai 239 / crs 20 / clauddy 5 / deepseek 4）；
  - 会话 session_meta 含 `cwd` + `git{commit_hash,branch,repository_url}`，无 profile 字段；
  - 无 `~/.codex/*.config.toml` profile 文件；`backup-deepseek/` 为用户自建安装备份（非官方机制）。
- **技术栈**：cct Rust 0.5.0；codex-cli 0.146.0；profiles.toml；官方文档 2026-08 版。

## Notes

### 机制确认（2026-08-01 追查，用户确认场景 (a)）

**根因（源码取证，见 refs/codex-resume-filtering-source.md）**：Codex 0.146.0 官方 TUI `codex resume` picker 本地运行时**恒定按 `config.model_provider_id` 过滤**（`resume_picker.rs` 的 `ProviderFilter::MatchDefault`，无切换 UI）；`--all` 仅关 cwd 过滤；`codex exec resume --last` 同样按当前 provider 过滤（`exec/src/lib.rs` `resume_lookup_model_providers`）；显式 `codex resume <session-id>` 可绕过。

用户本机组合：唯一 codex profile `clauddy-codex`（proxy → `model_provider=custom`）+ `~/.codex/config.toml` 手动设 `model_provider=deepseek`（08-01 安装，backup-deepseek/ 为备份）→ 直接 `codex resume` 只见 deepseek 4 条，clauddy-codex（custom）会话不可见。threads 表 268 条分属 openai/crs/clauddy/deepseek 四类。

**原 [UNCERTAIN] 消解**：
1. 场景 (a) 已确认（用户回答）；机制 = provider 过滤（同 provider 同仓库应可见；跨 provider 不可见）
2. 期望行为仍待用户确认（跨 provider 统一列表 vs 文档澄清）→ 保留为需求故事的关键决策点
3. `--all` 已知官方语义（仅跨目录）；用户痛点主要在 provider 维度
4. history_mode=legacy：与可见性无证据关联，保持低优先 [UNCERTAIN]

### 第二个独立发现：cct proxy 死锁（2026-08-01 实测）

用户报告：cct 启动 codex 后连上 proxy 无限卡住等待第一个 Response（阻塞"同 provider 可见性"验证前提）。诊断结论（refs/proxy-deadlock-diagnosis.md）：
- 根因：`run_control_socket` 用 `std::os::unix::net::UnixListener`（同步阻塞）跑在 tokio `current_thread` runtime；`accept()` 阻塞整个线程 → runtime 死锁 → TCP HTTP 服务永不调度
- 证据：`sample` 100% 栈在 `__accept`；netstat 显示 ESTABLISHED + CLOSE_WAIT 堆积；curl 直连 8s 无响应
- 附带缺陷：`check_proxy_running`（仅 socket connect）误判死 proxy 为健康；第二次启动因端口占用 panic
- 影响：**修复 proxy 后才能验证同 provider 会话可见性**（用户前提）；已作为独立需求故事与 AC 纳入 requirements.md
- 调查期间注意：本机 curl 测试需 `--noproxy '*'`（shell 环境 `http_proxy=127.0.0.1:7892` 会劫持 localhost 请求）；Clash 在 7892 运行
- 待清理：调查时 `cct proxy start` 曾 panic 留下死 `proxy.sock` 文件（指向已退出进程），需在修复 proxy 后删除/重启时重建；运行中的旧 proxy（PID 29182，`~/.local/bin/cct`，14:42 启动）为死锁状态

### 上一轮流程状态（勿混淆）

- `tdd-codex-shared-history-20260801023840/`：**abandoned**（0/25，plan 锚定旧代码）——本轮不续用；
- `intake-codex-conversation-history-20260402144823/`：draft 已关闭，其 revision 2 设计（共享 CODEX_HOME + --profile overlay + 冲突对话框 + 迁移）经上轮 review 全部否决；本轮不复活。

### 建议下一步

进入 `/debate` 前，先请用户回答 Notes #1/#2（现象场景 + 期望），并可用零代码实验（C3）验证：同仓库两个 profile 各建会话 → `codex resume` 观察；跨仓库 → `codex resume --all` 对比。

---

## Debate Interview 记录（2026-08-01）

[coverage] [problem] Debate 范围确认：两个一起（Part A proxy 死锁修复 + Part B 会话可见性）— 依赖链：proxy 修好才能实测同 provider 可见性
[coverage] [behavior] 可见性期望行为确认：同 provider 可见 + 文档澄清（拒绝跨 provider 统一列表 / 仅文档收尾）— 与官方 resume 语义一致
[coverage] [ux] 死 proxy 恢复策略：自动重启（应用层探测失败 → 清理死 socket → 重新 spawn），用户无感知
[coverage] [arch] Part A 机制选型：异步 accept（tokio::net::UnixListener），handle_control 同步逻辑经 spawn_blocking；tokio full features 已含 net，无新依赖
[coverage] [failure] 并发启动防护：有活 proxy 时探测+报错退出（不删 socket 文件）；死 socket 清理重建；无 socket 直接 bind
[coverage] [delivery] AC7 文档残留盘点：launch.md（14 处）、CLAUDE.md（4 处）、ARCHITECTURE.md（1 处）、codex-home-storage-layout.md（整篇）、codex-backend-development-guide.md；session-cards/procs/context-* 为历史快照不更新
[coverage] [delivery] 验收路径确认：stub/契约测试先行（含 stub 上游解耦转发正确性）+ 用户配合 live 实测（同 provider 可见性 + 跨 provider 不可见）
[coverage] [premortem] Result: 上游连通被误判 — 修复后用户实测仍连不上（clash 代理/上游 API 环境因素）被归咎于 cct 修复。Mitigation: 契约测试用 stub 上游验证 proxy 转发链路（与真实上游解耦）；实测脚本分层诊断（curl --noproxy '*' 验证 proxy 层 → codex 对话验证上游层）

[smoke-tests]
- [Smoke 1] Given 修复后的 cct proxy 运行中且控制 socket 有并发命令, when HTTP 请求到达 127.0.0.1:19191/v1, then 请求得到响应而非无限挂起；Given 存在死 proxy（进程已退出的僵尸：socket 残留、端口空闲）, when 用户从 cct 启动 codex, then cct 自动清理死 socket 并重启 proxy, codex 正常收到第一个 Response
- [Smoke 2] Given proxy 指向 stub 上游（本地假 HTTP 服务）, when codex 经 cct 启动并发起对话, then 请求经 proxy 正确转发（Bearer key）且流式返回 — 与真实上游连通性解耦
- [Smoke 3] Given 两个同 provider（proxy/custom）cct profile 同一仓库, when profile A 对话后从 profile B 启动 codex 并 resume, then 能看到 profile A 的会话（会话连续性不依赖 profile 名）
- [Smoke 4] Given 同一仓库存在不同 provider 会话, when 切换 provider 后 resume（或 --last）, then 看不到另一 provider 会话；显式 codex resume <session-id> 可恢复
- [Smoke 5] Given 同 provider 跨仓库会话, when 当前 cwd 下 resume --last, then 不可见；--all 可见（cwd 过滤维度）
- [Smoke 自动化方案] TUI picker 无法由 agent 操作 → 全部走 codex exec 非交互链路：创建会话用 `codex exec --dangerously-bypass-approvals-and-sandbox "<prompt>"`；可见性验证用 `codex exec resume --last`（同 provider 命中/跨 provider 变化）；显式恢复用 `codex exec resume <session-id>`；断言用 `-o/--output-last-message <FILE>` 确定性捕获 + rollout 首条 session_meta 对比 session id。exec 与 TUI picker 过滤语义一致（exec 用 resume_lookup_model_providers，TUI 用 ProviderFilter::MatchDefault，源码取证）。前置条件：必须复刻 build_codex_proxy_config_args 的 6 个 --config 旗标 + OPENAI_API_KEY 注入，否则会话归错 provider

[verify-interview] 非交互启动路径 → cct run <profile> 子命令（既有命令，走完整 proxy 链路 ensure→switch→env→flags；exec-replace 仅影响子进程，smoke 脚本以子进程方式调用）——不新增命令
[verify-interview] L2 测试会话位置 → 临时 CODEX_HOME（不碰真实 ~/.codex，测试后清理；产品运行时共享约束不变）
[verify-interview] 占端口死 proxy 处理 → 僵尸自愈 + 报错手动（自动自愈仅限进程已退出/端口空闲；进程存活占端口→明确报错含 PID 提示，不自动 kill，不引入 pidfile/lsof 机制；旧实例一次性手动迁移）
[coverage] [data] cct 不读写 Codex 内部状态（不变量 4）；L2 测试会话写临时 CODEX_HOME 隔离真实 ~/.codex；proxy 状态为内存态无持久化
[coverage] [tradeoff] 跨 provider 统一列表 vs 文档澄清取舍已确认（决策 2）；KISS 否决 pidfile/lsof 自动 kill 机制（决策：僵尸自愈+报错手动）

[verify-round-1] completeness=8, consistency=7, implementability=6, risk=4, simplicity=9, testdepth=7, terminology=4
[verify-interview] 占端口报错 PID 获取 → 只读 lsof 诊断（单次调用仅报错展示；Alpine/musl 缺失降级为定位命令文本；"无 lsof 机制"= 无 lsof 自动终止机制）
[verify-round-2] completeness=9, consistency=9, implementability=7, risk=8, simplicity=10, testdepth=7, terminology=8
[verify-round-3] completeness=9, consistency=8, implementability=9, risk=9, simplicity=9, testdepth=9, terminology=8
[verify-round-4] consistency=8, terminology=8（措辞级 ADVISORY 全部修复）
[verify-round-5] consistency=10, terminology=9 — 全部 7 角度 ≥9，验证循环闭合
