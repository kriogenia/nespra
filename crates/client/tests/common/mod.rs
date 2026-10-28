//! Scripted HTTP/1.1 server to test the client without a Vespa.
#![allow(dead_code)]

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use nespra::{HttpVersion, RetryPolicy, VespaBuilder};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

/// Response sent by the [`Mock`].
#[derive(Clone)]
pub struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    hang: bool,
}

impl Reply {
    pub fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.into(),
            hang: false,
        }
    }

    pub fn json(status: u16, body: &str) -> Self {
        Self::new(status, body).header("content-type", "application/json")
    }

    /// Valid search response without hits.
    pub fn ok() -> Self {
        Self::json(200, r#"{"root":{"id":"toplevel","relevance":1.0}}"#)
    }

    /// Response with a Vespa error code in the body.
    pub fn vespa_error(status: u16, code: i32) -> Self {
        Self::json(
            status,
            &format!(
                r#"{{"root":{{"relevance":0.0,"errors":[{{"code":{code},"summary":"failed"}}]}}}}"#
            ),
        )
    }

    /// Reads the request and never answers.
    pub fn hang() -> Self {
        Self {
            hang: true,
            ..Self::new(200, "")
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }
}

/// Request received by the [`Mock`].
#[derive(Debug, Clone)]
pub struct Received {
    /// Request line and headers, lowercased.
    pub head: String,
    pub body: Vec<u8>,
}

impl Received {
    pub fn has_header(&self, header: &str) -> bool {
        self.head
            .contains(&format!("\r\n{}", header.to_lowercase()))
    }
}

/// Server that answers the connections with the replies in order, repeating the last one.
pub struct Mock {
    pub url: String,
    received: Arc<Mutex<Vec<Received>>>,
}

impl Mock {
    pub async fn start(replies: Vec<Reply>) -> Mock {
        assert!(!replies.is_empty());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let received = Arc::new(Mutex::new(Vec::new()));

        let log = received.clone();
        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let reply = {
                    let mut log = log.lock().unwrap();
                    let reply = replies[log.len().min(replies.len() - 1)].clone();
                    log.push(Received {
                        head: String::new(),
                        body: Vec::new(),
                    });
                    reply
                };
                let log = log.clone();
                tokio::spawn(async move {
                    let received = read_request(&mut socket).await;
                    {
                        let mut log = log.lock().unwrap();
                        // The slot was reserved on accept, find the first still empty
                        let slot = log.iter_mut().find(|r| r.head.is_empty()).unwrap();
                        *slot = received;
                    }
                    if reply.hang {
                        tokio::time::sleep(Duration::from_secs(60)).await;
                        return;
                    }
                    let mut response = format!("HTTP/1.1 {} Mock\r\n", reply.status);
                    for (name, value) in &reply.headers {
                        response.push_str(&format!("{name}: {value}\r\n"));
                    }
                    response.push_str(&format!(
                        "connection: close\r\ncontent-length: {}\r\n\r\n",
                        reply.body.len()
                    ));
                    let mut bytes = response.into_bytes();
                    bytes.extend_from_slice(&reply.body);
                    let _ = socket.write_all(&bytes).await;
                });
            }
        });

        Mock { url, received }
    }

    pub fn received(&self) -> Vec<Received> {
        self.received.lock().unwrap().clone()
    }

    pub fn count(&self) -> usize {
        self.received.lock().unwrap().len()
    }

    /// Builder for HTTP/1.1 (the mock does not speak HTTP/2) without retries.
    pub fn builder(&self) -> VespaBuilder {
        nespra::Vespa::builder(self.url.clone())
            .http_version(HttpVersion::Http1)
            .retry(RetryPolicy::none())
    }
}

async fn read_request(socket: &mut tokio::net::TcpStream) -> Received {
    let mut data = Vec::new();
    let mut chunk = [0; 4096];
    let (head_end, length) = loop {
        let n = socket.read(&mut chunk).await.unwrap();
        data.extend_from_slice(&chunk[..n]);
        if let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&data[..end]).to_lowercase();
            let length = head
                .lines()
                .find_map(|l| l.strip_prefix("content-length: "))
                .map_or(0, |l| l.trim().parse().unwrap());
            break (end + 4, length);
        }
        assert!(n > 0, "connection closed before the request was complete");
    };
    while data.len() < head_end + length {
        let n = socket.read(&mut chunk).await.unwrap();
        assert!(n > 0, "connection closed before the body was complete");
        data.extend_from_slice(&chunk[..n]);
    }
    Received {
        head: format!(
            "{}\r\n",
            String::from_utf8_lossy(&data[..head_end - 4]).to_lowercase()
        ),
        body: data[head_end..head_end + length].to_vec(),
    }
}
