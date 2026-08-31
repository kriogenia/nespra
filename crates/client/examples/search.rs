use std::error::Error;

use serde::{Deserialize, Serialize};
use serde_json::json;
use vespa_client::{Vespa, search::Response};

#[derive(Deserialize, Serialize)]
struct Document {
    fields: Fields,
}

#[derive(Deserialize, Serialize)]
struct Fields {
    title: String,
    body: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let vespa = Vespa::new("http://localhost:8080".to_string());
    let response: Response<Document> = vespa
        .search(&json!({
                "yql": "select title, body from doc where userQuery()",
                "query": "Is statin use connected to breast cancer?",
                "ranking": "bm25",
                "presentation.timing": true,
                // "trace.level": 7,
        }))
        .await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
