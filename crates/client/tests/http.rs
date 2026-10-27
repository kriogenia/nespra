use nespra::{HttpVersion, Vespa};
use reqwest::Client;
use serde_json::json;
use tokio::{io::AsyncReadExt, net::TcpListener};

const H2_PREFACE: &[u8] = b"PRI * HTTP/2.0";
const H1_REQUEST: &[u8] = b"POST /search/";

/// Sends a search to a server that only records the first bytes of the connection.
async fn first_bytes(build: impl FnOnce(String) -> Vespa) -> Vec<u8> {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = vec![0; H2_PREFACE.len()];
        socket.read_exact(&mut buffer).await.unwrap();
        buffer
    });

    // The server drops the connection, so the search is expected to fail
    let _ = build(url)
        .search::<_, serde_json::Value>(&json!({"yql": "select * from doc where true"}))
        .await;
    server.await.unwrap()
}

#[tokio::test]
async fn test_http2() {
    let bytes = first_bytes(|url| {
        Vespa::builder(url)
            .http_version(HttpVersion::Http2)
            .build()
            .unwrap()
    })
    .await;
    assert_eq!(bytes, H2_PREFACE);
}

#[tokio::test]
async fn test_http1() {
    let bytes = first_bytes(|url| {
        Vespa::builder(url)
            .http_version(HttpVersion::Http1)
            .build()
            .unwrap()
    })
    .await;
    assert!(bytes.starts_with(H1_REQUEST));
}

#[tokio::test]
async fn test_custom_client_uses_its_own_configuration() {
    let bytes = first_bytes(|url| Vespa::with_client(url, Client::new()).unwrap()).await;
    assert!(bytes.starts_with(H1_REQUEST));
}
