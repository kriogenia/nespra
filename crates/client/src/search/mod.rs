mod request;
mod response;

pub use request::*;
pub use response::*; // TODO: hide behind feature

use serde::{Serialize, de::DeserializeOwned};

use crate::{Vespa, error::SearchError};

impl Vespa {
    /// Runs a search request with the specified body and returns a results
    /// with the modelled response or the search error.
    ///
    /// # Request
    ///
    /// Can be any JSON serializable object. This means that you can use the
    /// helping `search::Request` struct, a string built `json!` invokation or
    /// use a custom struct implementating Serialize.
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
    /// Any of `vespa_client::error::SearchError`.
    pub async fn search<B: Serialize + ?Sized, H: DeserializeOwned>(
        &self,
        body: &B,
    ) -> Result<Response<H>, SearchError> {
        let response: Response<H> = self
            .client
            .post(&self.search_url)
            // TODO: .version(Version::HTTP_2)
            .json(body)
            .send()
            .await
            .map_err(SearchError::Request)?
            .json()
            .await
            .map_err(SearchError::Response)?;

        if response.root.errors.is_empty() {
            Ok(response)
        } else {
            Err(SearchError::Vespa(response.root.errors))
        }
    }
}
