use reqwest::Client;

pub mod error;
mod format;
pub mod search;

pub use format::Format;

pub struct Vespa {
    client: Client,
    search_url: String,
    format: Format,
}

impl Vespa {
    pub fn with_client(mut url: String, client: Client) -> Self {
        while url.ends_with("/") {
            url.remove(url.len() - 1);
        }

        let search_url = format!("{url}/search/");

        Self {
            client,
            search_url,
            format: Format::default(),
        }
    }

    pub fn new(url: String) -> Self {
        Vespa::with_client(url, Client::new())
    }

    /// Sets the format requested for the responses. Requests are always sent as JSON.
    pub fn format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }
}
