use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::json;
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
async fn json_flag_returns_raw_compact_json() {
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
        r#"{"data":{"id":"test"},"meta":{"request_id":"req_json"}}"#.to_string() + "\n",
    ));
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
