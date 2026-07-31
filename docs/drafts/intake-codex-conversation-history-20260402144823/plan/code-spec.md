---
title: "Code Spec: Codex history shared across profiles"
doc_type: proc
brief: "Step-by-step implementation: shared CODEX_HOME + --profile overlay + two-way binding + migration"
confidence: verified
yields_from:
  - spec.md
created: 2026-08-01
updated: 2026-08-01
revision: 1
---

# Code Spec

## Files Changed

| File | Change Type |
|------|-------------|
| src/launch.rs | Major edit |
| src/config.rs | Major edit |
| src/app.rs | Major edit |
| src/ui.rs | Major edit |
| src/main.rs | Minor edit |
| tests/ 契约测试（launch.rs tests 模块内） | Major edit |
| docs/modules/launch.md | Major edit |
| docs/references/codex-backend-development-guide.md | Major edit |
| docs/references/codex-home-storage-layout.md | Major edit |
| CLAUDE.md | Minor edit |

约束见 [constraints.md](constraints.md)，领域概念见 [domain-knowledge.md](domain-knowledge.md)，依赖关系见 [architecture.md](architecture.md)。

---

## Group A — 共享 HOME 启动链（src/launch.rs）

## Step 1 — 新增 `CodexLayout` 与 `resolve_codex_layout` 纯函数

**File**: `src/launch.rs`
**What**: 在 `exec_codex` 前新增布局结构体与纯路径解析，共享根不含 profile 名段。

**Old**:
```rust
/// Generate codex config, inject profile env vars, set CODEX_HOME, and exec-replace with `codex`.
pub fn exec_codex(profile: &Profile) -> anyhow::Error {
```

**New**:
```rust
/// Layout contract for the shared Codex home. Pure path resolution — no I/O.
#[derive(Debug, Clone, PartialEq)]
pub struct CodexLayout {
    /// Single shared CODEX_HOME for all Codex profiles.
    pub shared_home: PathBuf,
    /// Per-profile overlay: `shared_home/<name>.config.toml`, loaded via `--profile`.
    pub overlay_path: PathBuf,
    /// Legacy per-profile home from the old layout (migration source).
    pub legacy_home: PathBuf,
}

/// Resolve the shared Codex layout for a profile. Pure — no filesystem access.
pub fn resolve_codex_layout(profile_name: &str) -> CodexLayout {
    let shared_home = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("cc-tui")
        .join("codex");
    let overlay_path = shared_home.join(format!("{profile_name}.config.toml"));
    let legacy_home = shared_home.join(profile_name);
    CodexLayout {
        shared_home,
        overlay_path,
        legacy_home,
    }
}

/// Generate codex config, inject profile env vars, set CODEX_HOME, and exec-replace with `codex`.
pub fn exec_codex(profile: &Profile) -> anyhow::Error {
```

**Verify**: `cargo test resolve_codex_layout` — 新增两个测试通过：
- `resolve_codex_layout_returns_shared_and_overlay_paths`：overlay 含 `{name}.config.toml` 且共享根不含 name 段
- `resolve_codex_layout_keeps_profile_name_in_overlay_only`

## Step 2 — 新增 `HISTORY_ARTIFACTS`、`plan_migration`、`run_migration`

**File**: `src/launch.rs`
**What**: 迁移决策（纯读）与执行（写入）分离；marker `.cct-migrated-v1` 保证幂等；版本化 sqlite（state_/memories_/goals_ + -wal/-shm）按前缀匹配。

**Old**（Step 1 产物 `resolve_codex_layout` 尾部 + `exec_codex` 头 — 在 Step 1 应用后唯一）:
```rust
    CodexLayout {
        shared_home,
        overlay_path,
        legacy_home,
    }
}

/// Generate codex config, inject profile env vars, set CODEX_HOME, and exec-replace with `codex`.
pub fn exec_codex(profile: &Profile) -> anyhow::Error {
```

**New**:
```rust
/// History-bearing artifacts moved from a legacy per-profile home into the
/// shared home. Explicit list — anything not listed stays in the legacy dir.
pub const HISTORY_ARTIFACTS: &[&str] = &[
    "history.jsonl",
    "session_index.jsonl",
    "sessions",
    "archived_sessions",
    "memories",
    "sqlite",
];

/// Versioned sqlite state files (`state_5.sqlite`, `memories_1.sqlite`,
/// `goals_1.sqlite`) plus their `-wal`/`-shm` companions.
fn is_versioned_sqlite(name: &str) -> bool {
    ["state_", "memories_", "goals_"]
        .iter()
        .any(|p| {
            name.starts_with(p)
                && (name.ends_with(".sqlite")
                    || name.ends_with(".sqlite-wal")
                    || name.ends_with(".sqlite-shm"))
        })
}

/// Decide which history artifacts to move from a legacy home into the shared
/// home. Skips targets already present in the shared home (never overwrite).
/// Reads only — no writes.
pub fn plan_migration(legacy_home: &Path, shared_home: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut moves: Vec<(PathBuf, PathBuf)> = HISTORY_ARTIFACTS
        .iter()
        .map(|name| (legacy_home.join(name), shared_home.join(name)))
        .filter(|(from, to)| from.exists() && !to.exists())
        .collect();
    if let Ok(entries) = fs::read_dir(legacy_home) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_versioned_sqlite(&name) {
                let from = legacy_home.join(&name);
                let to = shared_home.join(&name);
                if !to.exists() {
                    moves.push((from, to));
                }
            }
        }
    }
    moves
}

/// Execute a migration plan for one profile and record it in the shared-home
/// marker. Idempotent: profiles already listed in `.cct-migrated-v1` are
/// skipped; a missing legacy home is a no-op.
pub fn run_migration(profile_name: &str, layout: &CodexLayout) -> Result<()> {
    let marker = layout.shared_home.join(".cct-migrated-v1");
    if marker.exists() {
        let recorded = fs::read_to_string(&marker)?;
        if recorded.lines().any(|l| l == profile_name) {
            return Ok(());
        }
    }
    if !layout.legacy_home.exists() {
        return Ok(());
    }
    for (from, to) in plan_migration(&layout.legacy_home, &layout.shared_home) {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(from, to)?;
    }
    fs::create_dir_all(&layout.shared_home)?;
    let mut recorded = if marker.exists() {
        fs::read_to_string(&marker)?
    } else {
        String::new()
    };
    recorded.push_str(profile_name);
    recorded.push('\n');
    fs::write(&marker, recorded)?;
    Ok(())
}

/// Generate codex config, inject profile env vars, set CODEX_HOME, and exec-replace with `codex`.
pub fn exec_codex(profile: &Profile) -> anyhow::Error {
```

