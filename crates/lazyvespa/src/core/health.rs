//! Health checks of the targets.

use std::time::{Duration, Instant};

use jiff::Timestamp;

use nespra::admin::{Admin, application::ApplicationStatus, error::AdminError, state::HealthCode};

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

/// Deployment of the application running in the target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Deployment {
    /// Vespa version running in the target.
    pub vespa_version: String,
    /// Name of the application.
    pub application: Option<String>,
    /// Config generation, increased on each deployment.
    pub generation: Option<u64>,
    /// When the application was deployed, `None` if it is not reported or not valid.
    pub deployed_at: Option<Timestamp>,
}

impl From<ApplicationStatus> for Deployment {
    fn from(status: ApplicationStatus) -> Self {
        Deployment {
            vespa_version: status.vespa_version,
            application: status.name,
            generation: status.generation,
            deployed_at: status
                .deployed_at
                .and_then(|millis| i64::try_from(millis).ok())
                .and_then(|millis| Timestamp::from_millisecond(millis).ok()),
        }
    }
}

/// Gets the deployment of the application running in the target.
///
/// # Errors
///
/// Same as [`Admin::application_status`].
pub async fn deployment(admin: &Admin) -> Result<Deployment, AdminError> {
    admin.application_status().await.map(Deployment::from)
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;

    async fn admin(endpoint: &str, response: ResponseTemplate) -> (MockServer, Admin) {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(endpoint))
            .respond_with(response)
            .mount(&server)
            .await;
        let admin = Admin::new(server.uri()).unwrap();
        (server, admin)
    }

    async fn check_answer(response: ResponseTemplate) -> Result<HealthCheck, AdminError> {
        let (_server, admin) = admin("/state/v1/health", response).await;
        check(&admin).await
    }

    #[tokio::test]
    async fn test_deployment() {
        let response = ResponseTemplate::new(200).set_body_json(json!({
            "application": {
                "vespa": {"version": "8.512.1"},
                "meta": {"name": "default", "generation": 2, "timestamp": 1795627162436_u64}
            }
        }));
        let (_server, admin) = admin("/ApplicationStatus", response).await;
        assert_eq!(
            deployment(&admin).await.unwrap(),
            Deployment {
                vespa_version: "8.512.1".to_string(),
                application: Some("default".to_string()),
                generation: Some(2),
                deployed_at: Some("2026-11-25T17:19:22.436Z".parse().unwrap()),
            }
        );
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
