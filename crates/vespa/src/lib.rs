use reqwest::Client;

pub mod search;

pub struct Vespa {
    client: Client,
    search_url: String,
}

impl Vespa {
    pub fn new(mut url: String) -> Self {
        while url.ends_with("/") {
            url.remove(url.len() - 1);
        }

        let search_url = format!("{url}/search/");

        Self {
            client: Client::new(),
            search_url,
        }
    }
}
