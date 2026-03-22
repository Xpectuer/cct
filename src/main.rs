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
use cct::{cli, config, launch, ui};

#[derive(Parser)]
#[command(name = "cct", about = "Terminal UI launcher for Claude Code")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new profile interactively
    Add,
}

fn main() -> Result<()> {
    config::ensure_default_config()?;

    if !launch::check_claude_installed() {
        launch::prompt_install()?;
    }

    let args = Cli::parse();
    match args.command {
        Some(Commands::Add) => cli::run_add(),
        None => run_tui(),
    }
}

fn enter_add_mode(app: &mut App) {
    app.mode = AppMode::AddForm(FormState::new_for_backend(app.active_backend.clone()));
}

fn enter_edit_mode(app: &mut App) {
    if app.profiles.is_empty() {
        return;
    }
    app.mode = AppMode::AddForm(FormState::from_profile(&app.profiles[app.selected]));
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
                                let old_val = profile.full_auto.unwrap_or(false);
                                let new_val = !old_val;
                                match config::toggle_full_auto(&profile.name, new_val) {
                                    Ok(()) => {
                                        profile.full_auto = Some(new_val);
                                    }
                                    Err(e) => {
                                        eprintln!("Warning: toggle failed: {e:#}");
                                    }
                                }
                            }
                        }
                    }
                    (KeyCode::Char('a'), _) => {
                        enter_add_mode(&mut app);
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
                                if form.active_field < 4 {
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
        assert!(matches!(cli.command, Some(Commands::Add)));
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
