use thiserror::Error;

/// Errors that can during API requests
#[derive(Error, Debug)]
pub enum HttpError {
    /// Error triggered while building and sending the HTTP request.
    /// Some common errors bundled on this one are:
    /// - Connection error
    /// - Invalid certificates
    /// - Failure while building the request body...
    #[error("{0}")]
    Request(reqwest::Error),

    /// Error triggered while handling the HTTP response.
    /// Usually a problem while deserializing the resposne.
    #[error("{0}")]
    Response(reqwest::Error),
}
