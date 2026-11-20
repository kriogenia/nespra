//! Unofficial async client for the operational APIs of [Vespa](https://vespa.ai).
//!
//! This is an independent community project, not affiliated with or endorsed by Vespa.ai.
#![cfg_attr(docsrs, feature(doc_cfg))]

use reqwest::{Client, Url};

pub mod error;

use crate::error::BuildError;

/// The HTTP client library used by nespra_admin. Re-exported so the types accepted by the
/// builders are always the ones of the version it uses.
pub use reqwest;

/// Client of the operational APIs of a Vespa node, such as a container.
#[derive(Debug, Clone)]
pub struct Admin {
    #[expect(dead_code, reason = "used by the endpoints")]
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

    /// Provides a builder to create a tailored client.
    pub fn builder(url: impl Into<String>) -> AdminBuilder {
        AdminBuilder::new(url)
    }

    /// Base URL of the node.
    pub fn url(&self) -> &Url {
        &self.url
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

/// Builder of [`Admin`] clients.
#[derive(Debug)]
pub struct AdminBuilder {
    url: String,
}

impl AdminBuilder {
    /// Creates a new builder for a client pointing to the specified URL, same as
    /// [`Admin::builder`].
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }

    /// Builds the client.
    ///
    /// # Errors
    ///
    /// Fails if the URL is not valid or the HTTP client cannot be built.
    pub fn build(self) -> Result<Admin, BuildError> {
        let url = base_url(self.url)?;
        let client = Client::builder().build().map_err(BuildError::Client)?;
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
}
