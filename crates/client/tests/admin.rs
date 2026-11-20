//! The admin client re-exported with the `admin` feature shares the HTTP client.
#![cfg(feature = "admin")]

use nespra::{Vespa, admin::Admin, reqwest::Client};

#[test]
fn test_admin_and_search_clients_share_the_http_client() {
    let client = Client::new();
    assert!(Vespa::with_client("http://localhost:8080", client.clone()).is_ok());
    assert!(Admin::with_client("http://localhost:8080", client).is_ok());
}
