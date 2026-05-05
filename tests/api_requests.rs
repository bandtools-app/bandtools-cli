use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::json;
use std::fs;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path, query_param},
};

#[tokio::test]
async fn subscribers_list_sends_auth_and_query_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/subscribers"))
        .and(query_param("page", "2"))
        .and(query_param("per_page", "10"))
        .and(header("authorization", "Bearer token-from-cli"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [],
            "meta": {
                "request_id": "req_test",
                "pagination": {
                    "page": 2,
                    "per_page": 10,
                    "total": 0,
                    "total_pages": 0,
                    "next_page": null,
                    "prev_page": null
                }
            }
        })))
        .mount(&server)
        .await;

    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.args([
        "--api-url",
        &server.uri(),
        "--api-token",
        "token-from-cli",
        "subscribers",
        "list",
        "--page",
        "2",
        "--per-page",
        "10",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "| __ )  __ _ _ __   __| |_   _|__   ___ | |___",
    ))
    .stdout(predicate::str::contains("fan@example.com").not())
    .stdout(predicate::str::contains("req_test"));
}

#[tokio::test]
async fn account_update_wraps_unwrapped_body() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/account"))
        .and(header("authorization", "Bearer env-token"))
        .and(body_json(json!({ "account": { "name": "Ava Band" } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "name": "Ava Band" },
            "meta": { "request_id": "req_update" }
        })))
        .mount(&server)
        .await;

    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.env("BANDTOOLS_API_TOKEN", "env-token")
        .args([
            "--api-url",
            &server.uri(),
            "account",
            "update",
            "--data",
            r#"{"name":"Ava Band"}"#,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("req_update"));
}

#[tokio::test]
async fn json_flag_returns_pretty_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "id": "test" },
            "meta": { "request_id": "req_json" }
        })))
        .mount(&server)
        .await;

    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.args([
        "--api-url",
        &server.uri(),
        "--api-token",
        "token",
        "--json",
        "account",
        "get",
    ])
    .assert()
    .success()
    .stdout(predicate::eq(
        "{\n  \"data\": {\n    \"id\": \"test\"\n  },\n  \"meta\": {\n    \"request_id\": \"req_json\"\n  }\n}\n",
    ));
}

#[tokio::test]
async fn compact_json_flag_returns_raw_compact_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "id": "test" },
            "meta": { "request_id": "req_json" }
        })))
        .mount(&server)
        .await;

    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.args([
        "--api-url",
        &server.uri(),
        "--api-token",
        "token",
        "--compact-json",
        "account",
        "get",
    ])
    .assert()
    .success()
    .stdout(predicate::eq(
        r#"{"data":{"id":"test"},"meta":{"request_id":"req_json"}}"#.to_string() + "\n",
    ));
}

#[tokio::test]
async fn plain_flag_returns_unornamented_text() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "id": "test",
                "name": "Ava Band"
            },
            "meta": { "request_id": "req_plain" }
        })))
        .mount(&server)
        .await;

    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.args([
        "--api-url",
        &server.uri(),
        "--api-token",
        "token",
        "--plain",
        "account",
        "get",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Data"))
    .stdout(predicate::str::contains("name: Ava Band"))
    .stdout(predicate::str::contains("request_id: req_plain"))
    .stdout(predicate::str::contains("┌").not())
    .stdout(predicate::str::contains("\x1b[").not());
}

#[tokio::test]
async fn configured_plain_output_is_used_without_an_output_flag() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "id": "test",
                "name": "Ava Band"
            },
            "meta": { "request_id": "req_config_plain" }
        })))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    fs::write(
        &config,
        format!(
            "api_token = \"token\"\napi_url = \"{}\"\noutput = \"plain\"\n",
            server.uri()
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.args(["--config", config.to_str().unwrap(), "account", "get"])
        .assert()
        .success()
        .stdout(predicate::str::contains("name: Ava Band"))
        .stdout(predicate::str::contains("req_config_plain"))
        .stdout(predicate::str::contains("┌").not())
        .stdout(predicate::str::contains("\x1b[").not());
}

#[tokio::test]
async fn output_flag_takes_precedence_over_configured_output() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "id": "test" },
            "meta": { "request_id": "req_output_precedence" }
        })))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    fs::write(
        &config,
        format!(
            "api_token = \"token\"\napi_url = \"{}\"\noutput = \"plain\"\n",
            server.uri()
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.args([
        "--config",
        config.to_str().unwrap(),
        "--compact-json",
        "account",
        "get",
    ])
    .assert()
    .success()
    .stdout(predicate::eq(
        r#"{"data":{"id":"test"},"meta":{"request_id":"req_output_precedence"}}"#.to_string()
            + "\n",
    ));
}

#[tokio::test]
async fn newsletters_pin_posts_to_pin_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/newsletters/newsletter123/pin"))
        .and(header("authorization", "Bearer token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "id": "newsletter123",
                "pinned": true
            },
            "meta": { "request_id": "req_pin" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.args([
        "--api-url",
        &server.uri(),
        "--api-token",
        "token",
        "--plain",
        "newsletters",
        "pin",
        "newsletter123",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("pinned: true"))
    .stdout(predicate::str::contains("req_pin"));
}

#[tokio::test]
async fn newsletters_unpin_deletes_pin_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/newsletters/newsletter123/pin"))
        .and(header("authorization", "Bearer token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "id": "newsletter123",
                "pinned": false
            },
            "meta": { "request_id": "req_unpin" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.args([
        "--api-url",
        &server.uri(),
        "--api-token",
        "token",
        "--plain",
        "newsletters",
        "unpin",
        "newsletter123",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("pinned: false"))
    .stdout(predicate::str::contains("req_unpin"));
}

#[tokio::test]
async fn api_token_cli_argument_takes_precedence_over_environment() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account"))
        .and(header("authorization", "Bearer cli-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "id": "test" },
            "meta": { "request_id": "req_precedence" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.env("BANDTOOLS_API_TOKEN", "env-token")
        .args([
            "--api-url",
            &server.uri(),
            "--api-token",
            "cli-token",
            "account",
            "get",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("req_precedence"));
}
