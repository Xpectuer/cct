use crate::config::{Backend, NewProfile, Profile};

pub fn field_labels(backend: &Backend) -> [&'static str; 6] {
    match backend {
        Backend::Claude => [
            "Name *",
            "Description",
            "Base URL",
            "API Key",
            "Pro Model",
            "Fast Model",
        ],
        Backend::Codex => ["Name *", "Base URL", "API Key", "Model", "Approval", ""],
        Backend::Kimi => [
            "Name *",
            "Description",
            "Base URL",
            "API Key",
            "Model",
            "Context (1m/260k)",
        ],
    }
}

pub enum AppMode {
    Normal,
    AddForm(Box<FormState>),
}

pub struct FormState {
    pub fields: [String; 6],
    pub active_field: usize,
    pub confirming: bool,
    pub error: Option<String>,
    pub backend: Backend,
    pub is_edit: bool,
    pub original_name: Option<String>,
    pub auth_type: Option<String>,
}

impl Default for FormState {
    fn default() -> Self {
        Self::new()
    }
}

impl FormState {
    pub fn new() -> Self {
        Self::new_for_backend(Backend::Claude)
    }

    pub fn new_for_backend(backend: Backend) -> Self {
        Self {
            fields: [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            active_field: 0,
            confirming: false,
            error: None,
            backend,
            is_edit: false,
            original_name: None,
            auth_type: None,
        }
    }

    pub fn from_profile(profile: &Profile) -> Self {
        let mut form = Self::new_for_backend(profile.backend.clone());
        form.is_edit = true;
        form.original_name = Some(profile.name.clone());

        match profile.backend {
            Backend::Claude => {
                let env = profile.env.as_ref();
                form.fields = [
                    profile.name.clone(),
                    profile.description.clone().unwrap_or_default(),
                    profile.base_url.clone().unwrap_or_else(|| {
                        env.and_then(|map| map.get("ANTHROPIC_BASE_URL").cloned())
                            .unwrap_or_default()
                    }),
                    env.and_then(|map| {
                        map.get("ANTHROPIC_API_KEY")
                            .or_else(|| map.get("ANTHROPIC_AUTH_TOKEN"))
                            .cloned()
                    })
                    .unwrap_or_default(),
                    profile.model.clone().unwrap_or_else(|| {
                        env.and_then(|map| map.get("ANTHROPIC_MODEL").cloned())
                            .unwrap_or_default()
                    }),
                    env.and_then(|map| {
                        map.get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
                            .or_else(|| map.get("ANTHROPIC_SMALL_FAST_MODEL"))
                            .cloned()
                    })
                    .unwrap_or_default(),
                ];
                form.auth_type = profile.auth_type.clone();
            }
            Backend::Codex => {
                let env = profile.env.as_ref();
                let full_auto = profile.full_auto.as_ref();
                form.fields = [
                    profile.name.clone(),
                    profile.base_url.clone().unwrap_or_default(),
                    env.and_then(|map| map.get("OPENAI_API_KEY").cloned())
                        .unwrap_or_default(),
                    profile.model.clone().unwrap_or_default(),
                    match full_auto {
                        Some(crate::config::ApprovalLevel::Danger) => "danger".into(),
                        Some(crate::config::ApprovalLevel::Never) => "never".into(),
                        Some(crate::config::ApprovalLevel::Untrusted) => "untrusted".into(),
                        None => String::new(),
                    },
                    String::new(),
                ];
            }
            Backend::Kimi => {
                let env = profile.env.as_ref();
                form.fields = [
                    profile.name.clone(),
                    profile.description.clone().unwrap_or_default(),
                    profile.base_url.clone().unwrap_or_else(|| {
                        env.and_then(|map| map.get("ANTHROPIC_BASE_URL").cloned())
                            .unwrap_or_default()
                    }),
                    env.and_then(|map| {
                        map.get("ANTHROPIC_AUTH_TOKEN")
                            .or_else(|| map.get("ANTHROPIC_API_KEY"))
                            .cloned()
                    })
                    .unwrap_or_default(),
                    profile.model.clone().unwrap_or_else(|| {
                        env.and_then(|map| map.get("ANTHROPIC_MODEL").cloned())
                            .unwrap_or_default()
                    }),
                    profile.max_context_size.clone().unwrap_or_default(),
                ];
            }
        }

        form
    }

    pub fn next_field(&mut self) {
        self.active_field = (self.active_field + 1).min(5);
    }

    pub fn prev_field(&mut self) {
        self.active_field = self.active_field.saturating_sub(1);
    }

    /// Map form fields to NewProfile based on backend.
    /// Single source of truth for field-index-to-semantic mapping.
    pub fn to_new_profile(&self) -> NewProfile {
        let name = self.fields[0].trim().to_string();
        match self.backend {
            Backend::Claude => {
                let desc = self.fields[1].trim().to_string();
                let base_url = self.fields[2].trim().to_string();
                let api_key = self.fields[3].trim().to_string();
                let model = self.fields[4].trim().to_string();
                let fast_model = self.fields[5].trim().to_string();
                NewProfile {
                    name,
                    description: if desc.is_empty() { None } else { Some(desc) },
                    base_url: if base_url.is_empty() {
                        None
                    } else {
                        Some(base_url)
                    },
                    api_key: if api_key.is_empty() {
                        None
                    } else {
                        Some(api_key)
                    },
                    model: if model.is_empty() { None } else { Some(model) },
                    fast_model: if fast_model.is_empty() {
                        None
                    } else {
                        Some(fast_model)
                    },
                    backend: Backend::Claude,
                    full_auto: None,
                    auth_type: self.auth_type.clone(),
                    max_context_size: None,
                }
            }
            Backend::Codex => {
                let base_url = self.fields[1].trim().to_string();
                let api_key = self.fields[2].trim().to_string();
                let model = self.fields[3].trim().to_string();
                let full_auto_str = self.fields[4].trim().to_lowercase();
                let full_auto = match full_auto_str.as_str() {
                    "danger" => Some(crate::config::ApprovalLevel::Danger),
                    "never" => Some(crate::config::ApprovalLevel::Never),
                    "untrusted" => Some(crate::config::ApprovalLevel::Untrusted),
                    "y" | "yes" => Some(crate::config::ApprovalLevel::Danger), // backward compat
                    _ => None,
                };
                NewProfile {
                    name,
                    description: None,
                    base_url: if base_url.is_empty() {
                        None
                    } else {
                        Some(base_url)
                    },
                    api_key: if api_key.is_empty() {
                        None
                    } else {
                        Some(api_key)
                    },
                    model: if model.is_empty() { None } else { Some(model) },
                    fast_model: None,
                    backend: Backend::Codex,
                    full_auto,
                    auth_type: None,
                    max_context_size: None,
                }
            }
            Backend::Kimi => {
                let desc = self.fields[1].trim().to_string();
                let base_url = self.fields[2].trim().to_string();
                let api_key = self.fields[3].trim().to_string();
                let model = self.fields[4].trim().to_string();
                let context = self.fields[5].trim().to_string();
                NewProfile {
                    name,
                    description: if desc.is_empty() { None } else { Some(desc) },
                    base_url: if base_url.is_empty() {
                        None
                    } else {
                        Some(base_url)
                    },
                    api_key: if api_key.is_empty() {
                        None
                    } else {
                        Some(api_key)
                    },
                    model: if model.is_empty() { None } else { Some(model) },
                    fast_model: None,
                    backend: Backend::Kimi,
                    full_auto: None,
                    auth_type: None,
                    max_context_size: if context.is_empty() {
                        None
                    } else {
                        Some(context)
                    },
                }
            }
        }
    }
}

pub struct App {
    pub profiles: Vec<Profile>,
    pub selected: usize,
    pub mode: AppMode,
    pub active_backend: Backend,
}

impl App {
    pub fn new(profiles: Vec<Profile>) -> Self {
        Self {
            profiles,
            selected: 0,
            mode: AppMode::Normal,
            active_backend: Backend::Claude,
        }
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        self.profiles
            .iter()
            .enumerate()
            .filter(|(_, p)| p.backend == self.active_backend)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn switch_backend(&mut self, backend: Backend) {
        self.active_backend = backend;
        let indices = self.filtered_indices();
        self.selected = indices.first().copied().unwrap_or(0);
    }

    pub fn next(&mut self) {
        let indices = self.filtered_indices();
        if indices.is_empty() {
            return;
        }
        if let Some(pos) = indices.iter().position(|&i| i == self.selected) {
            let next_pos = (pos + 1) % indices.len();
            self.selected = indices[next_pos];
        } else {
            self.selected = indices[0];
        }
    }

    pub fn prev(&mut self) {
        let indices = self.filtered_indices();
        if indices.is_empty() {
            return;
        }
        if let Some(pos) = indices.iter().position(|&i| i == self.selected) {
            let prev_pos = if pos == 0 { indices.len() - 1 } else { pos - 1 };
            self.selected = indices[prev_pos];
        } else {
            self.selected = indices[0];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_state_field_navigation() {
        let mut form = FormState::new();
        assert_eq!(form.active_field, 0);

        form.next_field();
        assert_eq!(form.active_field, 1);

        form.next_field();
        assert_eq!(form.active_field, 2);

        form.next_field();
        assert_eq!(form.active_field, 3);

        form.next_field();
        assert_eq!(form.active_field, 4);

        form.next_field();
        assert_eq!(form.active_field, 5);

        // Should clamp at max (5)
        form.next_field();
        assert_eq!(form.active_field, 5);

        form.prev_field();
        assert_eq!(form.active_field, 4);

        form.prev_field();
        assert_eq!(form.active_field, 3);

        form.prev_field();
        assert_eq!(form.active_field, 2);

        form.prev_field();
        assert_eq!(form.active_field, 1);

        form.prev_field();
        assert_eq!(form.active_field, 0);

        // Should clamp at min (0)
        form.prev_field();
        assert_eq!(form.active_field, 0);
    }

    #[test]
    fn app_mode_transitions() {
        let app = App::new(vec![]);
        assert!(matches!(app.mode, AppMode::Normal));

        // Transition to AddForm
        let mut app = App::new(vec![]);
        app.mode = AppMode::AddForm(Box::default());
        match &app.mode {
            AppMode::AddForm(form) => {
                assert_eq!(form.active_field, 0);
                assert!(!form.confirming);
                assert!(form.error.is_none());
                assert!(!form.is_edit);
                assert!(form.original_name.is_none());
            }
            _ => panic!("expected AddForm mode"),
        }

        // Transition back to Normal
        app.mode = AppMode::Normal;
        assert!(matches!(app.mode, AppMode::Normal));
    }

    fn make_profile(name: &str, backend: crate::config::Backend) -> Profile {
        Profile {
            name: name.into(),
            description: None,
            env: None,
            extra_args: None,
            skip_permissions: None,
            model: None,
            backend,
            base_url: None,
            full_auto: None,
            auth_type: None,
            max_context_size: None,
        }
    }

    #[test]
    fn filtered_indices_returns_correct_backend_subset() {
        use crate::config::Backend;
        let profiles = vec![
            make_profile("claude-1", Backend::Claude),
            make_profile("codex-1", Backend::Codex),
            make_profile("claude-2", Backend::Claude),
            make_profile("codex-2", Backend::Codex),
        ];
        let mut app = App::new(profiles);

        // Default active_backend is Claude
        assert_eq!(app.filtered_indices(), vec![0, 2]);

        app.active_backend = Backend::Codex;
        assert_eq!(app.filtered_indices(), vec![1, 3]);
    }

    #[test]
    fn switch_backend_resets_selected_to_first_matching() {
        use crate::config::Backend;
        let profiles = vec![
            make_profile("claude-1", Backend::Claude),
            make_profile("codex-1", Backend::Codex),
            make_profile("claude-2", Backend::Claude),
        ];
        let mut app = App::new(profiles);
        app.selected = 2; // pointing at claude-2

        app.switch_backend(Backend::Codex);
        assert_eq!(app.active_backend, Backend::Codex);
        assert_eq!(app.selected, 1); // first codex profile
    }

    #[test]
    fn next_prev_navigate_within_filtered_backend() {
        use crate::config::Backend;
        let profiles = vec![
            make_profile("claude-1", Backend::Claude),
            make_profile("codex-1", Backend::Codex),
            make_profile("claude-2", Backend::Claude),
            make_profile("codex-2", Backend::Codex),
        ];
        let mut app = App::new(profiles);
        // active_backend = Claude, filtered = [0, 2]
        assert_eq!(app.selected, 0);

        app.next();
        assert_eq!(app.selected, 2); // skip codex-1

        app.next();
        assert_eq!(app.selected, 0); // wrap around

        app.prev();
        assert_eq!(app.selected, 2); // wrap backward

        app.prev();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn field_labels_returns_backend_specific_labels() {
        use crate::config::Backend;
        let claude_labels = field_labels(&Backend::Claude);
        assert_eq!(
            claude_labels,
            [
                "Name *",
                "Description",
                "Base URL",
                "API Key",
                "Pro Model",
                "Fast Model"
            ]
        );

        let codex_labels = field_labels(&Backend::Codex);
        assert_eq!(
            codex_labels,
            ["Name *", "Base URL", "API Key", "Model", "Approval", ""]
        );

        let kimi_labels = field_labels(&Backend::Kimi);
        assert_eq!(
            kimi_labels,
            [
                "Name *",
                "Description",
                "Base URL",
                "API Key",
                "Model",
                "Context (1m/260k)"
            ]
        );
    }

    #[test]
    fn form_state_six_fields() {
        // field_labels for Claude should have 6 entries
        let claude_labels = field_labels(&Backend::Claude);
        assert_eq!(
            claude_labels.len(),
            6,
            "Claude field_labels must have 6 entries"
        );
        assert_eq!(
            claude_labels,
            [
                "Name *",
                "Description",
                "Base URL",
                "API Key",
                "Pro Model",
                "Fast Model"
            ]
        );

        // FormState.fields should have 6 elements
        let form = FormState::new();
        assert_eq!(
            form.fields.len(),
            6,
            "FormState.fields must have 6 elements"
        );

        // next_field should clamp at 5 (index of last field)
        let mut form = FormState::new();
        for _ in 0..10 {
            form.next_field();
        }
        assert_eq!(form.active_field, 5, "next_field must clamp at 5");
    }

    /// Invariant: for every backend, field_labels()[i] describes what
    /// to_new_profile() puts into the corresponding NewProfile field.
    /// If someone reorders labels without updating the mapping (or vice versa),
    /// this test catches it.
    #[test]
    fn claude_form_field_mapping_matches_labels() {
        use crate::config::Backend;
        let labels = field_labels(&Backend::Claude);

        // Fill fields with unique sentinel values matching the label semantics
        let mut form = FormState::new();
        form.backend = Backend::Claude;
        form.fields[0] = "my-profile".into(); // labels[0] = "Name *"
        form.fields[1] = "A description".into(); // labels[1] = "Description"
        form.fields[2] = "https://example.com".into(); // labels[2] = "Base URL"
        form.fields[3] = "sk-secret-123".into(); // labels[3] = "API Key"
        form.fields[4] = "claude-opus".into(); // labels[4] = "Pro Model"
        form.fields[5] = "claude-haiku".into(); // labels[5] = "Fast Model"

        let np = form.to_new_profile();

        // Verify each label's position maps to the correct NewProfile field
        assert!(labels[0].contains("Name"));
        assert_eq!(np.name, "my-profile");

        assert!(labels[1].contains("Description"));
        assert_eq!(np.description.as_deref(), Some("A description"));

        assert!(labels[2].contains("Base URL"));
        assert_eq!(np.base_url.as_deref(), Some("https://example.com"));

        assert!(labels[3].contains("Key"));
        assert_eq!(np.api_key.as_deref(), Some("sk-secret-123"));

        assert!(labels[4].contains("Model"));
        assert_eq!(np.model.as_deref(), Some("claude-opus"));

        assert!(labels[5].contains("Fast"));
        assert_eq!(np.fast_model.as_deref(), Some("claude-haiku"));

        assert_eq!(np.backend, Backend::Claude);
        assert!(np.full_auto.is_none());
    }

    #[test]
    fn from_profile_claude_prefills_fields() {
        use std::collections::HashMap;

        let mut env = HashMap::new();
        env.insert("ANTHROPIC_API_KEY".into(), "sk-ant-123".into());
        let profile = Profile {
            name: "claude-edit".into(),
            description: Some("Claude profile".into()),
            env: Some(env),
            extra_args: None,
            skip_permissions: Some(false),
            model: Some("claude-sonnet-4-6".into()),
            backend: Backend::Claude,
            base_url: Some("https://example.com/v1".into()),
            full_auto: None,
            auth_type: None,
            max_context_size: None,
        };

        let form = FormState::from_profile(&profile);

        assert!(form.is_edit);
        assert_eq!(form.original_name.as_deref(), Some("claude-edit"));
        assert_eq!(form.backend, Backend::Claude);
        assert_eq!(
            form.fields,
            [
                "claude-edit".to_string(),
                "Claude profile".to_string(),
                "https://example.com/v1".to_string(),
                "sk-ant-123".to_string(),
                "claude-sonnet-4-6".to_string(),
                String::new(),
            ]
        );
    }

    #[test]
    fn from_profile_codex_prefills_fields() {
        use std::collections::HashMap;

        let mut env = HashMap::new();
        env.insert("OPENAI_API_KEY".into(), "sk-openai-123".into());
        let profile = Profile {
            name: "codex-edit".into(),
            description: Some("ignored".into()),
            env: Some(env),
            extra_args: None,
            skip_permissions: None,
            model: Some("gpt-5.4".into()),
            backend: Backend::Codex,
            base_url: Some("https://api.openai.com/v1".into()),
            full_auto: Some(crate::config::ApprovalLevel::Danger),
            auth_type: None,
            max_context_size: None,
        };

        let form = FormState::from_profile(&profile);

        assert!(form.is_edit);
        assert_eq!(form.original_name.as_deref(), Some("codex-edit"));
        assert_eq!(form.backend, Backend::Codex);
        assert_eq!(
            form.fields,
            [
                "codex-edit".to_string(),
                "https://api.openai.com/v1".to_string(),
                "sk-openai-123".to_string(),
                "gpt-5.4".to_string(),
                "danger".to_string(),
                String::new(),
            ]
        );
    }

    #[test]
    fn codex_form_field_mapping_matches_labels() {
        use crate::config::Backend;
        let labels = field_labels(&Backend::Codex);

        let mut form = FormState::new();
        form.backend = Backend::Codex;
        form.fields[0] = "my-codex".into(); // labels[0] = "Name *"
        form.fields[1] = "https://api.openai.com".into(); // labels[1] = "Base URL"
        form.fields[2] = "sk-openai-key".into(); // labels[2] = "API Key"
        form.fields[3] = "gpt-4.1".into(); // labels[3] = "Model"
        form.fields[4] = "y".into(); // labels[4] = "Approval"

        let np = form.to_new_profile();

        assert!(labels[0].contains("Name"));
        assert_eq!(np.name, "my-codex");

        assert!(labels[1].contains("Base URL"));
        assert_eq!(np.base_url.as_deref(), Some("https://api.openai.com"));

        assert!(labels[2].contains("Key"));
        assert_eq!(np.api_key.as_deref(), Some("sk-openai-key"));

        assert!(labels[3].contains("Model"));
        assert_eq!(np.model.as_deref(), Some("gpt-4.1"));

        assert!(labels[4].contains("Approval"));
        assert_eq!(np.full_auto, Some(crate::config::ApprovalLevel::Danger));

        assert_eq!(np.backend, Backend::Codex);
        assert!(np.description.is_none());
    }

    /// Codex form must NOT produce API key in model field or vice versa.
    /// This is the exact regression that the user reported.
    #[test]
    fn codex_form_does_not_swap_api_key_and_model() {
        use crate::config::Backend;
        let mut form = FormState::new();
        form.backend = Backend::Codex;
        form.fields[0] = "test".into();
        form.fields[1] = "https://clauddy.com/v1".into(); // Base URL
        form.fields[2] = "sk-secret-key".into(); // API Key
        form.fields[3] = "gpt-5.3-codex".into(); // Model
        form.fields[4] = "n".into(); // Full Auto

        let np = form.to_new_profile();

        // The bug was: model got the API key value, and api_key got the base_url
        assert_eq!(
            np.model.as_deref(),
            Some("gpt-5.3-codex"),
            "model must be 'gpt-5.3-codex', not an API key"
        );
        assert_eq!(
            np.api_key.as_deref(),
            Some("sk-secret-key"),
            "api_key must be 'sk-secret-key', not a URL"
        );
        assert_eq!(
            np.base_url.as_deref(),
            Some("https://clauddy.com/v1"),
            "base_url must be the URL, not something else"
        );
    }

    #[test]
    fn from_profile_preserves_auth_type_token() {
        use std::collections::HashMap;
        let mut env = HashMap::new();
        env.insert("ANTHROPIC_AUTH_TOKEN".into(), "sk-token".into());
        let profile = Profile {
            name: "token-prof".into(),
            description: None,
            env: Some(env),
            extra_args: None,
            skip_permissions: None,
            model: None,
            backend: Backend::Claude,
            base_url: None,
            full_auto: None,
            auth_type: Some("token".into()),
            max_context_size: None,
        };
        let form = FormState::from_profile(&profile);
        assert_eq!(form.auth_type.as_deref(), Some("token"));
        assert_eq!(form.fields[3], "sk-token");
    }

    #[test]
    fn to_new_profile_passes_auth_type() {
        let mut form = FormState::new();
        form.auth_type = Some("token".into());
        form.fields[0] = "test".into();
        form.fields[3] = "sk-key".into();
        let np = form.to_new_profile();
        assert_eq!(np.auth_type.as_deref(), Some("token"));
        assert_eq!(np.api_key.as_deref(), Some("sk-key"));
    }

    #[test]
    fn kimi_form_field_mapping_matches_labels() {
        use crate::config::Backend;
        let labels = field_labels(&Backend::Kimi);

        let mut form = FormState::new();
        form.backend = Backend::Kimi;
        form.fields[0] = "my-kimi".into(); // labels[0] = "Name *"
        form.fields[1] = "Kimi profile".into(); // labels[1] = "Description"
        form.fields[2] = "https://api.kimi.com/v1".into(); // labels[2] = "Base URL"
        form.fields[3] = "sk-kimi-key".into(); // labels[3] = "API Key"
        form.fields[4] = "k3".into(); // labels[4] = "Model"
        form.fields[5] = "1m".into(); // labels[5] = "Context (1m/260k)"

        let np = form.to_new_profile();

        assert!(labels[0].contains("Name"));
        assert_eq!(np.name, "my-kimi");

        assert!(labels[1].contains("Description"));
        assert_eq!(np.description.as_deref(), Some("Kimi profile"));

        assert!(labels[2].contains("Base URL"));
        assert_eq!(np.base_url.as_deref(), Some("https://api.kimi.com/v1"));

        assert!(labels[3].contains("Key"));
        assert_eq!(np.api_key.as_deref(), Some("sk-kimi-key"));

        assert!(labels[4].contains("Model"));
        assert_eq!(np.model.as_deref(), Some("k3"));

        assert!(labels[5].contains("Context"));
        assert_eq!(np.max_context_size.as_deref(), Some("1m"));

        assert_eq!(np.backend, Backend::Kimi);
        assert!(np.fast_model.is_none());
        assert!(np.full_auto.is_none());
        assert!(np.auth_type.is_none());
    }

    #[test]
    fn kimi_form_empty_context_maps_to_none() {
        use crate::config::Backend;
        let mut form = FormState::new();
        form.backend = Backend::Kimi;
        form.fields[0] = "my-kimi".into();

        let np = form.to_new_profile();
        assert!(np.max_context_size.is_none());
    }

    #[test]
    fn from_profile_kimi_prefills_fields() {
        use std::collections::HashMap;

        let mut env = HashMap::new();
        env.insert("ANTHROPIC_API_KEY".into(), "sk-kimi-123".into());
        env.insert("ANTHROPIC_BASE_URL".into(), "https://env.example/v1".into());
        let profile = Profile {
            name: "kimi-edit".into(),
            description: Some("Kimi profile".into()),
            env: Some(env),
            extra_args: None,
            skip_permissions: None,
            model: Some("kimi-k2".into()),
            backend: Backend::Kimi,
            base_url: None,
            full_auto: None,
            auth_type: None,
            max_context_size: Some("260k".into()),
        };

        let form = FormState::from_profile(&profile);

        assert!(form.is_edit);
        assert_eq!(form.original_name.as_deref(), Some("kimi-edit"));
        assert_eq!(form.backend, Backend::Kimi);
        assert_eq!(
            form.fields,
            [
                "kimi-edit".to_string(),
                "Kimi profile".to_string(),
                "https://env.example/v1".to_string(), // falls back to env
                "sk-kimi-123".to_string(),
                "kimi-k2".to_string(),
                "260k".to_string(),
            ]
        );

        // Round-trip: editing without changes preserves max_context_size
        let np = form.to_new_profile();
        assert_eq!(np.max_context_size.as_deref(), Some("260k"));
        assert_eq!(np.backend, Backend::Kimi);
    }
}
