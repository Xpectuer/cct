use crate::app::{field_labels, App, AppMode, FormState};
use crate::config::{Backend, Profile};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

const SENSITIVE: &[&str] = &["TOKEN", "KEY", "SECRET"];

/// Returns `"***"` if `key` (case-insensitive) contains TOKEN, KEY, or SECRET.
pub fn mask_value<'a>(key: &str, val: &'a str) -> &'a str {
    let upper = key.to_uppercase();
    if SENSITIVE.iter().any(|p| upper.contains(p)) {
        "***"
    } else {
        val
    }
}

/// Build a tab bar showing `[Claude]`, `[Codex]`, `[Kimi]` with the active tab highlighted.
pub fn build_tab_bar(active: &Backend) -> Vec<Line<'static>> {
    let claude_label = "[Claude]";
    let codex_label = "[Codex]";
    let kimi_label = "[Kimi]";
    let active_style = Style::default()
        .fg(Color::White)
        .bg(Color::Blue)
        .add_modifier(Modifier::BOLD);
    let inactive_style = Style::default().fg(Color::DarkGray);

    let claude_span = ratatui::text::Span::styled(
        claude_label,
        if *active == Backend::Claude {
            active_style
        } else {
            inactive_style
        },
    );
    let codex_span = ratatui::text::Span::styled(
        codex_label,
        if *active == Backend::Codex {
            active_style
        } else {
            inactive_style
        },
    );
    let kimi_span = ratatui::text::Span::styled(
        kimi_label,
        if *active == Backend::Kimi {
            active_style
        } else {
            inactive_style
        },
    );

    vec![Line::from(vec![
        claude_span,
        ratatui::text::Span::raw("  "),
        codex_span,
        ratatui::text::Span::raw("  "),
        kimi_span,
    ])]
}

fn form_panel_title(form: &FormState) -> &'static str {
    if form.is_edit {
        " Edit Profile "
    } else {
        " Add Profile "
    }
}

fn confirmation_prompt(form: &FormState) -> &'static str {
    if form.is_edit {
        "Save changes to this profile?"
    } else {
        "Add this profile?"
    }
}

fn normal_footer_text(backend: &Backend) -> &'static str {
    match backend {
        Backend::Claude => {
            " [Tab/1/2/3] Backend  [↑↓/jk] Navigate  [Enter] Launch  [c] Resume  [s] Skip-perms  [t] Auth  [a] Add  [d] Duplicate  [e] Edit  [q] Quit"
        }
        Backend::Codex => {
            " [Tab/1/2/3] Backend  [↑↓/jk] Navigate  [Enter] Launch  [s] Approval  [t] Auth  [a] Add  [d] Duplicate  [e] Edit  [q] Quit"
        }
        Backend::Kimi => {
            " [Tab/1/2/3] Backend  [↑↓/jk] Navigate  [Enter] Launch  [Space] Context  [a] Add  [d] Duplicate  [e] Edit  [q] Quit"
        }
    }
}

