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
fn api_commands_require_token() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");

    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.args(["--config", config.to_str().unwrap(), "subscribers", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing BandTools API token"));
}
