//! [State API](https://docs.vespa.ai/en/reference/state-v1.html) of the Vespa nodes.

use std::fmt;

use serde::Deserialize;

use crate::{Admin, error::AdminError};

impl Admin {
    /// Gets the health of the node from `/state/v1/health`.
    ///
    /// A node that is not up can still answer with an error status and a valid body, which
    /// is returned as a [`Health`] with the corresponding [`HealthCode`].
    ///
    /// # Errors
    ///
    /// Any of [`AdminError`], when the node cannot be reached or the response is not
    /// a health report.
    pub async fn health(&self) -> Result<Health, AdminError> {
        self.get("state/v1/health").await
    }
}

/// Health report of a node.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct Health {
    /// Status of the node.
    pub status: HealthStatus,
}

/// Status of the node in a [`Health`] report.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct HealthStatus {
    /// Whether the node is up.
    pub code: HealthCode,
    /// Explanation of the status, usually present when the node is not up.
    #[serde(default)]
    pub message: Option<String>,
}

/// Health of a node.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(from = "String")]
#[non_exhaustive]
pub enum HealthCode {
    /// The node is up and serving.
    Up,
    /// The node is down or out of service.
    Down,
    /// The node is starting.
    Initializing,
    /// A code unknown to this version of the library.
    Unknown(String),
}

impl HealthCode {
    /// Whether the node is up.
    pub fn is_up(&self) -> bool {
        matches!(self, HealthCode::Up)
    }
}

impl From<String> for HealthCode {
    fn from(code: String) -> Self {
        match code.to_ascii_lowercase().as_str() {
            "up" => HealthCode::Up,
            "down" => HealthCode::Down,
            "initializing" => HealthCode::Initializing,
            _ => HealthCode::Unknown(code),
        }
    }
}

impl fmt::Display for HealthCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthCode::Up => f.write_str("up"),
            HealthCode::Down => f.write_str("down"),
            HealthCode::Initializing => f.write_str("initializing"),
            HealthCode::Unknown(code) => f.write_str(code),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_decode_health_up() {
        let health: Health = serde_json::from_str(
            r#"{"time":1700000000000,"status":{"code":"up"},"metrics":{"values":[]}}"#,
        )
        .unwrap();
        assert!(health.status.code.is_up());
        assert_eq!(health.status.message, None);
    }

    #[test]
    fn test_decode_health_down_with_message() {
        let health: Health =
            serde_json::from_str(r#"{"status":{"code":"down","message":"No search backends"}}"#)
                .unwrap();
        assert_eq!(health.status.code, HealthCode::Down);
        assert_eq!(health.status.message.as_deref(), Some("No search backends"));
    }

    #[test]
    fn test_health_codes() {
        assert_eq!(HealthCode::from("UP".to_string()), HealthCode::Up);
        assert_eq!(
            HealthCode::from("initializing".to_string()),
            HealthCode::Initializing
        );
        let unknown = HealthCode::from("degraded".to_string());
        assert_eq!(unknown, HealthCode::Unknown("degraded".to_string()));
        assert_eq!(unknown.to_string(), "degraded");
    }
}
