use nespra_admin::Admin;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

#[tokio::test]
async fn test_application_status() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ApplicationStatus"))
        .and(header("accept", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"application":{"vespa":{"version":"8.600.35"},"meta":{"generation":7}}}"#,
        ))
        .mount(&server)
        .await;
    let status = Admin::new(server.uri())
        .unwrap()
        .application_status()
        .await
        .unwrap();
    assert_eq!(status.vespa_version, "8.600.35");
    assert_eq!(status.generation, Some(7));
    assert_eq!(status.name, None);
}
