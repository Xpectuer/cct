use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, fs, path::PathBuf};
use toml_edit::{value, Item, Table};

#[derive(Debug, Default, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    #[default]
    Claude,
    Codex,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Profile {
    pub name: String,
    pub description: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub extra_args: Option<Vec<String>>,
    pub skip_permissions: Option<bool>,
    pub model: Option<String>,
    #[serde(default)]
    pub backend: Backend,
    pub base_url: Option<String>,
    pub full_auto: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct Config {
    profiles: Vec<Profile>,
}

pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("CCT_CONFIG") {
        return PathBuf::from(p);
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("cc-tui")
        .join("profiles.toml")
}

const DEFAULT_CONFIG: &str = r#"# cct — Claude Code TUI profile configuration
# Each [[profiles]] block defines one launch profile.

[[profiles]]
name = "default"
description = "Default Claude Code"
# model = "claude-sonnet-4-6"
# skip_permissions = false
# extra_args = []

# [profiles.env]
# ANTHROPIC_API_KEY = "sk-ant-..."
"#;

pub fn ensure_default_config() -> Result<()> {
    let path = config_path();
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create config dir {parent:?}"))?;
        }
        fs::write(&path, DEFAULT_CONFIG)
            .with_context(|| format!("write default config to {path:?}"))?;
    }
    Ok(())
}

pub fn validate_profiles(profiles: &[Profile]) -> Result<()> {
    for p in profiles {
        if p.backend == Backend::Codex && p.skip_permissions.unwrap_or(false) {
            anyhow::bail!(
                "Profile {:?}: codex backend does not support skip_permissions",
                p.name
            );
        }
        if p.backend == Backend::Claude && p.full_auto.unwrap_or(false) {
            anyhow::bail!(
                "Profile {:?}: claude backend does not support full_auto",
                p.name
            );
        }
    }
    Ok(())
}

pub fn load_profiles() -> Result<Vec<Profile>> {
    let path = config_path();
    let content = fs::read_to_string(&path).with_context(|| format!("read config {path:?}"))?;
    let config: Config =
        toml::from_str(&content).with_context(|| format!("parse TOML in {path:?}"))?;
    validate_profiles(&config.profiles)?;
    Ok(config.profiles)
}

pub struct NewProfile {
    pub name: String,
    pub description: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub fast_model: Option<String>,
    pub backend: Backend,
    pub full_auto: Option<bool>,
}

pub fn profile_name_exists(name: &str) -> Result<bool> {
    let profiles = load_profiles()?;
    Ok(profiles.iter().any(|p| p.name.eq_ignore_ascii_case(name)))
}

pub fn find_profile_by_name(name: &str) -> Result<Option<Profile>> {
    let profiles = load_profiles()?;
    Ok(profiles
        .into_iter()
        .find(|p| p.name.eq_ignore_ascii_case(name)))
}

/// Returns the non-empty string value if present, or None.
fn non_empty(opt: &Option<String>) -> Option<&str> {
    opt.as_deref().filter(|s| !s.is_empty())
}

fn set_optional_string(entry: &mut Table, key: &str, new_value: Option<&str>) {
    if let Some(new_str) = new_value {
        entry[key] = value(new_str);
    } else {
        entry.remove(key);
    }
}

fn set_optional_bool(entry: &mut Table, key: &str, new_value: Option<bool>) {
    if let Some(new_bool) = new_value {
        entry[key] = value(new_bool);
    } else {
        entry.remove(key);
    }
}

fn ensure_env_table(entry: &mut Table) -> &mut Table {
    if !matches!(entry.get("env"), Some(Item::Table(_))) {
        entry["env"] = Item::Table(Table::new());
    }
    entry["env"]
        .as_table_mut()
        .expect("env item should be a table")
}

fn prune_empty_env_table(entry: &mut Table) {
    let should_remove = entry
        .get("env")
        .and_then(Item::as_table)
        .map(Table::is_empty)
        .unwrap_or(false);
    if should_remove {
        entry.remove("env");
    }
}

