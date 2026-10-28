mod common;

use common::{Mock, Reply};
use http::{HeaderMap, HeaderValue, header::USER_AGENT};
use nespra::{Format, Vespa, error::BuildError};
use reqwest::Client;
use serde_json::{Value, json};

async fn send(vespa: &Vespa) {
    let _ = vespa
        .search::<_, Value>(&json!({"yql": "select * from doc where true"}))
        .await;
}

fn json_builder(mock: &Mock) -> nespra::VespaBuilder {
    mock.builder().format(Format::Json)
}

#[tokio::test]
async fn test_default_user_agent() {
    let mock = Mock::start(vec![Reply::ok()]).await;
    send(&json_builder(&mock).build().unwrap()).await;

    let agent = format!("user-agent: nespra/{}", env!("CARGO_PKG_VERSION"));
    assert!(mock.received()[0].has_header(&agent));
}

#[tokio::test]
async fn test_user_agent_can_be_replaced() {
    let mock = Mock::start(vec![Reply::ok(), Reply::ok()]).await;

    let vespa = json_builder(&mock)
        .user_agent(HeaderValue::from_static("mine/1"))
        .build()
        .unwrap();
    send(&vespa).await;
    // A header set explicitly wins over the dedicated setter
    let vespa = json_builder(&mock)
        .user_agent(HeaderValue::from_static("mine/1"))
        .header(USER_AGENT, HeaderValue::from_static("header/2"))
        .build()
        .unwrap();
    send(&vespa).await;

    assert!(mock.received()[0].has_header("user-agent: mine/1"));
    assert!(mock.received()[1].has_header("user-agent: header/2"));
}

#[tokio::test]
async fn test_bearer_token_and_custom_headers() {
    let mock = Mock::start(vec![Reply::ok()]).await;
    let mut headers = HeaderMap::new();
    headers.insert("x-first", HeaderValue::from_static("1"));

    let vespa = json_builder(&mock)
        .bearer_token("secret")
        .default_headers(headers)
        .header("x-second".parse().unwrap(), HeaderValue::from_static("2"))
        .build()
        .unwrap();
    send(&vespa).await;

    let request = &mock.received()[0];
    assert!(request.has_header("authorization: bearer secret"));
    assert!(request.has_header("x-first: 1"));
    assert!(request.has_header("x-second: 2"));
}

#[test]
fn test_bearer_token_is_not_leaked_in_debug_output() {
    let builder = Vespa::builder("http://localhost:8080").bearer_token("secret");
    assert!(!format!("{builder:?}").contains("secret"));
}

#[tokio::test]
async fn test_with_client_uses_the_provided_client() {
    let mock = Mock::start(vec![Reply::ok()]).await;
    let client = Client::builder()
        .user_agent("custom-client/1")
        .build()
        .unwrap();

    let vespa = Vespa::with_client(mock.url.clone(), client).unwrap();
    send(&vespa).await;

    assert!(mock.received()[0].has_header("user-agent: custom-client/1"));
}

#[tokio::test]
async fn test_trailing_slashes_are_ignored() {
    let mock = Mock::start(vec![Reply::ok()]).await;

    let vespa = Vespa::builder(format!("{}//", mock.url))
        .http_version(nespra::HttpVersion::Http1)
        .format(Format::Json)
        .build()
        .unwrap();
    send(&vespa).await;

    assert!(mock.received()[0].head.starts_with("post /search/ "));
}

#[test]
fn test_invalid_urls_are_rejected() {
    for url in [
        "",
        "localhost:8080",
        "ftp://localhost",
        "http://",
        "not a url",
    ] {
        assert!(
            matches!(Vespa::new(url), Err(BuildError::InvalidUrl { .. })),
            "{url:?} was accepted"
        );
        assert!(matches!(
            Vespa::with_client(url, Client::new()),
            Err(BuildError::InvalidUrl { .. })
        ));
        assert!(matches!(
            Vespa::builder(url).build(),
            Err(BuildError::InvalidUrl { .. })
        ));
    }
}

#[test]
fn test_valid_urls_are_accepted() {
    for url in [
        "http://localhost:8080",
        "https://vespa.example.com/",
        "http://127.0.0.1",
    ] {
        assert!(Vespa::new(url).is_ok(), "{url:?} was rejected");
    }
}

#[test]
fn test_errors_of_the_setters_are_reported_when_building() {
    let invalid_token = Vespa::builder("http://localhost")
        .bearer_token("a\nb")
        .build();
    assert!(matches!(invalid_token, Err(BuildError::InvalidHeader(_))));

    let invalid_pem = Vespa::builder("http://localhost")
        .client_pem(b"not a certificate", b"not a key")
        .build();
    assert!(matches!(invalid_pem, Err(BuildError::Identity(_))));
}

#[test]
fn test_the_first_error_is_the_one_reported() {
    let result = Vespa::builder("http://localhost")
        .bearer_token("a\nb")
        .client_pem(b"not a certificate", b"not a key")
        .build();
    assert!(matches!(result, Err(BuildError::InvalidHeader(_))));

    // A configuration error wins over the URL validation
    let result = Vespa::builder("not a url").bearer_token("a\nb").build();
    assert!(matches!(result, Err(BuildError::InvalidHeader(_))));
}

#[cfg(feature = "gzip")]
#[tokio::test]
async fn test_asks_for_gzip_responses() {
    let mock = Mock::start(vec![Reply::ok()]).await;
    send(&json_builder(&mock).build().unwrap()).await;

    assert!(mock.received()[0].has_header("accept-encoding: gzip"));
}
