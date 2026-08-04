use anyhow::Result;
use std::io::{self, BufRead, Write};

use crate::config::{self, NewProfile, Profile};

fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        "*".repeat(key.len())
    } else {
        format!("{}...{}", &key[..4], &key[key.len() - 4..])
    }
}

/// Parse the `--backend` flag: "claude" (default) or "codex"
/// (case-insensitive). Errors on unknown values.
fn resolve_backend(backend: Option<String>) -> Result<config::Backend> {
    match backend.as_deref() {
        None => Ok(config::Backend::Claude),
        Some(s) => match s.to_lowercase().as_str() {
            "claude" => Ok(config::Backend::Claude),
            "codex" => Ok(config::Backend::Codex),
            other => {
                anyhow::bail!("Unknown backend: '{other}'. Expected one of: claude, codex")
            }
        },
    }
}

// One flat option per CLI flag — a struct would just relocate the fields.
#[allow(clippy::too_many_arguments)]
pub fn run_add(
    auth_type: Option<String>,
    backend: Option<String>,
    name: Option<String>,
    description: Option<String>,
    base_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    fast_model: Option<String>,
) -> Result<()> {
    let backend = resolve_backend(backend)?;
    // Non-interactive mode: agents and scripts provide --name (and optionally
    // the other fields) and get no prompts or confirmations.
    let Some(name) = name else {
        return run_add_with(io::stdin().lock(), io::stdout(), auth_type, backend);
    };
    let name = name.trim().to_string();
    if name.is_empty() {
        anyhow::bail!("Error: --name must not be empty.");
    }
    if config::profile_name_exists(&name)? {
        anyhow::bail!("Error: profile '{}' already exists.", name);
    }
    let profile = NewProfile {
        name: name.clone(),
        description,
        base_url,
        api_key,
        model,
        fast_model,
        backend,
        full_auto: None,
        auth_type,
    };
    config::append_profile(&profile)?;
    println!("Profile '{}' added.", name);
    Ok(())
}

pub fn run_add_with<R: BufRead, W: Write>(
    mut reader: R,
    mut writer: W,
    auth_type: Option<String>,
    backend: config::Backend,
) -> Result<()> {
    // Name (required)
    let name = loop {
        write!(writer, "Name: ")?;
        writer.flush()?;
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            writeln!(writer, "Name is required.")?;
            continue;
        }
        if config::profile_name_exists(&trimmed)? {
            eprintln!("Error: profile '{}' already exists.", trimmed);
            std::process::exit(1);
        }
        break trimmed;
    };

    // Description (optional)
    write!(writer, "Description (optional): ")?;
    writer.flush()?;
    let mut desc_line = String::new();
    reader.read_line(&mut desc_line)?;
    let description = {
        let t = desc_line.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    };

    // Base URL (optional)
    write!(writer, "Base URL (optional): ")?;
    writer.flush()?;
    let mut url_line = String::new();
    reader.read_line(&mut url_line)?;
    let base_url = {
        let t = url_line.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    };

    // API Key (optional)
    write!(writer, "API Key (optional): ")?;
    writer.flush()?;
    let mut key_line = String::new();
    reader.read_line(&mut key_line)?;
    let api_key = {
        let t = key_line.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    };

    // Model (optional)
    write!(writer, "Model (optional): ")?;
    writer.flush()?;
    let mut model_line = String::new();
    reader.read_line(&mut model_line)?;
    let model = {
        let t = model_line.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    };

    // Fast Model (optional)
    write!(writer, "Fast Model (optional, for Haiku/SmallFast tier): ")?;
    writer.flush()?;
    let mut fast_model_line = String::new();
    reader.read_line(&mut fast_model_line)?;
    let fast_model = {
        let t = fast_model_line.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    };

    // Summary
    writeln!(writer)?;
    writeln!(writer, "--- New Profile ---")?;
    writeln!(writer, "  Name:        {}", name)?;
    writeln!(
        writer,
        "  Description: {}",
        description.as_deref().unwrap_or("(none)")
    )?;
    writeln!(
        writer,
        "  Base URL:    {}",
        base_url.as_deref().unwrap_or("(none)")
    )?;
    writeln!(
        writer,
        "  API Key:     {}",
        api_key
            .as_ref()
            .map(|k| mask_key(k))
            .unwrap_or_else(|| "(none)".into())
    )?;
    writeln!(
        writer,
        "  Model:       {}",
        model.as_deref().unwrap_or("(none)")
    )?;
    writeln!(
        writer,
        "  Fast Model:  {}",
        fast_model.as_deref().unwrap_or("(none)")
    )?;
    writeln!(writer)?;

    // Confirm
    write!(writer, "Save? (y/n): ")?;
    writer.flush()?;
    let mut confirm = String::new();
    reader.read_line(&mut confirm)?;
    if confirm.trim().to_lowercase() != "y" {
        writeln!(writer, "Cancelled.")?;
        return Ok(());
    }

    let profile = NewProfile {
        name: name.clone(),
        description,
        base_url,
        api_key,
        model,
        fast_model,
        backend,
        full_auto: None,
        auth_type,
    };
    config::append_profile(&profile)?;
    writeln!(writer, "Profile '{}' added.", name)?;
    Ok(())
}

