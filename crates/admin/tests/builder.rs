use std::time::Duration;

use nespra_admin::{
    Admin,
    error::{AdminError, BuildError},
    reqwest::header::{HeaderName, HeaderValue},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, path},
};

const UP: &str = r#"{"status":{"code":"up"}}"#;

async fn server(mock: Mock) -> MockServer {
    let server = MockServer::start().await;
    mock.mount(&server).await;
    server
}

fn health() -> wiremock::MockBuilder {
    Mock::given(path("/state/v1/health"))
}

#[tokio::test]
async fn test_default_user_agent() {
    let user_agent = concat!("nespra_admin/", env!("CARGO_PKG_VERSION"));
    let server = server(
        health()
            .and(header("user-agent", user_agent))
            .respond_with(ResponseTemplate::new(200).set_body_string(UP)),
    )
    .await;
    assert!(Admin::new(server.uri()).unwrap().health().await.is_ok());
}

#[tokio::test]
async fn test_custom_user_agent_and_headers() {
    let server = server(
        health()
            .and(header("user-agent", "lazyvespa"))
            .and(header("x-tenant", "acme"))
            .respond_with(ResponseTemplate::new(200).set_body_string(UP)),
    )
    .await;
    let admin = Admin::builder(server.uri())
        .user_agent(HeaderValue::from_static("lazyvespa"))
        .header(
            HeaderName::from_static("x-tenant"),
            HeaderValue::from_static("acme"),
        )
        .build()
        .unwrap();
    assert!(admin.health().await.is_ok());
}

#[tokio::test]
async fn test_timeout() {
    let server = server(
        health().respond_with(
            ResponseTemplate::new(200)
                .set_body_string(UP)
                .set_delay(Duration::from_secs(5)),
        ),
    )
    .await;
    let admin = Admin::builder(server.uri())
        .timeout(Duration::from_millis(100))
        .build()
        .unwrap();
    match admin.health().await {
        Err(AdminError::Request(e)) => assert!(e.is_timeout()),
        other => panic!("expected a timeout, got {other:?}"),
    }
}

#[tokio::test]
async fn test_bearer_token_is_sent() {
    let server = server(
        health()
            .and(header("authorization", "Bearer secret"))
            .respond_with(ResponseTemplate::new(200).set_body_string(UP)),
    )
    .await;
    let admin = Admin::builder(server.uri())
        .bearer_token("secret")
        .build()
        .unwrap();
    assert!(admin.health().await.is_ok());
}

#[test]
fn test_bearer_token_is_hidden_from_debug_output() {
    let builder = Admin::builder("http://localhost").bearer_token("secret");
    assert!(!format!("{builder:?}").contains("secret"));
}

#[test]
fn test_invalid_token_is_reported_when_building() {
    let result = Admin::builder("http://localhost")
        .bearer_token("line\nbreak")
        .build();
    assert!(matches!(result, Err(BuildError::InvalidHeader(_))));
}

#[test]
fn test_invalid_identity_is_reported_when_building() {
    let result = Admin::builder("http://localhost")
        .client_pem(b"not a cert", b"not a key")
        .build();
    assert!(matches!(result, Err(BuildError::Identity(_))));
}

#[test]
fn test_the_first_error_is_the_one_reported() {
    let result = Admin::builder("not a url")
        .client_pem(b"not a cert", b"not a key")
        .bearer_token("line\nbreak")
        .build();
    assert!(matches!(result, Err(BuildError::Identity(_))));
}
