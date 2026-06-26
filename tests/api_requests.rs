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
async fn account_social_links_get_returns_social_links_from_account() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account"))
        .and(header("authorization", "Bearer token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "id": "test",
                "name": "Ava Band",
                "social_links": {
                    "bandcamp": "https://ava.example.com",
                    "instagram": "https://instagram.example.com/ava"
                }
            },
            "meta": { "request_id": "req_social_get" }
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
        "--compact-json",
        "account",
        "social-links",
        "get",
    ])
    .assert()
    .success()
    .stdout(predicate::eq(
        r#"{"data":{"bandcamp":"https://ava.example.com","instagram":"https://instagram.example.com/ava"},"meta":{"request_id":"req_social_get"}}"#
            .to_string()
            + "\n",
    ));
}

#[tokio::test]
async fn account_social_links_update_patches_social_links() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/account"))
        .and(header("authorization", "Bearer token"))
        .and(body_json(json!({
            "account": {
                "social_links": {
                    "bandcamp": "https://ava.example.com",
                    "bluesky": "https://bsky.app/profile/ava.example.com",
                    "instagram": null,
                    "x": "https://x.com/ava"
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "id": "test",
                "social_links": {
                    "bandcamp": "https://ava.example.com",
                    "bluesky": "https://bsky.app/profile/ava.example.com",
                    "x": "https://x.com/ava"
                }
            },
            "meta": { "request_id": "req_social_update" }
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
        "account",
        "social-links",
        "update",
        "--bandcamp",
        "https://ava.example.com",
        "--bluesky",
        "https://bsky.app/profile/ava.example.com",
        "--x",
        "https://x.com/ava",
        "--clear",
        "instagram",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("req_social_update"));
}

#[test]
fn account_social_links_update_requires_a_change() {
    let mut cmd = Command::cargo_bin("bt").unwrap();
    cmd.args([
        "--api-url",
        "http://localhost:3000/api/v1",
        "--api-token",
        "token",
        "account",
        "social-links",
        "update",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains(
        "social-links update requires at least one platform URL or --clear",
    ));
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
                "name": "Ava Band",
                "features": {
                    "automatic_newsletters": true,
                    "duplicate_newsletter": true,
                    "subscriber_limit": 1000,
                    "unlimited_newsletters": true
                }
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
        .stdout(predicate::str::contains("name    : Ava Band"))
        .stdout(predicate::str::contains("\"subscriber_limit\":1000"))
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
async fn newsletters_archive_posts_to_archive_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/newsletters/newsletter123/archive"))
        .and(header("authorization", "Bearer token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "id": "newsletter123",
                "public": true
            },
            "meta": { "request_id": "req_archive" }
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
        "archive",
        "newsletter123",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("public: true"))
    .stdout(predicate::str::contains("req_archive"));
}

#[tokio::test]
async fn newsletters_unarchive_deletes_archive_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/newsletters/newsletter123/archive"))
        .and(header("authorization", "Bearer token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "id": "newsletter123",
                "public": false,
                "pinned": false
            },
            "meta": { "request_id": "req_unarchive" }
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
        "unarchive",
        "newsletter123",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("public: false"))
    .stdout(predicate::str::contains("req_unarchive"));
}

#[tokio::test]
async fn newsletters_duplicate_posts_to_duplicate_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/newsletters/newsletter123/duplicate"))
        .and(header("authorization", "Bearer token"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "data": {
                "id": "newsletter456",
                "subject": "Tour announcement copy",
                "status": "draft"
            },
            "meta": { "request_id": "req_duplicate" }
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
        "duplicate",
        "newsletter123",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("newsletter456"))
    .stdout(predicate::str::contains("req_duplicate"));
}

#[tokio::test]
async fn newsletters_send_to_new_subscribers_posts_to_action_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/newsletters/newsletter123/send-to-new-subscribers"))
        .and(header("authorization", "Bearer token"))
        .respond_with(ResponseTemplate::new(202).set_body_json(json!({
            "data": {
                "status": "queued",
                "new_subscribers_count": 12
            },
            "meta": { "request_id": "req_send_new" }
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
        "send-to-new-subscribers",
        "newsletter123",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("new_subscribers_count: 12"))
    .stdout(predicate::str::contains("req_send_new"));
}

#[tokio::test]
async fn webhooks_list_sends_pagination_query_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/webhooks"))
        .and(query_param("page", "2"))
        .and(query_param("per_page", "5"))
        .and(query_param("sort", "name_asc"))
        .and(header("authorization", "Bearer token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [],
            "meta": { "request_id": "req_webhooks_list" }
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
        "webhooks",
        "list",
        "--page",
        "2",
        "--per-page",
        "5",
        "--sort",
        "name-asc",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("req_webhooks_list"));
}

#[tokio::test]
async fn collaborators_list_sends_sort_query_param() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/newsletters/newsletter123/collaborators"))
        .and(query_param("sort", "email_desc"))
        .and(header("authorization", "Bearer token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [],
            "meta": { "request_id": "req_collaborators_sort" }
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
        "collaborators",
        "list",
        "newsletter123",
        "--sort",
        "email-desc",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("req_collaborators_sort"));
}

#[tokio::test]
async fn automatic_newsletters_list_sends_sort_query_param() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/automatic-newsletters"))
        .and(query_param("page", "3"))
        .and(query_param("sort", "created_asc"))
        .and(header("authorization", "Bearer token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [],
            "meta": { "request_id": "req_automatic_sort" }
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
        "automatic-newsletters",
        "list",
        "--page",
        "3",
        "--sort",
        "created-asc",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("req_automatic_sort"));
}

#[tokio::test]
async fn webhooks_create_wraps_unwrapped_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/webhooks"))
        .and(header("authorization", "Bearer token"))
        .and(body_json(json!({
            "webhook": {
                "name": "Production sync",
                "url": "https://hooks.example.com/bandtools",
                "event_types": ["newsletter.sent"]
            }
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "data": {
                "id": 42,
                "name": "Production sync",
                "signing_secret": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            },
            "meta": { "request_id": "req_webhook_create" }
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
        "webhooks",
        "create",
        "--data",
        r#"{"name":"Production sync","url":"https://hooks.example.com/bandtools","event_types":["newsletter.sent"]}"#,
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("signing_secret"))
    .stdout(predicate::str::contains("req_webhook_create"));
}

#[tokio::test]
async fn webhooks_update_patches_wrapped_body_by_id() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/webhooks/42"))
        .and(header("authorization", "Bearer token"))
        .and(body_json(json!({
            "webhook": {
                "enabled": false,
                "event_types": ["subscriber.created"]
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "id": 42,
                "enabled": false,
                "event_types": ["subscriber.created"]
            },
            "meta": { "request_id": "req_webhook_update" }
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
        "webhooks",
        "update",
        "42",
        "--data",
        r#"{"webhook":{"enabled":false,"event_types":["subscriber.created"]}}"#,
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("enabled    : false"))
    .stdout(predicate::str::contains("req_webhook_update"));
}

#[tokio::test]
async fn webhooks_rotate_signing_secret_posts_to_action_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/webhooks/42/rotate-signing-secret"))
        .and(header("authorization", "Bearer token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "id": 42,
                "signing_secret": "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"
            },
            "meta": { "request_id": "req_webhook_rotate" }
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
        "webhooks",
        "rotate-signing-secret",
        "42",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("signing_secret"))
    .stdout(predicate::str::contains("req_webhook_rotate"));
}

#[tokio::test]
async fn webhooks_delete_deletes_by_id() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/webhooks/42"))
        .and(header("authorization", "Bearer token"))
        .respond_with(ResponseTemplate::new(204))
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
        "webhooks",
        "delete",
        "42",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("status: 204"));
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
