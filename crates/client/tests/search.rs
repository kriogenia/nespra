use nespra::{Format, Vespa, search::Response};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize, Serialize)]
struct Document {
    fields: Fields,
}

#[derive(Deserialize, Serialize)]
struct Fields {
    title: String,
    body: String,
}

fn vespa() -> Vespa {
    Vespa::new("http://localhost:8080").unwrap()
}

#[ignore]
#[tokio::test]
async fn test_search_success() {
    search_success(vespa()).await;
}

#[ignore]
#[tokio::test]
async fn test_search_success_json() {
    let vespa = Vespa::builder("http://localhost:8080")
        .format(Format::Json)
        .build()
        .unwrap();

    search_success(vespa).await;
}

async fn search_success(vespa: Vespa) {
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

// TODO: test search response error
// TODO: test search vespa error
