//! Set of functions and models to query Vespa using the `/search` endpoint.

#[cfg(feature = "request-models")]
mod request;
mod response;

#[cfg(feature = "request-models")]
#[cfg_attr(docsrs, doc(cfg(feature = "request-models")))]
pub use request::*;
pub use response::*;

use std::time::Duration;

use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, RETRY_AFTER};
use serde::{Serialize, de::DeserializeOwned};

use crate::{Format, RetryPolicy, Vespa, error::SearchError, format::encode_request};

/// Options for a single search call, see [`Vespa::search_with`].
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct SearchOptions {
    /// Overrides the client timeout for this call only.
    pub timeout: Option<Duration>,
    /// Overrides the format used to read the response for this call only.
    pub format: Option<Format>,
    /// Overrides the retry policy for this call only.
    pub retry: Option<RetryPolicy>,
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

    /// Overrides the retry policy for this call only.
    pub fn retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = Some(retry);
        self
    }
}

impl Vespa {
    /// Runs a search request with the specified body and returns a results
    /// with the modelled response or the search error.
    ///
    /// Transient failures are retried following the retry policy, see
    /// [`SearchError::is_retryable`]. The timeout applies to each attempt.
    ///
    /// # Request
    ///
    /// Can be any serializable object, always sent as JSON. This means that you can use the
    /// helping `search::Request` struct, a `json!` invocation or use a custom struct
    /// implementing Serialize.
    ///
    /// # Response
    ///
    /// Must be deserializable representation of the documents returned in the
    /// response. The rest of fields are already implemented in the `search::Response`
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
        let body = encode_request(body)?;
        let policy = options.retry.unwrap_or(self.retry);

        let mut attempt = 1;
        loop {
            match self.search_once(&body, options).await {
                Err(failure) if attempt < policy.max_attempts && failure.error.is_retryable() => {
                    let wait = policy.backoff(attempt - 1, failure.retry_after);
                    tokio::time::sleep(wait).await;
                    attempt += 1;
                }
                result => return result.map_err(|failure| failure.error),
            }
        }
    }

    async fn search_once<H: DeserializeOwned>(
        &self,
        body: &[u8],
        options: &SearchOptions,
    ) -> Result<Response<H>, Failure> {
        let format = options.format.unwrap_or(self.format);
        let mut request = self
            .client
            .post(&self.search_url)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, format.mime())
            .body(body.to_vec());

        if let Some(timeout) = options.timeout {
            request = request.timeout(timeout);
        }

        let response = request.send().await.map_err(SearchError::Request)?;

        let status = response.status();
        let retry_after = retry_after(response.headers());
        let bytes = response.bytes().await.map_err(SearchError::Response)?;

        let failure = |error: SearchError| Failure { error, retry_after };

        let response: Response<H> = match format.decode(&bytes) {
            Ok(response) => response,
            Err(_) if !status.is_success() => {
                return Err(failure(SearchError::Status {
                    status,
                    body: bytes.to_vec(),
                }));
            }
            Err(e) => return Err(failure(e.into())),
        };

        if !response.root.errors.is_empty() {
            return Err(failure(SearchError::Vespa(response.root.errors)));
        }

        if status.is_success() {
            Ok(response)
        } else {
            Err(failure(SearchError::Status {
                status,
                body: bytes.to_vec(),
            }))
        }
    }
}

/// Failed attempt, with the wait requested by the server if any.
struct Failure {
    error: SearchError,
    retry_after: Option<Duration>,
}

impl From<SearchError> for Failure {
    fn from(error: SearchError) -> Self {
        Failure {
            error,
            retry_after: None,
        }
    }
}

/// Reads a `Retry-After` header expressed in seconds, the date form is ignored.
fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    headers
        .get(RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()
        .map(Duration::from_secs)
}

#[cfg(test)]
mod test {
    use super::*;

    fn headers(value: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, value.parse().unwrap());
        headers
    }

    #[test]
    fn test_retry_after_in_seconds() {
        assert_eq!(retry_after(&headers("3")), Some(Duration::from_secs(3)));
        assert_eq!(retry_after(&headers(" 10 ")), Some(Duration::from_secs(10)));
    }

    #[test]
    fn test_retry_after_ignores_other_forms() {
        assert_eq!(retry_after(&headers("Wed, 21 Oct 2026 07:28:00 GMT")), None);
        assert_eq!(retry_after(&headers("soon")), None);
        assert_eq!(retry_after(&HeaderMap::new()), None);
    }
}
