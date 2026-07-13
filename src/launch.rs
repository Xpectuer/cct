use crate::config::Profile;
use anyhow::{Context, Result};
use crossterm::{execute, terminal::LeaveAlternateScreen};
use std::{
    env, fs, io,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
};

/// Resolve the real Codex home directory without side effects.
/// Respects `CODEX_HOME` env var; falls back to `~/.codex`.
fn codex_home_dir() -> PathBuf {
    if let Ok(home) = env::var("CODEX_HOME") {
        let p = PathBuf::from(&home);
        if p.is_absolute() && p.exists() {
            return p;
        }
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".codex")
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

/// Generate codex config.toml at a specified directory.
/// Content is derived from the profile's name, model, and base_url fields.
///
/// When an API key is present in profile.env, uses `env_key` to reference it.
/// Otherwise falls back to `requires_openai_auth` for ChatGPT login flow.
pub fn generate_codex_config(profile: &Profile, codex_home: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(codex_home)?;
    let model = profile.model.as_deref().unwrap_or("gpt-4.1");
    let name = &profile.name;
    let base_url = profile.base_url.as_deref().unwrap_or("");
    let has_api_key = profile
        .env
        .as_ref()
        .and_then(|m| m.get("OPENAI_API_KEY"))
        .map(|k| !k.is_empty())
        .unwrap_or(false);

    // Use env_key when an API key is provided; requires_openai_auth otherwise
    let auth_line = if has_api_key {
        "env_key = \"OPENAI_API_KEY\"\n"
    } else {
        "requires_openai_auth = true\n"
    };

    let config_content = format!(
        "model_provider = \"custom\"\nmodel = \"{model}\"\n\n[model_providers.custom]\nname = \"{name}\"\nbase_url = \"{base_url}\"\nwire_api = \"responses\"\n{auth_line}"
    );
    fs::write(codex_home.join("config.toml"), config_content)?;
    Ok(())
}

/// Write `{codex_home}/auth.json` with the OPENAI_API_KEY from the profile's env.
/// If OPENAI_API_KEY is absent, returns Ok(()) without creating the file.
pub fn write_codex_auth(profile: &Profile, codex_home: &Path) -> Result<()> {
    let key = profile.env.as_ref().and_then(|m| m.get("OPENAI_API_KEY"));
    if let Some(api_key) = key {
        fs::create_dir_all(codex_home)?;
        let json =
            format!("{{\n  \"auth_mode\": \"apikey\",\n  \"OPENAI_API_KEY\": \"{api_key}\"\n}}\n");
        fs::write(codex_home.join("auth.json"), json)?;
    }
    Ok(())
}

/// Verify that the written config.toml contains the expected values from the profile.
/// Reads back the file and checks key fields. Returns an error if verification fails.
fn verify_codex_config(profile: &Profile, codex_home: &Path) -> Result<()> {
    let config_path = codex_home.join("config.toml");
    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("cannot read back config.toml at {}", config_path.display()))?;

    let model = profile.model.as_deref().unwrap_or("gpt-4.1");
    let base_url = profile.base_url.as_deref().unwrap_or("");
    let has_api_key = profile
        .env
        .as_ref()
        .and_then(|m| m.get("OPENAI_API_KEY"))
        .map(|k| !k.is_empty())
        .unwrap_or(false);

    // Verify core fields are present
    let checks = [
        ("model_provider = \"custom\"", "model_provider"),
        (&format!("model = \"{model}\""), "model"),
        ("[model_providers.custom]", "provider table"),
        (&format!("name = \"{}\"", profile.name), "provider name"),
        (&format!("base_url = \"{base_url}\""), "base_url"),
        ("wire_api = \"responses\"", "wire_api"),
    ];

    for (expected, label) in &checks {
        if !content.contains(expected) {
            anyhow::bail!(
                "config.toml verification failed: missing expected content for {label}\n\
                 expected: {expected}\n\
                 file: {}",
                config_path.display(),
            );
        }
    }

    // Verify auth method
    if has_api_key {
        if !content.contains("env_key = \"OPENAI_API_KEY\"") {
            anyhow::bail!(
                "config.toml verification failed: expected env_key for API key profile\n\
                 file: {}",
                config_path.display(),
            );
        }
    } else if !content.contains("requires_openai_auth = true") {
        anyhow::bail!(
            "config.toml verification failed: expected requires_openai_auth for non-API-key profile\n\
             file: {}",
            config_path.display(),
        );
    }

    Ok(())
}