pub fn update_profile(original_name: &str, updated: &NewProfile) -> Result<()> {
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

    entry["name"] = value(updated.name.as_str());
    set_optional_string(entry, "description", non_empty(&updated.description));
    set_optional_string(entry, "model", non_empty(&updated.model));
    set_optional_string(entry, "base_url", non_empty(&updated.base_url));

    match updated.backend {
        Backend::Claude => {
            entry.remove("backend");
            entry.remove("full_auto");

            let env = ensure_env_table(entry);
            set_optional_string(env, "ANTHROPIC_BASE_URL", non_empty(&updated.base_url));
            set_optional_string(env, "ANTHROPIC_API_KEY", non_empty(&updated.api_key));

            if let Some(model) = non_empty(&updated.model) {
                for key in [
                    "ANTHROPIC_MODEL",
                    "ANTHROPIC_DEFAULT_SONNET_MODEL",
                    "ANTHROPIC_DEFAULT_OPUS_MODEL",
                    "CLAUDE_CODE_SUBAGENT_MODEL",
                ] {
                    env[key] = value(model);
                }
                env["API_TIMEOUT_MS"] = value("600000");
                env["CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"] = value("1");
                env["CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK"] = value("1");
                env["CLAUDE_CODE_EFFORT_LEVEL"] = value("max");
            } else {
                for key in [
                    "ANTHROPIC_MODEL",
                    "ANTHROPIC_DEFAULT_SONNET_MODEL",
                    "ANTHROPIC_DEFAULT_OPUS_MODEL",
                    "CLAUDE_CODE_SUBAGENT_MODEL",
                    "API_TIMEOUT_MS",
                    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
                    "CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK",
                    "CLAUDE_CODE_EFFORT_LEVEL",
                ] {
                    env.remove(key);
                }
            }

            if let Some(fm) = non_empty(&updated.fast_model) {
                env["ANTHROPIC_DEFAULT_HAIKU_MODEL"] = value(fm);
                env["ANTHROPIC_SMALL_FAST_MODEL"] = value(fm);
            } else {
                for key in [
                    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
                    "ANTHROPIC_SMALL_FAST_MODEL",
                ] {
                    env.remove(key);
                }
            }
        }
        Backend::Codex => {
            entry["backend"] = value("codex");
            entry.remove("description");
            set_optional_bool(entry, "full_auto", updated.full_auto);

            let env = ensure_env_table(entry);
            set_optional_string(env, "OPENAI_API_KEY", non_empty(&updated.api_key));
        }
    }

    prune_empty_env_table(entry);
    fs::write(&path, doc.to_string()).with_context(|| format!("write config {path:?}"))?;
    Ok(())
}

pub fn append_profile(profile: &NewProfile) -> Result<()> {
    let path = config_path();
    let mut block = String::from("\n[[profiles]]\n");
    block.push_str(&format!("name = {:?}\n", profile.name));
    if let Some(desc) = non_empty(&profile.description) {
        block.push_str(&format!("description = {:?}\n", desc));
    }
    if profile.backend != Backend::Claude {
        block.push_str(&format!(
            "backend = {:?}\n",
            match profile.backend {
                Backend::Codex => "codex",
                Backend::Claude => "claude",
            }
        ));
    }
    if let Some(model) = non_empty(&profile.model) {
        block.push_str(&format!("model = {:?}\n", model));
    }
    if let Some(base_url) = non_empty(&profile.base_url) {
        block.push_str(&format!("base_url = {:?}\n", base_url));
    }
    if let Some(full_auto) = profile.full_auto {
        block.push_str(&format!("full_auto = {full_auto}\n"));
    }

    match profile.backend {
        Backend::Claude => {
            let base_url = non_empty(&profile.base_url);
            let api_key = non_empty(&profile.api_key);
            let model = non_empty(&profile.model);
            let fast_model = non_empty(&profile.fast_model);

            if base_url.is_some() || api_key.is_some() || model.is_some() || fast_model.is_some() {
                block.push_str("\n[profiles.env]\n");
                if let Some(url) = base_url {
                    block.push_str(&format!("ANTHROPIC_BASE_URL = {:?}\n", url));
                }
                if let Some(key) = api_key {
                    block.push_str(&format!("ANTHROPIC_API_KEY = {:?}\n", key));
                }
                if let Some(m) = model {
                    block.push_str(&format!("ANTHROPIC_MODEL = {:?}\n", m));
                    block.push_str(&format!("ANTHROPIC_DEFAULT_SONNET_MODEL = {:?}\n", m));
                    block.push_str(&format!("ANTHROPIC_DEFAULT_OPUS_MODEL = {:?}\n", m));
                    block.push_str(&format!("CLAUDE_CODE_SUBAGENT_MODEL = {:?}\n", m));
                    block.push_str("API_TIMEOUT_MS = \"600000\"\n");
                    block.push_str("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = \"1\"\n");
                    block.push_str("CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK = \"1\"\n");
                    block.push_str("CLAUDE_CODE_EFFORT_LEVEL = \"max\"\n");
                }
                if let Some(fm) = fast_model {
                    block.push_str(&format!("ANTHROPIC_DEFAULT_HAIKU_MODEL = {:?}\n", fm));
                    block.push_str(&format!("ANTHROPIC_SMALL_FAST_MODEL = {:?}\n", fm));
                }
            }
        }
        Backend::Codex => {
            let api_key = non_empty(&profile.api_key);
            if api_key.is_some() {
                block.push_str("\n[profiles.env]\n");
                if let Some(key) = api_key {
                    block.push_str(&format!("OPENAI_API_KEY = {:?}\n", key));
                }
            }
        }
    }

    let mut content = fs::read_to_string(&path).with_context(|| format!("read config {path:?}"))?;
    content.push_str(&block);
    fs::write(&path, content).with_context(|| format!("write config {path:?}"))?;
    Ok(())
}