**Verify**: `cargo test migration` — 新增测试（tempdir）：
- `plan_migration_lists_history_artifacts_only`：legacy 含 `config.toml`/`log/`/`history.jsonl`，plan 只列 `history.jsonl`
- `plan_migration_skips_existing_targets`：共享根已存在同名 → 不列
- `run_migration_moves_history_and_marks_profile`：移动 + marker 出现 profile 名
- `run_migration_is_idempotent`：二次调用不重复移动、不报错

## Step 3 — `generate_codex_config` 改为共享 base（去掉 profile 特定键）

**File**: `src/launch.rs`
**What**: base 只写 `model_provider` + features 默认；`model`/`model_providers.custom.*` 移入 overlay（Step 4）。签名去掉 `profile` 参数。**单个整函数替换**（旧函数含 doc 注释，当前文件 72-111 行）。

**Old**（整函数 — 含现有 doc 注释）:
```rust
/// Generate codex config.toml at a specified directory.
/// Content is derived from the profile's name, model, and base_url fields.
/// Existing files are merged surgically: only the keys cct owns are refreshed,
/// so user hand-edits ([features], [projects], …) survive the next launch.
pub fn generate_codex_config(profile: &Profile, codex_home: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(codex_home)?;
    let path = codex_home.join("config.toml");
    let model = profile.model.as_deref().unwrap_or("gpt-4.1");
    let name = &profile.name;
    let base_url = profile.base_url.as_deref().unwrap_or("");

    let mut doc = if path.exists() {
        fs::read_to_string(&path)?
            .parse::<toml_edit::DocumentMut>()
            .with_context(|| format!("parse existing codex config {path:?}"))?
    } else {
        toml_edit::DocumentMut::new()
    };

    doc["model_provider"] = toml_edit::value("custom");
    doc["model"] = toml_edit::value(model);
    let providers = ensure_subtable(doc.as_table_mut(), "model_providers");
    let custom = ensure_subtable(providers, "custom");
    custom["name"] = toml_edit::value(name);
    custom["base_url"] = toml_edit::value(base_url);
    custom["requires_openai_auth"] = toml_edit::value(true);
    // Issue #10: prefer asking before running commands in default mode. Only set
    // when absent so an explicit user override wins.
    if doc
        .get("features")
        .and_then(|f| f.get("default_mode_request_user_input"))
        .is_none()
    {
        let features = ensure_subtable(doc.as_table_mut(), "features");
        features["default_mode_request_user_input"] = toml_edit::value(true);
    }

    fs::write(&path, doc.to_string())?;
    Ok(())
}
```

**New**（整函数 — 新 doc 注释 + 新签名 + 只写公共键）:
```rust
/// Generate the shared codex base config.toml at the shared home.
/// Only cct-owned shared keys are written: `model_provider` and the
/// `[features] default_mode_request_user_input` default. Per-profile keys
/// (model, model_providers.custom.*) live in the `<name>.config.toml` overlay.
/// Existing files are merged surgically: user hand-edits survive the next launch.
pub fn generate_codex_config(codex_home: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(codex_home)?;
    let path = codex_home.join("config.toml");

    let mut doc = if path.exists() {
        fs::read_to_string(&path)?
            .parse::<toml_edit::DocumentMut>()
            .with_context(|| format!("parse existing codex config {path:?}"))?
    } else {
        toml_edit::DocumentMut::new()
    };

    doc["model_provider"] = toml_edit::value("custom");
    // Issue #10: prefer asking before running commands in default mode. Only set
    // when absent so an explicit user override wins.
    if doc
        .get("features")
        .and_then(|f| f.get("default_mode_request_user_input"))
        .is_none()
    {
        let features = ensure_subtable(doc.as_table_mut(), "features");
        features["default_mode_request_user_input"] = toml_edit::value(true);
    }

    fs::write(&path, doc.to_string())?;
    Ok(())
}
```

**Verify**: `cargo test generate_codex_config` — 更新现有 5 个测试为新签名（`generate_codex_config(tmp.path())`），断言：含 `model_provider = "custom"`、features 默认、用户 `[projects]` 保留、显式 `default_mode_request_user_input = false` 尊重、转义。**不再断言** `model`/`model_providers.custom` 在 base 中。

## Step 4 — 新增 `write_codex_profile_overlay`

**File**: `src/launch.rs`
**What**: 写 `<name>.config.toml` 叠加层：model、model_providers.custom.{name, base_url, env_key}；surgical merge；无 `requires_openai_auth`。

**Old**:
```rust
/// Write `{codex_home}/auth.json` with the OPENAI_API_KEY from the profile's env.
/// If OPENAI_API_KEY is absent, a stale auth.json from an earlier launch is
/// removed so an old key is not served after the profile's key is dropped.
pub fn write_codex_auth(profile: &Profile, codex_home: &Path) -> Result<()> {
```

