use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn top_level_help_lists_command_groups() {
    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("subscribers"))
        .stdout(predicate::str::contains("newsletters"))
        .stdout(predicate::str::contains("automatic-newsletters"))
        .stdout(predicate::str::contains("--compact-json"))
        .stdout(predicate::str::contains("--plain"))
        .stdout(predicate::str::contains("--no-colour"))
        .stdout(predicate::str::contains("completions"))
        .stdout(predicate::str::contains("config"));
}

#[test]
fn nested_help_is_available() {
    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.args(["subscribers", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--page"))
        .stdout(predicate::str::contains("--filter"))
        .stdout(predicate::str::contains("--sort"));
}

#[test]
fn newsletters_help_lists_pin_commands() {
    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.args(["newsletters", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pin"))
        .stdout(predicate::str::contains("unpin"));
}

#[test]
fn config_commands_do_not_require_token() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");

    let mut set = Command::cargo_bin("bt").unwrap();
    set.args([
        "--config",
        config.to_str().unwrap(),
        "config",
        "set-api-url",
        "http://localhost:3000/api/v1/",
    ])
    .assert()
    .success();

    let mut show = Command::cargo_bin("bt").unwrap();
    show.args(["--config", config.to_str().unwrap(), "config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("http://localhost:3000/api/v1"));
}

#[test]
fn config_output_preference_can_be_set_and_unset() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");

    let mut set = Command::cargo_bin("bt").unwrap();
    set.args([
        "--config",
        config.to_str().unwrap(),
        "config",
        "set-output",
        "plain",
    ])
    .assert()
    .success();

    let mut show = Command::cargo_bin("bt").unwrap();
    show.args(["--config", config.to_str().unwrap(), "config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("plain"));

    let mut unset = Command::cargo_bin("bt").unwrap();
    unset
        .args([
            "--config",
            config.to_str().unwrap(),
            "config",
            "unset",
            "output",
        ])
        .assert()
        .success();

    let mut show = Command::cargo_bin("bt").unwrap();
    show.args([
        "--config",
        config.to_str().unwrap(),
        "--plain",
        "config",
        "show",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("output     : -"));
}

#[test]
fn completions_command_does_not_require_token() {
    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef bt"))
        .stdout(predicate::str::contains("newsletters"));
}

#[test]
fn api_commands_require_token() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");

    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.args(["--config", config.to_str().unwrap(), "subscribers", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing BandTools API token"));
}
