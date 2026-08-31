use reqwest::Client;

pub mod search;

pub struct Vespa {
    client: Client,
    search_url: String,
}

impl Vespa {
    pub fn with_client(mut url: String, client: Client) -> Self {
        while url.ends_with("/") {
            url.remove(url.len() - 1);
        }

        let search_url = format!("{url}/search/");

        Self { client, search_url }
    }

    pub fn new(url: String) -> Self {
        Vespa::with_client(url, Client::new())
    }
}
