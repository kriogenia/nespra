mod common;

use std::time::{Duration, Instant};

use common::{Mock, Reply};
use nespra::{
    Format, HttpVersion, Vespa,
    error::{CodecError, SearchError},
    search::{Response, SearchOptions},
};
use serde_json::{Value, json};

fn body() -> Value {
    json!({"yql": "select * from doc where true"})
}

async fn search(mock: &Mock) -> Result<Response<Value>, SearchError> {
    mock.builder().build().unwrap().search(&body()).await
}

#[tokio::test]
async fn test_sends_a_json_post_to_the_search_endpoint() {
    let mock = Mock::start(vec![Reply::ok()]).await;
    let vespa = mock.builder().format(Format::Json).build().unwrap();
    vespa.search::<_, Value>(&body()).await.unwrap();

    let request = &mock.received()[0];
    assert!(request.head.starts_with("post /search/ "));
    assert!(request.has_header("content-type: application/json"));
    assert_eq!(
        serde_json::from_slice::<Value>(&request.body).unwrap(),
        body()
    );
}

#[tokio::test]
async fn test_decodes_a_json_response() {
    let mock = Mock::start(vec![Reply::json(
        200,
        r#"{"root":{"relevance":1.0,"children":[{"id":"a"},{"id":"b"}],"fields":{"totalCount":2}}}"#,
    )])
    .await;
    let vespa = mock.builder().format(Format::Json).build().unwrap();

    let response: Response<Value> = vespa.search(&body()).await.unwrap();

    assert_eq!(response.hits().len(), 2);
    assert_eq!(response.total_count(), 2);
    assert!(mock.received()[0].has_header("accept: application/json"));
}

#[tokio::test]
async fn test_decodes_a_cbor_response_by_default() {
    let mut cbor = Vec::new();
    ciborium::into_writer(
        &json!({"root": {"relevance": 1.0, "children": [{"id": "a"}]}}),
        &mut cbor,
    )
    .unwrap();
    let mock = Mock::start(vec![
        Reply::new(200, cbor).header("content-type", "application/cbor"),
    ])
    .await;

    let response = search(&mock).await.unwrap();

    assert_eq!(response.hits().len(), 1);
    assert!(mock.received()[0].has_header("accept: application/cbor"));
}

#[tokio::test]
async fn test_format_can_be_overridden_per_call() {
    let mock = Mock::start(vec![Reply::ok()]).await;
    // The client defaults to CBOR, the mock answers JSON
    let vespa = mock.builder().build().unwrap();

    let default: Result<Response<Value>, _> = vespa.search(&body()).await;
    let overridden: Result<Response<Value>, _> = vespa
        .search_with(&body(), &SearchOptions::new().format(Format::Json))
        .await;

    assert!(matches!(
        default,
        Err(SearchError::Codec(CodecError::Decode(_)))
    ));
    assert!(overridden.is_ok());
    assert!(mock.received()[1].has_header("accept: application/json"));
}

#[tokio::test]
async fn test_vespa_errors_in_a_successful_status() {
    let mock = Mock::start(vec![
        Reply::vespa_error(200, 3).header("content-type", "application/json"),
    ])
    .await;
    let vespa = mock.builder().format(Format::Json).build().unwrap();

    let error = vespa.search::<_, Value>(&body()).await.unwrap_err();

    assert!(matches!(&error, SearchError::Vespa(errors) if errors[0].code == 3));
}

#[tokio::test]
async fn test_vespa_errors_in_an_error_status() {
    let mock = Mock::start(vec![Reply::vespa_error(400, 4)]).await;
    let vespa = mock.builder().format(Format::Json).build().unwrap();

    let error = vespa.search::<_, Value>(&body()).await.unwrap_err();

    assert!(matches!(&error, SearchError::Vespa(errors) if errors[0].code == 4));
}

#[tokio::test]
async fn test_error_status_without_a_vespa_body_keeps_the_raw_body() {
    let mock = Mock::start(vec![Reply::new(502, "<html>Bad Gateway</html>")]).await;

    let error = search(&mock).await.unwrap_err();

    match error {
        SearchError::Status { status, body } => {
            assert_eq!(status, 502);
            assert_eq!(body, b"<html>Bad Gateway</html>");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn test_error_status_with_a_valid_body_without_errors() {
    let mock = Mock::start(vec![Reply::json(503, r#"{"root":{"relevance":0.0}}"#)]).await;
    let vespa = mock.builder().format(Format::Json).build().unwrap();

    let error = vespa.search::<_, Value>(&body()).await.unwrap_err();

    assert!(matches!(error, SearchError::Status { status, .. } if status == 503));
}

#[tokio::test]
async fn test_success_with_an_undecodable_body_is_a_codec_error() {
    let mock = Mock::start(vec![Reply::new(200, "not a response")]).await;

    let error = search(&mock).await.unwrap_err();

    assert!(matches!(error, SearchError::Codec(CodecError::Decode(_))));
}

#[tokio::test]
async fn test_unreachable_server_is_a_request_error() {
    let vespa = Vespa::builder(format!("http://127.0.0.1:{}", free_port()))
        .http_version(HttpVersion::Http1)
        .build()
        .unwrap();

    let error = vespa.search::<_, Value>(&body()).await.unwrap_err();

    assert!(matches!(&error, SearchError::Request(e) if e.is_connect()));
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[tokio::test]
async fn test_client_timeout() {
    let mock = Mock::start(vec![Reply::hang()]).await;
    let vespa = mock
        .builder()
        .timeout(Duration::from_millis(200))
        .build()
        .unwrap();

    let start = Instant::now();
    let error = vespa.search::<_, Value>(&body()).await.unwrap_err();

    assert!(matches!(&error, SearchError::Request(e) if e.is_timeout()));
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn test_timeout_can_be_overridden_per_call() {
    let mock = Mock::start(vec![Reply::hang()]).await;
    // The default timeout is 30 seconds
    let vespa = mock.builder().build().unwrap();

    let start = Instant::now();
    let error = vespa
        .search_with::<_, Value>(
            &body(),
            &SearchOptions::new().timeout(Duration::from_millis(200)),
        )
        .await
        .unwrap_err();

    assert!(matches!(&error, SearchError::Request(e) if e.is_timeout()));
    assert!(start.elapsed() < Duration::from_secs(5));
}
