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

    /// Error encoding the request body or decoding the response body.
    #[error(transparent)]
    Codec(#[from] CodecError),

    /// Errors returned inside the Vespa response.
    /// Commonly related to some error in Vespa query, schemas or Searchers.
    #[error("Errors in Vespa search response, first error: {} - {}", .0[0].code, .0[0].summary)]
    Vespa(Vec<crate::search::Error>),
}

/// Errors encoding request bodies or decoding response bodies.
#[derive(Error, Debug)]
pub enum CodecError {
    /// The request body could not be serialized.
    #[error("Failed to encode the request: {0}")]
    Encode(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// The response body could not be deserialized. When using CBOR a common
    /// cause is a Vespa version older than 8.623.5 answering with JSON.
    #[error("Failed to decode the response: {0}")]
    Decode(#[source] Box<dyn std::error::Error + Send + Sync>),
}
