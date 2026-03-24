use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

fn plugin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/plugins/hello-world")
        .canonicalize()
        .expect("hello-world plugin dir must exist")
}

fn rusty_cmd(home: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.env("RUSTY_HOME", home);
    cmd
}

fn setup_with_plugin() -> (tempfile::TempDir, PathBuf) {
    let home = tempfile::tempdir().unwrap();
    let plugin = plugin_dir();

    // Install the plugin
    rusty_cmd(home.path())
        .args(["install", plugin.to_str().unwrap()])
        .assert()
        .success();

    (home, plugin)
}

// ─── Install ─────────────────────────────────────────────────

#[test]
fn install_succeeds() {
    let home = tempfile::tempdir().unwrap();

    rusty_cmd(home.path())
        .args(["install", plugin_dir().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed plugin: hello-world"));
}

#[test]
fn install_nonexistent_path_fails() {
    let home = tempfile::tempdir().unwrap();

    rusty_cmd(home.path())
        .args(["install", "/nonexistent/path"])
        .assert()
        .failure();
}

// ─── List ────────────────────────────────────────────────────

#[test]
fn list_empty_shows_no_plugins() {
    let home = tempfile::tempdir().unwrap();

    rusty_cmd(home.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No plugins installed"));
}

#[test]
fn list_shows_installed_plugin() {
    let (home, _) = setup_with_plugin();

    rusty_cmd(home.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello-world"))
        .stdout(predicate::str::contains("Hello World"))
        .stdout(predicate::str::contains("greet"));
}

// ─── Inspect ─────────────────────────────────────────────────

#[test]
fn inspect_shows_plugin_details() {
    let (home, _) = setup_with_plugin();

    rusty_cmd(home.path())
        .args(["inspect", "hello-world"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello World"))
        .stdout(predicate::str::contains("0.1.0"))
        .stdout(predicate::str::contains("Tyler Gibbs"))
        .stdout(predicate::str::contains("greet"))
        .stdout(predicate::str::contains("read-only"))
        .stdout(predicate::str::contains("input schema"));
}

#[test]
fn inspect_nonexistent_plugin_fails() {
    let home = tempfile::tempdir().unwrap();

    rusty_cmd(home.path())
        .args(["inspect", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// ─── Invoke ──────────────────────────────────────────────────

#[test]
fn invoke_success() {
    let (home, _) = setup_with_plugin();

    rusty_cmd(home.path())
        .args([
            "invoke",
            "hello-world",
            "greet",
            "--input",
            r#"{"name": "Test"}"#,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, Test!"))
        .stdout(predicate::str::contains("completed"));
}

#[test]
fn invoke_with_trace_flag() {
    let (home, _) = setup_with_plugin();

    rusty_cmd(home.path())
        .args([
            "invoke",
            "hello-world",
            "greet",
            "--input",
            r#"{"name": "Traced"}"#,
            "--trace",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, Traced!"))
        .stdout(predicate::str::contains("Trace for run"))
        .stdout(predicate::str::contains("validation_passed"))
        .stdout(predicate::str::contains("execution_succeeded"));
}

#[test]
fn invoke_invalid_input_shows_validation_error() {
    let (home, _) = setup_with_plugin();

    rusty_cmd(home.path())
        .args(["invoke", "hello-world", "greet", "--input", "{}"])
        .assert()
        .success() // the CLI itself succeeds, the invocation fails gracefully
        .stdout(predicate::str::contains("validation_failed"))
        .stdout(predicate::str::contains("name"));
}

#[test]
fn invoke_bad_json_input_fails() {
    let (home, _) = setup_with_plugin();

    rusty_cmd(home.path())
        .args([
            "invoke",
            "hello-world",
            "greet",
            "--input",
            "not-json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid JSON"));
}

#[test]
fn invoke_nonexistent_plugin_fails() {
    let home = tempfile::tempdir().unwrap();

    rusty_cmd(home.path())
        .args(["invoke", "nope", "greet", "--input", "{}"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn invoke_with_deny_policy() {
    let (home, _) = setup_with_plugin();

    // Write a deny-all policy
    std::fs::write(
        home.path().join("policy.toml"),
        "default-effect = \"deny\"\n",
    )
    .unwrap();

    rusty_cmd(home.path())
        .args([
            "invoke",
            "hello-world",
            "greet",
            "--input",
            r#"{"name": "Denied"}"#,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("policy_denied"));
}

// ─── Trace ───────────────────────────────────────────────────

#[test]
fn trace_nonexistent_run_fails() {
    let home = tempfile::tempdir().unwrap();

    rusty_cmd(home.path())
        .args(["trace", "00000000-0000-0000-0000-000000000000"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn trace_shows_saved_trace() {
    let (home, _) = setup_with_plugin();

    // Invoke to generate a trace
    let output = rusty_cmd(home.path())
        .args([
            "invoke",
            "hello-world",
            "greet",
            "--input",
            r#"{"name": "Saved"}"#,
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    // Extract the run ID from "Run: <uuid>"
    let run_id = stdout
        .lines()
        .find(|l| l.starts_with("Run: "))
        .unwrap()
        .trim_start_matches("Run: ")
        .trim();

    // Now retrieve the trace
    rusty_cmd(home.path())
        .args(["trace", run_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Trace for run"))
        .stdout(predicate::str::contains("hello-world"))
        .stdout(predicate::str::contains("greet"));
}

// ─── Help ────────────────────────────────────────────────────

#[test]
fn help_flag_works() {
    Command::cargo_bin("rusty")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("WASM plugin host platform"));
}

#[test]
fn version_flag_works() {
    Command::cargo_bin("rusty")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("rusty"));
}