**New**:
```rust
/// Write the per-profile overlay `<name>.config.toml` in the shared codex home.
/// cct-owned keys (model, model_providers.custom.{name,base_url,env_key}) are
/// refreshed from the profile; user hand-edits to other keys survive via
/// surgical toml_edit merge. The API key is read from the environment
/// (`OPENAI_API_KEY` injected by exec_codex) via `env_key` — no auth.json.
pub fn write_codex_profile_overlay(profile: &Profile, layout: &CodexLayout) -> anyhow::Result<()> {
    fs::create_dir_all(&layout.shared_home)?;
    let path = &layout.overlay_path;
    let model = profile.model.as_deref().unwrap_or("gpt-4.1");
    let base_url = profile.base_url.as_deref().unwrap_or("");

    let mut doc = if path.exists() {
        fs::read_to_string(path)?
            .parse::<toml_edit::DocumentMut>()
            .with_context(|| format!("parse existing codex overlay {path:?}"))?
    } else {
        toml_edit::DocumentMut::new()
    };

    doc["model"] = toml_edit::value(model);
    let providers = ensure_subtable(doc.as_table_mut(), "model_providers");
    let custom = ensure_subtable(providers, "custom");
    custom["name"] = toml_edit::value(&profile.name);
    custom["base_url"] = toml_edit::value(base_url);
    custom["env_key"] = toml_edit::value("OPENAI_API_KEY");

    fs::write(path, doc.to_string())?;
    Ok(())
}

/// Write `{codex_home}/auth.json` with the OPENAI_API_KEY from the profile's env.
/// If OPENAI_API_KEY is absent, a stale auth.json from an earlier launch is
/// removed so an old key is not served after the profile's key is dropped.
pub fn write_codex_auth(profile: &Profile, codex_home: &Path) -> Result<()> {
```

**Verify**: `cargo test overlay` — 新增测试：
- `write_codex_profile_overlay_writes_model_provider_env_key`：含 model/base_url/env_key，**不含** `requires_openai_auth`
- `write_codex_profile_overlay_preserves_user_edits`：用户手写 `[features]` 保留，cct 键刷新
- `write_codex_profile_overlay_escapes_profile_values`：引号/反斜杠不破坏 TOML

## Step 5 — `exec_codex` 编排改共享 home + 迁移 + overlay

**File**: `src/launch.rs`
**What**: 替换函数体：layout 解析 → run_migration → base config → overlay → CODEX_HOME 共享根 → env → args → exec。**此时 `write_codex_auth` 调用被移除但函数本体保留**（Step 7 删除；避免中间态编译断裂）。

**Old**:
```rust
pub fn exec_codex(profile: &Profile) -> anyhow::Error {
    if !check_codex_installed() {
        return anyhow::anyhow!(
            "codex CLI not found in PATH. Install it first: npm install -g @openai/codex"
        );
    }
    let codex_home = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("cc-tui")
        .join("codex")
        .join(&profile.name);
    if let Err(e) = generate_codex_config(profile, &codex_home) {
        return anyhow::anyhow!("failed to generate codex config: {e}");
    }
    if let Err(e) = write_codex_auth(profile, &codex_home) {
        return anyhow::anyhow!("failed to write codex auth: {e}");
    }
    env::set_var("CODEX_HOME", &codex_home);
    if let Some(env_map) = &profile.env {
        for (k, v) in env_map {
            env::set_var(k, v);
        }
    }
    let args = build_codex_args(profile);
    let err = Command::new("codex").args(&args).exec();
    anyhow::anyhow!("exec codex: {err}")
}
```

**New**:
```rust
pub fn exec_codex(profile: &Profile) -> anyhow::Error {
    if !check_codex_installed() {
        return anyhow::anyhow!(
            "codex CLI not found in PATH. Install it first: npm install -g @openai/codex"
        );
    }
    let layout = resolve_codex_layout(&profile.name);
    if let Err(e) = run_migration(&profile.name, &layout) {
        return anyhow::anyhow!("failed to migrate legacy codex home: {e}");
    }
    if let Err(e) = generate_codex_config(&layout.shared_home) {
        return anyhow::anyhow!("failed to generate codex config: {e}");
    }
    if let Err(e) = write_codex_profile_overlay(profile, &layout) {
        return anyhow::anyhow!("failed to write codex profile overlay: {e}");
    }
    env::set_var("CODEX_HOME", &layout.shared_home);
    if let Some(env_map) = &profile.env {
        for (k, v) in env_map {
            env::set_var(k, v);
        }
    }
    let args = build_codex_args(profile);
    let err = Command::new("codex").args(&args).exec();
    anyhow::anyhow!("exec codex: {err}")
}
```

**Verify**: `cargo build` 通过（`write_codex_auth` 函数仍在但不再被调用——pub 函数无 unused 警告）；`cargo test` 全绿（含旧契约测试 `update_codex_api_key_reaches_auth`，它仍引用 auth 函数）。

## Step 6 — `build_codex_args` 增加 `--profile <name>`

**File**: `src/launch.rs`
**What**: args 以 `--profile <name>` 开头，后接 full_auto/extra_args（顺序不变）。

**Old**:
```rust
pub fn build_codex_args(profile: &Profile) -> Vec<String> {
    let mut args = Vec::new();
    if profile.full_auto.unwrap_or(false) {
```

**New**:
```rust
pub fn build_codex_args(profile: &Profile) -> Vec<String> {
    let mut args = Vec::new();
    args.push("--profile".to_string());
    args.push(profile.name.clone());
    if profile.full_auto.unwrap_or(false) {
```

**Verify**: `cargo test build_codex_args` — 更新 5 个现有测试：期望值前插 `["--profile", "test"]`（`build_codex_args_empty` 改为 `vec!["--profile", "test"]`）；`build_launch_command_dispatches_codex` 断言同步更新。

## Step 7 — 删除 `write_codex_auth` 函数与其全部引用测试

**File**: `src/launch.rs`
**What**: 删除 `write_codex_auth` 完整函数（doc 注释 + 签名 + 函数体，当前文件 123-145 行）与 8 个引用它的测试：7 个直接单测（`exec_codex_calls_write_auth`、`write_codex_auth_overwrites_existing`、`write_codex_auth_skips_when_no_key`、`write_codex_auth_writes_correct_json`、`write_codex_auth_honors_top_level_api_key`、`write_codex_auth_removes_stale_auth_when_key_absent`、`write_codex_auth_escapes_special_chars`）与契约测试 `update_codex_api_key_reaches_auth`（Step 13 重建为新形态）。**执行本步前先确认 Step 5 已完成**（exec_codex 不再调用该函数）。

