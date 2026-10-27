//! Unofficial async client for [Vespa](https://vespa.ai).
//!
//! This is an independent community project, not affiliated with or endorsed by Vespa.ai.

use std::time::Duration;

use http::{HeaderMap, HeaderName, HeaderValue, header::AUTHORIZATION};
use reqwest::{Client, ClientBuilder, Identity, Url};

use crate::error::BuildError;

pub mod error;
mod format;
pub mod search;

pub use format::Format;

/// The HTTP client library used by nespra. Re-exported so the `Client`, `Identity` and
/// other types accepted by the builders are always the ones of the version nespra uses.
pub use reqwest;

/// Default time to establish a connection.
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Default time for a whole request, from connecting to reading the full response.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct Vespa {
    client: Client,
    search_url: String,
    format: Format,
}

impl Vespa {
    /// Creates a client with the default configuration.
    ///
    /// # Errors
    ///
    /// Fails if the URL is not a valid `http` or `https` URL.
    pub fn new(url: impl Into<String>) -> Result<Self, BuildError> {
        Vespa::builder(url).build()
    }

    /// Creates a client that sends the requests with the provided HTTP client, which
    /// keeps its own configuration (timeouts, identity, headers, HTTP version...).
    ///
    /// # Errors
    ///
    /// Fails if the URL is not a valid `http` or `https` URL.
    pub fn with_client(url: impl Into<String>, client: Client) -> Result<Self, BuildError> {
        Ok(Vespa {
            client,
            search_url: search_url(url.into())?,
            format: Format::default(),
        })
    }

    pub fn builder(url: impl Into<String>) -> VespaBuilder {
        VespaBuilder::new(url)
    }
}

/// Validates the base URL and returns the one of the search endpoint.
fn search_url(url: String) -> Result<String, BuildError> {
    let invalid = |reason: &str| BuildError::InvalidUrl {
        url: url.clone(),
        reason: reason.to_string(),
    };
    let parsed = Url::parse(&url).map_err(|e| invalid(&e.to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(invalid("the scheme must be http or https"));
    }
    if parsed.host_str().is_none() {
        return Err(invalid("the URL has no host"));
    }
    Ok(format!("{}/search/", url.trim_end_matches('/')))
}

/// Builder of [`Vespa`] clients. The errors of its methods are reported by [`build`](Self::build).
#[derive(Debug)]
pub struct VespaBuilder {
    url: String,
    error: Option<BuildError>,
    client: ClientBuilder,
    format: Format,
    headers: HeaderMap,
    user_agent: HeaderValue,
    connect_timeout: Duration,
    timeout: Duration,
}

impl VespaBuilder {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            error: None,
            client: Client::builder(),
            format: Format::default(),
            headers: HeaderMap::new(),
            user_agent: HeaderValue::from_static(concat!("nespra/", env!("CARGO_PKG_VERSION"))),
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Keeps the first error to report it when building.
    fn record(&mut self, error: impl Into<BuildError>) {
        self.error.get_or_insert(error.into());
    }

    pub fn identity(mut self, identity: Identity) -> Self {
        self.client = self.client.identity(identity);
        self
    }

    /// Sets the mTLS identity from PEM encoded certificate and private key, for example
    /// when they are read from environment variables instead of files.
    ///
    pub fn client_pem(mut self, cert: &[u8], key: &[u8]) -> Self {
        let mut pem = Vec::with_capacity(cert.len() + key.len() + 1);
        pem.extend_from_slice(cert);
        if !cert.ends_with(b"\n") {
            pem.push(b'\n');
        }
        pem.extend_from_slice(key);
        match Identity::from_pem(&pem) {
            Ok(identity) => self.client = self.client.identity(identity),
            Err(e) => self.record(BuildError::Identity(e)),
        }
        self
    }

    /// Authenticates every request with a bearer token, such as a Vespa Cloud data plane
    /// token. The header is marked as sensitive so it is left out of debug output.
    ///
    pub fn bearer_token(mut self, token: &str) -> Self {
        match HeaderValue::from_str(&format!("Bearer {token}")) {
            Ok(mut value) => {
                value.set_sensitive(true);
                self.headers.insert(AUTHORIZATION, value);
            }
            Err(e) => self.record(e),
        }
        self
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

    /// Builds the client.
    ///
    /// # Errors
    ///
    /// Fails with the first error found in the configuration, such as an invalid URL,
    /// token or identity.
    pub fn build(self) -> Result<Vespa, BuildError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        let search_url = search_url(self.url)?;

        let client = self
            .client
            .user_agent(self.user_agent)
            .default_headers(self.headers)
            .connect_timeout(self.connect_timeout)
            .timeout(self.timeout)
            .build()
            .map_err(BuildError::Client)?;

        Ok(Vespa {
            client,
            search_url,
            format: self.format,
        })
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
