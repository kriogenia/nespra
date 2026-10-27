//! Unofficial async client for [Vespa](https://vespa.ai).
//!
//! This is an independent community project, not affiliated with or endorsed by Vespa.ai.

use reqwest::{Client, ClientBuilder, Identity};

pub mod error;
mod format;
pub mod search;

pub use format::Format;

pub struct Vespa {
    client: Client,
    search_url: String,

    /// Application format to read Vespa responses
    pub format: Format,
}

impl Vespa {
    pub fn new(url: String) -> Self {
        Vespa::builder(url).build()
    }

    pub fn with_client(url: String, client: Client) -> Self {
        let mut vespa = Vespa::new(url);
        vespa.client = client;
        vespa
    }

    pub fn builder(url: String) -> VespaBuilder {
        VespaBuilder::new(url)
    }
}

pub struct VespaBuilder {
    url: String,
    client: ClientBuilder,
    format: Format,
}

impl VespaBuilder {
    pub fn new(mut url: String) -> Self {
        while url.ends_with("/") {
            url.remove(url.len() - 1);
        }

        Self {
            url,
            client: Client::builder(),
            format: Format::default(),
        }
    }

    pub fn identity(mut self, identity: Identity) -> Self {
        self.client = self.client.identity(identity);
        self
    }

    pub fn format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    /// Sets the HTTP version, HTTP/2 by default.
    pub fn http_version(mut self, http_version: HttpVersion) -> Self {
        self.client = match http_version {
            HttpVersion::Http1 => self.client.http1_only(),
            HttpVersion::Http2 if self.url.starts_with("http://") => {
                self.client.http2_prior_knowledge()
            }
            HttpVersion::Http2 => self.client,
        };
        self
    }

    pub fn build(self) -> Vespa {
        let client = self
            .client
            .build()
            .expect("clients built with the builder to be always correct");

        let search_url = format!("{}/search/", self.url);

        Vespa {
            client,
            search_url,
            format: self.format,
        }
    }
}

/// HTTP version used to talk with Vespa.
///
/// HTTP/2 is the default. Plain `http://` URLs use it with prior knowledge, as there is no
/// negotiation without TLS, so a server or proxy that only speaks HTTP/1.1 requires
/// [`HttpVersion::Http1`]. On `https://` URLs the version is negotiated with ALPN and
/// falls back to HTTP/1.1 on its own.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HttpVersion {
    /// HTTP/2, with prior knowledge on plain `http://` URLs and ALPN on `https://`.
    #[default]
    Http2,
    /// HTTP/1.1 only.
    Http1,
}
