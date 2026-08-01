---
title: "Requirements: Codex conversation history isolation re-evaluation"
doc_type: proc
brief: "Re-investigate why Codex sessions still appear isolated across cct profiles after shared CODEX_HOME"
confidence: speculative
created: 2026-08-01
updated: 2026-08-01
revision: 2
source_skill: intake
---

# Requirements: Codex conversation history isolation re-evaluation

## 0. Signal Check

### Causal Check

| # | Question | Signal | Answer |
|---|----------|--------|--------|
| C1 | **Problem or solution?** Is the user describing a pain they feel, or prescribing a specific implementation? | GREEN | 用户描述的是可观察的痛点（"当前 HEAD 仍存在被隔离的 codex 会话"），并仅提供调查线索（官方文档链接），没有规定实现方案。 |
| C2 | **Root cause — ask "why" 3–5 times.** Does it converge to a core problem, or loop on symptoms? | GREEN | 已收敛至根因（源码取证）：物理隔离（07-13 已解决）→ 共享后仍"隔离"→ **Codex 0.146.0 官方 TUI resume picker 按 `config.model_provider_id` 过滤会话**（`ProviderFilter::MatchDefault`，本地运行恒定，无切换 UI），`--all` 仅关 cwd 过滤、关不掉 provider 过滤。cct proxy profile（`model_provider=custom`）与直接运行 codex（本机 config 为 `deepseek`）或订阅模式（`openai`）之间互相不可见。证据：refs/codex-resume-filtering-source.md。 |
| C3 | **Cheapest validation?** What's the smallest hack to test whether solving this matters? | GREEN | 零代码验证已隐含完成：`codex resume`（当前 config deepseek）只见 deepseek 会话（4 条），而 threads 表共 268 条分属 4 个 provider——provider 过滤可观察。 |

### Boundary Check

| # | Question | Signal | Answer |
|---|----------|--------|--------|
| B1 | **Who has this problem? Who doesn't?** | GREEN | 具体用户：cct 的 Codex 后端用户，多 profile 且在多项目间切换的人（issue 作者 + "v0.4.0 仍然有同样问题"评论者 + 本次用户）。单 profile 单目录用户不受影响。 |
| B2 | **When would this feature make things worse?** | GREEN | 明确有负面影响面：若强制跨项目统一会话可见，会造成会话列表噪音、跨项目上下文/隐私泄漏（项目 A 的对话出现在项目 B 的 resume 列表）、违反 codex 官方"按仓库过滤"的产品语义。 |
| B3 | **Hidden dependencies?** | GREEN | 依赖已全部确认存在：Codex 0.146.0 官方 resume 过滤语义（源码取证）、官方 profile 配置层（0.134.0+）、用户本机 profile 组合（1 个 proxy profile `clauddy-codex`，config.toml 为 deepseek）。无不可获得的依赖。 |

### Verdict
- [x] **PASS**（≤1 RED）— proceed to §1
- [ ] **BLOCKED**（≥2 RED）

0 RED / 2 AMBER（C2 与 B3 指向同一未确认事实：用户观察隔离的具体场景）。

## 1. Problem Statement

issue #9 描述"每个 profile 使用独立 CODEX_HOME 导致会话历史不共享"，该物理隔离在 2026-07-13 重构（`afc1d11`，共享 `~/.codex` + `--config` flags）后已从代码中移除，上一轮 review 据此判定"核心目标已达成"并关闭 draft。

但用户实测（2026-08-01）明确反驳：**当前 HEAD 实际运行时仍可观察到被隔离的 codex 会话**。经官方文档调查（learn.chatgpt.com）+ **Codex 源码取证**（github.com/openai/codex，与本地 0.146.0 一致），机制已完全确认：