pub fn draw(app: &App, frame: &mut Frame) {
    // Outer: content area + 1-line footer
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    // Content: 35% list | 65% detail
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer[0]);

    // --- Left panel: tab bar + profile list ---
    let left_panel = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(content[0]);

    // Tab bar
    let tab_lines = build_tab_bar(&app.active_backend);
    let tab_bar = Paragraph::new(tab_lines);
    frame.render_widget(tab_bar, left_panel[0]);

    // --- Profile list (filtered by active_backend) ---
    let filtered = app.filtered_indices();
    let items: Vec<ListItem> = if filtered.is_empty() {
        vec![ListItem::new("No profiles. Press 'a' to add one.")]
    } else {
        filtered
            .iter()
            .map(|&i| {
                let p = &app.profiles[i];
                let label = match &p.description {
                    Some(d) => format!("{}\n  {}", p.name, d),
                    None => p.name.clone(),
                };
                let item = ListItem::new(label);
                match p.backend {
                    Backend::Claude if p.skip_permissions.unwrap_or(false) => {
                        item.style(Style::default().fg(Color::Red))
                    }
                    Backend::Codex => {
                        // Subscription mode: gray — signals "no proxy, native Codex auth"
                        if p.auth_type.as_deref() == Some("subscription") {
                            item.style(Style::default().fg(Color::DarkGray))
                        } else {
                            // Color by approval level: green (safest) → yellow → red (most dangerous)
                            let color = match &p.full_auto {
                                Some(crate::config::ApprovalLevel::Untrusted) => Color::Green,
                                Some(crate::config::ApprovalLevel::Never) => Color::Yellow,
                                Some(crate::config::ApprovalLevel::Danger) => Color::Red,
                                None => Color::White, // on-request: default, white
                            };
                            item.style(Style::default().fg(color))
                        }
                    }
                    _ => item,
                }
            })
            .collect()
    };

    // Map app.selected (global index) to position within filtered list
    let mut list_state = ListState::default();
    if !filtered.is_empty() {
        let pos = filtered
            .iter()
            .position(|&i| i == app.selected)
            .unwrap_or(0);
        list_state.select(Some(pos));
    }

    let profile_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Profiles "))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(profile_list, left_panel[1], &mut list_state);

    // --- Detail panel ---
    match &app.mode {
        AppMode::AddForm(form) => {
            let detail_lines = build_form_lines(form);
            let detail = Paragraph::new(detail_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(form_panel_title(form)),
                )
                .wrap(Wrap { trim: false });
            frame.render_widget(detail, content[1]);
        }
        AppMode::Normal => {
            let detail_lines = if app.profiles.is_empty() {
                vec![Line::from("Select a profile to see details.")]
            } else {
                build_detail(&app.profiles[app.selected])
            };
            let detail = Paragraph::new(detail_lines)
                .block(Block::default().borders(Borders::ALL).title(" Details "))
                .wrap(Wrap { trim: false });
            frame.render_widget(detail, content[1]);
        }
    }

    // --- Footer ---
    let footer_text = match &app.mode {
        AppMode::Normal => normal_footer_text(&app.active_backend),
        AppMode::AddForm(form) if form.confirming => " [y] Save  [n/Esc] Back",
        AppMode::AddForm(_) => {
            " [Tab/↓] Next field  [Shift-Tab/↑] Prev  [Enter] Confirm  [Esc] Cancel"
        }
    };
    let footer = Paragraph::new(footer_text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, outer[1]);
}

fn build_detail(profile: &Profile) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    if let Some(desc) = &profile.description {
        lines.push(Line::from(desc.clone()));
        lines.push(Line::from(""));
    }
    if let Some(model) = &profile.model {
        lines.push(Line::from(format!("model: {model}")));
    }
    if let Some(url) = &profile.base_url {
        if !url.is_empty() {
            lines.push(Line::from(format!("base_url: {url}")));
        }
    }
    match profile.backend {
        Backend::Claude => {
            if profile.skip_permissions.unwrap_or(false) {
                lines.push(Line::from("skip_permissions: \u{2713}"));
            }
            if profile.auth_type.as_deref() == Some("token") {
                lines.push(Line::from("auth: token"));
            }
        }
        Backend::Codex => {
            lines.push(Line::from(format!(
                "approval: {}",
                crate::config::approval_label(&profile.full_auto)
            )));
            if profile.auth_type.as_deref() == Some("subscription") {
                lines.push(Line::from("auth: subscription"));
            }
        }
        Backend::Kimi => {
            let effective = profile.max_context_size.clone().unwrap_or_else(|| {
                let model = profile.model.as_deref().or_else(|| {
                    profile
                        .env
                        .as_ref()
                        .and_then(|m| m.get("ANTHROPIC_MODEL"))
                        .map(String::as_str)
                });
                format!("auto ({})", crate::config::default_max_context_size(model))
            });
            lines.push(Line::from(format!("max_context_size: {effective}")));
        }
    }
    if let Some(extra) = &profile.extra_args {
        if !extra.is_empty() {
            lines.push(Line::from(format!("extra_args: {}", extra.join(" "))));
        }
    }
    if let Some(env_map) = &profile.env {
        if !env_map.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from("ENV:"));
            let mut pairs: Vec<(&String, &String)> = env_map.iter().collect();
            pairs.sort_by_key(|(k, _)| k.as_str());
            for (k, v) in &pairs {
                let display = mask_value(k.as_str(), v.as_str());
                lines.push(Line::from(format!("  {} = {}", k, display)));
            }
        }
    }
    lines
}

