use std::{error::Error, fs::File, io::Read};

use reqwest::{Client, Identity};
use serde_json::{Value, json};
use vespa::{Vespa, search::Response};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut identity = Vec::new();
    File::open("my-certs.pem")?.read_to_end(&mut identity)?;
    let identity = Identity::from_pem(&identity)?;
    let client = Client::builder().identity(identity).build()?;

    let vespa = Vespa::with_client("http://localhost:8080".to_string(), client);
    let response: Response<Value> = vespa
        .search(&json!({
                "yql": "select title, body from doc where userQuery()",
                "query": "Is statin use connected to breast cancer?",
                "ranking": "bm25",
        }))
        .await?;
    println!("{}", serde_json::to_string_pretty(&response)?);

    Ok(())
}
