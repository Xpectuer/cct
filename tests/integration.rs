use std::io::Write;
use std::process::Command;

use cct::config::{self, Profile};
use cct::launch;

fn helpers_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/helpers")
}

fn prepend_path(dir: &std::path::Path) -> String {
    let orig = std::env::var("PATH").unwrap_or_default();
    format!("{}:{}", dir.display(), orig)
}

fn write_temp_toml(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().expect("create temp file");
    f.write_all(content.as_bytes()).expect("write temp toml");
    f.flush().expect("flush temp toml");
    f
}

/// Run the exec_profile example as a subprocess with the given env vars.
fn run_exec_profile(envs: Vec<(&str, String)>) -> std::process::Output {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--example", "exec_profile", "--quiet"]);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output()
        .expect("spawn cargo run --example exec_profile")
}

// --- Test 1: config round-trip ---

#[test]
fn config_round_trip() {
    let toml = r#"
[[profiles]]
name = "test-profile"
description = "A test profile"
model = "test-model"
skip_permissions = true
extra_args = ["--verbose", "--debug"]

[profiles.env]
MY_VAR = "my_value"
"#;
    let f = write_temp_toml(toml);
    std::env::set_var("CCT_CONFIG", f.path());
    let profiles = config::load_profiles().expect("load profiles");
    std::env::remove_var("CCT_CONFIG");

    assert_eq!(profiles.len(), 1);
    let p = &profiles[0];
    assert_eq!(p.name, "test-profile");
    assert_eq!(p.description.as_deref(), Some("A test profile"));
    assert_eq!(p.model.as_deref(), Some("test-model"));
    assert_eq!(p.skip_permissions, Some(true));
    assert_eq!(
        p.extra_args.as_deref(),
        Some(&["--verbose".to_string(), "--debug".to_string()][..])
    );
    let env = p.env.as_ref().expect("env map");
    assert_eq!(env.get("MY_VAR").map(String::as_str), Some("my_value"));
}

// --- Test 2: build_args ordering ---

#[test]
fn build_args_ordering() {
    let profile = Profile {
        name: "full".into(),
        description: None,
        env: None,
        model: Some("opus".into()),
        skip_permissions: Some(true),
        extra_args: Some(vec!["--verbose".into(), "--debug".into()]),
        backend: cct::config::Backend::Claude,
        base_url: None,
        full_auto: None,
        auth_type: None,
        max_context_size: None,
    };
    let args = launch::build_args(&profile, false);
    assert_eq!(
        args,
        vec![
            "--model",
            "opus",
            "--dangerously-skip-permissions",
            "--verbose",
            "--debug"
        ]
    );
}

// --- Test 3: build_args empty profile ---

#[test]
fn build_args_empty_profile() {
    let profile = Profile {
        name: "minimal".into(),
        description: None,
        env: None,
        model: None,
        skip_permissions: None,
        extra_args: None,
        backend: cct::config::Backend::Claude,
        base_url: None,
        full_auto: None,
        auth_type: None,
        max_context_size: None,
    };
    let args = launch::build_args(&profile, false);
    assert!(args.is_empty());
}

// --- Test 4: exec full profile via fake binary ---

#[test]
fn exec_full_profile_fake_binary() {
    let toml = r#"
[[profiles]]
name = "fake-test"
model = "test-model"
skip_permissions = true
extra_args = ["--verbose"]
"#;
    let toml_file = write_temp_toml(toml);
    let args_file = tempfile::NamedTempFile::new().expect("create args temp");
    let args_path = args_file.path().to_str().unwrap().to_string();

    let output = run_exec_profile(vec![
        (
            "CCT_TEST_TOML",
            toml_file.path().to_str().unwrap().to_string(),
        ),
        ("CCT_TEST_ARGS_FILE", args_path.clone()),
        ("PATH", prepend_path(&helpers_dir())),
    ]);

    assert!(
        output.status.success(),
        "exec_profile failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let captured = std::fs::read_to_string(&args_path).expect("read args file");
    assert_eq!(
        captured,
        "--model test-model --dangerously-skip-permissions --verbose"
    );
}

// --- Test 5: exec env injection ---

#[test]
fn exec_env_injection() {
    let toml = r#"
[[profiles]]
name = "env-test"

[profiles.env]
CCT_INJECTED_VAR = "hello-from-profile"
"#;
    let toml_file = write_temp_toml(toml);
    let args_file = tempfile::NamedTempFile::new().expect("create args temp");
    let args_path = args_file.path().to_str().unwrap().to_string();

    // Create a custom stub that writes args AND the injected env var.
    let tmp_dir = tempfile::tempdir().expect("create temp dir");
    let custom_stub = tmp_dir.path().join("claude");
    std::fs::write(
        &custom_stub,
        "#!/bin/sh\nprintf '%s' \"$*\" > \"${CCT_TEST_ARGS_FILE}\"\nprintf '%s' \"${CCT_INJECTED_VAR}\" > \"${CCT_TEST_ARGS_FILE}.env\"\nexit 0\n",
    )
    .expect("write custom stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&custom_stub, std::fs::Permissions::from_mode(0o755))
            .expect("chmod custom stub");
    }

    let output = run_exec_profile(vec![
        (
            "CCT_TEST_TOML",
            toml_file.path().to_str().unwrap().to_string(),
        ),
        ("CCT_TEST_ARGS_FILE", args_path.clone()),
        ("PATH", prepend_path(tmp_dir.path())),
    ]);

    assert!(
        output.status.success(),
        "exec_profile failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let env_val =
        std::fs::read_to_string(format!("{}.env", args_path)).expect("read env capture file");
    assert_eq!(env_val, "hello-from-profile");
}
