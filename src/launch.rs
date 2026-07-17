use crate::config::Profile;
use anyhow::{Context, Result};
use crossterm::{execute, terminal::LeaveAlternateScreen};
use std::{
    env, fs, io,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
};

/// Default environment variables injected before launching Claude.
/// These mirror the privacy/telemetry defaults commonly set in
/// `~/.claude/settings.json`. Profile-level `env` entries override them.
const CLAUDE_DEFAULT_ENV: &[(&str, &str)] = &[
    ("DISABLE_AUTOUPDATER", "1"),
    ("CLAUDE_CODE_ATTRIBUTION_HEADER", "0"),
    ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1"),
    ("CLAUDE_CODE_ENABLE_TELEMETRY", "0"),
    ("CLAUDE_CODE_ENHANCED_TELEMETRY_BETA", "0"),
    ("CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY", "1"),
    ("CLAUDE_CODE_BYOC_ENABLE_DATADOG", "0"),
    ("CLAUDE_CODE_PROPAGATE_TRACEPARENT", "0"),
    ("DISABLE_GROWTHBOOK", "1"),
    ("DISABLE_INSTALLATION_CHECKS", "1"),
];

/// Apply `CLAUDE_DEFAULT_ENV` to the current process. Call before
/// `Command::exec` so the spawned `claude` process inherits them.
fn set_claude_default_env() {
    for (k, v) in CLAUDE_DEFAULT_ENV {
        env::set_var(k, v);
    }
}

