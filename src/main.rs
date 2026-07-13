use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use cct::app::{App, AppMode, FormState};
use cct::config::Profile;
use cct::{cli, config, launch, proxy, ui};

#[derive(Parser)]
#[command(name = "cct", about = "Terminal UI launcher for Claude Code")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new profile interactively
    Add {
        /// Auth type: "api_key" (default) or "token" (uses ANTHROPIC_AUTH_TOKEN)
        #[arg(long)]
        auth_type: Option<String>,
    },
    /// Open profiles.toml in $EDITOR
    Edit,
    /// Launch a profile by name (interactive picker if no name given)
    Run {
        /// Profile name to launch (case-insensitive)
        name: Option<String>,
    },
    /// Run a command with a profile's environment variables, or list profiles when no args given
    Env {
        /// Profile name whose environment to load (omit to list all profiles)
        profile_name: Option<String>,
        /// Command and arguments (preceded by --)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Start the local proxy daemon (internal use)
    #[command(name = "proxy")]
    Proxy,
}

fn main() -> Result<()> {
    config::ensure_default_config()?;

    // Ignore errors — failing to set onboarding is non-fatal
    let _ = launch::ensure_claude_onboarding();

    if !launch::check_claude_installed() {
        launch::prompt_install()?;
    }

    let args = Cli::parse();
    match args.command {
        Some(Commands::Add { auth_type }) => cli::run_add(auth_type),
        Some(Commands::Edit) => launch::open_editor(&config::config_path()),
        Some(Commands::Run { name }) => run_profile(name),
        Some(Commands::Env {
            profile_name,
            command,
        }) => run_env(profile_name.as_deref(), &command),
        Some(Commands::Proxy) => run_proxy(),
        None => run_tui(),
    }
}

fn run_profile(name: Option<String>) -> Result<()> {
    let profile = match name {
        Some(n) => config::find_profile_by_name(&n)?
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found.", n))?,
        None => {
            let profiles = config::load_profiles()?;
            let idx = cli::run_pick_profile(&profiles)?;
            profiles.into_iter().nth(idx).unwrap()
        }
    };
    let err = match profile.backend {
        config::Backend::Claude => launch::exec_claude(&profile, false),
        config::Backend::Codex => launch::exec_codex(&profile),
    };
    eprintln!("Error: {err:#}");
    std::process::exit(1);
}

fn run_env(profile_name: Option<&str>, command: &[String]) -> Result<()> {
    let Some(pn) = profile_name else {
        let profiles = config::load_profiles()?;
        if profiles.is_empty() {
            println!("No profiles configured. Run 'cct add' to create one.");
            return Ok(());
        }
        for p in &profiles {
            let tag = match p.backend {
                config::Backend::Claude => "[claude]",
                config::Backend::Codex => "[codex] ",
            };
            let desc = p.description.as_deref().unwrap_or("");
            println!("{}  {}  {}", p.name, tag, desc);
        }
        return Ok(());
    };
    if command.is_empty() {
        anyhow::bail!("No command specified. Usage: cct env <profile> -- <command> [args...]");
    }
    let profile = config::find_profile_by_name(pn)?
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found.", pn))?;
    let shell_cmd = shell_words::join(command.iter().map(|s| s.as_str()));
    let err = launch::exec_with_env(&profile, &shell_cmd);
    eprintln!("Error: {err:#}");
    std::process::exit(1);
}

fn enter_add_mode(app: &mut App) {
    app.mode = AppMode::AddForm(Box::new(FormState::new_for_backend(
        app.active_backend.clone(),
    )));
}

fn enter_edit_mode(app: &mut App) {
    if app.profiles.is_empty() {
        return;
    }
    app.mode = AppMode::AddForm(Box::new(FormState::from_profile(
        &app.profiles[app.selected],
    )));
}

fn enter_duplicate_mode(app: &mut App) {
    if app.profiles.is_empty() {
        return;
    }
    let mut form = FormState::from_profile(&app.profiles[app.selected]);
    form.is_edit = false;
    form.original_name = None;
    form.fields[0].push_str("_copy");
    app.mode = AppMode::AddForm(Box::new(form));
}

fn validate_form_name(
    form: &FormState,
    profiles: &[Profile],
) -> std::result::Result<String, String> {
    let name = form.fields[0].trim().to_string();
    if name.is_empty() {
        return Err("Name is required.".into());
    }

    let unchanged_name = form
        .original_name
        .as_deref()
        .map(|original| original.eq_ignore_ascii_case(&name))
        .unwrap_or(false);

    let duplicate = profiles
        .iter()
        .any(|profile| profile.name.eq_ignore_ascii_case(&name));

    if duplicate && !(form.is_edit && unchanged_name) {
        return Err(format!("Profile '{}' already exists.", name));
    }

    Ok(name)
}