1. 官方会话存储确实物理共享：CODEX_HOME（默认 `~/.codex`）承载全部本地状态，sessions/、state_5.sqlite、history.jsonl 均为全局；
2. 但官方 `codex resume` TUI picker 本地运行时**按 `config.model_provider_id` 过滤会话**（`resume_picker.rs`：`ProviderFilter::MatchDefault(config.model_provider_id)`，创建后不可切换），同时按当前 cwd 过滤；
3. `--all` 只关闭 cwd（仓库）过滤，**关闭不了 provider 过滤**；`codex exec resume --last` 同样带 `model_providers=[当前 provider]`（`exec/src/lib.rs` 的 `resume_lookup_model_providers`）；
4. 只有显式 `codex resume <session-id>`（UUID/名称）绕过过滤，恢复任意会话；
5. 会话元数据（session_meta / threads 表）绑定 `cwd`、`git.repository_url`、`model_provider`，无 profile 字段；官方 `--profile` 是配置叠加层，不是状态隔离层。

**因此 issue #9 的"隔离"在 07-13 后换了形态：从 CODEX_HOME 物理隔离变为 Codex 官方 resume 的 model_provider 过滤。** cct proxy profile 传 `--config model_provider=custom`，其会话仅对同为 custom 的启动可见；与直接运行 codex（本机 config 为 deepseek）、订阅模式（openai）互相不可见。用户本机 threads 表 268 条分属 openai/crs/clauddy/deepseek 四类即为此机制的直接证据。

**此外（2026-08-01 实测发现，阻塞验证的第二个独立问题）**：cct proxy 存在致命 bug——控制 socket 用 `std` 同步阻塞 `UnixListener` 跑在 tokio `current_thread` runtime 上，`accept()` 从启动起就阻塞整个 runtime，**TCP HTTP 服务永不处理请求**（端口监听、连接可建立、请求无限挂起）。现象即用户报告的"codex 连上 proxy 后无限卡住等待第一个 Response"。这直接阻塞了"同 provider 会话可见性"的验证前提（需要 codex 在 proxy 模式下能正常对话）。证据与修复方向见 refs/proxy-deadlock-diagnosis.md。

## 2. Desired Outcome

- 用户从 cct 启动任意 codex profile 后，能够**可预期地**恢复之前创建的会话（同一仓库内跨 profile 无缝可见；跨仓库行为明确、有文档、可操作）。
- cct 与官方机制一致：不破坏 codex 的仓库维度 resume 语义，也不改写官方配置文件。
- 对"隔离"的机制有权威、可验证的结论（官方文档 + 本地实测），并同步更新陈旧的文档叙述（上一轮遗留的 AC7：launch.md / codex-home-storage-layout.md / codex-backend-development-guide.md / CLAUDE.md 仍描述 per-profile CODEX_HOME 与 generate_codex_config）。

## 3. User Stories

- As a cct Codex 用户（proxy profile 与其他 provider 切换），I want 在 `codex resume` 中看到我创建的**所有**会话，so that 我不会因 model_provider 过滤而误以为历史丢失。 [关键点：官方当前行为是 provider 过滤——需要确认用户要的是"跨 provider 统一列表"（与官方语义冲突，需 --all 之外的机制）还是"知道过滤存在 + 显式 id 恢复"]
- As a cct 用户（同 provider 多 profile），I want 同一 model_provider 的会话在 resume 中互相可见，so that 会话连续性不依赖 profile 名。 [已确认：同 provider（如都是 proxy/custom）时官方机制下本就可见——若用户实测不符才是真 bug]
- As a cct 维护者，I want 文档（CLAUDE.md / launch.md / codex 参考文档）准确描述当前共享 ~/.codex 架构与官方 resume 的 provider/cwd 过滤语义，so that 后续开发者与用户不会基于过时叙述或错误预期做出错误决策。
- As a cct Codex 用户（proxy 模式），I want cct 启动 codex 后能正常收到第一个 Response，so that 我才能验证并依赖会话连续性（当前 proxy 半死 bug 直接阻断）。

## 4. Constraints

- **Tech stack**: cct（Rust 0.5.0，edition 2021）；codex-cli 0.146.0（用户本机实测版本）；`profiles.toml`（TOML）；官方文档 learn.chatgpt.com（2026-08 版，URL 已改版：`config-file/` 404，有效页为 `config-file/config-basic`、`config-file/config-advanced`、`config-file/config-reference`、`developer-commands`）
- **Hard boundaries**:
  - 所有 codex profile 继续共享默认 `~/.codex`（07-13 架构既定，不回退 per-profile CODEX_HOME）
  - cct 不写任何 Codex 配置文件（`config.toml` / `profile-*.config.toml` / `auth.json`）——避免重蹈 `9a09b39` 修复的"覆盖用户 config"覆辙；配置一律走 `--config` flags
  - 不手动编辑/迁移 Codex 内部 sqlite / rollout 文件（外部工具内部状态，违反已验证的外部契约）
  - 不改变 codex 官方"resume 按仓库过滤"的产品语义；如需跨仓库查看，用官方 `--all`
