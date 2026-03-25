use crate::config::Profile;
use anyhow::{Context, Result};
use crossterm::{execute, terminal::LeaveAlternateScreen};
use std::{
    env, fs, io,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
};

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
pub fn generate_codex_config(profile: &Profile, codex_home: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(codex_home)?;
    let model = profile.model.as_deref().unwrap_or("gpt-4.1");
    let name = &profile.name;
    let base_url = profile.base_url.as_deref().unwrap_or("");
    let config_content = format!(
        "model_provider = \"custom\"\nmodel = \"{model}\"\n\n[model_providers.custom]\nname = \"{name}\"\nbase_url = \"{base_url}\"\nrequires_openai_auth = true\n"
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

/// Build the CLI argument list for `codex` from a profile. Pure — no side effects.
pub fn build_codex_args(profile: &Profile) -> Vec<String> {
    let mut args = Vec::new();
    if profile.full_auto.unwrap_or(false) {
        args.push("--full-auto".to_string());
    }
    if let Some(extra) = &profile.extra_args {
        args.extend(extra.iter().cloned());
    }
    args
}

/// Generate codex config, inject profile env vars, set CODEX_HOME, and exec-replace with `codex`.
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
        full_auto: Option<bool>,
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
        }
    }

    #[test]
    fn build_codex_args_empty() {
        let p = codex_profile("test", None, None, None, None);
        assert!(build_codex_args(&p).is_empty());
    }

    #[test]
    fn build_codex_args_full_auto_only() {
        let p = codex_profile("test", None, None, Some(true), None);
        assert_eq!(build_codex_args(&p), vec!["--full-auto"]);
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
            Some(true),
            Some(vec!["--quiet", "--json"]),
        );
        assert_eq!(
            build_codex_args(&p),
            vec!["--full-auto", "--quiet", "--json"]
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
        let p = codex_profile("test", None, None, Some(true), Some(vec!["--quiet"]));
        let (bin, args) = build_launch_command(&p, false);
        assert_eq!(bin, "codex");
        assert_eq!(args, vec!["--full-auto", "--quiet"]);
        // with_continue is ignored for codex
        let (bin2, args2) = build_launch_command(&p, true);
        assert_eq!(bin2, "codex");
        assert_eq!(args2, vec!["--full-auto", "--quiet"]);
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
}