pub fn run_pick_profile(profiles: &[Profile]) -> Result<usize> {
    run_pick_profile_with(profiles, io::stdin().lock(), io::stdout())
}

pub fn run_pick_profile_with<R: BufRead, W: Write>(
    profiles: &[Profile],
    mut reader: R,
    mut writer: W,
) -> Result<usize> {
    if profiles.is_empty() {
        anyhow::bail!("No profiles configured. Run 'cct add' to create one.");
    }
    writeln!(writer, "Select a profile:")?;
    for (i, p) in profiles.iter().enumerate() {
        let desc = p.description.as_deref().unwrap_or("");
        let tag = match p.backend {
            config::Backend::Claude => "[claude]",
            config::Backend::Codex => "[codex]",
        };
        writeln!(writer, "  {}) {} {} {}", i + 1, p.name, tag, desc)?;
    }
    write!(writer, "Enter number (1-{}): ", profiles.len())?;
    writer.flush()?;

    let mut line = String::new();
    reader.read_line(&mut line)?;
    let num: usize = line
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid input: '{}'. Expected a number.", line.trim()))?;
    if num == 0 || num > profiles.len() {
        anyhow::bail!(
            "Number out of range: {}. Choose between 1 and {}.",
            num,
            profiles.len()
        );
    }
    Ok(num - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn cli_run_add_rejects_duplicate() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, "[[profiles]]\nname = \"existing\"\n").unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        // Verify duplicate detection
        assert!(config::profile_name_exists("existing").unwrap());
        assert!(config::profile_name_exists("EXISTING").unwrap());

        // Test that a valid add flow works (6 fields: name, desc, base_url, api_key, model, fast_model)
        let input = b"newprofile\nmy desc\nhttps://api.example.com\nsk-test\nMiniMax-M2.1\n\ny\n";
        let mut output: Vec<u8> = Vec::new();
        run_add_with(&input[..], &mut output, None, config::Backend::Claude).unwrap();

        let profiles = config::load_profiles().unwrap();
        assert_eq!(profiles.len(), 2);
        assert!(profiles.iter().any(|p| p.name == "newprofile"));

        // Verify env var generation
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[profiles.env]"));
        assert!(content.contains("ANTHROPIC_BASE_URL"));
        assert!(content.contains("ANTHROPIC_MODEL = \"MiniMax-M2.1\""));

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn run_add_flags_skips_prompts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, "[[profiles]]\nname = \"existing\"\n").unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        // All fields via flags — no stdin involved, no confirmation
        run_add(
            None,
            Some("claude".into()),
            Some("flag-profile".into()),
            Some("from flags".into()),
            Some("https://api.example.com".into()),
            Some("sk-flag".into()),
            Some("MiniMax-M2.1".into()),
            None,
        )
        .unwrap();

        let profiles = config::load_profiles().unwrap();
        let p = profiles.iter().find(|p| p.name == "flag-profile").unwrap();
        assert_eq!(p.description.as_deref(), Some("from flags"));
        assert_eq!(p.base_url.as_deref(), Some("https://api.example.com"));
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("ANTHROPIC_BASE_URL"));

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn run_add_flags_rejects_empty_name() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, "").unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let err =
            run_add(None, None, Some("   ".into()), None, None, None, None, None).unwrap_err();
        assert!(err.to_string().contains("--name"), "got: {err}");

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn run_add_flags_rejects_duplicate() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, "[[profiles]]\nname = \"existing\"\n").unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        // Case-insensitive duplicate guard applies to flag mode too
        let err = run_add(
            None,
            None,
            Some("EXISTING".into()),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();
        assert!(err.to_string().contains("already exists"), "got: {err}");

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn cli_add_sets_claude_backend_and_no_full_auto() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, "[[profiles]]\nname = \"existing\"\n").unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        // Input: name, desc, base_url, api_key, model, fast_model, confirm
        let input = b"cli-test\nsome desc\n\n\n\n\ny\n";
        let mut output: Vec<u8> = Vec::new();
        run_add_with(&input[..], &mut output, None, config::Backend::Claude).unwrap();

        let profiles = config::load_profiles().unwrap();
        let p = profiles.iter().find(|p| p.name == "cli-test").unwrap();
        assert_eq!(p.backend, config::Backend::Claude);
        assert!(
            p.full_auto.is_none(),
            "CLI add for Claude should not set full_auto"
        );

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    fn pick_profile_selects_valid() {
        let profiles = vec![
            Profile {
                name: "alpha".into(),
                description: Some("First".into()),
                env: None,
                extra_args: None,
                skip_permissions: None,
                model: None,
                backend: config::Backend::Claude,
                base_url: None,
                full_auto: None,
                auth_type: None,
            },
            Profile {
                name: "beta".into(),
                description: None,
                env: None,
                extra_args: None,
                skip_permissions: None,
                model: None,
                backend: config::Backend::Codex,
                base_url: None,
                full_auto: None,
                auth_type: None,
            },
        ];
        let input = b"2\n";
        let mut output: Vec<u8> = Vec::new();
        let idx = run_pick_profile_with(&profiles, &input[..], &mut output).unwrap();
        assert_eq!(idx, 1);
        let out = String::from_utf8(output).unwrap();
        assert!(out.contains("alpha"));
        assert!(out.contains("[claude]"));
        assert!(out.contains("[codex]"));
    }

    #[test]
    fn pick_profile_rejects_invalid_input() {
        let profiles = vec![Profile {
            name: "only".into(),
            description: None,
            env: None,
            extra_args: None,
            skip_permissions: None,
            model: None,
            backend: config::Backend::Claude,
            base_url: None,
            full_auto: None,
            auth_type: None,
        }];
        let input = b"abc\n";
        let mut output: Vec<u8> = Vec::new();
        let err = run_pick_profile_with(&profiles, &input[..], &mut output).unwrap_err();
        assert!(err.to_string().contains("Invalid input"));
    }

    #[test]
    fn pick_profile_rejects_out_of_range() {
        let profiles = vec![Profile {
            name: "only".into(),
            description: None,
            env: None,
            extra_args: None,
            skip_permissions: None,
            model: None,
            backend: config::Backend::Claude,
            base_url: None,
            full_auto: None,
            auth_type: None,
        }];
        let input = b"99\n";
        let mut output: Vec<u8> = Vec::new();
        let err = run_pick_profile_with(&profiles, &input[..], &mut output).unwrap_err();
        assert!(err.to_string().contains("out of range"));
    }

    #[test]
    fn pick_profile_empty_list_errors() {
        let input = b"1\n";
        let mut output: Vec<u8> = Vec::new();
        let err = run_pick_profile_with(&[], &input[..], &mut output).unwrap_err();
        assert!(err.to_string().contains("No profiles"));
    }

    #[test]
    fn resolve_backend_parses_known_backends() {
        assert_eq!(resolve_backend(None).unwrap(), config::Backend::Claude);
        assert_eq!(
            resolve_backend(Some("claude".into())).unwrap(),
            config::Backend::Claude
        );
        assert_eq!(
            resolve_backend(Some("codex".into())).unwrap(),
            config::Backend::Codex
        );
        // Unknown values are rejected
        let err = resolve_backend(Some("gpt".into())).unwrap_err();
        assert!(err.to_string().contains("Unknown backend"));
    }
}
