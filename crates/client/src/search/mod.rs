mod request;
mod response;

pub use request::*;
pub use response::*; // TODO: hide behind feature

use reqwest::header::{ACCEPT, CONTENT_TYPE};
use serde::{Serialize, de::DeserializeOwned};

use crate::{Vespa, error::SearchError, format::encode_request};

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
        let response = self
            .client
            .post(&self.search_url)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, self.format.mime())
            .body(encode_request(body)?)
            .send()
            .await
            .map_err(SearchError::Request)?;

        let status = response.status();
        let bytes = response.bytes().await.map_err(SearchError::Response)?;

        let response: Response<H> = match self.format.decode(&bytes) {
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