/// Toggle `skip_permissions` for a named profile in the config file.
/// Uses toml_edit for surgical edits that preserve comments and formatting.
pub fn toggle_skip_permissions(profile_name: &str, new_value: bool) -> Result<()> {
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
        .find(|t| t.get("name").and_then(|v| v.as_str()) == Some(profile_name))
        .with_context(|| format!("profile {profile_name:?} not found in config"))?;

    entry["skip_permissions"] = value(new_value);
    fs::write(&path, doc.to_string()).with_context(|| format!("write config {path:?}"))?;
    Ok(())
}

/// Toggle `full_auto` for a named Codex profile in the config file.
/// Uses toml_edit for surgical edits that preserve comments and formatting.
pub fn toggle_full_auto(profile_name: &str, new_value: bool) -> Result<()> {
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
        .find(|t| t.get("name").and_then(|v| v.as_str()) == Some(profile_name))
        .with_context(|| format!("profile {profile_name:?} not found in config"))?;

    entry["full_auto"] = value(new_value);
    fs::write(&path, doc.to_string()).with_context(|| format!("write config {path:?}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn parse_full_profile() {
        let src = r#"
[[profiles]]
name = "kclaude"
description = "Kimi AI"
model = "kimi-k1.5"
skip_permissions = true
extra_args = ["--verbose"]

[profiles.env]
ANTHROPIC_BASE_URL = "https://api.example.com"
ANTHROPIC_AUTH_TOKEN = "sk-secret"
"#;
        let cfg: Config = toml::from_str(src).unwrap();
        assert_eq!(cfg.profiles.len(), 1);
        let p = &cfg.profiles[0];
        assert_eq!(p.name, "kclaude");
        assert_eq!(p.model.as_deref(), Some("kimi-k1.5"));
        assert_eq!(p.skip_permissions, Some(true));
        assert_eq!(
            p.extra_args.as_deref(),
            Some(&["--verbose".to_string()][..])
        );
        let env = p.env.as_ref().unwrap();
        assert_eq!(
            env.get("ANTHROPIC_BASE_URL").map(String::as_str),
            Some("https://api.example.com")
        );
    }

    #[test]
    fn parse_minimal_profile() {
        let src = "[[profiles]]\nname = \"default\"";
        let cfg: Config = toml::from_str(src).unwrap();
        assert_eq!(cfg.profiles[0].name, "default");
        assert!(cfg.profiles[0].description.is_none());
        assert!(cfg.profiles[0].env.is_none());
    }

    #[test]
    fn default_config_is_valid_toml() {
        let _: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
    }

    #[test]
    #[serial]
    fn append_profile_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, DEFAULT_CONFIG).unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let new = NewProfile {
            name: "test-profile".into(),
            description: Some("A test".into()),
            base_url: None,
            api_key: None,
            model: Some("claude-sonnet-4-6".into()),
            fast_model: None,
            backend: Backend::Claude,
            full_auto: None,
        };
        append_profile(&new).unwrap();
        let profiles = load_profiles().unwrap();
        assert!(profiles.iter().any(|p| p.name == "test-profile"));
        assert_eq!(profiles.len(), 2); // default + new

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn append_preserves_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        let original = "# My comment\n\n[[profiles]]\nname = \"orig\"\n";
        std::fs::write(&path, original).unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let new = NewProfile {
            name: "added".into(),
            description: None,
            base_url: None,
            api_key: None,
            model: None,
            fast_model: None,
            backend: Backend::Claude,
            full_auto: None,
        };
        append_profile(&new).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("# My comment"));
        let profiles = load_profiles().unwrap();
        assert_eq!(profiles.len(), 2);

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn profile_name_exists_case_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, "[[profiles]]\nname = \"MyProfile\"\n").unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        assert!(profile_name_exists("myprofile").unwrap());
        assert!(profile_name_exists("MYPROFILE").unwrap());
        assert!(!profile_name_exists("other").unwrap());

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn find_profile_by_name_returns_profile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(
            &path,
            "[[profiles]]\nname = \"MyProfile\"\ndescription = \"Test\"\n",
        )
        .unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let p = find_profile_by_name("myprofile")
            .unwrap()
            .expect("profile should exist");
        assert_eq!(p.name, "MyProfile");
        assert_eq!(p.description.as_deref(), Some("Test"));

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn find_profile_by_name_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, "[[profiles]]\nname = \"other\"\n").unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        assert!(find_profile_by_name("missing").unwrap().is_none());

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn append_profile_generates_env_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, DEFAULT_CONFIG).unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let new = NewProfile {
            name: "env-test".into(),
            description: Some("Test env generation".into()),
            base_url: Some("https://api.example.com".into()),
            api_key: Some("sk-test-key-123".into()),
            model: Some("kimi-k2".into()),
            fast_model: None,
            backend: Backend::Claude,
            full_auto: None,
        };
        append_profile(&new).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        // Must contain [profiles.env] section
        assert!(
            content.contains("ANTHROPIC_BASE_URL"),
            "Expected ANTHROPIC_BASE_URL in output, got:\n{content}"
        );
        assert!(
            content.contains("https://api.example.com"),
            "Expected base_url value in output"
        );
        assert!(
            content.contains("ANTHROPIC_API_KEY"),
            "Expected ANTHROPIC_API_KEY in output"
        );
        assert!(
            content.contains("sk-test-key-123"),
            "Expected api_key value in output"
        );
        assert!(
            content.contains("ANTHROPIC_MODEL"),
            "Expected ANTHROPIC_MODEL in output"
        );
        assert!(
            content.contains("ANTHROPIC_DEFAULT_SONNET_MODEL"),
            "Expected ANTHROPIC_DEFAULT_SONNET_MODEL in output"
        );
        assert!(
            content.contains("ANTHROPIC_DEFAULT_OPUS_MODEL"),
            "Expected ANTHROPIC_DEFAULT_OPUS_MODEL in output"
        );
        assert!(
            content.contains("CLAUDE_CODE_SUBAGENT_MODEL"),
            "Expected CLAUDE_CODE_SUBAGENT_MODEL in output"
        );
        assert!(
            content.contains("API_TIMEOUT_MS"),
            "Expected API_TIMEOUT_MS in output"
        );
        assert!(
            content.contains("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"),
            "Expected CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC in output"
        );
        assert!(
            content.contains("CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK"),
            "Expected CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK in output"
        );
        assert!(
            content.contains("CLAUDE_CODE_EFFORT_LEVEL"),
            "Expected CLAUDE_CODE_EFFORT_LEVEL in output"
        );
        // fast_model is None, so HAIKU_MODEL and SMALL_FAST_MODEL should NOT be present
        assert!(
            !content.contains("ANTHROPIC_DEFAULT_HAIKU_MODEL"),
            "ANTHROPIC_DEFAULT_HAIKU_MODEL should NOT be present when fast_model is None"
        );
        assert!(
            !content.contains("ANTHROPIC_SMALL_FAST_MODEL"),
            "ANTHROPIC_SMALL_FAST_MODEL should NOT be present when fast_model is None"
        );

        // Verify the profile round-trips through TOML parsing
        let profiles = load_profiles().unwrap();
        let p = profiles.iter().find(|p| p.name == "env-test").unwrap();
        let env = p.env.as_ref().expect("env section should exist");
        assert_eq!(
            env.get("ANTHROPIC_BASE_URL").map(String::as_str),
            Some("https://api.example.com")
        );
        assert_eq!(
            env.get("ANTHROPIC_API_KEY").map(String::as_str),
            Some("sk-test-key-123")
        );
        assert_eq!(
            env.get("ANTHROPIC_MODEL").map(String::as_str),
            Some("kimi-k2")
        );

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn append_profile_base_url_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, DEFAULT_CONFIG).unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let new = NewProfile {
            name: "base-url-only".into(),
            description: Some("Only base URL".into()),
            base_url: Some("https://api.third-party.com/v1".into()),
            api_key: None,
            model: None,
            fast_model: None,
            backend: Backend::Claude,
            full_auto: None,
        };
        append_profile(&new).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        // Must contain [profiles.env] with ANTHROPIC_BASE_URL
        assert!(
            content.contains("[profiles.env]"),
            "Expected [profiles.env] section in output, got:\n{content}"
        );
        assert!(
            content.contains("ANTHROPIC_BASE_URL"),
            "Expected ANTHROPIC_BASE_URL in output"
        );
        assert!(
            content.contains("https://api.third-party.com/v1"),
            "Expected base_url value in output"
        );
        // Must NOT contain model-derived env vars
        assert!(
            !content.contains("ANTHROPIC_MODEL"),
            "ANTHROPIC_MODEL should NOT be present when model is None"
        );
        assert!(
            !content.contains("API_TIMEOUT_MS"),
            "API_TIMEOUT_MS should NOT be present when model is None"
        );

        // Round-trip verification
        let profiles = load_profiles().unwrap();
        let p = profiles.iter().find(|p| p.name == "base-url-only").unwrap();
        let env = p.env.as_ref().expect("env section should exist");
        assert_eq!(
            env.get("ANTHROPIC_BASE_URL").map(String::as_str),
            Some("https://api.third-party.com/v1")
        );
        assert!(
            env.get("ANTHROPIC_MODEL").is_none(),
            "ANTHROPIC_MODEL should not exist in env"
        );

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn append_minimal_no_env_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, DEFAULT_CONFIG).unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let new = NewProfile {
            name: "no-env".into(),
            description: Some("No env vars at all".into()),
            base_url: None,
            api_key: None,
            model: None,
            fast_model: None,
            backend: Backend::Claude,
            full_auto: None,
        };
        append_profile(&new).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        // The appended block should NOT contain [profiles.env]
        // Find the appended block by locating name = "no-env"
        let block_start = content
            .find("name = \"no-env\"")
            .expect("profile should exist");
        let appended_block = &content[block_start..];
        assert!(
            !appended_block.contains("[profiles.env]"),
            "Expected NO [profiles.env] section for minimal profile, got:\n{appended_block}"
        );
        assert!(
            !appended_block.contains("ANTHROPIC_BASE_URL"),
            "Expected NO ANTHROPIC_BASE_URL for minimal profile"
        );
        assert!(
            !appended_block.contains("ANTHROPIC_API_KEY"),
            "Expected NO ANTHROPIC_API_KEY for minimal profile"
        );
        assert!(
            !appended_block.contains("ANTHROPIC_MODEL"),
            "Expected NO ANTHROPIC_MODEL for minimal profile"
        );

        // Round-trip: env should be None
        let profiles = load_profiles().unwrap();
        let p = profiles.iter().find(|p| p.name == "no-env").unwrap();
        assert!(
            p.env.is_none(),
            "env section should be None for minimal profile"
        );

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn toggle_skip_permissions_insert() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, "# comment\n[[profiles]]\nname = \"test\"\n").unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        toggle_skip_permissions("test", true).unwrap();
        let profiles = load_profiles().unwrap();
        assert_eq!(profiles[0].skip_permissions, Some(true));

        // Verify comment is preserved
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# comment"), "comment should be preserved");

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn toggle_skip_permissions_flip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(
            &path,
            "[[profiles]]\nname = \"test\"\nskip_permissions = true\n",
        )
        .unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        toggle_skip_permissions("test", false).unwrap();
        let profiles = load_profiles().unwrap();
        assert_eq!(profiles[0].skip_permissions, Some(false));

        toggle_skip_permissions("test", true).unwrap();
        let profiles = load_profiles().unwrap();
        assert_eq!(profiles[0].skip_permissions, Some(true));

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn toggle_skip_permissions_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, "[[profiles]]\nname = \"other\"\n").unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let result = toggle_skip_permissions("missing", true);
        assert!(result.is_err());

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn append_minimal_profile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, DEFAULT_CONFIG).unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let new = NewProfile {
            name: "minimal".into(),
            description: None,
            base_url: None,
            api_key: None,
            model: None,
            fast_model: None,
            backend: Backend::Claude,
            full_auto: None,
        };
        append_profile(&new).unwrap();
        let profiles = load_profiles().unwrap();
        let p = profiles.iter().find(|p| p.name == "minimal").unwrap();
        assert!(p.description.is_none());
        assert!(p.model.is_none());

        std::env::remove_var("CCT_CONFIG");
    }

    // --- TDD Step 1: New test cases ---

    #[test]
    fn backend_enum_deserialization() {
        // No backend field => defaults to Claude
        let src = "[[profiles]]\nname = \"default\"\n";
        let cfg: Config = toml::from_str(src).unwrap();
        assert_eq!(cfg.profiles[0].backend, Backend::Claude);

        // Explicit backend = "codex"
        let src2 = "[[profiles]]\nname = \"codex-test\"\nbackend = \"codex\"\n";
        let cfg2: Config = toml::from_str(src2).unwrap();
        assert_eq!(cfg2.profiles[0].backend, Backend::Codex);

        // Explicit backend = "claude"
        let src3 = "[[profiles]]\nname = \"claude-test\"\nbackend = \"claude\"\n";
        let cfg3: Config = toml::from_str(src3).unwrap();
        assert_eq!(cfg3.profiles[0].backend, Backend::Claude);
    }

    #[test]
    fn profile_with_base_url_roundtrips() {
        let src = r#"
[[profiles]]
name = "custom"
base_url = "https://api.example.com/v1"
"#;
        let cfg: Config = toml::from_str(src).unwrap();
        let p = &cfg.profiles[0];
        assert_eq!(p.base_url.as_deref(), Some("https://api.example.com/v1"));
    }

    #[test]
    fn validate_profiles_rejects_codex_skip_permissions() {
        let profiles = vec![Profile {
            name: "bad-codex".into(),
            description: None,
            env: None,
            extra_args: None,
            skip_permissions: Some(true),
            model: None,
            backend: Backend::Codex,
            base_url: None,
            full_auto: None,
        }];
        let result = validate_profiles(&profiles);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("codex") || msg.contains("skip_permissions"),
            "Error should mention codex or skip_permissions, got: {msg}"
        );
    }

    #[test]
    fn validate_profiles_rejects_claude_full_auto() {
        let profiles = vec![Profile {
            name: "bad-claude".into(),
            description: None,
            env: None,
            extra_args: None,
            skip_permissions: None,
            model: None,
            backend: Backend::Claude,
            base_url: None,
            full_auto: Some(true),
        }];
        let result = validate_profiles(&profiles);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("claude") || msg.contains("full_auto"),
            "Error should mention claude or full_auto, got: {msg}"
        );
    }

    #[test]
    #[serial]
    fn toggle_full_auto_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(
            &path,
            "[[profiles]]\nname = \"other\"\nbackend = \"codex\"\n",
        )
        .unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let result = toggle_full_auto("missing", true);
        assert!(result.is_err());

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn toggle_full_auto_flip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(
            &path,
            "[[profiles]]\nname = \"codex-test\"\nbackend = \"codex\"\nfull_auto = true\n",
        )
        .unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        toggle_full_auto("codex-test", false).unwrap();
        let profiles = load_profiles().unwrap();
        assert_eq!(profiles[0].full_auto, Some(false));

        toggle_full_auto("codex-test", true).unwrap();
        let profiles = load_profiles().unwrap();
        assert_eq!(profiles[0].full_auto, Some(true));

        std::env::remove_var("CCT_CONFIG");
    }

    // --- toggle_full_auto tests ---

    #[test]
    #[serial]
    fn toggle_full_auto_insert() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(
            &path,
            "# comment\n[[profiles]]\nname = \"codex-test\"\nbackend = \"codex\"\n",
        )
        .unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        toggle_full_auto("codex-test", true).unwrap();
        let profiles = load_profiles().unwrap();
        assert_eq!(profiles[0].full_auto, Some(true));

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# comment"), "comment should be preserved");

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn append_codex_profile_generates_openai_env() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, DEFAULT_CONFIG).unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let new = NewProfile {
            name: "codex-profile".into(),
            description: Some("Codex backend".into()),
            base_url: Some("https://api.openai.com/v1".into()),
            api_key: Some("sk-openai-key-123".into()),
            model: Some("o3".into()),
            fast_model: None,
            backend: Backend::Codex,
            full_auto: Some(true),
        };
        append_profile(&new).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        // Extract only the appended block (after "codex-profile")
        let block_start = content
            .find("name = \"codex-profile\"")
            .expect("codex profile should exist");
        let appended_block = &content[block_start..];

        // Codex should generate OPENAI_API_KEY
        assert!(
            appended_block.contains("OPENAI_API_KEY"),
            "Expected OPENAI_API_KEY in codex profile, got:\n{appended_block}"
        );
        // Codex should NOT generate ANTHROPIC_* vars
        assert!(
            !appended_block.contains("ANTHROPIC_"),
            "Codex profile should NOT contain ANTHROPIC_* vars, got:\n{appended_block}"
        );
        // Should contain backend = "codex"
        assert!(
            appended_block.contains("backend = \"codex\""),
            "Expected backend field in output, got:\n{appended_block}"
        );
        // Should contain full_auto = true
        assert!(
            appended_block.contains("full_auto = true"),
            "Expected full_auto field in output, got:\n{appended_block}"
        );
        // Should contain base_url as a profile-level field
        assert!(
            appended_block.contains("base_url = \"https://api.openai.com/v1\""),
            "Expected base_url as profile-level field, got:\n{appended_block}"
        );

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn update_profile_preserves_extra_args() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(
            &path,
            r#"[[profiles]]
name = "codex-profile"
backend = "codex"
model = "gpt-4.1"
base_url = "https://old.example/v1"
full_auto = false
extra_args = ["--sandbox", "workspace-write"]

[profiles.env]
OPENAI_API_KEY = "sk-old"
"#,
        )
        .unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let updated = NewProfile {
            name: "codex-profile".into(),
            description: None,
            base_url: Some("https://new.example/v1".into()),
            api_key: Some("sk-new".into()),
            model: Some("gpt-5.4".into()),
            fast_model: None,
            backend: Backend::Codex,
            full_auto: Some(true),
        };

        update_profile("codex-profile", &updated).unwrap();

        let profiles = load_profiles().unwrap();
        let profile = &profiles[0];
        assert_eq!(
            profile.extra_args.as_deref(),
            Some(&["--sandbox".to_string(), "workspace-write".to_string()][..])
        );
        assert_eq!(profile.base_url.as_deref(), Some("https://new.example/v1"));
        assert_eq!(profile.model.as_deref(), Some("gpt-5.4"));
        assert_eq!(profile.full_auto, Some(true));
        assert_eq!(
            profile
                .env
                .as_ref()
                .and_then(|env| env.get("OPENAI_API_KEY"))
                .map(String::as_str),
            Some("sk-new")
        );

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn update_profile_preserves_unknown_env_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(
            &path,
            r#"[[profiles]]
name = "claude-profile"
description = "Old description"
model = "old-model"
base_url = "https://old.example/v1"

[profiles.env]
ANTHROPIC_BASE_URL = "https://old.example/v1"
ANTHROPIC_API_KEY = "sk-old"
ANTHROPIC_MODEL = "old-model"
ANTHROPIC_DEFAULT_SONNET_MODEL = "old-model"
ANTHROPIC_DEFAULT_OPUS_MODEL = "old-model"
CLAUDE_CODE_SUBAGENT_MODEL = "old-model"
API_TIMEOUT_MS = "600000"
CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = "1"
CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK = "1"
CLAUDE_CODE_EFFORT_LEVEL = "max"
CUSTOM_HEADER = "keep-me"
"#,
        )
        .unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let updated = NewProfile {
            name: "claude-profile".into(),
            description: Some("New description".into()),
            base_url: Some("https://new.example/v1".into()),
            api_key: Some("sk-new".into()),
            model: Some("new-model".into()),
            fast_model: None,
            backend: Backend::Claude,
            full_auto: None,
        };

        update_profile("claude-profile", &updated).unwrap();

        let profiles = load_profiles().unwrap();
        let profile = &profiles[0];
        let env = profile.env.as_ref().unwrap();
        assert_eq!(profile.description.as_deref(), Some("New description"));
        assert_eq!(profile.base_url.as_deref(), Some("https://new.example/v1"));
        assert_eq!(profile.model.as_deref(), Some("new-model"));
        assert_eq!(
            env.get("CUSTOM_HEADER").map(String::as_str),
            Some("keep-me")
        );
        assert_eq!(
            env.get("ANTHROPIC_API_KEY").map(String::as_str),
            Some("sk-new")
        );
        assert_eq!(
            env.get("ANTHROPIC_MODEL").map(String::as_str),
            Some("new-model")
        );
        assert_eq!(
            env.get("CLAUDE_CODE_SUBAGENT_MODEL").map(String::as_str),
            Some("new-model")
        );
        assert_eq!(
            env.get("CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK")
                .map(String::as_str),
            Some("1")
        );
        assert_eq!(
            env.get("CLAUDE_CODE_EFFORT_LEVEL").map(String::as_str),
            Some("max")
        );
        // fast_model is None, so HAIKU_MODEL and SMALL_FAST_MODEL should be removed
        assert!(
            env.get("ANTHROPIC_DEFAULT_HAIKU_MODEL").is_none(),
            "ANTHROPIC_DEFAULT_HAIKU_MODEL should be removed when fast_model is None"
        );
        assert!(
            env.get("ANTHROPIC_SMALL_FAST_MODEL").is_none(),
            "ANTHROPIC_SMALL_FAST_MODEL should be removed when fast_model is None"
        );

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn update_profile_renames_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(
            &path,
            r#"[[profiles]]
name = "first"
description = "First profile"

[[profiles]]
name = "second"
description = "Second profile"
"#,
        )
        .unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let updated = NewProfile {
            name: "renamed".into(),
            description: Some("Updated profile".into()),
            base_url: None,
            api_key: None,
            model: None,
            fast_model: None,
            backend: Backend::Claude,
            full_auto: None,
        };

        update_profile("first", &updated).unwrap();

        let profiles = load_profiles().unwrap();
        assert_eq!(profiles[0].name, "renamed");
        assert_eq!(profiles[0].description.as_deref(), Some("Updated profile"));
        assert_eq!(profiles[1].name, "second");

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("name = \"renamed\""));
        assert!(!content.contains("name = \"first\""));

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    #[serial]
    fn update_profile_missing_original_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.toml");
        std::fs::write(&path, "[[profiles]]\nname = \"other\"\n").unwrap();
        std::env::set_var("CCT_CONFIG", &path);

        let updated = NewProfile {
            name: "renamed".into(),
            description: None,
            base_url: None,
            api_key: None,
            model: None,
            fast_model: None,
            backend: Backend::Claude,
            full_auto: None,
        };

        let result = update_profile("missing", &updated);
        assert!(result.is_err());

        std::env::remove_var("CCT_CONFIG");
    }
}
