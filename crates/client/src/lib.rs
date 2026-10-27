//! Unofficial async client for [Vespa](https://vespa.ai).
//!
//! This is an independent community project, not affiliated with or endorsed by Vespa.ai.

use http::{
    HeaderMap, HeaderName, HeaderValue,
    header::{AUTHORIZATION, InvalidHeaderValue},
};
use std::time::Duration;

use reqwest::{Client, ClientBuilder, Identity};

pub mod error;
mod format;
pub mod search;

pub use format::Format;

/// Default time to establish a connection.
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Default time for a whole request, from connecting to reading the full response.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

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
    headers: HeaderMap,
    user_agent: HeaderValue,
    connect_timeout: Duration,
    timeout: Duration,
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
            headers: HeaderMap::new(),
            user_agent: HeaderValue::from_static(concat!("nespra/", env!("CARGO_PKG_VERSION"))),
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn identity(mut self, identity: Identity) -> Self {
        self.client = self.client.identity(identity);
        self
    }

    /// Sets the mTLS identity from PEM encoded certificate and private key, for example
    /// when they are read from environment variables instead of files.
    ///
    /// # Errors
    ///
    /// Fails if the certificate or the key are not valid PEM.
    pub fn client_pem(mut self, cert: &[u8], key: &[u8]) -> Result<Self, reqwest::Error> {
        let mut pem = Vec::with_capacity(cert.len() + key.len() + 1);
        pem.extend_from_slice(cert);
        if !cert.ends_with(b"\n") {
            pem.push(b'\n');
        }
        pem.extend_from_slice(key);
        self.client = self.client.identity(Identity::from_pem(&pem)?);
        Ok(self)
    }

    /// Authenticates every request with a bearer token, such as a Vespa Cloud data plane
    /// token. The header is marked as sensitive so it is left out of debug output.
    ///
    /// # Errors
    ///
    /// Fails if the token contains characters that are not valid in a header value.
    pub fn bearer_token(mut self, token: &str) -> Result<Self, InvalidHeaderValue> {
        let mut value = HeaderValue::from_str(&format!("Bearer {token}"))?;
        value.set_sensitive(true);
        self.headers.insert(AUTHORIZATION, value);
        Ok(self)
    }

    /// Adds a header sent with every request, replacing any previous value for that name.
    pub fn header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Adds the headers sent with every request, replacing any previous value for the same names.
    pub fn default_headers(mut self, headers: HeaderMap) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Sets the `User-Agent` of the requests, `nespra/<version>` by default.
    pub fn user_agent(mut self, user_agent: HeaderValue) -> Self {
        self.user_agent = user_agent;
        self
    }

    /// Sets the time allowed to establish a connection, 10 seconds by default.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Sets the time allowed for a whole request, from connecting to reading the full
    /// response, 30 seconds by default. It can be overridden per request with
    /// the extended functions like [`Vespa::search_with`].
    ///
    /// This is independent of the `timeout` parameter of the search request, which is
    /// the time Vespa spends on the query. Keep this one above it.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
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
            .user_agent(self.user_agent)
            .default_headers(self.headers)
            .connect_timeout(self.connect_timeout)
            .timeout(self.timeout)
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
