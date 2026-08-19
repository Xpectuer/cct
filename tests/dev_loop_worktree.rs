//! dev-loop test-writer additions for issue #7 — `claude --worktree` support.
//!
//! These are tests the implementation plan did NOT ask for: module-seam and
//! boundary tests written independently of the developer's suite. They
//! exercise the public interfaces the change crosses, as a consumer would:
//!
//! - config seam: `load_profiles` / `find_profile_by_name` over a real TOML
//!   file (validation rejects codex+worktree through the user-facing path),
//!   plus the non-boolean `worktree` type boundary.
//! - persistence seam: the `s`/`t` toggles and the Claude arm of
//!   `update_profile` must never drop a `worktree` key (criterion 4).
//! - launch seam: `build_launch_command` dispatch — full 5-item ordering
//!   contract, and the invariant that codex launch args never contain
//!   `--worktree`.
//! - cross-module/CLI seams: end-to-end argv through `exec_claude` (stub
//!   binary) with the flag positioned between `--dangerously-skip-permissions`
//!   and `extra_args`; the real `cct run <name>` CLI path (acceptance
//!   criterion 1); and `cct env` never leaking `--worktree` (launch flag,
//!   not env).

use std::io::Write;
use std::process::Command;

use cct::config::{self, Backend, NewProfile, Profile};
use cct::launch;

// --- Shared harness (mirrors tests/integration.rs) ---

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

/// Run the exec_profile example as a subprocess (crosses config → launch).
fn run_exec_profile(envs: Vec<(&str, String)>) -> std::process::Output {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--example", "exec_profile", "--quiet"]);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output()
        .expect("spawn cargo run --example exec_profile")
}

/// Run the real `cct` binary with the given args and child env.
fn run_cct_binary(args: &[&str], envs: Vec<(&str, String)>) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cct"));
    cmd.args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("spawn cct binary")
}

/// Write an executable shell stub into a fresh temp dir; returns the dir.
fn stub_bin(dir: &std::path::Path, name: &str, script: &str) {
    let file = dir.join(name);
    std::fs::write(&file, script).expect("write stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755))
            .expect("chmod stub");
    }
}

const ARGS_STUB: &str = "#!/bin/sh\nprintf '%s' \"$*\" > \"${CCT_TEST_ARGS_FILE}\"\nexit 0\n";

// --- config seam: load_profiles over a real file ---

#[test]
#[serial_test::serial]
fn devloop_load_profiles_claude_worktree_true() {
    let f = write_temp_toml("[[profiles]]\nname = \"w\"\nworktree = true\n");
    std::env::set_var("CCT_CONFIG", f.path());
    let profiles = config::load_profiles().expect("claude + worktree must load");
    std::env::remove_var("CCT_CONFIG");

    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].name, "w");
    assert_eq!(profiles[0].backend, Backend::Claude);
    assert_eq!(profiles[0].worktree, Some(true));
}

#[test]
#[serial_test::serial]
fn devloop_load_profiles_rejects_codex_worktree_true() {
    // Acceptance criterion 3 through the user-facing path: a hand-edited
    // config carrying worktree = true on a codex profile fails to LOAD,
    // naming the profile and the key.
    let f = write_temp_toml(
        "[[profiles]]\nname = \"bad-codex\"\nbackend = \"codex\"\nworktree = true\n",
    );
    std::env::set_var("CCT_CONFIG", f.path());
    let result = config::load_profiles();
    std::env::remove_var("CCT_CONFIG");

    let err = result.expect_err("codex + worktree = true must fail to load");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("bad-codex") && msg.contains("worktree"),
        "error should name the profile and 'worktree', got: {msg}"
    );
}

