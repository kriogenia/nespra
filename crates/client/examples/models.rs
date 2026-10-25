//! Equivalent to the `search` example but using the helper models to
//! built the request with additional fields.

use std::error::Error;

use serde::{Deserialize, Serialize};
use vespa_client::{Vespa, search::Request, search::Response};

#[derive(Deserialize, Serialize)]
struct Document {
    fields: Fields,
}

#[derive(Deserialize, Serialize)]
struct Fields {
    title: String,
    body: String,
}

#[derive(Default, Serialize)]
struct Extension<'a> {
    query: &'a str,
    ranking: &'a str,
    #[serde(rename = "presentation.timing")]
    timing: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let vespa = Vespa::new("http://localhost:8080".to_string());
    let response: Response<Document> = vespa
        .search(&Request::<Extension> {
            yql: "select title, body from doc where userQuery()",
            extra_fields: Extension {
                query: "Is statin use connected to breast cancer?",
                ranking: "bm25",
                timing: true,
            },
            ..Default::default()
        })
        .await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
