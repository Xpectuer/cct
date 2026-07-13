use crate::config::Profile;
use anyhow::{Context, Result};
use crossterm::{execute, terminal::LeaveAlternateScreen};
use std::{env, fs, io, os::unix::process::CommandExt, path::Path, process::Command};

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
/// Dispatches by `profile.backend`: Claude uses `build_args`, Codex uses `build_codex_args`.
/// The `with_continue` flag only applies to Claude (ignored for Codex).
pub fn build_launch_command(profile: &Profile, with_continue: bool) -> (String, Vec<String>) {
    match profile.backend {
        crate::config::Backend::Claude => ("claude".into(), build_args(profile, with_continue)),
        crate::config::Backend::Codex => ("codex".into(), build_codex_args(profile)),
    }
}

/// Inject profile env vars and exec-replace the current process with `claude`.
/// Returns only on error (process was not replaced).
pub fn exec_claude(profile: &Profile, with_continue: bool) -> anyhow::Error {
    env::set_var("DISABLE_AUTOUPDATER", "1");
    env::set_var("CLAUDE_CODE_ATTRIBUTION_HEADER", "0");
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
    env::set_var("DISABLE_AUTOUPDATER", "1");
    env::set_var("CLAUDE_CODE_ATTRIBUTION_HEADER", "0");
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
}
