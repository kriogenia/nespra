use std::time::Duration;

/// Retry behaviour for failed requests.
///
/// Only transient failures are retried. Each attempt has its own timeout, and
/// the wait between attempts grows exponentially with full jitter.
/// A `Retry-After` header in seconds is honoured instead, up to `max_backoff`.
///
/// [`SearchError::is_retryable`]: crate::error::SearchError::is_retryable
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RetryPolicy {
    /// Total number of attempts, the first one included.
    pub max_attempts: u32,
    /// Upper bound of the wait before the first retry.
    pub initial_backoff: Duration,
    /// Cap of the wait between attempts.
    pub max_backoff: Duration,
}

impl RetryPolicy {
    /// Policy that makes up to `max_attempts` attempts with the default backoff.
    pub fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            ..Self::default()
        }
    }

    /// Policy that never retries.
    pub fn none() -> Self {
        Self::new(1)
    }

    /// Sets the upper bound of the wait before the first retry.
    pub fn initial_backoff(mut self, backoff: Duration) -> Self {
        self.initial_backoff = backoff;
        self
    }

    /// Sets the cap of the wait between attempts.
    pub fn max_backoff(mut self, backoff: Duration) -> Self {
        self.max_backoff = backoff;
        self
    }

    /// Wait before the next attempt, being `retry` the zero-based number of the retry.
    pub(crate) fn backoff(&self, retry: u32, retry_after: Option<Duration>) -> Duration {
        if let Some(retry_after) = retry_after {
            return retry_after.min(self.max_backoff);
        }
        let ceiling = self
            .initial_backoff
            .saturating_mul(2u32.saturating_pow(retry))
            .min(self.max_backoff);
        ceiling.mul_f64(fastrand::f64())
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(5),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_backoff_grows_and_is_capped() {
        let policy = RetryPolicy::default();
        for _ in 0..100 {
            assert!(policy.backoff(0, None) <= Duration::from_millis(100));
            assert!(policy.backoff(2, None) <= Duration::from_millis(400));
            assert!(policy.backoff(30, None) <= Duration::from_secs(5));
        }
    }

    #[test]
    fn test_retry_after_is_honoured_up_to_the_cap() {
        let policy = RetryPolicy::default();
        assert_eq!(
            policy.backoff(0, Some(Duration::from_secs(2))),
            Duration::from_secs(2)
        );
        assert_eq!(
            policy.backoff(0, Some(Duration::from_secs(60))),
            Duration::from_secs(5)
        );
    }

    #[test]
    fn test_none_and_minimum_attempts() {
        assert_eq!(RetryPolicy::none().max_attempts, 1);
        assert_eq!(RetryPolicy::new(0).max_attempts, 1);
    }
}
