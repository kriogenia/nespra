mod common;

use std::time::Duration;

use common::{Mock, Reply};
use nespra::{Format, error::SearchError, search::SearchOptions};

const BODY: &[u8] = br#"{"yql":"select * from doc where true"}"#;

#[tokio::test]
async fn test_returns_the_body_verbatim() {
    let payload = [0xbf, 0x00, 0xff, 0x13];
    let mock = Mock::start(vec![Reply::new(200, payload)]).await;
    let vespa = mock.builder().build().unwrap();

    let response = vespa.search_raw(BODY).await.unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.format, Format::Cbor);
    assert_eq!(response.body.as_ref(), payload);

    let received = &mock.received()[0];
    assert_eq!(received.body, BODY);
    assert!(received.has_header("content-type: application/json"));
    assert!(received.has_header("accept: application/cbor"));
}

#[tokio::test]
async fn test_format_can_be_overridden_per_call() {
    let mock = Mock::start(vec![Reply::ok()]).await;
    let vespa = mock.builder().build().unwrap();

    let options = SearchOptions::new().format(Format::Json);
    let response = vespa.search_raw_with(BODY, &options).await.unwrap();

    assert_eq!(response.format, Format::Json);
    assert!(mock.received()[0].has_header("accept: application/json"));
}

#[tokio::test]
async fn test_failures_are_not_reported() {
    for (status, reply) in [
        (400, Reply::vespa_error(400, 3)),
        (503, Reply::new(503, "unavailable")),
    ] {
        let mock = Mock::start(vec![reply]).await;
        let vespa = mock.builder().build().unwrap();

        let response = vespa.search_raw(BODY).await.unwrap();

        assert_eq!(response.status, status);
        assert!(!response.body.is_empty());
    }
}

#[tokio::test]
async fn test_errors_in_a_successful_response_are_not_reported() {
    let mock = Mock::start(vec![Reply::json(
        200,
        r#"{"root":{"relevance":0.0,"errors":[{"code":12,"summary":"timeout"}]}}"#,
    )])
    .await;
    let vespa = mock.builder().build().unwrap();

    let response = vespa.search_raw(BODY).await.unwrap();

    assert_eq!(response.status, 200);
}

#[tokio::test]
async fn test_never_retries() {
    let mock = Mock::start(vec![Reply::new(503, ""), Reply::ok()]).await;
    let vespa = mock
        .builder()
        .retry(nespra::RetryPolicy::new(5))
        .build()
        .unwrap();

    let options = SearchOptions::new().retry(nespra::RetryPolicy::new(5));
    let response = vespa.search_raw_with(BODY, &options).await.unwrap();

    assert_eq!(response.status, 503);
    assert_eq!(mock.count(), 1);
}

#[tokio::test]
async fn test_timeout_per_call() {
    let mock = Mock::start(vec![Reply::hang()]).await;
    let vespa = mock.builder().build().unwrap();

    let options = SearchOptions::new().timeout(Duration::from_millis(100));
    let error = vespa.search_raw_with(BODY, &options).await.unwrap_err();

    assert!(matches!(error, SearchError::Request(e) if e.is_timeout()));
}

#[tokio::test]
async fn test_connection_failure_is_a_request_error() {
    let vespa = nespra::Vespa::new("http://127.0.0.1:1").unwrap();

    let error = vespa.search_raw(BODY).await.unwrap_err();

    assert!(matches!(error, SearchError::Request(_)));
}