#[test]
#[serial_test::serial]
fn devloop_load_profiles_accepts_codex_worktree_false() {
    // worktree = false on codex is legal and must round-trip as Some(false).
    let f = write_temp_toml(
        "[[profiles]]\nname = \"ok-codex\"\nbackend = \"codex\"\nworktree = false\n",
    );
    std::env::set_var("CCT_CONFIG", f.path());
    let profiles = config::load_profiles().expect("codex + worktree = false must load");
    std::env::remove_var("CCT_CONFIG");

    assert_eq!(profiles[0].backend, Backend::Codex);
    assert_eq!(profiles[0].worktree, Some(false));
}

#[test]
#[serial_test::serial]
fn devloop_load_profiles_rejects_codex_worktree_after_valid_profile() {
    // Validation covers EVERY profile: a valid claude entry before a bad
    // codex entry must not mask the error, and the error names the bad one.
    let f = write_temp_toml(
        "[[profiles]]\nname = \"good-claude\"\nworktree = true\n\n\
         [[profiles]]\nname = \"bad-codex\"\nbackend = \"codex\"\nworktree = true\n",
    );
    std::env::set_var("CCT_CONFIG", f.path());
    let result = config::load_profiles();
    std::env::remove_var("CCT_CONFIG");

    let err = result.expect_err("second-profile codex + worktree must fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("bad-codex"),
        "error should name the offending codex profile, got: {msg}"
    );
}

#[test]
#[serial_test::serial]
fn devloop_load_profiles_rejects_non_boolean_worktree() {
    // Type boundary: a non-boolean worktree value must be a parse error, not
    // silently coerced. (Users hand-edit TOML; a string here must not pass.)
    let f = write_temp_toml("[[profiles]]\nname = \"w\"\nworktree = \"yes\"\n");
    std::env::set_var("CCT_CONFIG", f.path());
    let result = config::load_profiles();
    std::env::remove_var("CCT_CONFIG");

    let err = result.expect_err("worktree = \"yes\" must fail to parse");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("expected a boolean"),
        "parse error should mention the boolean expectation, got: {msg}"
    );
}

#[test]
#[serial_test::serial]
fn devloop_find_profile_by_name_returns_worktree() {
    // `cct run <name>` data path: case-insensitive lookup carries the flag.
    let f = write_temp_toml("[[profiles]]\nname = \"WorkTreeProf\"\nworktree = true\n");
    std::env::set_var("CCT_CONFIG", f.path());
    let found = config::find_profile_by_name("worktreeprof")
        .expect("lookup")
        .expect("profile exists");
    std::env::remove_var("CCT_CONFIG");

    assert_eq!(found.name, "WorkTreeProf");
    assert_eq!(found.worktree, Some(true));
}

// --- persistence seam: toggles and update_profile must not drop worktree ---

#[test]
#[serial_test::serial]
fn devloop_toggle_skip_permissions_preserves_worktree() {
    // Criterion 4: the `s` toggle is a surgical toml_edit write that must
    // never touch the worktree key.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profiles.toml");
    std::fs::write(&path, "[[profiles]]\nname = \"t\"\nworktree = true\n").unwrap();
    std::env::set_var("CCT_CONFIG", &path);

    config::toggle_skip_permissions("t", true).unwrap();

    let profiles = config::load_profiles().unwrap();
    assert_eq!(profiles[0].skip_permissions, Some(true));
    assert_eq!(
        profiles[0].worktree,
        Some(true),
        "s-toggle must not drop worktree"
    );
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("worktree = true"),
        "worktree key must survive in the file, got:\n{content}"
    );

    std::env::remove_var("CCT_CONFIG");
}

#[test]
#[serial_test::serial]
fn devloop_toggle_auth_type_preserves_worktree() {
    // Criterion 4: the `t` toggle rewrites env keys and auth_type; the
    // worktree key must survive untouched.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profiles.toml");
    std::fs::write(
        &path,
        "[[profiles]]\nname = \"t\"\nworktree = true\n\n[profiles.env]\nANTHROPIC_API_KEY = \"sk-123\"\n",
    )
    .unwrap();
    std::env::set_var("CCT_CONFIG", &path);

    config::toggle_auth_type("t").unwrap();

    let profiles = config::load_profiles().unwrap();
    assert_eq!(profiles[0].auth_type.as_deref(), Some("token"));
    assert_eq!(
        profiles[0].worktree,
        Some(true),
        "t-toggle must not drop worktree"
    );
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("worktree = true"),
        "worktree key must survive in the file, got:\n{content}"
    );

    std::env::remove_var("CCT_CONFIG");
}