/// Restore terminal to cooked mode. Must be called before exec or editor spawn.
pub fn restore_terminal() {
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

/// Build the CLI argument list for `claude` from a profile. Pure — no side effects.
pub fn build_args(profile: &Profile, with_continue: bool) -> Vec<String> {
    let mut args = Vec::new();
    if with_continue {
        args.push("--continue".to_string());
    }
    if let Some(model) = &profile.model {
        args.push("--model".to_string());
        args.push(model.clone());
    }
    if profile.skip_permissions.unwrap_or(false) {
        args.push("--dangerously-skip-permissions".to_string());
    }
    if let Some(extra) = &profile.extra_args {
        args.extend(extra.iter().cloned());
    }
    args
}

/// Return the binary name and argument list for launching a profile.
/// Dispatches by `profile.backend`: Claude uses `build_args`, Codex uses
/// `build_codex_args`, Kimi uses `build_kimi_args`.
/// The `with_continue` flag only applies to Claude (ignored for Codex/Kimi).
pub fn build_launch_command(profile: &Profile, with_continue: bool) -> (String, Vec<String>) {
    match profile.backend {
        crate::config::Backend::Claude => ("claude".into(), build_args(profile, with_continue)),
        crate::config::Backend::Codex => ("codex".into(), build_codex_args(profile)),
        crate::config::Backend::Kimi => ("kimi".into(), build_kimi_args(profile)),
    }
}

/// Inject profile env vars and exec-replace the current process with `claude`.
/// Returns only on error (process was not replaced).
pub fn exec_claude(profile: &Profile, with_continue: bool) -> anyhow::Error {
    set_claude_default_env();
    if let Some(env_map) = &profile.env {
        for (k, v) in env_map {
            env::set_var(k, v);
        }
    }
    let args = build_args(profile, with_continue);
    let err = Command::new("claude").args(&args).exec();
    anyhow::anyhow!("exec claude: {err}")
}

/// Check if `codex` is available in PATH.
pub fn check_codex_installed() -> bool {
    Command::new("which")
        .arg("codex")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if `kimi` is available in PATH.
pub fn check_kimi_installed() -> bool {
    Command::new("which")
        .arg("kimi")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Build `--config` CLI flags to configure the custom provider pointing at cct proxy.
/// Equivalent to the old config.toml approach, but passed inline so CODEX_HOME is
/// left at its default (~/.codex) and all profiles share history/sessions.
pub fn build_codex_proxy_config_args(model: &str, port: u16) -> Vec<String> {
    vec![
        "--config".to_string(),
        "model_provider=custom".to_string(),
        "--config".to_string(),
        format!("model={model}"),
        "--config".to_string(),
        "model_providers.custom.name=cct-proxy".to_string(),
        "--config".to_string(),
        format!("model_providers.custom.base_url=http://127.0.0.1:{port}/v1"),
        "--config".to_string(),
        "model_providers.custom.wire_api=responses".to_string(),
        "--config".to_string(),
        "model_providers.custom.env_key=OPENAI_API_KEY".to_string(),
    ]
}

/// Ensure the proxy is running. Spawns `cct proxy` if needed.
///
/// When `CCT_PROXY_LOG` is set, stderr is written to `~/.config/cc-tui/proxy.log`
/// instead of being discarded — useful for debugging proxy behavior.
pub fn ensure_proxy_running(_port: u16, socket_path: &Path) -> Result<()> {
    if crate::proxy::check_proxy_running(socket_path) {
        return Ok(());
    }
    let exe = std::env::current_exe().context("cannot find own executable")?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("proxy")
        .arg("start")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null());

    if std::env::var("CCT_PROXY_LOG").is_ok() {
        let log_path = crate::proxy::proxy_log_path();
        if let Ok(file) = std::fs::File::create(&log_path) {
            cmd.stderr(file);
        }
    } else {
        cmd.stderr(std::process::Stdio::null());
    }

    cmd.spawn().context("failed to spawn cct proxy")?;

    // Wait up to 5s for the socket to appear.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if crate::proxy::check_proxy_running(socket_path) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    anyhow::bail!("proxy did not start within 5 seconds")
}

/// Build the CLI argument list for `codex` from a profile. Pure — no side effects.
pub fn build_codex_args(profile: &Profile) -> Vec<String> {
    build_shared_codex_args(profile)
}

/// Build args for subscription mode — passes `--config model_provider=openai`
/// to use Codex's built-in OpenAI provider with native OAuth authentication.
fn build_codex_subscription_args(profile: &Profile) -> Vec<String> {
    let mut args = vec!["--config".to_string(), "model_provider=openai".to_string()];
    if let Some(model) = &profile.model {
        args.push("--config".to_string());
        args.push(format!("model={model}"));
    }
    args.append(&mut build_shared_codex_args(profile));
    args
}

/// Approval + extra_args common to both proxy and subscription paths.
fn build_shared_codex_args(profile: &Profile) -> Vec<String> {
    let mut args = Vec::new();
    match &profile.full_auto {
        Some(crate::config::ApprovalLevel::Danger) => {
            args.push("--dangerously-bypass-approvals-and-sandbox".to_string());
        }
        Some(crate::config::ApprovalLevel::Never) => {
            args.push("--ask-for-approval".to_string());
            args.push("never".to_string());
        }
        Some(crate::config::ApprovalLevel::Untrusted) => {
            args.push("--ask-for-approval".to_string());
            args.push("untrusted".to_string());
        }
        None => {}
    }
    if let Some(extra) = &profile.extra_args {
        args.extend(extra.iter().cloned());
    }
    args
}

/// Launch Codex through the local proxy (API key mode).
///
/// 1. Ensure proxy is running (spawn if needed).
/// 2. Switch proxy to this profile's upstream.
/// 3. Pass custom provider config via `--config` CLI flags (no config.toml).
/// 4. Leave CODEX_HOME at default (~/.codex) so all profiles share history/sessions.
fn exec_codex_proxy(profile: &Profile) -> anyhow::Error {
    let port: u16 = crate::proxy::proxy_port();
    let socket_path = crate::proxy::proxy_socket_path();

    if let Err(e) = ensure_proxy_running(port, &socket_path) {
        return anyhow::anyhow!("failed to start proxy: {e}");
    }

    let base_url = profile.base_url.clone().unwrap_or_default();
    let api_key = profile
        .env
        .as_ref()
        .and_then(|m| m.get("OPENAI_API_KEY"))
        .cloned()
        .unwrap_or_default();
    let model = profile
        .model
        .clone()
        .unwrap_or_else(|| "gpt-4.1".to_string());

    if let Err(e) = crate::proxy::switch_profile(&socket_path, &base_url, &api_key, &model) {
        return anyhow::anyhow!("failed to switch proxy: {e}");
    }

    if let Some(env_map) = &profile.env {
        for (k, v) in env_map {
            env::set_var(k, v);
        }
    }
    let mut args = build_codex_proxy_config_args(&model, port);
    args.append(&mut build_shared_codex_args(profile));
    let err = Command::new("codex").args(&args).exec();
    anyhow::anyhow!("exec codex: {err}")
}

/// Launch Codex with subscription (OAuth) authentication.
///
/// No proxy, no per-profile CODEX_HOME — uses default `~/.codex` so the
/// login session, memory DB, and other state from `codex login` are preserved.
/// Passes `--config model_provider=openai` to use the built-in OpenAI provider.
fn exec_codex_subscription(profile: &Profile) -> anyhow::Error {
    env::set_var("DISABLE_AUTOUPDATER", "1");
    if let Some(env_map) = &profile.env {
        for (k, v) in env_map {
            env::set_var(k, v);
        }
    }
    let args = build_codex_subscription_args(profile);
    let err = Command::new("codex").args(&args).exec();
    anyhow::anyhow!("exec codex: {err}")
}

/// Launch Codex. Dispatches to proxy mode (API key) or subscription mode (OAuth).
pub fn exec_codex(profile: &Profile) -> anyhow::Error {
    if !check_codex_installed() {
        return anyhow::anyhow!(
            "codex CLI not found in PATH. Install it first: npm install -g @openai/codex"
        );
    }

    if profile.auth_type.as_deref() == Some("subscription") {
        return exec_codex_subscription(profile);
    }
    exec_codex_proxy(profile)
}

/// Check if `claude` (or override via CCT_CLAUDE_BIN) is available in PATH.
pub fn check_claude_installed() -> bool {
    let bin = std::env::var("CCT_CLAUDE_BIN").unwrap_or_else(|_| "claude".to_string());
    Command::new("which")
        .arg(&bin)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if an arbitrary command is available in PATH.
pub fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Inject profile env vars and exec-replace with the command wrapped in `bash -c`.
/// Returns only on error (process was not replaced).
pub fn exec_with_env(profile: &Profile, shell_cmd: &str) -> anyhow::Error {
    set_claude_default_env();
    if let Some(env_map) = &profile.env {
        for (k, v) in env_map {
            env::set_var(k, v);
        }
    }
    let err = Command::new("bash").args(["-c", shell_cmd]).exec();
    anyhow::anyhow!("exec bash -c {shell_cmd:?}: {err}")
}

/// Set hasCompletedOnboarding: true in ~/.claude.json so Claude Code skips
/// the onboarding flow. Creates the file if it doesn't exist.
pub fn ensure_claude_onboarding() -> Result<()> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    let path = home.join(".claude.json");

    let content = if path.exists() {
        let text = fs::read_to_string(&path)?;
        let mut json: serde_json::Value = serde_json::from_str(&text)?;
        json["hasCompletedOnboarding"] = serde_json::Value::Bool(true);
        serde_json::to_string_pretty(&json)?
    } else {
        r#"{
  "hasCompletedOnboarding": true
}"#
        .to_string()
    };

    fs::write(&path, content)?;
    Ok(())
}

/// Prompt user to install claude via the official installer script.
/// Must be called BEFORE entering raw mode / alternate screen.
/// Returns Ok(()) on successful install, Err on failure or user decline.
pub fn prompt_install() -> Result<()> {
    use std::io::{BufRead, Write};

    println!("Claude CLI not found in PATH.");
    print!("Install now? [Y/n] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    let trimmed = input.trim().to_lowercase();

    if trimmed == "n" || trimmed == "no" {
        println!("\nTo install manually, run:");
        println!("  curl -fsSL https://claude.ai/install.sh | bash");
        std::process::exit(0);
    }

    println!("\nInstalling Claude CLI...\n");
    let status = Command::new("bash")
        .arg("-c")
        .arg("curl -fsSL https://claude.ai/install.sh | bash")
        .status()
        .context("failed to run installer")?;

    if !status.success() {
        anyhow::bail!(
            "Installation failed (exit code: {:?}). Install manually:\n  curl -fsSL https://claude.ai/install.sh | bash",
            status.code()
        );
    }

    // Re-check: try PATH first, then ~/.local/bin/claude as fallback
    if check_claude_installed() {
        println!("\nClaude CLI installed successfully.");
        return Ok(());
    }

    let home = dirs::home_dir().unwrap_or_default();
    let fallback = home.join(".local/bin/claude");
    if fallback.exists() {
        println!("\nClaude CLI installed at {}.", fallback.display());
        println!("Note: You may need to add ~/.local/bin to your PATH.");
        return Ok(());
    }

    anyhow::bail!("Installation completed but `claude` not found in PATH.\nAdd ~/.local/bin to your PATH and restart your shell.")
}

/// Prompt user to install Kimi Code CLI via the official installer script.
/// Must be called BEFORE entering raw mode / alternate screen.
/// Returns Ok(()) on successful install, Err on failure or user decline.
pub fn prompt_install_kimi() -> Result<()> {
    use std::io::{BufRead, Write};

    println!("Kimi Code CLI not found in PATH.");
    print!("Install now? [Y/n] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    let trimmed = input.trim().to_lowercase();

    if trimmed == "n" || trimmed == "no" {
        println!("\nTo install manually, run:");
        println!("  curl -fsSL https://code.kimi.com/kimi-code/install.sh | bash");
        std::process::exit(0);
    }

    println!("\nInstalling Kimi Code CLI...\n");
    let status = Command::new("bash")
        .arg("-c")
        .arg("curl -fsSL https://code.kimi.com/kimi-code/install.sh | bash")
        .status()
        .context("failed to run installer")?;

    if !status.success() {
        anyhow::bail!(
            "Installation failed (exit code: {:?}). Install manually:\n  curl -fsSL https://code.kimi.com/kimi-code/install.sh | bash",
            status.code()
        );
    }

    if check_kimi_installed() {
        println!("\nKimi Code CLI installed successfully.");
        return Ok(());
    }

    let home = dirs::home_dir().unwrap_or_default();
    let fallback = home.join(".local/bin/kimi");
    if fallback.exists() {
        println!("\nKimi Code CLI installed at {}.", fallback.display());
        println!("Note: You may need to add ~/.local/bin to your PATH.");
        return Ok(());
    }

    anyhow::bail!("Installation completed but `kimi` not found in PATH.\nAdd ~/.local/bin to your PATH and restart your shell.")
}

/// Path to the Kimi Code CLI config file. Honors the `CCT_KIMI_CONFIG`
/// override (mirrors `config_path()`'s `CCT_CONFIG`) so tests never touch
/// the real `~/.kimi-code/config.toml`.
pub fn kimi_config_path() -> PathBuf {
    if let Ok(p) = env::var("CCT_KIMI_CONFIG") {
        return PathBuf::from(p);
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".kimi-code")
        .join("config.toml")
}

/// Normalize a base_url for the kimi config: ensure an `https://` scheme,
/// ensure a `/v1` suffix, and collapse duplicate slashes after the scheme
/// (the naive `append "/v1"` approach produces bugs like `coding//v1`).
fn normalize_kimi_base_url(raw: &str) -> String {
    let mut url = raw.trim().to_string();
    if url.is_empty() {
        return url;
    }
    if !url.starts_with("https://") && !url.starts_with("http://") {
        url = format!("https://{url}");
    }
    if !url.ends_with("/v1") {
        url = format!("{url}/v1");
    }
    if let Some(scheme_end) = url.find("://") {
        let (scheme, rest) = url.split_at(scheme_end + 3);
        let mut collapsed = String::with_capacity(rest.len());
        let mut last_was_slash = false;
        for c in rest.chars() {
            if c == '/' {
                if !last_was_slash {
                    collapsed.push(c);
                }
                last_was_slash = true;
            } else {
                collapsed.push(c);
                last_was_slash = false;
            }
        }
        url = format!("{scheme}{collapsed}");
    }
    url
}

/// Surgically write this profile's provider/model entries into the Kimi
/// Code CLI config (`~/.kimi-code/config.toml`). All pre-existing tables
/// (e.g. `managed:kimi-code` providers created by `kimi login`, `services.*`,
/// `default_model`, `thinking`) are preserved.
pub fn generate_kimi_config(profile: &Profile) -> Result<()> {
    let path = kimi_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create kimi config dir {parent:?}"))?;
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .with_context(|| format!("parse TOML in {path:?}"))?;

    let env_map = profile.env.as_ref();
    let env_get = |key: &str| env_map.and_then(|m| m.get(key)).map(String::as_str);

    let provider_id = profile.name.as_str();
    let base_url = profile
        .base_url
        .as_deref()
        .filter(|s| !s.is_empty())
        .or_else(|| env_get("ANTHROPIC_BASE_URL"))
        .map(normalize_kimi_base_url)
        .unwrap_or_default();
    let api_key = env_get("ANTHROPIC_AUTH_TOKEN")
        .or_else(|| env_get("ANTHROPIC_API_KEY"))
        .unwrap_or_default();

    // [providers."<id>"] — parent table is implicit so no bare `[providers]`
    // header is emitted (matches the kimi CLI's own config layout).
    if !matches!(doc.get("providers"), Some(toml_edit::Item::Table(_))) {
        let mut t = toml_edit::Table::new();
        t.set_implicit(true);
        doc["providers"] = toml_edit::Item::Table(t);
    }
    let providers = doc["providers"]
        .as_table_mut()
        .expect("providers item should be a table");
    let provider = providers
        .entry(provider_id)
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
    let provider_table = provider
        .as_table_mut()
        .expect("provider entry should be a table");
    provider_table["type"] = toml_edit::value("kimi");
    provider_table["base_url"] = toml_edit::value(base_url.as_str());
    provider_table["api_key"] = toml_edit::value(api_key);

    // [models."<id>/<model>"] — skipped when the profile has no model.
    let model = profile
        .model
        .as_deref()
        .filter(|s| !s.is_empty())
        .or_else(|| env_get("ANTHROPIC_MODEL"))
        .map(str::to_string);
    if let Some(model) = model {
        if !matches!(doc.get("models"), Some(toml_edit::Item::Table(_))) {
            let mut t = toml_edit::Table::new();
            t.set_implicit(true);
            doc["models"] = toml_edit::Item::Table(t);
        }
        let models = doc["models"]
            .as_table_mut()
            .expect("models item should be a table");
        let model_key = format!("{provider_id}/{model}");
        let model_entry = models
            .entry(&model_key)
            .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
        let model_table = model_entry
            .as_table_mut()
            .expect("model entry should be a table");
        model_table["provider"] = toml_edit::value(provider_id);
        model_table["model"] = toml_edit::value(model.as_str());
        let max_context_size =
            crate::config::resolve_max_context_size(profile.max_context_size.as_deref().or(Some(
                crate::config::default_max_context_size(Some(model.as_str())),
            )));
        model_table["max_context_size"] = toml_edit::value(max_context_size as i64);
        let mut capabilities = toml_edit::Array::new();
        for cap in [
            "thinking",
            "always_thinking",
            "image_in",
            "video_in",
            "tool_use",
        ] {
            capabilities.push(cap);
        }
        model_table["capabilities"] = toml_edit::value(capabilities);
        model_table["display_name"] = toml_edit::value(model.to_uppercase());
        if model.starts_with("k3") {
            let mut efforts = toml_edit::Array::new();
            efforts.push("max");
            model_table["support_efforts"] = toml_edit::value(efforts);
            model_table["default_effort"] = toml_edit::value("max");
        } else {
            // Keep re-generation correct after a model change away from k3*.
            model_table.remove("support_efforts");
            model_table.remove("default_effort");
        }
    }

    fs::write(&path, doc.to_string()).with_context(|| format!("write kimi config {path:?}"))?;
    Ok(())
}

/// Build the CLI argument list for `kimi` from a profile. Pure — no side effects.
pub fn build_kimi_args(profile: &Profile) -> Vec<String> {
    let mut args = Vec::new();
    let model = profile
        .model
        .as_deref()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            profile
                .env
                .as_ref()
                .and_then(|m| m.get("ANTHROPIC_MODEL"))
                .map(String::as_str)
        });
    if let Some(model) = model {
        args.push("-m".to_string());
        args.push(format!("{}/{model}", profile.name));
    }
    if let Some(extra) = &profile.extra_args {
        args.extend(extra.iter().cloned());
    }
    args
}

/// Write the kimi config entries for `profile`, inject its env vars, and
/// exec-replace the current process with `kimi`. Returns only on error.
pub fn exec_kimi(profile: &Profile) -> anyhow::Error {
    if !check_kimi_installed() {
        return anyhow::anyhow!(
            "kimi CLI not found in PATH. Install it first: curl -fsSL https://code.kimi.com/kimi-code/install.sh | bash"
        );
    }
    if let Err(e) = generate_kimi_config(profile) {
        return anyhow::anyhow!("failed to write kimi config: {e:#}");
    }
    if let Some(env_map) = &profile.env {
        for (k, v) in env_map {
            env::set_var(k, v);
        }
    }
    let args = build_kimi_args(profile);
    let err = Command::new("kimi").args(&args).exec();
    anyhow::anyhow!("exec kimi: {err}")
}

/// Suspend TUI, open $EDITOR (or vi) on path, block until editor exits.
pub fn open_editor(path: &Path) -> Result<()> {
    let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    Command::new(&editor)
        .arg(path)
        .status()
        .with_context(|| format!("spawn editor {editor:?}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Profile;
    use std::env;

    fn profile(model: Option<&str>, skip: Option<bool>, extra: Option<Vec<&str>>) -> Profile {
        Profile {
            name: "t".into(),
            description: None,
            env: None,
            model: model.map(Into::into),
            skip_permissions: skip,
            extra_args: extra.map(|v| v.into_iter().map(Into::into).collect()),
            backend: crate::config::Backend::Claude,
            base_url: None,
            full_auto: None,
            auth_type: None,
            max_context_size: None,
        }
    }

    #[test]
    fn check_claude_installed_found() {
        // "true" is always in PATH on Unix
        std::env::set_var("CCT_CLAUDE_BIN", "true");
        assert!(super::check_claude_installed());
        std::env::remove_var("CCT_CLAUDE_BIN");
    }

    #[test]
    fn check_claude_installed_not_found() {
        std::env::set_var("CCT_CLAUDE_BIN", "nonexistent-binary-xyz-12345");
        assert!(!super::check_claude_installed());
        std::env::remove_var("CCT_CLAUDE_BIN");
    }

    #[test]
    fn build_args_empty() {
        assert!(build_args(&profile(None, None, None), false).is_empty());
    }

    #[test]
    fn build_args_model_only() {
        assert_eq!(
            build_args(&profile(Some("kimi-k1.5"), None, None), false),
            vec!["--model", "kimi-k1.5"]
        );
    }

    #[test]
    fn build_args_full() {
        let p = profile(Some("opus"), Some(true), Some(vec!["--verbose"]));
        assert_eq!(
            build_args(&p, false),
            vec![
                "--model",
                "opus",
                "--dangerously-skip-permissions",
                "--verbose"
            ]
        );
    }

    #[test]
    fn build_args_with_continue_false() {
        // Existing behavior preserved when with_continue=false
        assert!(build_args(&profile(None, None, None), false).is_empty());
    }

    #[test]
    fn build_args_continue_only() {
        assert_eq!(
            build_args(&profile(None, None, None), true),
            vec!["--continue"]
        );
    }

    #[test]
    fn build_args_continue_with_flags() {
        let p = profile(Some("opus"), Some(true), Some(vec!["--verbose"]));
        assert_eq!(
            build_args(&p, true),
            vec![
                "--continue",
                "--model",
                "opus",
                "--dangerously-skip-permissions",
                "--verbose",
            ]
        );
    }

    // --- Codex tests ---

    fn codex_profile(
        name: &str,
        model: Option<&str>,
        base_url: Option<&str>,
        full_auto: Option<crate::config::ApprovalLevel>,
        extra: Option<Vec<&str>>,
    ) -> Profile {
        Profile {
            name: name.into(),
            description: None,
            env: None,
            model: model.map(Into::into),
            skip_permissions: None,
            extra_args: extra.map(|v| v.into_iter().map(Into::into).collect()),
            backend: crate::config::Backend::Codex,
            base_url: base_url.map(Into::into),
            full_auto,
            auth_type: None,
            max_context_size: None,
        }
    }

    #[test]
    fn build_codex_args_empty() {
        let p = codex_profile("test", None, None, None, None);
        assert!(build_codex_args(&p).is_empty());
    }

    #[test]
    fn build_codex_args_full_auto_only() {
        let p = codex_profile(
            "test",
            None,
            None,
            Some(crate::config::ApprovalLevel::Danger),
            None,
        );
        assert_eq!(
            build_codex_args(&p),
            vec!["--dangerously-bypass-approvals-and-sandbox"]
        );
    }

    #[test]
    fn build_codex_args_extra_only() {
        let p = codex_profile("test", None, None, None, Some(vec!["--quiet"]));
        assert_eq!(build_codex_args(&p), vec!["--quiet"]);
    }

    #[test]
    fn build_codex_args_full_auto_and_extra() {
        let p = codex_profile(
            "test",
            None,
            None,
            Some(crate::config::ApprovalLevel::Danger),
            Some(vec!["--quiet", "--json"]),
        );
        assert_eq!(
            build_codex_args(&p),
            vec![
                "--dangerously-bypass-approvals-and-sandbox",
                "--quiet",
                "--json"
            ]
        );
    }

    // --- build_codex_subscription_args tests ---

    #[test]
    fn build_codex_subscription_args_empty() {
        let p = codex_profile("test", None, None, None, None);
        assert_eq!(
            build_codex_subscription_args(&p),
            vec!["--config", "model_provider=openai"]
        );
    }

    #[test]
    fn build_codex_subscription_args_with_model() {
        let p = codex_profile("test", Some("gpt-5-codex"), None, None, None);
        assert_eq!(
            build_codex_subscription_args(&p),
            vec![
                "--config",
                "model_provider=openai",
                "--config",
                "model=gpt-5-codex",
            ]
        );
    }

    #[test]
    fn build_codex_subscription_args_with_full_auto() {
        let p = codex_profile(
            "test",
            None,
            None,
            Some(crate::config::ApprovalLevel::Danger),
            None,
        );
        assert_eq!(
            build_codex_subscription_args(&p),
            vec![
                "--config",
                "model_provider=openai",
                "--dangerously-bypass-approvals-and-sandbox",
            ]
        );
    }

    #[test]
    fn build_codex_proxy_config_args_includes_model_and_port() {
        let args = build_codex_proxy_config_args("gpt-4.1", 19191);
        assert_eq!(
            args,
            vec![
                "--config",
                "model_provider=custom",
                "--config",
                "model=gpt-4.1",
                "--config",
                "model_providers.custom.name=cct-proxy",
                "--config",
                "model_providers.custom.base_url=http://127.0.0.1:19191/v1",
                "--config",
                "model_providers.custom.wire_api=responses",
                "--config",
                "model_providers.custom.env_key=OPENAI_API_KEY",
            ]
        );
    }

    #[test]
    fn build_codex_proxy_config_args_different_model_and_port() {
        let args = build_codex_proxy_config_args("o4-mini", 29999);
        assert!(args.contains(&"model=o4-mini".to_string()));
        assert!(
            args.contains(&"model_providers.custom.base_url=http://127.0.0.1:29999/v1".to_string())
        );
    }

    #[test]
    #[serial_test::serial]
    fn claude_default_env_is_injected() {
        let keys: Vec<&str> = CLAUDE_DEFAULT_ENV.iter().map(|(k, _)| *k).collect();
        let previous: Vec<Option<String>> = keys.iter().map(|k| env::var(k).ok()).collect();
        for k in &keys {
            env::remove_var(k);
        }

        set_claude_default_env();

        for (k, expected) in CLAUDE_DEFAULT_ENV {
            assert_eq!(
                env::var(k).unwrap(),
                *expected,
                "expected {k} to be set to {expected}"
            );
        }

        for (k, prev) in keys.iter().zip(previous.iter()) {
            match prev {
                Some(val) => env::set_var(k, val),
                None => env::remove_var(k),
            }
        }
    }

    // --- Kimi tests ---

    fn kimi_profile(
        name: &str,
        model: Option<&str>,
        base_url: Option<&str>,
        max_context_size: Option<&str>,
    ) -> Profile {
        Profile {
            name: name.into(),
            description: None,
            env: None,
            model: model.map(Into::into),
            skip_permissions: None,
            extra_args: None,
            backend: crate::config::Backend::Kimi,
            base_url: base_url.map(Into::into),
            full_auto: None,
            auth_type: None,
            max_context_size: max_context_size.map(Into::into),
        }
    }

    #[test]
    fn build_kimi_args_model() {
        let p = kimi_profile("my-kimi", Some("kimi-k2"), None, None);
        assert_eq!(build_kimi_args(&p), vec!["-m", "my-kimi/kimi-k2"]);
    }

    #[test]
    fn build_kimi_args_no_model_extra_only() {
        let mut p = kimi_profile("my-kimi", None, None, None);
        p.extra_args = Some(vec!["--verbose".into()]);
        assert_eq!(build_kimi_args(&p), vec!["--verbose"]);
    }

    #[test]
    fn build_kimi_args_model_and_extra() {
        let mut p = kimi_profile("my-kimi", Some("k3"), None, None);
        p.extra_args = Some(vec!["--verbose".into()]);
        assert_eq!(build_kimi_args(&p), vec!["-m", "my-kimi/k3", "--verbose"]);
    }

    #[test]
    fn build_kimi_args_model_from_env_fallback() {
        let mut p = kimi_profile("my-kimi", None, None, None);
        p.env = Some(std::collections::HashMap::from([(
            "ANTHROPIC_MODEL".into(),
            "kimi-k2".into(),
        )]));
        assert_eq!(build_kimi_args(&p), vec!["-m", "my-kimi/kimi-k2"]);
    }

    #[test]
    fn build_launch_command_dispatches_kimi() {
        let p = kimi_profile("my-kimi", Some("kimi-k2"), None, None);
        let (bin, args) = build_launch_command(&p, false);
        assert_eq!(bin, "kimi");
        assert_eq!(args, vec!["-m", "my-kimi/kimi-k2"]);
    }

    #[test]
    #[serial_test::serial]
    fn kimi_config_path_honors_override() {
        std::env::set_var("CCT_KIMI_CONFIG", "/tmp/cct-test-kimi-config.toml");
        assert_eq!(
            kimi_config_path(),
            PathBuf::from("/tmp/cct-test-kimi-config.toml")
        );
        std::env::remove_var("CCT_KIMI_CONFIG");
    }

    #[test]
    fn normalize_kimi_base_url_cases() {
        assert_eq!(
            normalize_kimi_base_url("https://api.kimi.com/v1"),
            "https://api.kimi.com/v1"
        );
        assert_eq!(
            normalize_kimi_base_url("api.kimi.com"),
            "https://api.kimi.com/v1"
        );
        assert_eq!(
            normalize_kimi_base_url("https://api.kimi.com"),
            "https://api.kimi.com/v1"
        );
        assert_eq!(
            normalize_kimi_base_url("https://x.com/coding/"),
            "https://x.com/coding/v1"
        );
        assert_eq!(
            normalize_kimi_base_url("https://x.com/coding//v1"),
            "https://x.com/coding/v1"
        );
        assert_eq!(normalize_kimi_base_url(""), "");
    }

    fn read_kimi_doc(path: &std::path::Path) -> toml_edit::DocumentMut {
        std::fs::read_to_string(path).unwrap().parse().unwrap()
    }

    #[test]
    #[serial_test::serial]
    fn generate_kimi_config_writes_provider_and_model() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("config.toml");
        std::env::set_var("CCT_KIMI_CONFIG", &path);

        let mut p = kimi_profile(
            "my-kimi",
            Some("kimi-k2"),
            Some("https://api.kimi.com"),
            None,
        );
        p.env = Some(std::collections::HashMap::from([(
            "ANTHROPIC_API_KEY".into(),
            "sk-kimi".into(),
        )]));
        generate_kimi_config(&p).unwrap();

        let doc = read_kimi_doc(&path);
        let provider = &doc["providers"]["my-kimi"];
        assert_eq!(provider["type"].as_str(), Some("kimi"));
        assert_eq!(
            provider["base_url"].as_str(),
            Some("https://api.kimi.com/v1")
        );
        assert_eq!(provider["api_key"].as_str(), Some("sk-kimi"));

        let model = &doc["models"]["my-kimi/kimi-k2"];
        assert_eq!(model["provider"].as_str(), Some("my-kimi"));
        assert_eq!(model["model"].as_str(), Some("kimi-k2"));
        assert_eq!(model["max_context_size"].as_integer(), Some(262_144));
        assert_eq!(model["display_name"].as_str(), Some("KIMI-K2"));
        let caps: Vec<&str> = model["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(
            caps,
            [
                "thinking",
                "always_thinking",
                "image_in",
                "video_in",
                "tool_use"
            ]
        );
        assert!(model.get("support_efforts").is_none());
        assert!(model.get("default_effort").is_none());

        std::env::remove_var("CCT_KIMI_CONFIG");
    }

    #[test]
    #[serial_test::serial]
    fn generate_kimi_config_k3_writes_effort_and_1m() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::env::set_var("CCT_KIMI_CONFIG", &path);

        let p = kimi_profile("my-kimi", Some("k3"), None, None);
        generate_kimi_config(&p).unwrap();

        let doc = read_kimi_doc(&path);
        let model = &doc["models"]["my-kimi/k3"];
        assert_eq!(model["max_context_size"].as_integer(), Some(1_000_000));
        assert_eq!(model["display_name"].as_str(), Some("K3"));
        let efforts: Vec<&str> = model["support_efforts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(efforts, ["max"]);
        assert_eq!(model["default_effort"].as_str(), Some("max"));

        std::env::remove_var("CCT_KIMI_CONFIG");
    }

    #[test]
    #[serial_test::serial]
    fn generate_kimi_config_explicit_max_context_size_wins() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::env::set_var("CCT_KIMI_CONFIG", &path);

        // Explicit "1m" on a non-k3 model → 1000000
        let p = kimi_profile("kimi-a", Some("kimi-k2"), None, Some("1m"));
        generate_kimi_config(&p).unwrap();
        // Explicit "260k" on a k3 model → 262144
        let p = kimi_profile("kimi-b", Some("k3"), None, Some("260k"));
        generate_kimi_config(&p).unwrap();

        let doc = read_kimi_doc(&path);
        assert_eq!(
            doc["models"]["kimi-a/kimi-k2"]["max_context_size"].as_integer(),
            Some(1_000_000)
        );
        assert_eq!(
            doc["models"]["kimi-b/k3"]["max_context_size"].as_integer(),
            Some(262_144)
        );

        std::env::remove_var("CCT_KIMI_CONFIG");
    }

    #[test]
    #[serial_test::serial]
    fn generate_kimi_config_preserves_existing_tables() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"default_model = "kimi-code/k3"

[providers."managed:kimi-code"]
type = "kimi"
api_key = ""
base_url = "https://api.kimi.com/coding/v1"

[providers."managed:kimi-code".oauth]
storage = "file"
key = "oauth/kimi-code"

[services.moonshot_search]
base_url = "https://api.kimi.com/coding/v1/search"
api_key = ""

[thinking]
enabled = true
effort = "max"
"#,
        )
        .unwrap();
        std::env::set_var("CCT_KIMI_CONFIG", &path);

        let p = kimi_profile("my-kimi", Some("kimi-k2"), Some("api.kimi.com"), None);
        generate_kimi_config(&p).unwrap();

        let doc = read_kimi_doc(&path);
        // Pre-existing tables survive untouched
        assert_eq!(doc["default_model"].as_str(), Some("kimi-code/k3"));
        let managed = &doc["providers"]["managed:kimi-code"];
        assert_eq!(
            managed["base_url"].as_str(),
            Some("https://api.kimi.com/coding/v1")
        );
        assert_eq!(managed["oauth"]["storage"].as_str(), Some("file"));
        assert_eq!(
            doc["services"]["moonshot_search"]["base_url"].as_str(),
            Some("https://api.kimi.com/coding/v1/search")
        );
        assert_eq!(doc["thinking"]["enabled"].as_bool(), Some(true));
        // New provider got scheme + /v1 normalization
        assert_eq!(
            doc["providers"]["my-kimi"]["base_url"].as_str(),
            Some("https://api.kimi.com/v1")
        );

        std::env::remove_var("CCT_KIMI_CONFIG");
    }

    #[test]
    #[serial_test::serial]
    fn generate_kimi_config_regeneration_removes_effort_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::env::set_var("CCT_KIMI_CONFIG", &path);

        let p = kimi_profile("my-kimi", Some("k3"), None, None);
        generate_kimi_config(&p).unwrap();
        // Same provider, model changed away from k3*
        let p = kimi_profile("my-kimi", Some("kimi-k2"), None, None);
        generate_kimi_config(&p).unwrap();

        let doc = read_kimi_doc(&path);
        let model = &doc["models"]["my-kimi/kimi-k2"];
        assert!(model.get("support_efforts").is_none());
        assert!(model.get("default_effort").is_none());
        assert_eq!(model["max_context_size"].as_integer(), Some(262_144));
        // The old k3 model entry is left as-is (different model table)
        assert_eq!(
            doc["models"]["my-kimi/k3"]["default_effort"].as_str(),
            Some("max")
        );

        std::env::remove_var("CCT_KIMI_CONFIG");
    }

    #[test]
    #[serial_test::serial]
    fn generate_kimi_config_no_model_skips_models_table() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::env::set_var("CCT_KIMI_CONFIG", &path);

        let p = kimi_profile("my-kimi", None, Some("https://api.kimi.com/v1"), None);
        generate_kimi_config(&p).unwrap();

        let doc = read_kimi_doc(&path);
        assert_eq!(doc["providers"]["my-kimi"]["type"].as_str(), Some("kimi"));
        assert!(doc.get("models").is_none());

        std::env::remove_var("CCT_KIMI_CONFIG");
    }
}
