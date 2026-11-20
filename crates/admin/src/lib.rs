//! Unofficial async client for the operational APIs of [Vespa](https://vespa.ai).
//!
//! This is an independent community project, not affiliated with or endorsed by Vespa.ai.
//!
//! It complements [`nespra`](https://docs.rs/nespra), which covers the data APIs. Both
//! accept a [`reqwest::Client`], so one client can be shared between them with
//! [`Admin::with_client`].
#![cfg_attr(docsrs, feature(doc_cfg))]

use std::time::Duration;

use http::{
    HeaderMap, HeaderName, HeaderValue, StatusCode,
    header::{ACCEPT, AUTHORIZATION},
};
use reqwest::{Client, ClientBuilder, Identity, Url};
use serde::de::DeserializeOwned;

pub mod application;
pub mod error;
pub mod state;

use crate::error::{AdminError, BuildError};

/// The HTTP client library used by nespra_admin. Re-exported so the types accepted by the
/// builders are always the ones of the version it uses.
pub use reqwest;

/// Default time to establish a connection.
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Default time for a whole request, from connecting to reading the full response.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Client of the operational APIs of a Vespa node, such as a container.
#[derive(Debug, Clone)]
pub struct Admin {
    client: Client,
    url: Url,
}

impl Admin {
    /// Creates a client with the default configuration.
    ///
    /// # Errors
    ///
    /// Fails if the URL is not a valid `http` or `https` URL.
    pub fn new(url: impl Into<String>) -> Result<Self, BuildError> {
        Admin::builder(url).build()
    }

    /// Creates a client that sends the requests with the provided HTTP client, which
    /// keeps its own configuration (timeouts, identity, headers, HTTP version...).
    ///
    /// This allows to share one client, and so its connections and TLS configuration,
    /// with other clients like the ones of [`nespra`](https://docs.rs/nespra).
    ///
    /// # Errors
    ///
    /// Fails if the URL is not a valid `http` or `https` URL.
    pub fn with_client(url: impl Into<String>, client: Client) -> Result<Self, BuildError> {
        Ok(Admin {
            client,
            url: base_url(url.into())?,
        })
    }

    /// Provides a builder to create a tailored client.
    pub fn builder(url: impl Into<String>) -> AdminBuilder {
        AdminBuilder::new(url)
    }

    /// Base URL of the node.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Sends a `GET` to the path and decodes the JSON body.
    ///
    /// The body is decoded whatever the status, as some APIs (like the health one) answer
    /// with a valid body on errors. When it cannot be decoded, a non-successful status is
    /// reported as [`AdminError::Status`].
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, AdminError> {
        let url = self.url.join(path).map_err(|e| AdminError::Url {
            path: path.to_string(),
            reason: e.to_string(),
        })?;
        let response = self
            .client
            .get(url)
            .header(ACCEPT, "application/json")
            .send()
            .await
            .map_err(AdminError::Request)?;
        let status = response.status();
        let body = response.bytes().await.map_err(AdminError::Response)?;
        decode(status, &body)
    }
}

/// Decodes a JSON body, reporting the status when a failed response has no valid body.
fn decode<T: DeserializeOwned>(status: StatusCode, body: &[u8]) -> Result<T, AdminError> {
    match serde_json::from_slice(body) {
        Ok(value) => Ok(value),
        Err(_) if !status.is_success() => Err(AdminError::Status {
            status,
            body: body.to_vec(),
        }),
        Err(e) => Err(AdminError::Decode(e)),
    }
}

/// Validates the base URL, making sure it ends with `/` so paths are joined to it.
fn base_url(url: String) -> Result<Url, BuildError> {
    let invalid = |reason: &str| BuildError::InvalidUrl {
        url: url.clone(),
        reason: reason.to_string(),
    };
    let mut parsed = Url::parse(&url).map_err(|e| invalid(&e.to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(invalid("the scheme must be http or https"));
    }
    if parsed.host_str().is_none() {
        return Err(invalid("the URL has no host"));
    }
    if !parsed.path().ends_with('/') {
        let path = format!("{}/", parsed.path());
        parsed.set_path(&path);
    }
    Ok(parsed)
}

/// Builder of [`Admin`] clients. The errors of its methods are reported by [`build`](Self::build).
#[derive(Debug)]
pub struct AdminBuilder {
    url: String,
    error: Option<BuildError>,
    client: ClientBuilder,
    headers: HeaderMap,
    user_agent: HeaderValue,
    connect_timeout: Duration,
    timeout: Duration,
}

impl AdminBuilder {
    /// Creates a new builder for a client pointing to the specified URL, same as
    /// [`Admin::builder`].
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            error: None,
            client: Client::builder(),
            headers: HeaderMap::new(),
            user_agent: HeaderValue::from_static(concat!(
                "nespra_admin/",
                env!("CARGO_PKG_VERSION")
            )),
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Keeps the first error to report it when building.
    fn record(&mut self, error: impl Into<BuildError>) {
        self.error.get_or_insert(error.into());
    }

    /// Sets the identity using a custom [`reqwest::Identity`].
    pub fn identity(mut self, identity: Identity) -> Self {
        self.client = self.client.identity(identity);
        self
    }

    /// Sets the mTLS identity from PEM encoded certificate and private key, such as the
    /// data plane certificate of a Vespa Cloud application.
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

    /// Sets the `User-Agent` of the requests, `nespra_admin/<version>` by default.
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
    /// response, 10 seconds by default.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Builds the client.
    ///
    /// # Errors
    ///
    /// Fails with the first error found in the configuration, such as an invalid URL,
    /// token or identity.
    pub fn build(self) -> Result<Admin, BuildError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        let url = base_url(self.url)?;
        let client = self
            .client
            .user_agent(self.user_agent)
            .default_headers(self.headers)
            .connect_timeout(self.connect_timeout)
            .timeout(self.timeout)
            .build()
            .map_err(BuildError::Client)?;
        Ok(Admin { client, url })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_base_url_gets_a_trailing_slash() {
        let url = base_url("http://localhost:8080".to_string()).unwrap();
        assert_eq!(url.as_str(), "http://localhost:8080/");
        let url = base_url("https://host/prefix".to_string()).unwrap();
        assert_eq!(
            url.join("state/v1/health").unwrap().path(),
            "/prefix/state/v1/health"
        );
    }

    #[test]
    fn test_invalid_base_urls() {
        for url in ["localhost:8080", "ftp://host", "not a url", "http://"] {
            assert!(
                matches!(Admin::new(url), Err(BuildError::InvalidUrl { .. })),
                "{url} must be rejected"
            );
        }
    }

    #[test]
    fn test_decode_reports_the_status_of_an_undecodable_failure() {
        let error = decode::<serde_json::Value>(StatusCode::BAD_GATEWAY, b"<html>").unwrap_err();
        assert!(matches!(error, AdminError::Status { status, .. } if status == 502));
    }

    #[test]
    fn test_decode_prefers_a_valid_body_over_the_status() {
        let value: serde_json::Value = decode(StatusCode::SERVICE_UNAVAILABLE, b"{}").unwrap();
        assert!(value.is_object());
    }

    #[test]
    fn test_decode_error_on_success() {
        let error = decode::<serde_json::Value>(StatusCode::OK, b"nope").unwrap_err();
        assert!(matches!(error, AdminError::Decode(_)));
    }
}
