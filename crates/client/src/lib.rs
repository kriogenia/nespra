//! Unofficial async client for [Vespa](https://vespa.ai).
//!
//! This is an independent community project, not affiliated with or endorsed by Vespa.ai.

use reqwest::{Client, ClientBuilder, Identity};

pub mod error;
mod format;
pub mod search;

pub use format::Format;

use crate::error::BuildingError;

pub struct Vespa {
    client: Client,
    search_url: String,

    /// Application format to read Vespa responses
    pub format: Format,
}

impl Vespa {
    pub fn new(url: String) -> Self {
        Vespa::builder()
            .url(url)
            .build()
            .expect("default client to always be valid")
    }

    pub fn with_client(url: String, client: Client) -> Self {
        let mut vespa = Vespa::new(url);
        vespa.client = client;
        vespa
    }

    pub fn builder() -> VespaBuilder {
        VespaBuilder::default()
    }
}

pub struct VespaBuilder {
    client: ClientBuilder,
    url: Option<String>,
    format: Format,
}

impl VespaBuilder {
    pub fn url(mut self, mut url: String) -> Self {
        while url.ends_with("/") {
            url.remove(url.len() - 1);
        }
        self.url = Some(url);
        self
    }

    pub fn identity(mut self, identity: Identity) -> Self {
        self.client = self.client.identity(identity);
        self
    }

    pub fn format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    pub fn build(self) -> Result<Vespa, BuildingError> {
        let client = self
            .client
            .build()
            .expect("clients built with the builder to be always correct");

        let url = self.url.ok_or(BuildingError::MissingUrl)?;
        let search_url = format!("{url}/search/");

        Ok(Vespa {
            client,
            search_url,
            format: self.format,
        })
    }
}

impl Default for VespaBuilder {
    fn default() -> Self {
        Self {
            client: Client::builder(),
            url: Default::default(),
            format: Default::default(),
        }
    }
}