#[test]
#[serial_test::serial]
fn devloop_toggle_full_auto_preserves_legal_codex_worktree_false() {
    // A legal `worktree = false` on a codex profile is not a validation
    // error; a codex toggle must leave the key in place (surgical writes).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profiles.toml");
    std::fs::write(
        &path,
        "[[profiles]]\nname = \"c\"\nbackend = \"codex\"\nworktree = false\n",
    )
    .unwrap();
    std::env::set_var("CCT_CONFIG", &path);

    config::toggle_full_auto("c", Some("danger")).unwrap();

    let profiles = config::load_profiles().unwrap();
    assert_eq!(profiles[0].full_auto, Some(config::ApprovalLevel::Danger));
    assert_eq!(profiles[0].worktree, Some(false));
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("worktree = false"),
        "legal worktree = false must survive codex toggle, got:\n{content}"
    );

    std::env::remove_var("CCT_CONFIG");
}

#[test]
#[serial_test::serial]
fn devloop_update_profile_claude_arm_preserves_worktree_and_skip_permissions() {
    // Criterion 4: an edit through the TUI form (Claude arm) must preserve
    // both claude-only flags and extra_args — not just worktree.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profiles.toml");
    std::fs::write(
        &path,
        "[[profiles]]\nname = \"w\"\nworktree = true\nskip_permissions = true\nextra_args = [\"--verbose\"]\n",
    )
    .unwrap();
    std::env::set_var("CCT_CONFIG", &path);

    let updated = NewProfile {
        name: "w".into(),
        description: Some("Edited".into()),
        base_url: None,
        api_key: None,
        model: Some("claude-opus-5".into()),
        fast_model: None,
        backend: Backend::Claude,
        full_auto: None,
        auth_type: None,
    };
    config::update_profile("w", &updated).unwrap();

    let profiles = config::load_profiles().unwrap();
    let p = &profiles[0];
    assert_eq!(p.model.as_deref(), Some("claude-opus-5"));
    assert_eq!(p.worktree, Some(true), "claude edit must keep worktree");
    assert_eq!(
        p.skip_permissions,
        Some(true),
        "claude edit must keep skip_permissions"
    );
    assert_eq!(p.extra_args.as_deref(), Some(&["--verbose".to_string()][..]));
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("worktree = true") && content.contains("skip_permissions = true"),
        "both claude-only keys must survive in the file, got:\n{content}"
    );

    std::env::remove_var("CCT_CONFIG");
}

// --- launch seam: build_launch_command dispatch ---

#[test]
fn devloop_build_launch_command_full_combination_ordering() {
    // The complete 5-item ordering contract in one assertion: continue,
    // model, skip-permissions, worktree, extra_args — via the public
    // dispatch seam main() actually uses.
    let profile = Profile {
        name: "full".into(),
        description: None,
        env: None,
        model: Some("opus".into()),
        skip_permissions: Some(true),
        worktree: Some(true),
        extra_args: Some(vec!["--verbose".into(), "--debug".into()]),
        backend: Backend::Claude,
        base_url: None,
        full_auto: None,
        auth_type: None,
    };
    let (bin, args) = launch::build_launch_command(&profile, true);
    assert_eq!(bin, "claude");
    assert_eq!(
        args,
        vec![
            "--continue",
            "--model",
            "opus",
            "--dangerously-skip-permissions",
            "--worktree",
            "--verbose",
            "--debug",
        ]
    );
}