**Old**（函数删除 — 当前文件 123-145 行全文）:
```rust
/// Write `{codex_home}/auth.json` with the OPENAI_API_KEY from the profile's env.
/// If OPENAI_API_KEY is absent, a stale auth.json from an earlier launch is
/// removed so an old key is not served after the profile's key is dropped.
pub fn write_codex_auth(profile: &Profile, codex_home: &Path) -> Result<()> {
    let auth_path = codex_home.join("auth.json");
    let key = profile.env.as_ref().and_then(|m| m.get("OPENAI_API_KEY"));
    match key {
        Some(api_key) => {
            fs::create_dir_all(codex_home)?;
            let json = serde_json::json!({
                "auth_mode": "apikey",
                "OPENAI_API_KEY": api_key,
            });
            fs::write(auth_path, serde_json::to_string_pretty(&json)?)?;
        }
        None => {
            if auth_path.exists() {
                fs::remove_file(auth_path)?;
            }
        }
    }
    Ok(())
}
```

**New**:
```rust
```

**Old**（测试删除 — **按名逐一删除**以下 8 个 `#[test]` 块，切勿按行范围批量删（`generate_codex_config_*` 测试交错其间必须保留）；首个锚 `exec_codex_calls_write_auth`）:
```rust
    #[test]
    fn exec_codex_calls_write_auth() {
```

**New**:
```rust
    // write_codex_auth removed in shared-home design; see Step 13 for the
    // rebuilt contract tests (key → env, overlay round-trip).
```

删除名单（8 个）：`exec_codex_calls_write_auth`、`write_codex_auth_overwrites_existing`、`write_codex_auth_skips_when_no_key`、`write_codex_auth_writes_correct_json`、`write_codex_auth_honors_top_level_api_key`、`write_codex_auth_removes_stale_auth_when_key_absent`、`write_codex_auth_escapes_special_chars`、`update_codex_api_key_reaches_auth`。保留：全部 `generate_codex_config_*`、`update_codex_model_reaches_config`。

**Verify**: `cargo build && cargo test` 全绿；`rg "write_codex_auth" src/` 空。

---

## Group B — 双向绑定冲突对话框（launch.rs + config.rs + app.rs + ui.rs + main.rs）

## Step 8 — config.rs 新增 `KeyDiff` 与 `apply_overlay_winner`

**File**: `src/config.rs`
**What**: `KeyDiff` 结构（双向绑定两侧取值）+ 落盘胜出回写：surgical toml_edit 只改分歧键（model/base_url），复用 `set_optional_string`。**先于 Step 9 定义**（Step 9 的 diff 函数返回 `KeyDiff`）。

**Old**:
```rust
#[derive(Debug, Clone)]
pub struct Profile {
```

**New**:
```rust
/// One divergent cct-owned key between the on-disk Codex overlay and
/// profiles.toml (two-way binding). `None` means the side has no value.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyDiff {
    pub key: String,
    pub profiles_value: Option<String>,
    pub overlay_value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Profile {
```

**Old**（`update_profile` 之后追加）:
```rust
pub fn append_profile(profile: &NewProfile) -> Result<()> {
```

**New**:
```rust
/// Write the on-disk overlay values back into profiles.toml — the "on-disk
/// wins" direction of two-way binding. Surgical toml_edit: only the divergent
/// keys (model/base_url) are touched; comments and other fields are preserved.
pub fn apply_overlay_winner(original_name: &str, diffs: &[KeyDiff]) -> Result<()> {
    let path = config_path();
    let content = fs::read_to_string(&path).with_context(|| format!("read config {path:?}"))?;
    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .with_context(|| format!("parse TOML in {path:?}"))?;

    let profiles = doc
        .get_mut("profiles")
        .and_then(|v| v.as_array_of_tables_mut())
        .with_context(|| "no [[profiles]] array in config")?;

    let entry = profiles
        .iter_mut()
        .find(|t| t.get("name").and_then(|v| v.as_str()) == Some(original_name))
        .with_context(|| format!("profile {original_name:?} not found in config"))?;

    for diff in diffs {
        match diff.key.as_str() {
            "model" => set_optional_string(entry, "model", diff.overlay_value.as_deref()),
            "base_url" => set_optional_string(entry, "base_url", diff.overlay_value.as_deref()),
            _ => {} // unknown key — ignore
        }
    }

    fs::write(&path, doc.to_string()).with_context(|| format!("write config {path:?}"))?;
    Ok(())
}

pub fn append_profile(profile: &NewProfile) -> Result<()> {
```

**Verify**: `cargo test apply_overlay_winner` — 新增测试（`CCT_CONFIG` tempdir + `#[serial]`）：
- `apply_overlay_winner_writes_model_and_base_url`：profiles.toml 更新为 overlay 值
- `apply_overlay_winner_preserves_other_fields`：description/注释保留
- `apply_overlay_winner_unknown_key_ignored`

## Step 9 — 新增 `diff_cct_owned_keys` 与 `read_overlay_diffs`

**File**: `src/launch.rs`
**What**: 纯函数对比 overlay 的 cct-owned 键（model、base_url）与 profile 当前值；默认值规则与 `write_codex_profile_overlay` 一致（model→gpt-4.1、base_url→""）避免误报；`name`/`env_key` 是固定派生物不参与 diff。`read_overlay_diffs` 读盘（overlay 不存在 → `None` = 无冲突）。**依赖 Step 8 的 `KeyDiff`；import 更新为 `use crate::config::{KeyDiff, Profile};`**。

**Old**:
```rust
/// Build the CLI argument list for `codex` from a profile. Pure — no side effects.
pub fn build_codex_args(profile: &Profile) -> Vec<String> {
```