/// Verify that the written auth.json is valid and contains the expected API key.
/// Skips verification if no API key is configured (file won't exist).
fn verify_codex_auth(profile: &Profile, codex_home: &Path) -> Result<()> {
    let expected_key = profile.env.as_ref().and_then(|m| m.get("OPENAI_API_KEY"));

    let auth_path = codex_home.join("auth.json");

    match expected_key {
        Some(key) if !key.is_empty() => {
            let content = fs::read_to_string(&auth_path).with_context(|| {
                format!("cannot read back auth.json at {}", auth_path.display())
            })?;
            if !content.contains("\"auth_mode\": \"apikey\"") {
                anyhow::bail!(
                    "auth.json verification failed: missing auth_mode\nfile: {}",
                    auth_path.display(),
                );
            }
            if !content.contains(key) {
                anyhow::bail!(
                    "auth.json verification failed: API key mismatch\nfile: {}",
                    auth_path.display(),
                );
            }
        }
        _ => {
            // No API key configured — auth.json should NOT exist (or may be user's own)
            // We don't enforce absence since the user may have pre-existing auth
        }
    }

    Ok(())
}

/// Build the CLI argument list for `codex` from a profile. Pure — no side effects.
pub fn build_codex_args(profile: &Profile) -> Vec<String> {
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
        None => {
            // default: on-request — no flags
        }
    }
    if let Some(extra) = &profile.extra_args {
        args.extend(extra.iter().cloned());
    }
    args
}

// ---- backup helpers -----------------------------------------------------------

const BACKUP_EXT: &str = "cct-backup";

/// Copy `path` to `path.<ext>.cct-backup` (e.g. `config.toml` → `config.toml.cct-backup`).
/// Returns the backup path if the original existed, `None` otherwise.
fn backup_file(path: &Path) -> io::Result<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }
    let backup_name = match path.extension() {
        Some(ext) => format!(
            "{}.{}.{BACKUP_EXT}",
            path.file_stem().unwrap_or_default().to_string_lossy(),
            ext.to_string_lossy(),
        ),
        None => format!(
            "{}.{BACKUP_EXT}",
            path.file_name().unwrap_or_default().to_string_lossy()
        ),
    };
    let backup = path.with_file_name(backup_name);
    fs::copy(path, &backup)?;
    Ok(Some(backup))
}

/// Restore a file from its backup, then remove the backup.
fn restore_file(original: &Path, backup: Option<&PathBuf>) {
    if let Some(b) = backup {
        if b.exists() {
            let _ = fs::rename(b, original);
        }
    }
}

/// Remove a backup file. No-op if `None`.
fn remove_backup(backup: Option<&PathBuf>) {
    if let Some(b) = backup {
        let _ = fs::remove_file(b);
    }
}

// ---- exec_codex ---------------------------------------------------------------