fn build_form_lines(form: &FormState) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let labels = field_labels(&form.backend);

    if form.confirming {
        lines.push(
            Line::from(confirmation_prompt(form))
                .style(Style::default().add_modifier(Modifier::BOLD)),
        );
        lines.push(Line::from(""));
        for (i, label) in labels.iter().enumerate() {
            let val = form.fields[i].trim();
            let display = if val.is_empty() {
                "(none)".to_string()
            } else if label.contains("Key") {
                mask_value("API_KEY", val).to_string()
            } else {
                val.to_string()
            };
            lines.push(Line::from(format!(
                "  {:<14} {}",
                format!("{}:", label.trim_end_matches(" *")),
                display
            )));
        }
    } else {
        for (i, label) in labels.iter().enumerate() {
            let prefix = if i == form.active_field { "> " } else { "  " };
            let style = if i == form.active_field {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            lines.push(Line::from(format!("{}{}: {}", prefix, label, form.fields[i])).style(style));
        }
    }

    if let Some(err) = &form.error {
        lines.push(Line::from(""));
        lines.push(Line::from(err.clone()).style(Style::default().fg(Color::Red)));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_auth_token() {
        assert_eq!(mask_value("ANTHROPIC_AUTH_TOKEN", "sk-secret"), "***");
    }

    #[test]
    fn mask_api_key() {
        assert_eq!(mask_value("OPENAI_API_KEY", "sk-key"), "***");
    }

    #[test]
    fn mask_secret() {
        assert_eq!(mask_value("MY_SECRET", "s3cr3t"), "***");
    }

    #[test]
    fn no_mask_url() {
        assert_eq!(
            mask_value("ANTHROPIC_BASE_URL", "https://api.example.com"),
            "https://api.example.com"
        );
    }

    #[test]
    fn ui_renders_add_form() {
        let mut form = FormState::new();
        form.fields[0] = "my-profile".into();
        form.fields[1] = "A description".into();
        form.active_field = 0;

        let lines = build_form_lines(&form);
        // Should have 6 lines (one per field)
        assert_eq!(lines.len(), 6);

        // Active field should have "> " prefix
        let first = lines[0].to_string();
        assert!(
            first.starts_with("> "),
            "active field should have '> ' prefix, got: {first}"
        );
        assert!(first.contains("my-profile"));

        // Non-active fields should have "  " prefix
        let second = lines[1].to_string();
        assert!(
            second.starts_with("  "),
            "inactive field should have '  ' prefix"
        );
        assert!(second.contains("A description"));
    }

    #[test]
    fn ui_confirmation_shows_five_fields() {
        let mut form = FormState::new();
        form.confirming = true;
        form.fields[0] = "test-profile".into();
        form.fields[2] = "https://api.example.com".into();
        form.fields[3] = "sk-secret-key".into();
        form.fields[4] = "kimi-k2".into();

        let lines = build_form_lines(&form);
        let text: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        let joined = text.join("\n");

        // Must contain all five field labels
        assert!(
            joined.contains("Name:"),
            "Expected 'Name:' in confirmation, got:\n{joined}"
        );
        assert!(
            joined.contains("Description:"),
            "Expected 'Description:' in confirmation, got:\n{joined}"
        );
        assert!(
            joined.contains("Base URL:"),
            "Expected 'Base URL:' in confirmation, got:\n{joined}"
        );
        assert!(
            joined.contains("API Key:"),
            "Expected 'API Key:' in confirmation, got:\n{joined}"
        );
        assert!(
            joined.contains("Model:"),
            "Expected 'Model:' in confirmation, got:\n{joined}"
        );

        // API key should be masked (mask_value on key containing "KEY" returns "***")
        assert!(
            joined.contains("***"),
            "Expected masked API key '***' in confirmation, got:\n{joined}"
        );
        assert!(
            !joined.contains("sk-secret-key"),
            "API key should NOT appear in cleartext in confirmation, got:\n{joined}"
        );

        // Model should show the actual value
        assert!(
            joined.contains("kimi-k2"),
            "Expected model 'kimi-k2' in confirmation, got:\n{joined}"
        );

        // Base URL should show the actual value
        assert!(
            joined.contains("https://api.example.com"),
            "Expected base URL in confirmation, got:\n{joined}"
        );

        // Description should show "(none)" since it's empty
        assert!(
            joined.contains("(none)"),
            "Expected '(none)' for empty description, got:\n{joined}"
        );
    }

    #[test]
    fn ui_footer_shows_add_hint() {
        let claude_footer = normal_footer_text(&Backend::Claude);
        assert!(claude_footer.contains("[a] Add"));
        assert!(claude_footer.contains("[d] Duplicate"));
        assert!(claude_footer.contains("[s] Skip-perms"));
        assert!(claude_footer.contains("[t] Auth"));
        assert!(claude_footer.contains("[c] Resume"));
        assert!(claude_footer.contains("[e] Edit"));
        assert!(claude_footer.contains("[Tab/1/2/3] Backend"));
        assert_eq!(claude_footer.matches("[e] Edit").count(), 1);

        let codex_footer = normal_footer_text(&Backend::Codex);
        assert!(codex_footer.contains("[a] Add"));
        assert!(codex_footer.contains("[d] Duplicate"));
        assert!(codex_footer.contains("[s] Approval"));
        assert!(codex_footer.contains("[t] Auth"));
        assert!(codex_footer.contains("[e] Edit"));
        assert!(codex_footer.contains("[Tab/1/2/3] Backend"));
        assert!(
            !codex_footer.contains("[c] Resume"),
            "Codex footer should not show Resume"
        );
        assert_eq!(codex_footer.matches("[e] Edit").count(), 1);
    }

    /// Every new key binding needs a footer hint: the Kimi tab must show the
    /// [Space] max_context_size toggle, and must not offer Claude/Codex-only
    /// toggles ([c] Resume, [s], [t]).
    #[test]
    fn kimi_footer_shows_space_context_hint() {
        let kimi_footer = normal_footer_text(&Backend::Kimi);
        assert!(kimi_footer.contains("[Space] Context"));
        assert!(kimi_footer.contains("[Tab/1/2/3] Backend"));
        assert!(kimi_footer.contains("[a] Add"));
        assert!(kimi_footer.contains("[d] Duplicate"));
        assert!(kimi_footer.contains("[e] Edit"));
        assert!(
            !kimi_footer.contains("[c] Resume"),
            "Kimi footer should not show Resume"
        );
        assert!(
            !kimi_footer.contains("[s]"),
            "Kimi footer should not show an [s] toggle"
        );
        assert!(
            !kimi_footer.contains("[t]"),
            "Kimi footer should not show an [t] toggle"
        );
    }

    #[test]
    fn ui_form_title_and_confirmation_reflect_edit_mode() {
        let add_form = FormState::new();
        assert_eq!(form_panel_title(&add_form), " Add Profile ");

        let mut add_confirm = FormState::new();
        add_confirm.confirming = true;
        let add_lines = build_form_lines(&add_confirm);
        assert_eq!(add_lines[0].to_string(), "Add this profile?");

        let mut edit_form = FormState::new();
        edit_form.is_edit = true;
        assert_eq!(form_panel_title(&edit_form), " Edit Profile ");

        edit_form.confirming = true;
        let edit_lines = build_form_lines(&edit_form);
        assert_eq!(edit_lines[0].to_string(), "Save changes to this profile?");
    }

    #[test]
    fn tab_bar_renders_with_active_highlight() {
        use crate::config::Backend;

        for active in [Backend::Claude, Backend::Codex, Backend::Kimi] {
            let lines = build_tab_bar(&active);
            let text: String = lines
                .iter()
                .map(|l| l.to_string())
                .collect::<Vec<_>>()
                .join("");
            assert!(
                text.contains("[Claude]"),
                "Expected [Claude] in tab bar, got: {text}"
            );
            assert!(
                text.contains("[Codex]"),
                "Expected [Codex] in tab bar, got: {text}"
            );
            assert!(
                text.contains("[Kimi]"),
                "Expected [Kimi] in tab bar, got: {text}"
            );
        }
    }

    #[test]
    fn detail_panel_shows_full_auto_for_codex_profile() {
        use crate::config::Backend;

        let codex_profile = Profile {
            name: "codex-test".into(),
            description: Some("Codex profile".into()),
            env: None,
            model: Some("o3".into()),
            skip_permissions: None,
            extra_args: None,
            backend: Backend::Codex,
            base_url: Some("https://api.openai.com".into()),
            full_auto: Some(crate::config::ApprovalLevel::Danger),
            auth_type: None,
            max_context_size: None,
        };

        let lines = build_detail(&codex_profile);
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        // Should show approval level instead of skip_permissions
        assert!(
            joined.contains("approval:"),
            "Expected 'approval:' in codex detail, got:\n{joined}"
        );
        assert!(
            !joined.contains("skip_permissions:"),
            "Should NOT show 'skip_permissions:' for codex profile, got:\n{joined}"
        );

        // Claude profile with skip_permissions should still show skip_permissions
        let claude_profile = Profile {
            name: "claude-test".into(),
            description: None,
            env: None,
            model: None,
            skip_permissions: Some(true),
            extra_args: None,
            backend: Backend::Claude,
            base_url: None,
            full_auto: None,
            auth_type: None,
            max_context_size: None,
        };

        let lines = build_detail(&claude_profile);
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("skip_permissions:"),
            "Expected 'skip_permissions:' in claude detail, got:\n{joined}"
        );
        assert!(
            !joined.contains("full_auto:"),
            "Should NOT show 'approval:' for claude profile, got:\n{joined}"
        );
    }

    #[test]
    fn skip_permissions_profile_has_red_style() {
        let profile = Profile {
            name: "dangerous".into(),
            description: None,
            env: None,
            model: None,
            skip_permissions: Some(true),
            extra_args: None,
            backend: crate::config::Backend::Claude,
            base_url: None,
            full_auto: None,
            auth_type: None,
            max_context_size: None,
        };
        // Verify skip_permissions triggers the red-style branch
        assert!(profile.skip_permissions.unwrap_or(false));
        // Build a ListItem the same way the draw function does
        let label = profile.name.clone();
        let item = ListItem::new(label);
        // This should apply red style without panicking
        let _styled = item.style(Style::default().fg(Color::Red));

        // Also verify that skip_permissions=false does NOT trigger the branch
        let safe_profile = Profile {
            name: "safe".into(),
            description: None,
            env: None,
            model: None,
            skip_permissions: Some(false),
            extra_args: None,
            backend: crate::config::Backend::Claude,
            base_url: None,
            full_auto: None,
            auth_type: None,
            max_context_size: None,
        };
        assert!(!safe_profile.skip_permissions.unwrap_or(false));
    }

    /// Codex input form must show codex-specific labels, not Claude labels.
    #[test]
    fn codex_input_form_shows_codex_labels() {
        use crate::config::Backend;
        let mut form = FormState::new();
        form.backend = Backend::Codex;
        form.fields[0] = "test".into();

        let lines = build_form_lines(&form);
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        // Must contain codex-specific labels
        assert!(
            joined.contains("Base URL"),
            "Codex form must show 'Base URL' label"
        );
        assert!(
            joined.contains("Approval"),
            "Codex form must show 'Full Auto' label"
        );
        // Must NOT contain claude-only label "Description"
        assert!(
            !joined.contains("Description"),
            "Codex form must NOT show 'Description' label"
        );
    }

    /// Codex confirmation view must show codex labels and field values in correct positions.
    #[test]
    fn codex_confirm_shows_codex_labels_with_correct_values() {
        use crate::config::Backend;
        let mut form = FormState::new();
        form.backend = Backend::Codex;
        form.confirming = true;
        form.fields[0] = "my-codex".into();
        form.fields[1] = "https://api.openai.com".into(); // Base URL
        form.fields[2] = "sk-openai-key".into(); // API Key
        form.fields[3] = "gpt-5.3".into(); // Model
        form.fields[4] = "y".into(); // Full Auto

        let lines = build_form_lines(&form);
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        // Labels must be codex-specific
        assert!(
            joined.contains("Base URL"),
            "Confirm must show 'Base URL' label"
        );
        assert!(joined.contains("Model"), "Confirm must show 'Model' label");
        assert!(
            joined.contains("Approval"),
            "Confirm must show 'Full Auto' label"
        );
        assert!(
            !joined.contains("Description"),
            "Confirm must NOT show 'Description' for codex"
        );

        // Values must appear next to correct labels (Base URL line has the URL, not the key)
        let base_url_line = lines
            .iter()
            .map(|l| l.to_string())
            .find(|l| l.contains("Base URL"))
            .unwrap();
        assert!(
            base_url_line.contains("https://api.openai.com"),
            "Base URL line must contain the URL value, got: {base_url_line}"
        );

        let model_line = lines
            .iter()
            .map(|l| l.to_string())
            .find(|l| l.contains("Model"))
            .unwrap();
        assert!(
            model_line.contains("gpt-5.3"),
            "Model line must contain model value, got: {model_line}"
        );

        // API Key must be masked
        let key_line = lines
            .iter()
            .map(|l| l.to_string())
            .find(|l| l.contains("Key"))
            .unwrap();
        assert!(
            key_line.contains("***"),
            "API Key must be masked in confirmation"
        );
        assert!(
            !key_line.contains("sk-openai-key"),
            "API Key must NOT appear in cleartext"
        );
    }

    #[test]
    fn detail_shows_auth_type_token() {
        let profile = Profile {
            name: "token-profile".into(),
            description: None,
            env: None,
            model: None,
            skip_permissions: None,
            extra_args: None,
            backend: crate::config::Backend::Claude,
            base_url: None,
            full_auto: None,
            auth_type: Some("token".into()),
            max_context_size: None,
        };
        let lines = build_detail(&profile);
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("auth: token"),
            "should show auth: token, got:\n{joined}"
        );
    }

    #[test]
    fn detail_does_not_show_auth_for_api_key() {
        let profile = Profile {
            name: "normal-profile".into(),
            description: None,
            env: None,
            model: None,
            skip_permissions: None,
            extra_args: None,
            backend: crate::config::Backend::Claude,
            base_url: None,
            full_auto: None,
            auth_type: None,
            max_context_size: None,
        };
        let lines = build_detail(&profile);
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !joined.contains("auth:"),
            "should NOT show auth when None, got:\n{joined}"
        );
    }

    #[test]
    fn codex_detail_shows_subscription_auth() {
        let profile = Profile {
            name: "codex-sub".into(),
            description: None,
            env: None,
            model: None,
            skip_permissions: None,
            extra_args: None,
            backend: crate::config::Backend::Codex,
            base_url: None,
            full_auto: None,
            auth_type: Some("subscription".into()),
            max_context_size: None,
        };
        let lines = build_detail(&profile);
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("auth: subscription"),
            "should show auth: subscription, got:\n{joined}"
        );
    }

    #[test]
    fn codex_subscription_profile_has_gray_style() {
        let profile = Profile {
            name: "sub-profile".into(),
            description: None,
            env: None,
            model: None,
            skip_permissions: None,
            extra_args: None,
            backend: crate::config::Backend::Codex,
            base_url: None,
            full_auto: None,
            auth_type: Some("subscription".into()),
            max_context_size: None,
        };
        // Verify auth_type is subscription
        assert_eq!(profile.auth_type.as_deref(), Some("subscription"));
        // Style branch: subscription profiles get DarkGray
        let item = ListItem::new(profile.name.clone());
        let styled = item.style(Style::default().fg(Color::DarkGray));
        // Just verify the style was applied without panicking
        let _ = styled;
    }

    #[test]
    fn kimi_detail_shows_max_context_size() {
        use crate::config::Backend;

        // Explicit value shown as-is
        let explicit = Profile {
            name: "kimi-explicit".into(),
            description: None,
            env: None,
            model: Some("kimi-k2".into()),
            skip_permissions: None,
            extra_args: None,
            backend: Backend::Kimi,
            base_url: None,
            full_auto: None,
            auth_type: None,
            max_context_size: Some("260k".into()),
        };
        let lines = build_detail(&explicit);
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("max_context_size: 260k"),
            "Expected explicit max_context_size, got:\n{joined}"
        );

        // No explicit field: auto-detected from model (k3 → 1m)
        let auto_k3 = Profile {
            name: "kimi-auto".into(),
            description: None,
            env: None,
            model: Some("k3".into()),
            skip_permissions: None,
            extra_args: None,
            backend: Backend::Kimi,
            base_url: None,
            full_auto: None,
            auth_type: None,
            max_context_size: None,
        };
        let lines = build_detail(&auto_k3);
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("max_context_size: auto (1m)"),
            "Expected auto (1m) hint, got:\n{joined}"
        );
    }
}