**New**:
```rust
/// Compare the cct-owned overlay keys (`model`, `base_url`) against the
/// current profiles.toml values. Pure. Defaults match write_codex_profile_overlay
/// (model → gpt-4.1, base_url → "") so untouched overlays never diverge.
/// `name`/`env_key` are fixed derivations with no user-editable meaning — not diffed.
pub fn diff_cct_owned_keys(profile: &Profile, overlay: &toml_edit::DocumentMut) -> Vec<KeyDiff> {
    let profiles_model = profile.model.as_deref().unwrap_or("gpt-4.1");
    let profiles_base_url = profile.base_url.as_deref().unwrap_or("");
    let overlay_model = overlay.get("model").and_then(|v| v.as_str());
    let overlay_base_url = overlay
        .get("model_providers")
        .and_then(|p| p.get("custom"))
        .and_then(|c| c.get("base_url"))
        .and_then(|v| v.as_str());

    let mut diffs = Vec::new();
    if overlay_model != Some(profiles_model) {
        diffs.push(KeyDiff {
            key: "model".into(),
            profiles_value: Some(profiles_model.into()),
            overlay_value: overlay_model.map(Into::into),
        });
    }
    if overlay_base_url != Some(profiles_base_url) {
        diffs.push(KeyDiff {
            key: "base_url".into(),
            profiles_value: Some(profiles_base_url.into()),
            overlay_value: overlay_base_url.map(Into::into),
        });
    }
    diffs
}

/// Read the on-disk overlay and diff cct-owned keys against the profile.
/// Returns None when the overlay does not exist yet (first launch — nothing to
/// conflict with).
pub fn read_overlay_diffs(profile: &Profile, layout: &CodexLayout) -> Option<Vec<KeyDiff>> {
    let text = fs::read_to_string(&layout.overlay_path).ok()?;
    let doc = text.parse::<toml_edit::DocumentMut>().ok()?;
    Some(diff_cct_owned_keys(profile, &doc))
}

/// "On-disk wins" direction of two-way binding: write the divergent overlay
/// values into profiles.toml, **reload the profile from disk**, and regenerate
/// the overlay from the fresh profile. Returns the reloaded profile so the
/// caller can exec with it. Never regenerates from a stale in-memory profile.
pub fn apply_on_disk_winner(original_name: &str, diffs: &[KeyDiff]) -> Result<Profile> {
    crate::config::apply_overlay_winner(original_name, diffs)?;
    let profile = crate::config::load_profiles()?
        .into_iter()
        .find(|p| p.name == original_name)
        .ok_or_else(|| anyhow::anyhow!("profile {original_name:?} not found after write-back"))?;
    let layout = resolve_codex_layout(&profile.name);
    write_codex_profile_overlay(&profile, &layout)?;
    Ok(profile)
}

/// Build the CLI argument list for `codex` from a profile. Pure — no side effects.
pub fn build_codex_args(profile: &Profile) -> Vec<String> {
```

**Verify**: `cargo test diff_cct_owned_keys` + `cargo test apply_on_disk_winner` — 新增测试：
- `diff_cct_owned_keys_reports_model_divergence`：overlay `model = "gpt-5.6"` vs profile `gpt-4.1` → 1 个 KeyDiff
- `diff_cct_owned_keys_is_empty_when_in_sync`：按 overlay 生成规则构造一致 doc → 空
- `diff_cct_owned_keys_ignores_fixed_keys`：手改 `env_key`/`name` → 空
- `read_overlay_diffs_none_when_missing`：overlay 不存在 → None
- `apply_on_disk_winner_reloads_and_regenerates`（`CCT_CONFIG` tempdir + `#[serial]`）：profiles.toml 更新为 overlay 值、返回的 Profile 含新值、overlay 已用新值重生成（`read_overlay_diffs` 返回空）

## Step 10 — app.rs 新增 `ConflictState` 与 `AppMode::ConflictConfirm`

**File**: `src/app.rs`
**What**: 新模式变体持有 profile 索引 + diffs；`enter_conflict` 便捷方法。**同时更新 import**：`use crate::config::{Backend, NewProfile, Profile, KeyDiff};`。

**Old**:
```rust
use crate::config::{Backend, NewProfile, Profile};

pub fn field_labels(backend: &Backend) -> [&'static str; 6] {
```

**New**:
```rust
use crate::config::{Backend, KeyDiff, NewProfile, Profile};

pub fn field_labels(backend: &Backend) -> [&'static str; 6] {
```

**Old**:
```rust
pub enum AppMode {
    Normal,
    AddForm(Box<FormState>),
}
```

**New**:
```rust
pub enum AppMode {
    Normal,
    AddForm(Box<FormState>),
    /// Two-way binding: on-disk Codex overlay diverges from profiles.toml.
    ConflictConfirm(ConflictState),
}

/// State for the conflict dialog. `profile_idx` indexes `App.profiles`.
pub struct ConflictState {
    pub profile_idx: usize,
    pub diffs: Vec<KeyDiff>,
}
```

**Old**（AppMode 定义之后）:
```rust
pub struct FormState {
```

**New**:
```rust
impl AppMode {
    /// Enter the conflict dialog for a profile's divergent cct-owned keys.
    pub fn enter_conflict(profile_idx: usize, diffs: Vec<KeyDiff>) -> Self {
        AppMode::ConflictConfirm(ConflictState { profile_idx, diffs })
    }
}

pub struct FormState {
```

**Verify**: `cargo check` 预期编译失败（ui.rs/main.rs 的 `match &app.mode` 非穷尽——新增变体未覆盖，编译器报 `non-exhaustive` 错误）；Step 11/12 补齐分支后 `cargo build && cargo test` 通过。

## Step 11 — ui.rs 渲染冲突对话框 + footer 键提示

**File**: `src/ui.rs`
**What**: detail panel 新增 `ConflictConfirm` 分支（逐键显示两侧值 + 操作提示）；footer 加 `[p]`/`[d]`/`[Esc]`。

**Old**:
```rust
        AppMode::Normal => {
            let detail_lines = if app.profiles.is_empty() {
```

**New**:
```rust
        AppMode::ConflictConfirm(state) => {
            let mut lines: Vec<Line<'static>> = vec![
                Line::from("  Codex config diverges from profiles.toml:"),
                Line::from(""),
            ];
            for d in &state.diffs {
                lines.push(Line::from(format!(
                    "  {}: profiles.toml = {:?}   on-disk = {:?}",
                    d.key, d.profiles_value, d.overlay_value
                )));
            }
            lines.push(Line::from(""));
            lines.push(Line::from("  [p] profiles.toml wins (regenerate overlay)"));
            lines.push(Line::from("  [d] on-disk wins (write back to profiles.toml)"));
            lines.push(Line::from("  [Esc] back"));
            let detail = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(" Config Conflict "))
                .wrap(Wrap { trim: false });
            frame.render_widget(detail, content[1]);
        }
        AppMode::Normal => {
            let detail_lines = if app.profiles.is_empty() {
```

