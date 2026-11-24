//! Searches of the queries written by the user.

use std::time::{Duration, Instant};

use nespra::{
    Format, Vespa,
    error::SearchError,
    search::{Response, SearchOptions},
};
use serde_json::Value;

use super::query::QueryRequest;

/// Response of a search with how long it took.
#[derive(Debug)]
pub struct SearchOutcome {
    /// Response of Vespa, with the hits as they came.
    pub response: Response<Value>,
    /// Time from sending the request to reading the whole response.
    pub elapsed: Duration,
}

/// Runs the search. The response is requested in JSON so it is shown as Vespa wrote it.
///
/// # Errors
///
/// Same as [`Vespa::search`].
pub async fn search(vespa: &Vespa, request: &QueryRequest) -> Result<SearchOutcome, SearchError> {
    let options = SearchOptions::new().format(Format::Json);
    let start = Instant::now();
    let response = vespa.search_with(request, &options).await?;
    Ok(SearchOutcome {
        response,
        elapsed: start.elapsed(),
    })
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_json, header, method, path},
    };

    use super::*;
    use crate::core::query::QueryDraft;

    #[tokio::test]
    async fn test_search_in_json() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/search/"))
            .and(header("accept", "application/json"))
            .and(body_json(
                json!({"yql": "select * from doc where true", "hits": 1}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "root": {
                    "id": "toplevel",
                    "relevance": 1.0,
                    "fields": {"totalCount": 42},
                    "children": [{"id": "id:doc:doc::1", "relevance": 0.5}]
                }
            })))
            .mount(&server)
            .await;
        let vespa = Vespa::with_client(server.uri(), nespra::reqwest::Client::new()).unwrap();
        let request = QueryDraft::new("select * from doc where true")
            .param("hits", "1")
            .request()
            .unwrap();

        let outcome = search(&vespa, &request).await.unwrap();
        assert_eq!(outcome.response.total_count(), 42);
        assert_eq!(outcome.response.hits()[0]["id"], "id:doc:doc::1");
    }
}
