---
title: "Spec: Agent-First Complete CLI"
doc_type: proc
brief: "Define an agent-first complete CLI for cct with explicit list/add/launch subcommands and JSON contracts"
confidence: medium
created: 2026-04-03
updated: 2026-04-03
revision: 1
---

# Spec

## Solution Summary

`cct` 保持现有根命令无参进入 TUI 的行为不变，但新增一套显式、零交互、面向 agent 的子命令契约。第一版命令面为 `cct list`、`cct add`、`cct launch`，并将旧的逐步问答式创建流程迁移为 `cct add interactive`。agent 场景下，调用方必须始终使用显式子命令，不依赖 TTY 探测或隐式模式切换。

三类子命令默认输出 JSON，并共享统一 envelope：成功返回 `ok: true` 与命令结果对象，失败返回 `ok: false` 与稳定错误对象。`list` 负责枚举 profile；`add` 负责通过单条命令创建 profile；`launch` 负责通过显式的 `--backend` 与 `--profile` 执行目标 profile。实现上复用现有 `config`、`launch` 和 backend/profile 数据模型，在 `main` 中扩展 clap 命令树，并新增专门的 CLI 输出层来隔离 JSON 序列化、错误码和进程退出语义。

## Decisions

- 根命令 `cct` 不带子命令时继续进入 TUI；agent 不应直接调用无参根命令。
- `cct add` 改为零交互命令，旧交互流程迁移到 `cct add interactive`。
- 第一版命令树采用动作优先结构：`cct list`、`cct add`、`cct launch`，不引入 `profile add` 这类额外层级。
- `cct launch` 必须显式要求 `--backend <claude|codex>` 与 `--profile <name>`，即使 profile 本身已记录 backend，也不省略该参数。
- 默认输出格式为 JSON；可选 `--output text` 提供人类可读结果。错误也遵循同一输出层。
- 推荐统一 envelope：
  - 成功：`{"ok":true,"command":"list","data":...}`
  - 失败：`{"ok":false,"command":"add","error":{"code":"profile_exists","message":"...","details":{...}}}`
- `list` 支持 `--backend` 过滤；默认返回所有 profiles，并在每条记录中显式包含 backend、name、description、model、flags 摘要。
- `add` 必须支持 backend 显式指定，并通过 flags 一次性声明 profile 所需字段。第一版至少支持 `--backend`、`--name`、`--description`、`--base-url`、`--api-key`、`--model`、`--full-auto`。后端不适用的参数必须报稳定错误，而不是忽略。
- `launch` 在成功路径上先输出一条结构化“即将执行”的结果，再进入 `exec` 替换。若需要避免 stdout 污染真实子进程，可将成功 JSON 输出到 stderr 或提供 `--dry-run` 返回计划结果，实际执行时只在失败前输出 JSON；实现阶段需择一并保持一致。
- 为符合 agent CLI 约束，每个子命令的 `--help` 都必须带真实示例，不允许缺失必填参数时掉回交互式补问。

## Open Questions

- `launch` 的成功 JSON 与 `exec` 替换如何兼容仍需实现时最终定案：建议优先支持 `--dry-run`，并将实际执行成功视为“无返回”进程替换语义。
- `add` 是否在第一版支持 `--extra-arg <arg>` 多次传入尚未在 intake 中明确，但从现有模型看应作为高优先级兼容项。
- `list` 的 JSON 是否默认扁平数组，还是包含 `profiles` 顶层字段，需在实现时固定并写入测试快照。
- 是否为稳定错误字段增加整数退出码映射尚未定案；最低要求是稳定的字符串 `error.code`。
- README 中需明确区分 “human path” 与 “agent path”，并给出从旧 `cct add` 迁移到 `cct add interactive` 的示例。

## Acceptance Criteria

- [ ] `cct` 提供显式子命令 `list`、`add`、`launch`，且 `cct add interactive` 承载旧问答式流程
- [ ] `cct` 无参继续进入 TUI，不因 agent-first CLI 引入破坏性默认行为变化
- [ ] `cct list` 支持默认 JSON 输出，并可按 `--backend` 过滤 profile
- [ ] `cct add` 支持零交互、全参数化创建 profile，不因缺失参数进入问答流程
- [ ] `cct launch` 强制要求 `--backend` 与 `--profile`
- [ ] 成功与失败结果均具备稳定 JSON 结构，失败至少包含 `error.code`、`error.message`
- [ ] 不适用参数、缺失参数、非法 backend、profile 不存在、重名 profile 等场景具备稳定错误模型
- [ ] `--help` 在根命令及各子命令层级均包含可复制示例
- [ ] README 更新为同时说明 TUI 路径与 agent-first 路径
- [ ] 单元测试与集成测试覆盖 `list/add/launch` 契约、JSON 输出和兼容迁移路径
