mod request;
mod response;

pub use request::*;
pub use response::*; // TODO: hide behind feature

use std::time::Duration;

use reqwest::header::{ACCEPT, CONTENT_TYPE};
use serde::{Serialize, de::DeserializeOwned};

use crate::{Format, Vespa, error::SearchError, format::encode_request};

/// Options for a single search call, see [`Vespa::search_with`].
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct SearchOptions {
    /// Overrides the client timeout for this call only.
    pub timeout: Option<Duration>,
    /// Overrides the format used to read the response for this call only.
    pub format: Option<Format>,
}

impl SearchOptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Overrides the client timeout for this call only.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Overrides the format used to read the response for this call only.
    pub fn format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
    }
}

impl Vespa {
    /// Runs a search request with the specified body and returns a results
    /// with the modelled response or the search error.
    ///
    /// # Request
    ///
    /// Can be any serializable object, always sent as JSON. This means that you can use the
    /// helping `search::Request` struct, a `json!` invokation or use a custom struct
    /// implementating Serialize.
    ///
    /// # Response
    ///
    /// Must be deserializable representation of the documents returned in the
    /// response. The rest of fields are already implemented in the the `search::Response`
    /// model. In case that you want a simple object you can just use a `json::Value` or
    /// any other generic type.
    ///
    /// # Errors
    ///
    /// Any of `nespra::error::SearchError`. A non-successful HTTP status is reported as
    /// `SearchError::Vespa` when the body carries Vespa errors, and as
    /// `SearchError::Status` with the raw body otherwise.
    pub async fn search<B: Serialize + ?Sized, H: DeserializeOwned>(
        &self,
        body: &B,
    ) -> Result<Response<H>, SearchError> {
        self.search_with(body, &SearchOptions::default()).await
    }

    /// Same as [`Vespa::search`] with options that apply to this call only.
    pub async fn search_with<B: Serialize + ?Sized, H: DeserializeOwned>(
        &self,
        body: &B,
        options: &SearchOptions,
    ) -> Result<Response<H>, SearchError> {
        let format = options.format.unwrap_or(self.format);
        let mut request = self
            .client
            .post(&self.search_url)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, format.mime())
            .body(encode_request(body)?);

        if let Some(timeout) = options.timeout {
            request = request.timeout(timeout);
        }

        let response = request.send().await.map_err(SearchError::Request)?;

        let status = response.status();
        let bytes = response.bytes().await.map_err(SearchError::Response)?;

        let response: Response<H> = match format.decode(&bytes) {
            Ok(response) => response,
            Err(_) if !status.is_success() => {
                return Err(SearchError::Status {
                    status,
                    body: bytes.to_vec(),
                });
            }
            Err(e) => return Err(e.into()),
        };

        if !response.root.errors.is_empty() {
            return Err(SearchError::Vespa(response.root.errors));
        }

        if status.is_success() {
            Ok(response)
        } else {
            Err(SearchError::Status {
                status,
                body: bytes.to_vec(),
            })
        }
    }
}
