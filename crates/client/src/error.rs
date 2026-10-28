use thiserror::Error;

/// Errors that can happen during API requests
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum SearchError {
    /// Error triggered while building and sending the HTTP request.
    /// Some common errors bundled on this one are:
    /// - Connection error
    /// - Invalid certificates
    /// - Failure while building the request body...
    #[error("{0}")]
    Request(reqwest::Error),

    /// Error triggered while handling the HTTP response.
    /// Usually a problem while deserializing the response.
    #[error("{0}")]
    Response(reqwest::Error),

    /// Error encoding the request body or decoding the response body.
    #[error(transparent)]
    Codec(#[from] CodecError),

    /// Non-successful HTTP status whose body is not a Vespa error response.
    /// Usually comes from a proxy, a load balancer or an overloaded container
    #[error("Unexpected HTTP status {status}")]
    Status {
        /// Status code of the response.
        status: http::StatusCode,
        /// Raw body of the response.
        body: Vec<u8>,
    },

    /// Errors returned inside the Vespa response.
    /// Commonly related to some error in Vespa query, schemas or Searchers.
    #[error("Errors in Vespa search response, first error: {} - {}", .0[0].code, .0[0].summary)]
    Vespa(Vec<crate::search::Error>),
}

/// Vespa error codes of failures that usually go away on their own:
/// `NO_BACKENDS_IN_SERVICE`, `BACKEND_COMMUNICATION_ERROR` and `TIMEOUT`.
const TRANSIENT_VESPA_CODES: [i32; 3] = [0, 10, 12];

impl SearchError {
    /// Whether repeating the search can succeed: connection failures, HTTP 429, 502, 503
    /// and 504, and Vespa errors about unavailable backends, backend communication
    /// or timeouts. Useful to implement a custom retry policy.
    pub fn is_retryable(&self) -> bool {
        match self {
            SearchError::Request(e) => !e.is_builder() && !e.is_timeout() && !e.is_redirect(),
            SearchError::Status { status, .. } => matches!(status.as_u16(), 429 | 502..=504),
            SearchError::Vespa(errors) => errors
                .iter()
                .any(|e| TRANSIENT_VESPA_CODES.contains(&e.code)),
            SearchError::Response(_) | SearchError::Codec(_) => false,
        }
    }
}

/// Errors encoding request bodies or decoding response bodies.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CodecError {
    /// The request body could not be serialized.
    #[error("Failed to encode the request: {0}")]
    Encode(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// The response body could not be deserialized. When using CBOR a common
    /// cause is a Vespa version older than 8.623.5 answering with JSON.
    #[error("Failed to decode the response: {0}")]
    Decode(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Errors building a [`Vespa`](crate::Vespa) client.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum BuildError {
    /// The URL is not a valid `http` or `https` URL with a host.
    #[error("Invalid URL {url:?}: {reason}")]
    InvalidUrl {
        /// The URL as provided.
        url: String,
        /// Why it was rejected.
        reason: String,
    },

    /// A header, or a token used to build one, is not a valid header value.
    #[error("Invalid header value: {0}")]
    InvalidHeader(#[from] http::header::InvalidHeaderValue),

    /// The certificate or the key of the client identity are not valid PEM.
    #[error("Invalid client identity: {0}")]
    Identity(#[source] reqwest::Error),

    /// The underlying HTTP client could not be built.
    #[error("Failed to build the HTTP client: {0}")]
    Client(#[source] reqwest::Error),
}

#[cfg(test)]
mod test {
    use http::StatusCode;

    use super::*;
    use crate::search;

    fn vespa_error(code: i32) -> SearchError {
        SearchError::Vespa(vec![search::Error {
            code,
            summary: "failed".to_string(),
            source: None,
            message: None,
            stack_trace: None,
        }])
    }

    fn status(status: StatusCode) -> SearchError {
        SearchError::Status {
            status,
            body: Vec::new(),
        }
    }

    #[test]
    fn test_retryable_statuses() {
        for code in [429, 502, 503, 504] {
            assert!(status(StatusCode::from_u16(code).unwrap()).is_retryable());
        }
        for code in [400, 401, 403, 404, 500, 501] {
            assert!(!status(StatusCode::from_u16(code).unwrap()).is_retryable());
        }
    }

    #[test]
    fn test_retryable_vespa_errors() {
        // NO_BACKENDS_IN_SERVICE, BACKEND_COMMUNICATION_ERROR and TIMEOUT
        for code in [0, 10, 12] {
            assert!(vespa_error(code).is_retryable());
        }
        // ILLEGAL_QUERY
        assert!(!vespa_error(3).is_retryable());
    }

    #[test]
    fn test_a_transient_error_among_others_is_retryable() {
        let SearchError::Vespa(mut errors) = vespa_error(3) else {
            unreachable!()
        };
        let SearchError::Vespa(transient) = vespa_error(12) else {
            unreachable!()
        };
        errors.extend(transient);
        assert!(SearchError::Vespa(errors).is_retryable());
    }

    #[test]
    fn test_codec_errors_are_not_retryable() {
        let error = CodecError::Decode("garbage".into());
        assert!(!SearchError::Codec(error).is_retryable());
    }
}
