//! Errors of the client.

use thiserror::Error;

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