**Old**（footer match）:
```rust
        AppMode::AddForm(_) => {
            " [Tab/↓] Next field  [Shift-Tab/↑] Prev  [Enter] Confirm  [Esc] Cancel"
        }
```

**New**:
```rust
        AppMode::AddForm(_) => {
            " [Tab/↓] Next field  [Shift-Tab/↑] Prev  [Enter] Confirm  [Esc] Cancel"
        }
        AppMode::ConflictConfirm(_) => " [p] profiles.toml wins  [d] on-disk wins  [Esc] Back",
```

**Verify**: `cargo test` — ui.rs 现有 footer/渲染测试更新：新增 `conflict_confirm_renders_both_values`（footer 含 `[p]`、`[d]`；detail 含 `profiles.toml` 与 `on-disk` 字样）。若 ui.rs 无测试设施，则把断言收敛到 footer 文本函数（按 ui.rs 现有测试风格）。

## Step 12 — main.rs Enter 预检 + ConflictConfirm 按键分发

**File**: `src/main.rs`
**What**: Enter 时 Codex profile 先 `read_overlay_diffs`，有分歧进对话框（不 launch）；`p` 重生成 overlay 后 launch，`d` 回写 profiles.toml 后重生成 + launch；`Esc` 返回 Normal。launch 路径统一 `restore_terminal`。

**Old**:
```rust
                    (KeyCode::Enter, _) if !app.profiles.is_empty() => {
                        launch::restore_terminal();
                        let profile = &app.profiles[app.selected];
                        let err = match profile.backend {
                            config::Backend::Claude => launch::exec_claude(profile, false),
                            config::Backend::Codex => launch::exec_codex(profile),
                        };
                        eprintln!("Error: {err:#}");
                        std::process::exit(1);
                    }
```

**New**:
```rust
                    (KeyCode::Enter, _) if !app.profiles.is_empty() => {
                        let profile = &app.profiles[app.selected];
                        if profile.backend == config::Backend::Codex {
                            let layout = launch::resolve_codex_layout(&profile.name);
                            if let Some(diffs) = launch::read_overlay_diffs(profile, &layout) {
                                if !diffs.is_empty() {
                                    app.mode = AppMode::enter_conflict(app.selected, diffs);
                                    continue;
                                }
                            }
                        }
                        launch_and_exit(profile);
                    }
```

**Old**（AddForm match 之后，新增冲突分支 —— 用 `AppMode::AddForm(form) =>` 的收尾做锚）:
```rust
                },
                AppMode::AddForm(form) => {
```

**New**:
```rust
                AppMode::ConflictConfirm(state) => match key.code {
                    KeyCode::Char('p') => {
                        let profile = &app.profiles[state.profile_idx];
                        let layout = launch::resolve_codex_layout(&profile.name);
                        if let Err(e) = launch::write_codex_profile_overlay(profile, &layout) {
                            eprintln!("Error: {e:#}");
                            app.mode = AppMode::Normal;
                        } else {
                            launch_and_exit(profile);
                        }
                    }
                    KeyCode::Char('d') => {
                        // On-disk wins: apply_on_disk_winner writes the overlay
                        // values into profiles.toml, RELOADS the profile from
                        // disk, and regenerates the overlay from the fresh
                        // profile — never from the stale in-memory one.
                        let name = app.profiles[state.profile_idx].name.clone();
                        match launch::apply_on_disk_winner(&name, &state.diffs) {
                            Ok(profile) => launch_and_exit(&profile),
                            Err(e) => {
                                eprintln!("Error: {e:#}");
                                app.mode = AppMode::Normal;
                            }
                        }
                    }
                    KeyCode::Esc => app.mode = AppMode::Normal,
                    _ => {}
                },
                AppMode::AddForm(form) => {
```

**New**（helper，`fn main` 前新增）:
```rust
/// Restore the terminal, exec-replace with the selected profile's backend,
/// and exit on error. No return path on success.
fn launch_and_exit(profile: &config::Profile) -> ! {
    launch::restore_terminal();
    let err = match profile.backend {
        config::Backend::Claude => launch::exec_claude(profile, false),
        config::Backend::Codex => launch::exec_codex(profile),
    };
    eprintln!("Error: {err:#}");
    std::process::exit(1);
}
```

**Verify**: `cargo build` + `cargo test` 全绿；`rg "launch_and_exit" src/main.rs` 有 3 处调用（Enter/Claude continue 键保留原 `exec_claude(profile, true)` 不动，除非顺手统一——不改，KISS）；冲突分发行为由 Step 9/8 的单测与 Group C 契约测试覆盖。

---

## Group C — 契约测试与文档

## Step 13 — 改造契约测试（update_profile → overlay 产物 + 回写闭环）

**File**: `src/launch.rs`（tests 模块）
**What**: `update_codex_api_key_reaches_auth` 改为验证 key 进入 overlay 的 `env_key` 机制（auth.json 不再生成）；`update_codex_model_reaches_config` 改为验证 `model`/`base_url` 落在 overlay；新增回写闭环测试。

**注意**：`update_codex_api_key_reaches_auth` 已在 Step 7 删除——本编辑是 **INSERT 完整新测试**（含全部 setup），锚定存活文本 `update_codex_model_reaches_config` 的 doc 注释之前。

**Old**（存活锚点 — `update_codex_model_reaches_config` doc 注释）:
```rust
    /// Editing a codex model via config::update_profile must be reflected in
    /// the config.toml generated at launch time.
    #[test]
    #[serial]
    fn update_codex_model_reaches_config() {
```