/// Write provider-only config (config.toml + auth.json) into the real Codex home,
/// inject profile env vars, and exec-replace with `codex`.
///
/// Only `config.toml` and `auth.json` are touched — conversation history, sessions,
/// memories, skills, and all other Codex state in `~/.codex` are left intact.
///
/// Existing config files are backed up before the write.  If any step fails
/// (write or verification), the original files are restored.
pub fn exec_codex(profile: &Profile) -> anyhow::Error {
    if !check_codex_installed() {
        return anyhow::anyhow!(
            "codex CLI not found in PATH. Install it first: npm install -g @openai/codex"
        );
    }

    // Use the real Codex home so conversation history / sessions / memories
    // are shared across profiles.  Only config.toml and auth.json are overwritten.
    let codex_home = codex_home_dir();
    let config_path = codex_home.join("config.toml");
    let auth_path = codex_home.join("auth.json");

    // Back up existing files before touching anything.
    let config_backup = match backup_file(&config_path) {
        Ok(b) => b,
        Err(e) => return anyhow::anyhow!("failed to back up config.toml: {e}"),
    };
    let auth_backup = match backup_file(&auth_path) {
        Ok(b) => b,
        Err(e) => {
            restore_file(&config_path, config_backup.as_ref());
            return anyhow::anyhow!("failed to back up auth.json: {e}");
        }
    };

    // Write + verify config.toml — roll back on any failure.
    if let Err(e) = generate_codex_config(profile, &codex_home) {
        restore_file(&config_path, config_backup.as_ref());
        restore_file(&auth_path, auth_backup.as_ref());
        return anyhow::anyhow!("failed to generate codex config (original restored): {e}");
    }
    if let Err(e) = verify_codex_config(profile, &codex_home) {
        restore_file(&config_path, config_backup.as_ref());
        restore_file(&auth_path, auth_backup.as_ref());
        return anyhow::anyhow!("codex config verification failed — original config restored: {e}");
    }

    // Write + verify auth.json — roll back on any failure.
    if let Err(e) = write_codex_auth(profile, &codex_home) {
        restore_file(&config_path, config_backup.as_ref());
        restore_file(&auth_path, auth_backup.as_ref());
        return anyhow::anyhow!("failed to write codex auth (original restored): {e}");
    }
    if let Err(e) = verify_codex_auth(profile, &codex_home) {
        restore_file(&config_path, config_backup.as_ref());
        restore_file(&auth_path, auth_backup.as_ref());
        return anyhow::anyhow!("codex auth verification failed — original auth restored: {e}");
    }

    // All good — discard backups.
    remove_backup(config_backup.as_ref());
    remove_backup(auth_backup.as_ref());

    // Only set CODEX_HOME if it wasn't already in the environment —
    // ensures Codex sees the same home we wrote to.
    if env::var("CODEX_HOME").is_err() {
        env::set_var("CODEX_HOME", &codex_home);
    }

    if let Some(env_map) = &profile.env {
        for (k, v) in env_map {
            env::set_var(k, v);
        }
    }
    let args = build_codex_args(profile);
    let err = Command::new("codex").args(&args).exec();
    anyhow::anyhow!("exec codex: {err}")
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

    #[test]
    fn generate_codex_config_writes_correct_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let p = codex_profile(
            "my-codex",
            Some("gpt-4.1"),
            Some("https://api.example.com/v1"),
            None,
            None,
        );
        generate_codex_config(&p, tmp.path()).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("config.toml")).unwrap();
        assert!(content.contains("model_provider = \"custom\""));
        assert!(content.contains("model = \"gpt-4.1\""));
        assert!(content.contains("name = \"my-codex\""));
        assert!(content.contains("base_url = \"https://api.example.com/v1\""));
        assert!(content.contains("[model_providers.custom]"));
        assert!(content.contains("requires_openai_auth = true"));
        assert!(!content.contains("env_key"), "should not contain env_key");
    }

    #[test]
    fn build_launch_command_dispatches_claude() {
        let p = profile(Some("opus"), Some(true), Some(vec!["--verbose"]));
        let (bin, args) = build_launch_command(&p, false);
        assert_eq!(bin, "claude");
        assert_eq!(
            args,
            vec![
                "--model",
                "opus",
                "--dangerously-skip-permissions",
                "--verbose"
            ]
        );
    }

    #[test]
    fn build_launch_command_dispatches_claude_with_continue() {
        let p = profile(None, None, None);
        let (bin, args) = build_launch_command(&p, true);
        assert_eq!(bin, "claude");
        assert_eq!(args, vec!["--continue"]);
    }

    #[test]
    fn build_launch_command_dispatches_codex() {
        let p = codex_profile(
            "test",
            None,
            None,
            Some(crate::config::ApprovalLevel::Danger),
            Some(vec!["--quiet"]),
        );
        let (bin, args) = build_launch_command(&p, false);
        assert_eq!(bin, "codex");
        assert_eq!(
            args,
            vec!["--dangerously-bypass-approvals-and-sandbox", "--quiet"]
        );
        // with_continue is ignored for codex
        let (bin2, args2) = build_launch_command(&p, true);
        assert_eq!(bin2, "codex");
        assert_eq!(
            args2,
            vec!["--dangerously-bypass-approvals-and-sandbox", "--quiet"]
        );
    }

    #[test]
    fn exec_codex_calls_write_auth() {
        // write_codex_auth must write auth.json when OPENAI_API_KEY present.
        // We verify the side-effect directly since exec_codex exits the process.
        let tmp = tempfile::tempdir().unwrap();
        let mut env_map = std::collections::HashMap::new();
        env_map.insert("OPENAI_API_KEY".to_string(), "sk-exec-test".to_string());
        let mut p = codex_profile("auth-test", None, None, None, None);
        p.env = Some(env_map);
        write_codex_auth(&p, tmp.path()).unwrap();
        let auth_path = tmp.path().join("auth.json");
        assert!(
            auth_path.exists(),
            "auth.json must exist after write_codex_auth"
        );
        let content = std::fs::read_to_string(&auth_path).unwrap();
        assert!(content.contains("sk-exec-test"));
    }

    #[test]
    fn write_codex_auth_overwrites_existing() {
        let tmp = tempfile::tempdir().unwrap();
        // Write stale content first
        std::fs::write(
            tmp.path().join("auth.json"),
            r#"{"openai_api_key":"old-key"}"#,
        )
        .unwrap();

        let mut env_map = std::collections::HashMap::new();
        env_map.insert("OPENAI_API_KEY".to_string(), "sk-new456".to_string());
        let mut p = codex_profile("test", None, None, None, None);
        p.env = Some(env_map);
        write_codex_auth(&p, tmp.path()).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("auth.json")).unwrap();
        assert!(content.contains("\"OPENAI_API_KEY\": \"sk-new456\""));
    }

    #[test]
    fn write_codex_auth_skips_when_no_key() {
        let tmp = tempfile::tempdir().unwrap();
        let p = codex_profile("test", None, None, None, None); // env is None
        write_codex_auth(&p, tmp.path()).unwrap();
        assert!(!tmp.path().join("auth.json").exists());
    }

    #[test]
    fn write_codex_auth_writes_correct_json() {
        let tmp = tempfile::tempdir().unwrap();
        let mut env_map = std::collections::HashMap::new();
        env_map.insert("OPENAI_API_KEY".to_string(), "sk-test123".to_string());
        let mut p = codex_profile("test", None, None, None, None);
        p.env = Some(env_map);
        write_codex_auth(&p, tmp.path()).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("auth.json")).unwrap();
        assert!(content.contains("\"auth_mode\": \"apikey\""));
        assert!(content.contains("\"OPENAI_API_KEY\": \"sk-test123\""));
    }

    #[test]
    fn generate_codex_config_defaults_model() {
        let tmp = tempfile::tempdir().unwrap();
        let p = codex_profile("fallback", None, None, None, None);
        generate_codex_config(&p, tmp.path()).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("config.toml")).unwrap();
        assert!(content.contains("model = \"gpt-4.1\""));
        assert!(content.contains("base_url = \"\""));
    }

    #[test]
    fn generate_codex_config_with_api_key_uses_env_key() {
        let tmp = tempfile::tempdir().unwrap();
        let mut env_map = std::collections::HashMap::new();
        env_map.insert("OPENAI_API_KEY".to_string(), "sk-test".to_string());
        let mut p = codex_profile(
            "with-key",
            Some("gpt-5"),
            Some("https://api.example.com/v1"),
            None,
            None,
        );
        p.env = Some(env_map);
        generate_codex_config(&p, tmp.path()).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("config.toml")).unwrap();
        assert!(content.contains("env_key = \"OPENAI_API_KEY\""));
        assert!(content.contains("wire_api = \"responses\""));
        assert!(!content.contains("requires_openai_auth"));
    }

    // --- verification tests ---

    #[test]
    fn verify_codex_config_passes_on_valid_config() {
        let tmp = tempfile::tempdir().unwrap();
        let mut env_map = std::collections::HashMap::new();
        env_map.insert("OPENAI_API_KEY".to_string(), "sk-verify".to_string());
        let mut p = codex_profile(
            "verify-me",
            Some("gpt-4.1"),
            Some("https://api.example.com/v1"),
            None,
            None,
        );
        p.env = Some(env_map);
        generate_codex_config(&p, tmp.path()).unwrap();
        // Must not panic / error
        verify_codex_config(&p, tmp.path()).expect("verification should pass on valid config");
    }

    #[test]
    fn verify_codex_config_fails_on_wrong_model() {
        let tmp = tempfile::tempdir().unwrap();
        let p = codex_profile(
            "original",
            Some("gpt-4.1"),
            Some("https://api.example.com/v1"),
            None,
            None,
        );
        generate_codex_config(&p, tmp.path()).unwrap();

        // Verify against a profile with a DIFFERENT model
        let wrong = codex_profile(
            "original",
            Some("wrong-model"),
            Some("https://api.example.com/v1"),
            None,
            None,
        );
        let result = verify_codex_config(&wrong, tmp.path());
        assert!(
            result.is_err(),
            "verification must fail for mismatched model"
        );
    }

    #[test]
    fn verify_codex_config_fails_when_env_key_missing_for_api_key_profile() {
        let tmp = tempfile::tempdir().unwrap();
        // Write config WITHOUT env_key (no API key in profile)
        let p_no_key = codex_profile(
            "no-key",
            Some("gpt-4.1"),
            Some("https://api.example.com/v1"),
            None,
            None,
        );
        generate_codex_config(&p_no_key, tmp.path()).unwrap();

        // Verify with a profile that HAS API key — should fail because config lacks env_key
        let mut env_map = std::collections::HashMap::new();
        env_map.insert("OPENAI_API_KEY".to_string(), "sk-missing".to_string());
        let mut p_with_key = codex_profile(
            "no-key",
            Some("gpt-4.1"),
            Some("https://api.example.com/v1"),
            None,
            None,
        );
        p_with_key.env = Some(env_map);
        let result = verify_codex_config(&p_with_key, tmp.path());
        assert!(
            result.is_err(),
            "verification must fail when env_key missing for API key profile"
        );
    }

    #[test]
    fn verify_codex_auth_passes_on_valid_auth() {
        let tmp = tempfile::tempdir().unwrap();
        let mut env_map = std::collections::HashMap::new();
        env_map.insert("OPENAI_API_KEY".to_string(), "sk-auth-verify".to_string());
        let mut p = codex_profile("auth-test", None, None, None, None);
        p.env = Some(env_map);
        write_codex_auth(&p, tmp.path()).unwrap();
        verify_codex_auth(&p, tmp.path()).expect("auth verification should pass");
    }

    #[test]
    fn verify_codex_auth_fails_on_key_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let mut env_map = std::collections::HashMap::new();
        env_map.insert("OPENAI_API_KEY".to_string(), "sk-original".to_string());
        let mut p = codex_profile("auth-test", None, None, None, None);
        p.env = Some(env_map);
        write_codex_auth(&p, tmp.path()).unwrap();

        // Verify with a different key
        let mut env_map2 = std::collections::HashMap::new();
        env_map2.insert("OPENAI_API_KEY".to_string(), "sk-different".to_string());
        let mut p2 = codex_profile("auth-test", None, None, None, None);
        p2.env = Some(env_map2);
        let result = verify_codex_auth(&p2, tmp.path());
        assert!(
            result.is_err(),
            "verification must fail for API key mismatch"
        );
    }

    #[test]
    fn verify_codex_auth_skips_when_no_key() {
        let tmp = tempfile::tempdir().unwrap();
        let p = codex_profile("no-key", None, None, None, None);
        // No auth.json written, no key in profile — should skip silently
        verify_codex_auth(&p, tmp.path()).expect("verification should skip when no key configured");
    }

    // --- codex_home_dir tests ---

    #[test]
    fn codex_home_dir_respects_env_var() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("CODEX_HOME", tmp.path().as_os_str());
        let home = codex_home_dir();
        assert_eq!(home, tmp.path());
        std::env::remove_var("CODEX_HOME");
    }

    #[test]
    fn codex_home_dir_falls_back_to_dot_codex() {
        // Remove CODEX_HOME to test fallback
        std::env::remove_var("CODEX_HOME");
        let home = codex_home_dir();
        assert!(
            home.ends_with(".codex"),
            "fallback should end with .codex, got: {home:?}"
        );
    }

    // --- backup / restore tests ---

    #[test]
    fn backup_file_skips_when_original_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let nonexistent = tmp.path().join("does-not-exist.toml");
        let backup = backup_file(&nonexistent).unwrap();
        assert!(backup.is_none(), "backup of missing file should be None");
    }

    #[test]
    fn backup_file_creates_backup_when_original_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let original = tmp.path().join("config.toml");
        fs::write(&original, "original content").unwrap();

        let backup = backup_file(&original).unwrap();
        assert!(backup.is_some(), "backup must exist when original exists");
        let backup_path = backup.unwrap();
        assert!(backup_path.exists(), "backup file must be on disk");
        assert_ne!(
            backup_path, original,
            "backup path must differ from original"
        );
        assert!(
            backup_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains("cct-backup"),
            "backup filename must contain 'cct-backup'"
        );
    }

    #[test]
    fn backup_file_preserves_original_content() {
        let tmp = tempfile::tempdir().unwrap();
        let original = tmp.path().join("auth.json");
        fs::write(&original, r#"{"key":"original-value"}"#).unwrap();

        let backup = backup_file(&original).unwrap().unwrap();

        // Original untouched
        assert_eq!(
            fs::read_to_string(&original).unwrap(),
            r#"{"key":"original-value"}"#
        );
        // Backup has same content
        assert_eq!(
            fs::read_to_string(&backup).unwrap(),
            r#"{"key":"original-value"}"#
        );
        // Overwrite original, verify backup still has old content
        fs::write(&original, "new content").unwrap();
        assert_eq!(fs::read_to_string(&original).unwrap(), "new content");
        assert_eq!(
            fs::read_to_string(&backup).unwrap(),
            r#"{"key":"original-value"}"#
        );
    }

    #[test]
    fn restore_file_puts_backup_back() {
        let tmp = tempfile::tempdir().unwrap();
        let original = tmp.path().join("config.toml");
        fs::write(&original, "original").unwrap();
        let backup = backup_file(&original).unwrap().unwrap();

        // Overwrite original
        fs::write(&original, "overwritten").unwrap();
        assert_eq!(fs::read_to_string(&original).unwrap(), "overwritten");

        // Restore
        restore_file(&original, Some(&backup));
        assert_eq!(fs::read_to_string(&original).unwrap(), "original");
        assert!(
            !backup.exists(),
            "backup file must be removed after restore"
        );
    }

    #[test]
    fn restore_file_noop_when_backup_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        let original = tmp.path().join("config.toml");
        fs::write(&original, "keep me").unwrap();
        restore_file(&original, None);
        assert_eq!(fs::read_to_string(&original).unwrap(), "keep me");
    }

    #[test]
    fn remove_backup_cleans_up() {
        let tmp = tempfile::tempdir().unwrap();
        let original = tmp.path().join("config.toml");
        fs::write(&original, "data").unwrap();
        let backup = backup_file(&original).unwrap().unwrap();
        assert!(backup.exists());

        remove_backup(Some(&backup));
        assert!(!backup.exists());
    }

    #[test]
    fn remove_backup_noop_when_none() {
        // Must not panic
        remove_backup(None);
    }
}
