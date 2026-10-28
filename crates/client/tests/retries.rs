mod common;

use std::time::{Duration, Instant};

use common::{Mock, Reply};
use nespra::{Format, RetryPolicy, Vespa, error::SearchError, search::SearchOptions};
use serde_json::{Value, json};

fn vespa(mock: &Mock, attempts: u32) -> Vespa {
    mock.builder()
        .format(Format::Json)
        .retry(RetryPolicy::new(attempts).initial_backoff(Duration::from_millis(5)))
        .build()
        .unwrap()
}

async fn search(vespa: &Vespa) -> Result<(), SearchError> {
    vespa
        .search::<_, Value>(&json!({"yql": "select * from doc where true"}))
        .await
        .map(|_| ())
}

#[tokio::test]
async fn test_retries_transient_statuses_until_success() {
    let mock = Mock::start(vec![Reply::new(503, ""), Reply::new(429, ""), Reply::ok()]).await;

    search(&vespa(&mock, 3)).await.unwrap();

    assert_eq!(mock.count(), 3);
}

#[tokio::test]
async fn test_gives_up_after_the_max_attempts() {
    let mock = Mock::start(vec![Reply::new(503, "")]).await;

    let error = search(&vespa(&mock, 3)).await.unwrap_err();

    assert!(matches!(error, SearchError::Status { status, .. } if status == 503));
    assert_eq!(mock.count(), 3);
}

#[tokio::test]
async fn test_retries_transient_vespa_errors() {
    // 12 is the Vespa timeout
    let mock = Mock::start(vec![Reply::vespa_error(504, 12), Reply::ok()]).await;

    search(&vespa(&mock, 3)).await.unwrap();

    assert_eq!(mock.count(), 2);
}

#[tokio::test]
async fn test_does_not_retry_permanent_failures() {
    // 3 is an illegal query
    let mock = Mock::start(vec![Reply::vespa_error(400, 3)]).await;
    assert!(matches!(
        search(&vespa(&mock, 3)).await,
        Err(SearchError::Vespa(_))
    ));
    assert_eq!(mock.count(), 1);

    let mock = Mock::start(vec![Reply::new(404, "")]).await;
    assert!(matches!(
        search(&vespa(&mock, 3)).await,
        Err(SearchError::Status { .. })
    ));
    assert_eq!(mock.count(), 1);

    let mock = Mock::start(vec![Reply::new(200, "not a response")]).await;
    assert!(matches!(
        search(&vespa(&mock, 3)).await,
        Err(SearchError::Codec(_))
    ));
    assert_eq!(mock.count(), 1);
}

#[tokio::test]
async fn test_retries_can_be_disabled_per_call() {
    let mock = Mock::start(vec![Reply::new(503, "")]).await;
    let vespa = vespa(&mock, 3);

    let options = SearchOptions::new().retry(RetryPolicy::none());
    let result = vespa
        .search_with::<_, Value>(&json!({"yql": "x"}), &options)
        .await;

    assert!(result.is_err());
    assert_eq!(mock.count(), 1);
}

#[tokio::test]
async fn test_retry_policy_can_be_raised_per_call() {
    let mock = Mock::start(vec![Reply::new(503, ""), Reply::new(503, ""), Reply::ok()]).await;
    // The client does not retry, the call does
    let vespa = mock.builder().format(Format::Json).build().unwrap();

    let options =
        SearchOptions::new().retry(RetryPolicy::new(3).initial_backoff(Duration::from_millis(5)));
    vespa
        .search_with::<_, Value>(&json!({"yql": "x"}), &options)
        .await
        .unwrap();

    assert_eq!(mock.count(), 3);
}

#[tokio::test]
async fn test_retry_after_is_honoured() {
    let mock = Mock::start(vec![
        Reply::new(429, "").header("retry-after", "1"),
        Reply::ok(),
    ])
    .await;
    let vespa = mock
        .builder()
        .format(Format::Json)
        .retry(RetryPolicy::new(2).max_backoff(Duration::from_secs(2)))
        .build()
        .unwrap();

    let start = Instant::now();
    search(&vespa).await.unwrap();

    assert!(start.elapsed() >= Duration::from_millis(950));
}

#[tokio::test]
async fn test_timeouts_are_not_retried() {
    let mock = Mock::start(vec![Reply::hang(), Reply::ok()]).await;
    let vespa = mock
        .builder()
        .format(Format::Json)
        .timeout(Duration::from_millis(200))
        .retry(RetryPolicy::new(2).initial_backoff(Duration::from_millis(5)))
        .build()
        .unwrap();

    let error = search(&vespa).await.unwrap_err();

    assert!(matches!(&error, SearchError::Request(e) if e.is_timeout()));
    assert_eq!(mock.count(), 1);
}