**New**（完整 INSERT — 两个新测试 + 保留原锚点）:
```rust
    /// Editing a codex api key via config::update_profile (the TUI edit flow)
    /// must land in profile env so the overlay's env_key mechanism serves it.
    #[test]
    #[serial]
    fn update_codex_api_key_reaches_env() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(
            &path,
            "[[profiles]]\nname = \"codex-1\"\nbackend = \"codex\"\n\n[profiles.env]\nOPENAI_API_KEY = \"sk-old\"\n",
        )
        .unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let updated = crate::config::NewProfile {
            name: "codex-1".into(),
            description: None,
            base_url: None,
            api_key: Some("sk-new".into()),
            model: None,
            fast_model: None,
            backend: crate::config::Backend::Codex,
            full_auto: None,
            auth_type: None,
        };
        crate::config::update_profile("codex-1", &updated).unwrap();
        let profiles = crate::config::load_profiles().unwrap();
        std::env::remove_var("CCT_CONFIG");

        let p = profiles.iter().find(|p| p.name == "codex-1").unwrap();
        assert_eq!(
            p.env.as_ref().and_then(|m| m.get("OPENAI_API_KEY")),
            Some(&"sk-new".to_string())
        );
    }

    /// Editing a codex model via config::update_profile must be reflected in
    /// the config.toml generated at launch time.
    #[test]
    #[serial]
    fn update_codex_model_reaches_config() {
```

**Old**（锚含函数体首行以区别于上一编辑的插入锚 — `update_codex_model_reaches_config` doc 注释 + 签名 + 函数体开头）:
```rust
    /// Editing a codex model via config::update_profile must be reflected in
    /// the config.toml generated at launch time.
    #[test]
    #[serial]
    fn update_codex_model_reaches_config() {
        let dir = tempfile::tempdir().unwrap();
```

**New**:
```rust
    /// Editing a codex model via config::update_profile must be reflected in
    /// the overlay generated at launch time.
    #[test]
    #[serial]
    fn update_codex_model_reaches_overlay() {
        let dir = tempfile::tempdir().unwrap();
```

**Old**（`update_codex_model_reaches_config` 函数体尾部 — 当前文件 833-847 行，从 `let p = ...` 到函数收尾）:
```rust
        let p = profiles.iter().find(|p| p.name == "codex-1").unwrap();
        let tmp = tempfile::tempdir().unwrap();
        generate_codex_config(p, tmp.path()).unwrap();
        let doc = std::fs::read_to_string(tmp.path().join("config.toml"))
            .unwrap()
            .parse::<toml_edit::DocumentMut>()
            .unwrap();
        assert_eq!(doc["model"].as_str().unwrap(), "gpt-5");
        assert_eq!(
            doc["model_providers"]["custom"]["base_url"]
                .as_str()
                .unwrap(),
            "https://new.example.com/v1"
        );
    }
```

**New**（函数体尾部替换；**用 tempdir 构造 `CodexLayout`，不调 `resolve_codex_layout`**，避免测试写入真实共享 home）:
```rust
        let p = profiles.iter().find(|p| p.name == "codex-1").unwrap();
        let layout = CodexLayout {
            shared_home: dir.path().join("codex"),
            overlay_path: dir.path().join("codex").join("codex-1.config.toml"),
            legacy_home: dir.path().join("codex").join("codex-1"),
        };
        write_codex_profile_overlay(p, &layout).unwrap();
        let doc = std::fs::read_to_string(&layout.overlay_path)
            .unwrap()
            .parse::<toml_edit::DocumentMut>()
            .unwrap();
        assert_eq!(doc["model"].as_str().unwrap(), "gpt-5");
        assert_eq!(
            doc["model_providers"]["custom"]["base_url"]
                .as_str()
                .unwrap(),
            "https://new.example.com/v1"
        );
    }

    /// Two-way binding closed loop: on-disk wins → profiles.toml updated →
    /// next diff is empty.
    #[test]
    #[serial]
    fn overlay_winner_writeback_closes_diff() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(
            &path,
            "[[profiles]]\nname = \"codex-1\"\nbackend = \"codex\"\nmodel = \"gpt-4.1\"\n",
        )
        .unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let profiles = crate::config::load_profiles().unwrap();
        let p = profiles.iter().find(|p| p.name == "codex-1").unwrap();
        // tempdir-based layout — never touch the real shared home
        let layout = CodexLayout {
            shared_home: dir.path().join("codex"),
            overlay_path: dir.path().join("codex").join("codex-1.config.toml"),
            legacy_home: dir.path().join("codex").join("codex-1"),
        };
        write_codex_profile_overlay(p, &layout).unwrap();

        // user hand-edits the overlay
        let mut doc = std::fs::read_to_string(&layout.overlay_path)
            .unwrap()
            .parse::<toml_edit::DocumentMut>()
            .unwrap();
        doc["model"] = toml_edit::value("gpt-5.6");
        std::fs::write(&layout.overlay_path, doc.to_string()).unwrap();

        let diffs = read_overlay_diffs(p, &layout).unwrap();
        assert_eq!(diffs.len(), 1);
        crate::config::apply_overlay_winner("codex-1", &diffs).unwrap();

        let profiles = crate::config::load_profiles().unwrap();
        std::env::remove_var("CCT_CONFIG");
        let p = profiles.iter().find(|p| p.name == "codex-1").unwrap();
        assert_eq!(p.model.as_deref(), Some("gpt-5.6"));

        // regenerate overlay → diff empty
        let layout = CodexLayout {
            shared_home: dir.path().join("codex"),
            overlay_path: dir.path().join("codex").join("codex-1.config.toml"),
            legacy_home: dir.path().join("codex").join("codex-1"),
        };
        write_codex_profile_overlay(p, &layout).unwrap();
        assert!(read_overlay_diffs(p, &layout).unwrap().is_empty());
    }
```

**Verify**: `cargo test` — 全部契约测试通过（`update_codex_api_key_reaches_env`、`update_codex_model_reaches_overlay`、`overlay_winner_writeback_closes_diff`）。

## Step 14 — 更新文档（6 个文件，含 ARCHITECTURE.md / README.md）

