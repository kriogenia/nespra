use nespra_admin::{Admin, error::AdminError, state::HealthCode};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

async fn server(response: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/state/v1/health"))
        .and(header("accept", "application/json"))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn test_health_up() {
    let server =
        server(ResponseTemplate::new(200).set_body_string(r#"{"status":{"code":"up"}}"#)).await;
    let health = Admin::new(server.uri()).unwrap().health().await.unwrap();
    assert!(health.status.code.is_up());
}

#[tokio::test]
async fn test_health_down_in_an_error_status() {
    let server = server(
        ResponseTemplate::new(503)
            .set_body_string(r#"{"status":{"code":"down","message":"out of service"}}"#),
    )
    .await;
    let health = Admin::new(server.uri()).unwrap().health().await.unwrap();
    assert_eq!(health.status.code, HealthCode::Down);
    assert_eq!(health.status.message.as_deref(), Some("out of service"));
}

#[tokio::test]
async fn test_error_status_without_a_valid_body() {
    let server = server(ResponseTemplate::new(403).set_body_string("Forbidden")).await;
    let error = Admin::new(server.uri())
        .unwrap()
        .health()
        .await
        .unwrap_err();
    match error {
        AdminError::Status { status, body } => {
            assert_eq!(status, 403);
            assert_eq!(body, b"Forbidden");
        }
        other => panic!("unexpected error {other:?}"),
    }
}

#[tokio::test]
async fn test_paths_are_joined_to_a_base_path() {
    let server = MockServer::start().await;
    Mock::given(path("/prefix/state/v1/health"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"status":{"code":"up"}}"#))
        .mount(&server)
        .await;
    let admin = Admin::new(format!("{}/prefix", server.uri())).unwrap();
    assert!(admin.health().await.unwrap().status.code.is_up());
}

#[tokio::test]
async fn test_unreachable_node_is_a_request_error() {
    let admin = Admin::new("http://127.0.0.1:1").unwrap();
    assert!(matches!(admin.health().await, Err(AdminError::Request(_))));
}
