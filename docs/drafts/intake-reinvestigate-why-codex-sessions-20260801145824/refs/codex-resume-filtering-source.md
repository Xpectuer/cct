# 源码取证：Codex 0.146.0 resume 会话过滤机制

调查日期：2026-08-01。来源：`github.com/openai/codex`（main 分支，与本地 0.146.0 二进制 strings 路径一致）+ 本地实测。

## 结论（一句话）

**Codex TUI 的 `codex resume` picker 本地运行时恒定按 `config.model_provider_id` 过滤会话；`--all` 只能关闭 cwd（仓库）过滤，关闭不了 provider 过滤。** 不同 model_provider 的会话互相不可见。

## 证据 1：TUI resume picker（codex-rs/tui/src/resume_picker.rs）

```rust
enum ProviderFilter {
    Any,
    MatchDefault(String),
}

fn picker_provider_filter(config: &Config, uses_remote_workspace: bool) -> ProviderFilter {
    if uses_remote_workspace {
        ProviderFilter::Any
    } else {
        ProviderFilter::MatchDefault(config.model_provider_id.to_string())  // 本地恒为 MatchDefault
    }
}

// ThreadListParams 构建：
model_providers: match provider_filter {
    ProviderFilter::Any => None,
    ProviderFilter::MatchDefault(default_provider) => Some(vec![default_provider]),
},
cwd: cwd_filter.map(|cwd| ThreadListCwdFilter::One(cwd.to_string_lossy().into_owned())),
```

- `provider_filter` 在 picker 创建时由 config 决定一次，**无用户切换 UI**（grep 确认无 toggle/cycle）。
- cwd 过滤由 `picker_cwd_filter` 决定：`show_all`（`--all`）时返回 None，本地运行时返回当前目录。

## 证据 2：codex exec resume（codex-rs/exec/src/lib.rs）

```rust
fn resume_lookup_model_providers(config: &Config, args: &crate::cli::ResumeArgs) -> Option<Vec<String>> {
    if args.last {
        Some(vec![config.model_provider_id.clone()])   // --last 时按当前 provider 过滤
    } else {
        None                                            // 显式 session_id 时不过滤
    }
}
```

- `resolve_resume_thread_id`：`--last` 分支 ThreadList 传 `model_providers`（当前 provider）+ 逐条 `cwds_match(config.cwd, session cwd)`（`--all` 跳过 cwd 匹配）。
- 显式 UUID 或名称：直接按 id 恢复，**不受 provider/cwd 过滤影响**。

## 证据 3：session_index.jsonl 的角色

`codex-rs/rollout/src/session_index.rs`：session_index.jsonl 只是 **thread name 的追加式索引**（`{id, thread_name, updated_at}`），用于把 thread id 显示为名字；**不是会话列表来源**。本机 44 条（vs threads 268 行）正常，不是"丢失"。

## 证据 4：thread-store 过滤字段（codex-rs/thread-store/src/types.rs）

```rust
pub model_providers: Option<Vec<String>>,   // None=全部；Some(vec!)=只匹配这些 provider
pub cwd_filters: Option<Vec<PathBuf>>,      // None=全部；空 vec=不匹配任何
```

## 对 cct 场景的含义

| 启动方式 | config.model_provider_id | resume picker 可见会话 |
|----------|--------------------------|------------------------|
| cct proxy profile（`--config model_provider=custom`） | `custom` | 仅 model_provider=custom 的会话 |
| cct subscription profile（`--config model_provider=openai`） | `openai` | 仅 openai 会话 |
| 直接 `codex`（用户 ~/.codex/config.toml，本机为 deepseek） | `deepseek` | 仅 deepseek 会话 |
| `codex resume <uuid>` 显式指定 | — | 任意会话（绕过过滤） |

- cct **proxy profile 之间**（都传 `model_provider=custom`）会话互相可见 ✓（用户场景 (a) 若为两个 proxy profile，应已可见）
- **proxy profile（custom）与直接运行（deepseek）/ subscription（openai）之间互相不可见** ✗ ← 用户感知的"隔离"主因
- 本机实测佐证：threads 268 行分属 openai/crs/clauddy/deepseek 四类；当前 config 为 deepseek，`codex resume` 只见 deepseek 4 条。

## 本地实测数据（2026-08-01，codex-cli 0.146.0）

- `~/.codex/sessions/` 全局（年/月/日分目录），rollout 首条 session_meta 含 `cwd`、`git{...}`、`model_provider`、`history_mode=legacy`，无 profile 字段
- threads 表：268 行；model_provider 分布 openai 239 / crs 20 / clauddy 5 / deepseek 4；`name` 字段全空
- session_index.jsonl：44 条（2026-06-05 ~ 07-31），仅 name 索引
- `~/.codex/config.toml` 当前 `model_provider = "deepseek"`（用户 08-01 手动安装，backup-deepseek/ 为安装备份）