#[test]
fn devloop_build_launch_command_codex_never_emits_worktree() {
    // Codex launch paths never read the worktree field — even a Some(true)
    // (unreachable via load, but the struct permits it) must not leak the
    // claude-only flag into codex argv.
    for worktree in [Some(true), Some(false), None] {
        let profile = Profile {
            name: "c".into(),
            description: None,
            env: None,
            model: None,
            skip_permissions: None,
            worktree,
            extra_args: Some(vec!["--quiet".into()]),
            backend: Backend::Codex,
            base_url: None,
            full_auto: None,
            auth_type: None,
        };
        let (bin, args) = launch::build_launch_command(&profile, false);
        assert_eq!(bin, "codex");
        assert_eq!(args, vec!["--quiet"]);
        assert!(
            !args.iter().any(|a| a == "--worktree"),
            "codex argv must never contain --worktree, got: {args:?}"
        );
    }
}

// --- cross-module seam: exec path with stub binaries ---

#[test]
fn devloop_exec_profile_stub_worktree_between_skip_and_extra() {
    // End-to-end through exec_claude (config → argv): the flag lands between
    // --dangerously-skip-permissions and extra_args in the real exec path.
    let toml = r#"
[[profiles]]
name = "fake-worktree-full"
model = "test-model"
skip_permissions = true
worktree = true
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
        "--model test-model --dangerously-skip-permissions --worktree --verbose"
    );
}

#[test]
fn devloop_cct_run_binary_emits_worktree() {
    // Acceptance criterion 1 path `cct run <name>`: the real binary must
    // launch the profile with --worktree in the argv the stub receives.
    let toml = r#"
[[profiles]]
name = "fake-run"
model = "test-model"
worktree = true
extra_args = ["--verbose"]
"#;
    let toml_file = write_temp_toml(toml);
    let args_file = tempfile::NamedTempFile::new().expect("create args temp");
    let args_path = args_file.path().to_str().unwrap().to_string();
    let stub_dir = tempfile::tempdir().expect("create stub dir");
    stub_bin(stub_dir.path(), "claude", ARGS_STUB);

    let output = run_cct_binary(
        &["run", "fake-run"],
        vec![
            ("CCT_CONFIG", toml_file.path().to_str().unwrap().to_string()),
            ("CCT_TEST_ARGS_FILE", args_path.clone()),
            ("PATH", prepend_path(stub_dir.path())),
        ],
    );

    assert!(
        output.status.success(),
        "cct run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let captured = std::fs::read_to_string(&args_path).expect("read args file");
    assert_eq!(captured, "--model test-model --worktree --verbose");
}

#[test]
fn devloop_cct_env_never_emits_worktree() {
    // `cct env` injects env vars only — it must NOT turn a worktree profile
    // into a launch with --worktree (launch flag, not env).
    let toml = "[[profiles]]\nname = \"fake-env\"\nworktree = true\n";
    let toml_file = write_temp_toml(toml);
    let args_file = tempfile::NamedTempFile::new().expect("create args temp");
    let args_path = args_file.path().to_str().unwrap().to_string();
    let stub_dir = tempfile::tempdir().expect("create stub dir");
    stub_bin(stub_dir.path(), "bash", ARGS_STUB);

    let output = run_cct_binary(
        &["env", "fake-env", "--", "echo", "hello"],
        vec![
            ("CCT_CONFIG", toml_file.path().to_str().unwrap().to_string()),
            ("CCT_TEST_ARGS_FILE", args_path.clone()),
            ("PATH", prepend_path(stub_dir.path())),
        ],
    );

    assert!(
        output.status.success(),
        "cct env failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let captured = std::fs::read_to_string(&args_path).expect("read args file");
    assert!(
        captured.contains("echo hello"),
        "env mode must run the user command, got: {captured:?}"
    );
    assert!(
        !captured.contains("--worktree"),
        "env mode must not leak --worktree into the command, got: {captured:?}"
    );
}
