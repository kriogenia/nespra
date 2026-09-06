use thiserror::Error;

/// Errors that can during API requests
#[derive(Error, Debug)]
pub enum SearchError {
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

    /// Errors returned inside the Vespa response.
    /// Commonly related to some error in Vespa query, schemas or Searchers.
    #[error("Errors in Vespa search response, first error: {} - {}", .0[0].code, .0[0].summary)]
    Vespa(Vec<crate::search::Error>),
}
