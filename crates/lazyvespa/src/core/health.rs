//! Health checks of the targets.

use std::time::{Duration, Instant};

use nespra::admin::{Admin, error::AdminError, state::HealthCode};

/// Health reported by a target, with how long it took to answer.
#[derive(Debug, Clone, PartialEq)]
pub struct HealthCheck {
    /// Whether the target is up.
    pub code: HealthCode,
    /// Explanation of the status, usually present when the target is not up.
    pub message: Option<String>,
    /// Time from sending the request to reading the whole response.
    pub elapsed: Duration,
}

/// Checks the health of the target.
///
/// # Errors
///
/// Same as [`Admin::health`].
pub async fn check(admin: &Admin) -> Result<HealthCheck, AdminError> {
    let start = Instant::now();
    let health = admin.health().await?;
    Ok(HealthCheck {
        code: health.status.code,
        message: health.status.message,
        elapsed: start.elapsed(),
    })
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;

    async fn check_answer(response: ResponseTemplate) -> Result<HealthCheck, AdminError> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/state/v1/health"))
            .respond_with(response)
            .mount(&server)
            .await;
        check(&Admin::new(server.uri()).unwrap()).await
    }

    #[tokio::test]
    async fn test_up() {
        let response = ResponseTemplate::new(200).set_body_json(json!({"status": {"code": "up"}}));
        let health = check_answer(response).await.unwrap();
        assert_eq!(health.code, HealthCode::Up);
        assert_eq!(health.message, None);
    }

    #[tokio::test]
    async fn test_down() {
        let response = ResponseTemplate::new(503)
            .set_body_json(json!({"status": {"code": "down", "message": "No search backends"}}));
        let health = check_answer(response).await.unwrap();
        assert_eq!(health.code, HealthCode::Down);
        assert_eq!(health.message.as_deref(), Some("No search backends"));
    }

    #[tokio::test]
    async fn test_not_a_health_report() {
        let response = ResponseTemplate::new(404).set_body_string("Not found");
        assert!(matches!(
            check_answer(response).await,
            Err(AdminError::Status { .. })
        ));
    }
}