**File**: `docs/modules/launch.md`、`docs/references/codex-backend-development-guide.md`、`docs/references/codex-home-storage-layout.md`、`CLAUDE.md`、`ARCHITECTURE.md`、`README.md`
**What**: 描述新布局（共享 home + overlay + `--profile`）、新函数（resolve_codex_layout、write_codex_profile_overlay、diff_cct_owned_keys、apply_overlay_winner、plan/run_migration）、`write_codex_auth` 移除、双向绑定行为、迁移行为。按 `docs/rules/update-docs-after-new-feature.md` 审计：ARCHITECTURE.md 的 "Main Use Case — Launch Codex Profile"（含 `generate_codex_config(&profile, codex_home)` 与 per-profile CODEX_HOME 描述）必须改为共享 home 流程；README.md 若含 Codex 布局/特性描述同步更新。

**Verify**: `rg "write_codex_auth" docs/ ARCHITECTURE.md README.md` 空；`rg -i "history is isolated per profile|per-profile.*codex home|codex/.*profile.name" ARCHITECTURE.md docs/ README.md CLAUDE.md` 无残留旧布局表述；6 个文件均含 `overlay` 与 `--profile` 关键词；`CLAUDE.md` 的模块描述表与 key design choices 已更新。

---

## 终态步骤

## Step 15 — Proof-Read End-to-End

Read each changed file in full. Check: formatting (`cargo fmt`), no leftover TODOs, spec intent preserved（[spec.md](../spec.md)）。

## Step 16 — Cross-Check Acceptance Criteria

| Criterion (constraints.md) | Addressed in Step |
|-----------|------------------|
| 1 同一历史（共享 CODEX_HOME） | Step 1, 5 |
| 2 per-profile model/provider/base_url 隔离 | Step 4, 6 |
| 3 双向绑定冲突对话框 + 回写 | Step 8, 9, 10, 11, 12 |
| 4 迁移一次 + 不覆盖 | Step 2 |
| 5 auth 走 env_key、auth.json 不写 | Step 3, 4, 7 |
| 6 自动化测试覆盖 | Step 1-13 |
| 7 文档更新 | Step 14 |

## Step 17 — Review

Follow the self-review checklist in [verification.md](verification.md). Writes `review.md`（本 plan 目录）。

## Step 18 — Commit

Use /commit. Suggested message:
```
feat(codex): share CODEX_HOME across profiles with two-way binding (#9)

- resolve_codex_layout + overlay via official --profile mechanism
- diff_cct_owned_keys + ConflictConfirm dialog (p/d) for two-way binding
- plan/run_migration moves legacy per-profile history once
- drop write_codex_auth; API key flows via model_providers.custom.env_key
- update docs (launch.md, codex-backend-development-guide.md, codex-home-storage-layout.md)
```

## Execution Order

```yaml
steps:
  - id: 1
    title: "新增 CodexLayout 与 resolve_codex_layout 纯函数"
    files: ["src/launch.rs"]
    depends_on: []
  - id: 2
    title: "新增 HISTORY_ARTIFACTS、plan_migration、run_migration"
    files: ["src/launch.rs"]
    depends_on: [1]
  - id: 3
    title: "generate_codex_config 改为共享 base（去掉 profile 特定键）"
    files: ["src/launch.rs"]
    depends_on: [1]
  - id: 4
    title: "新增 write_codex_profile_overlay"
    files: ["src/launch.rs"]
    depends_on: [3]
  - id: 5
    title: "exec_codex 编排改共享 home + 迁移 + overlay"
    files: ["src/launch.rs"]
    depends_on: [2, 3, 4]
  - id: 6
    title: "build_codex_args 增加 --profile <name>"
    files: ["src/launch.rs"]
    depends_on: [1]
  - id: 7
    title: "删除 write_codex_auth 函数与其全部引用测试"
    files: ["src/launch.rs"]
    depends_on: [5]
  - id: 8
    title: "config.rs 新增 KeyDiff 与 apply_overlay_winner"
    files: ["src/config.rs"]
    depends_on: []
  - id: 9
    title: "新增 diff_cct_owned_keys 与 read_overlay_diffs"
    files: ["src/launch.rs"]
    depends_on: [4, 8]
  - id: 10
    title: "app.rs 新增 ConflictState 与 AppMode::ConflictConfirm"
    files: ["src/app.rs"]
    depends_on: [8]
  - id: 11
    title: "ui.rs 渲染冲突对话框 + footer 键提示"
    files: ["src/ui.rs"]
    depends_on: [10]
  - id: 12
    title: "main.rs Enter 预检 + ConflictConfirm 按键分发"
    files: ["src/main.rs"]
    depends_on: [9, 10, 11]
  - id: 13
    title: "改造契约测试（update_profile → overlay 产物 + 回写闭环）"
    files: ["src/launch.rs"]
    depends_on: [8, 9]
  - id: 14
    title: "更新文档（6 个文件，含 ARCHITECTURE.md / README.md）"
    files: ["docs/modules/launch.md", "docs/references/codex-backend-development-guide.md", "docs/references/codex-home-storage-layout.md", "CLAUDE.md", "ARCHITECTURE.md", "README.md"]
    depends_on: [7, 12]
  - id: 15
    title: "Proof-Read End-to-End"
    files: []
    depends_on: [14]
  - id: 16
    title: "Cross-Check Acceptance Criteria"
    files: []
    depends_on: [15]
  - id: 17
    title: "Review"
    files: ["plan/review.md"]
    depends_on: [16]
  - id: 18
    title: "Commit"
    files: []
    depends_on: [17]
```

Step 1 → 2 → ... → 18 线性推进；Step 8（config.rs）与 Step 5/6/7（launch.rs）无相互依赖可并行；Step 13 与 Step 12 不同文件可并行；同文件步骤（launch.rs 的 1-7/9/13）由 validate-dag.sh 按文件自动串行化。

---

**MANUAL 标注扫描结果**：本计划无硬件操作、无外部系统动作、无判断类动作——除 Step 18 提交需用户触发外，全部步骤可自动执行。按 reference/02-generate-plan.md 规则，无 MANUAL 标签（默认）。

**Open Questions 前验证（不属于本 plan 步骤，属于实现前手动验证）**：spec.md Open Questions 的 3 项实测（env_key 鉴权、嵌套表 overlay、--profile + --full-auto）由 /execute 或实现者在 Step 1 之前手动完成；若某项失败需回改 Step 4/6 的 overlay 内容与参数顺序。
