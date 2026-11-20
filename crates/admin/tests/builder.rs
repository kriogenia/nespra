use std::time::Duration;

use nespra_admin::{
    Admin,
    error::AdminError,
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