fn save_form(form: &FormState) -> std::result::Result<String, String> {
    let final_name = form.fields[0].trim().to_string();
    let new_profile = form.to_new_profile();

    let result = if form.is_edit {
        let original_name = form
            .original_name
            .as_deref()
            .ok_or_else(|| "Missing original profile name.".to_string())?;
        config::update_profile(original_name, &new_profile)
    } else {
        config::append_profile(&new_profile)
    };

    result
        .map(|_| final_name)
        .map_err(|e| format!("Save failed: {e:#}"))
}

fn reload_profiles_and_select(app: &mut App, selected_name: &str) -> Result<()> {
    let updated = config::load_profiles()?;
    let selected = updated
        .iter()
        .position(|profile| profile.name.eq_ignore_ascii_case(selected_name))
        .unwrap_or_else(|| updated.len().saturating_sub(1));
    app.selected = selected;
    app.profiles = updated;
    Ok(())
}

fn run_proxy() -> Result<()> {
    let port = proxy::proxy_port();
    proxy::run_foreground(port)?;
    Ok(())
}

fn run_tui() -> Result<()> {
    let profiles = config::load_profiles()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut tui = Terminal::new(CrosstermBackend::new(stdout))?;

    let mut app = App::new(profiles);

    loop {
        tui.draw(|f| ui::draw(&app, f))?;

        if let Event::Key(key) = event::read()? {
            let mut post_save_select: Option<String> = None;

            match &mut app.mode {
                AppMode::Normal => match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        launch::restore_terminal();
                        return Ok(());
                    }
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => app.next(),
                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => app.prev(),
                    (KeyCode::Tab, _) => {
                        let opposite = match app.active_backend {
                            config::Backend::Claude => config::Backend::Codex,
                            config::Backend::Codex => config::Backend::Claude,
                        };
                        app.switch_backend(opposite);
                    }
                    (KeyCode::Char('1'), _) => {
                        app.switch_backend(config::Backend::Claude);
                    }
                    (KeyCode::Char('2'), _) => {
                        app.switch_backend(config::Backend::Codex);
                    }
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
                    (KeyCode::Char('c'), _) if !app.profiles.is_empty() => {
                        let profile = &app.profiles[app.selected];
                        if profile.backend == config::Backend::Claude {
                            launch::restore_terminal();
                            let err = launch::exec_claude(profile, true);
                            eprintln!("Error: {err:#}");
                            std::process::exit(1);
                        }
                    }
                    (KeyCode::Char('e'), _) => {
                        enter_edit_mode(&mut app);
                    }
                    (KeyCode::Char('s'), _) if !app.profiles.is_empty() => {
                        let profile = &mut app.profiles[app.selected];
                        match profile.backend {
                            config::Backend::Claude => {
                                let old_val = profile.skip_permissions.unwrap_or(false);
                                let new_val = !old_val;
                                match config::toggle_skip_permissions(&profile.name, new_val) {
                                    Ok(()) => {
                                        profile.skip_permissions = Some(new_val);
                                    }
                                    Err(e) => {
                                        eprintln!("Warning: toggle failed: {e:#}");
                                    }
                                }
                            }
                            config::Backend::Codex => {
                                let next = config::ApprovalLevel::next(&profile.full_auto);
                                let next_str = next.as_ref().map(|a| a.as_str());
                                match config::toggle_full_auto(&profile.name, next_str) {
                                    Ok(()) => {
                                        profile.full_auto = next;
                                    }
                                    Err(e) => {
                                        eprintln!("Warning: toggle failed: {e:#}");
                                    }
                                }
                            }
                        }
                    }
                    (KeyCode::Char('t'), _) if !app.profiles.is_empty() => {
                        let profile = &app.profiles[app.selected];
                        if profile.backend == config::Backend::Claude {
                            match config::toggle_auth_type(&profile.name) {
                                Ok(()) => match config::load_profiles() {
                                    Ok(updated) => {
                                        if let Some(up) = updated
                                            .into_iter()
                                            .find(|p| p.name.eq_ignore_ascii_case(&profile.name))
                                        {
                                            app.profiles[app.selected] = up;
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "Warning: reload after auth toggle failed: {e:#}"
                                        );
                                    }
                                },
                                Err(e) => {
                                    eprintln!("Warning: auth toggle failed: {e:#}");
                                }
                            }
                        }
                    }
                    (KeyCode::Char('a'), _) => {
                        enter_add_mode(&mut app);
                    }
                    (KeyCode::Char('d'), _) => {
                        enter_duplicate_mode(&mut app);
                    }
                    _ => {}
                },
                AppMode::AddForm(form) => {
                    if form.confirming {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                let final_name = match validate_form_name(form, &app.profiles) {
                                    Ok(name) => name,
                                    Err(message) => {
                                        form.error = Some(message);
                                        form.confirming = false;
                                        continue;
                                    }
                                };

                                if let Err(message) = save_form(form) {
                                    form.error = Some(message);
                                    form.confirming = false;
                                    continue;
                                }

                                post_save_select = Some(final_name);
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                form.confirming = false;
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char(c) => {
                                form.error = None;
                                form.fields[form.active_field].push(c);
                            }
                            KeyCode::Backspace => {
                                form.error = None;
                                form.fields[form.active_field].pop();
                            }
                            KeyCode::Tab | KeyCode::Down => form.next_field(),
                            KeyCode::BackTab | KeyCode::Up => form.prev_field(),
                            KeyCode::Enter => {
                                if form.active_field < 5 {
                                    form.next_field();
                                } else {
                                    form.confirming = true;
                                }
                            }
                            KeyCode::Esc => {
                                app.mode = AppMode::Normal;
                            }
                            _ => {}
                        }
                    }
                }
            }

            if let Some(final_name) = post_save_select {
                match reload_profiles_and_select(&mut app, &final_name) {
                    Ok(()) => {
                        app.mode = AppMode::Normal;
                    }
                    Err(e) => {
                        if let AppMode::AddForm(form) = &mut app.mode {
                            form.error = Some(format!("Reload failed: {e:#}"));
                            form.confirming = false;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use serial_test::serial;

    #[test]
    fn clap_routing_no_subcommand() {
        // No args → command should be None (TUI mode)
        let cli = Cli::try_parse_from(["cct"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn clap_routing_add_subcommand() {
        // "cct add" → command should be Some(Commands::Add)
        let cli = Cli::try_parse_from(["cct", "add"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Add { .. })));
    }

    #[test]
    fn clap_routing_add_with_auth_type() {
        let cli = Cli::try_parse_from(["cct", "add", "--auth-type", "token"]).unwrap();
        match cli.command {
            Some(Commands::Add { auth_type }) => assert_eq!(auth_type.as_deref(), Some("token")),
            _ => panic!("expected Add command with auth_type"),
        }
    }

    #[test]
    fn clap_routing_edit_subcommand() {
        // "cct edit" → command should be Some(Commands::Edit)
        let cli = Cli::try_parse_from(["cct", "edit"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Edit)));
    }

    #[test]
    fn clap_routing_run_with_name() {
        let cli = Cli::try_parse_from(["cct", "run", "my-profile"]).unwrap();
        match cli.command {
            Some(Commands::Run { name }) => assert_eq!(name, Some("my-profile".into())),
            _ => panic!("expected Run command"),
        }
    }

    #[test]
    fn clap_routing_run_without_name() {
        let cli = Cli::try_parse_from(["cct", "run"]).unwrap();
        match cli.command {
            Some(Commands::Run { name }) => assert!(name.is_none()),
            _ => panic!("expected Run command"),
        }
    }

    #[test]
    fn clap_routing_env_with_command() {
        let cli =
            Cli::try_parse_from(["cct", "env", "my-profile", "--", "python", "-c", "print(1)"])
                .unwrap();
        match cli.command {
            Some(Commands::Env {
                profile_name,
                command,
            }) => {
                assert_eq!(profile_name.as_deref(), Some("my-profile"));
                assert_eq!(command, vec!["python", "-c", "print(1)"]);
            }
            _ => panic!("expected Env command"),
        }
    }

    #[test]
    fn clap_routing_env_without_double_dash() {
        let cli = Cli::try_parse_from(["cct", "env", "my-profile", "python", "script.py"]).unwrap();
        match cli.command {
            Some(Commands::Env {
                profile_name,
                command,
            }) => {
                assert_eq!(profile_name.as_deref(), Some("my-profile"));
                assert_eq!(command, vec!["python", "script.py"]);
            }
            _ => panic!("expected Env command"),
        }
    }

    #[test]
    fn clap_routing_env_empty_command() {
        let cli = Cli::try_parse_from(["cct", "env", "my-profile"]).unwrap();
        match cli.command {
            Some(Commands::Env {
                profile_name,
                command,
            }) => {
                assert_eq!(profile_name.as_deref(), Some("my-profile"));
                assert!(command.is_empty());
            }
            _ => panic!("expected Env command"),
        }
    }

    #[test]
    fn clap_routing_env_no_args_lists_profiles() {
        let cli = Cli::try_parse_from(["cct", "env"]).unwrap();
        match cli.command {
            Some(Commands::Env {
                profile_name,
                command,
            }) => {
                assert!(profile_name.is_none());
                assert!(command.is_empty());
            }
            _ => panic!("expected Env command"),
        }
    }

    #[test]
    fn e_key_enters_prefilled_edit_form() {
        let profile = Profile {
            name: "edit-me".into(),
            description: Some("Editable".into()),
            env: Some(std::collections::HashMap::from([(
                "ANTHROPIC_API_KEY".into(),
                "sk-ant".into(),
            )])),
            extra_args: None,
            skip_permissions: None,
            model: Some("claude-sonnet-4-6".into()),
            backend: config::Backend::Claude,
            base_url: Some("https://example.com/v1".into()),
            full_auto: None,
            auth_type: None,
        };
        let mut app = App::new(vec![profile]);

        enter_edit_mode(&mut app);

        match &app.mode {
            AppMode::AddForm(form) => {
                assert!(form.is_edit);
                assert_eq!(form.original_name.as_deref(), Some("edit-me"));
                assert_eq!(form.fields[0], "edit-me");
                assert_eq!(form.fields[3], "sk-ant");
            }
            AppMode::Normal => panic!("expected add form"),
        }
    }

    #[test]
    fn d_key_enters_duplicate_form_with_copy_suffix() {
        let profile = Profile {
            name: "my-profile".into(),
            description: Some("Original".into()),
            env: Some(std::collections::HashMap::from([(
                "ANTHROPIC_API_KEY".into(),
                "sk-ant".into(),
            )])),
            extra_args: None,
            skip_permissions: None,
            model: Some("claude-sonnet-4-6".into()),
            backend: config::Backend::Claude,
            base_url: Some("https://example.com/v1".into()),
            full_auto: None,
            auth_type: None,
        };
        let mut app = App::new(vec![profile]);

        enter_duplicate_mode(&mut app);

        match &app.mode {
            AppMode::AddForm(form) => {
                assert!(!form.is_edit, "duplicate should not be edit mode");
                assert!(
                    form.original_name.is_none(),
                    "duplicate should have no original_name"
                );
                assert_eq!(form.fields[0], "my-profile_copy");
                assert_eq!(form.fields[1], "Original");
                assert_eq!(form.fields[3], "sk-ant");
            }
            AppMode::Normal => panic!("expected add form"),
        }
    }

    #[test]
    fn duplicate_always_appends_copy_suffix() {
        let profile = Profile {
            name: "existing_copy".into(),
            description: None,
            env: None,
            extra_args: None,
            skip_permissions: None,
            model: None,
            backend: config::Backend::Claude,
            base_url: None,
            full_auto: None,
            auth_type: None,
        };
        let mut app = App::new(vec![profile]);

        enter_duplicate_mode(&mut app);

        match &app.mode {
            AppMode::AddForm(form) => {
                assert_eq!(form.fields[0], "existing_copy_copy");
            }
            AppMode::Normal => panic!("expected add form"),
        }
    }

    #[test]
    fn edit_mode_validates_duplicate_rename_and_keeps_unchanged_name() {
        let profiles = vec![
            Profile {
                name: "alpha".into(),
                description: None,
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
                backend: config::Backend::Claude,
                base_url: None,
                full_auto: None,
                auth_type: None,
            },
        ];

        let mut unchanged = FormState::new();
        unchanged.is_edit = true;
        unchanged.original_name = Some("alpha".into());
        unchanged.fields[0] = "alpha".into();
        assert_eq!(validate_form_name(&unchanged, &profiles).unwrap(), "alpha");

        let mut duplicate = FormState::new();
        duplicate.is_edit = true;
        duplicate.original_name = Some("alpha".into());
        duplicate.fields[0] = "beta".into();
        assert_eq!(
            validate_form_name(&duplicate, &profiles).unwrap_err(),
            "Profile 'beta' already exists."
        );
    }

    #[test]
    #[serial]
    fn edit_mode_save_reloads_and_reselects_updated_profile() {
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

        let profiles = config::load_profiles().unwrap();
        let mut app = App::new(profiles);
        app.selected = 0;
        enter_edit_mode(&mut app);

        let profiles_snapshot = app.profiles.clone();
        let final_name = if let AppMode::AddForm(form) = &mut app.mode {
            form.fields[0] = "renamed".into();
            form.fields[1] = "Updated description".into();
            let final_name = validate_form_name(form, &profiles_snapshot).unwrap();
            save_form(form).unwrap();
            final_name
        } else {
            panic!("expected add form");
        };

        reload_profiles_and_select(&mut app, &final_name).unwrap();

        assert_eq!(app.selected, 0);
        assert_eq!(app.profiles[0].name, "renamed");
        assert_eq!(
            app.profiles[0].description.as_deref(),
            Some("Updated description")
        );

        std::env::remove_var("CCT_CONFIG");
    }

    #[test]
    fn readme_documents_inline_edit_keybinding() {
        let readme = include_str!("../README.md");
        assert!(readme.contains("press `e` to edit it inline"));
        assert!(readme.contains("| `e` | Edit the selected profile inline |"));
        assert!(!readme.contains(concat!("Edit ", "config in `$EDITOR`")));
        assert!(!readme.contains("Hot-reload"));
    }
}
