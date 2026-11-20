//! Errors of the client.

use thiserror::Error;

/// Errors of the requests to the operational APIs.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AdminError {
    /// The path of the API could not be joined to the base URL.
    #[error("Invalid URL for {path:?}: {reason}")]
    Url {
        /// Path of the API.
        path: String,
        /// Why it could not be joined.
        reason: String,
    },

    /// The request could not be sent, such as a connection or certificate error.
    #[error("{0}")]
    Request(#[source] reqwest::Error),

    /// The response could not be read.
    #[error("{0}")]
    Response(#[source] reqwest::Error),

    /// Non-successful HTTP status without a valid body.
    /// Usually comes from a proxy, a load balancer or a disabled API.
    #[error("Unexpected HTTP status {status}")]
    Status {
        /// Status code of the response.
        status: http::StatusCode,
        /// Raw body of the response.
        body: Vec<u8>,
    },

    /// A successful response whose body is not the expected JSON.
    #[error("Failed to decode the response: {0}")]
    Decode(#[source] serde_json::Error),
}

/// Errors building an [`Admin`](crate::Admin) client.
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

    /// The underlying HTTP client could not be built.
    #[error("Failed to build the HTTP client: {0}")]
    Client(#[source] reqwest::Error),
}