- **Relevant rules**: refs/ 中规则文档——`test-boundaries-with-stubs-before-manual-verification`（外部行为先桩测）、`external-tool-config-schema-must-be-verified`（官方机制实测验证，本轮已执行）、`preserve-user-edited-config-structure`（用户 config 不动）、`cross-module-features-need-contract-tests`、`pure-builders-thin-effectful-edges`、`update-docs-after-new-feature`（AC7 收尾）

## 5. Scope

### In Scope
- 确认"隔离"的真实机制：官方 resume 按仓库过滤 vs 其他维度（用户复现步骤确认后）
- 评估 cct 层可做的增量（取决于机制确认）：
  - 会话恢复入口（如 `c` 键 resume / `--all` 透传 / extra_args 引导）[UNCERTAIN]
  - 启动时 cwd 与 profile 的映射是否影响会话可见性 [UNCERTAIN]
  - 文档收尾（AC7）：launch.md、codex-home-storage-layout.md、codex-backend-development-guide.md、CLAUDE.md、ARCHITECTURE.md 中 per-profile CODEX_HOME 陈旧叙述（README 实测 0 处，无陈旧叙述）
- 官方文档调查结果沉淀为参考（本 draft refs/issues.md 已含取证）

### Out of Scope
- 改动 Codex 官方会话存储/恢复机制（非 cct 可控）
- 手动合并或迁移 sqlite / rollout 历史文件
- 改动 Claude / Kimi 后端
- 重新引入 per-profile CODEX_HOME 或任何 on-disk 配置 overlay（上轮设计已否决，KISS）
- 改变 profiles.toml schema

## 6. Acceptance Criteria

- [x] Given 官方源码与本地实测，when 核对 resume 过滤逻辑，then 确认隔离机制 = Codex 0.146.0 官方 TUI picker 按 `model_provider_id` 过滤（`--all` 关不掉）+ cwd 过滤；显式 session-id 可绕过。 [已完成，证据 refs/codex-resume-filtering-source.md]
- [ ] Given 用户明确了期望（跨 provider 统一列表 vs 同 provider 可见 vs 仅文档澄清），when 进入 debate/设计，then 方案范围与该期望一致。
- [ ] Given 同 provider（同为 proxy/custom）的 cct profile，when 任一 profile 下 `codex resume`，then 会话互相可见——若实测不符，定义为 cct 层 bug 并修复。
- [ ] Given 跨 provider 场景（proxy vs subscription vs 直接运行），when 用户需要恢复旧会话，then 有明确、官方一致的操作路径（显式 `codex resume <id>`；文档说明 `--all` 仅跨目录）且 cct 文档说明之。
- [ ] Given 文档收尾改动，when 完成，then CLAUDE.md / launch.md / ARCHITECTURE.md / codex 参考文档（codex-home-storage-layout.md / codex-backend-development-guide.md）不再存在 per-profile CODEX_HOME 与 generate_codex_config 的陈旧叙述，并新增"resume 按 provider/cwd 过滤"语义说明。
- [ ] Given 任何 cct 层改动，when 合入，then 有 stub/契约测试覆盖（遵守 test-boundaries-with-stubs 规则），且不写任何 Codex 配置文件。
- [ ] Given cct proxy 运行中，when 有 HTTP 请求到达（含控制命令并发），then 请求得到响应而非无限挂起（修复 current_thread 死锁；契约测试覆盖控制 socket 与 HTTP 并发）。

## 7. References

See [Directory Index](./index.md) and [refs/](./refs/)（官方文档取证见 refs/issues.md；先前流程见 refs/prev-* 符号链接）。

---
*Generated by intake skill on 2026-08-01*
*Directory index: ./index.md | Session log: ./session-log.md*
