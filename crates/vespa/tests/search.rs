use serde::{Deserialize, Serialize};
use serde_json::json;
use vespa::{Vespa, search::Response};

#[derive(Deserialize, Serialize)]
struct Document {
    fields: Fields,
}

#[derive(Deserialize, Serialize)]
struct Fields {
    title: String,
    body: String,
}

#[ignore]
#[tokio::test]
async fn test_search_success() {
    let vespa = Vespa::new("http://localhost:8080".to_string());
    let response: Response<Document> = vespa
        .search(&json!({
                "yql": "select title, body from doc where userQuery()",
                "query": "Is statin use connected to breast cancer?",
                "ranking": "bm25",
                "presentation.timing": true,
                "trace.level": 1,
        }))
        .await
        .unwrap();

    assert!(!response.hits().is_empty());
    assert!(response.timing.is_some());
    assert!(response.trace.is_some());
}
