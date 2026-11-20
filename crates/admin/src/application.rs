//! Status of the application running in a Vespa container.

use crate::{Admin, error::AdminError};

impl Admin {
    /// Gets the status of the application deployed in the container from
    /// `/ApplicationStatus`.
    ///
    /// # Errors
    ///
    /// Any of [`AdminError`], when the container cannot be reached or the response is not
    /// an application status.
    pub async fn application_status(&self) -> Result<ApplicationStatus, AdminError> {
        self.get::<raw::ApplicationStatus>("ApplicationStatus")
            .await
            .map(ApplicationStatus::from)
    }
}

/// Status of the application deployed in a container.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ApplicationStatus {
    /// Vespa version running in the container.
    pub vespa_version: String,
    /// Name of the application.
    pub name: Option<String>,
    /// Config generation of the deployment, increased on each deployment.
    pub generation: Option<u64>,
    /// When the application was deployed, in milliseconds since the Unix epoch.
    pub deployed_at: Option<u64>,
}

impl From<raw::ApplicationStatus> for ApplicationStatus {
    fn from(status: raw::ApplicationStatus) -> Self {
        let raw::Application { vespa, meta } = status.application;
        let meta = meta.unwrap_or_default();
        ApplicationStatus {
            vespa_version: vespa.version,
            name: meta.name,
            generation: meta.generation,
            deployed_at: meta.timestamp,
        }
    }
}

/// Shape of the response, only with the fields of interest.
mod raw {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub(super) struct ApplicationStatus {
        pub application: Application,
    }

    #[derive(Deserialize)]
    pub(super) struct Application {
        pub vespa: Vespa,
        pub meta: Option<Meta>,
    }

    #[derive(Deserialize)]
    pub(super) struct Vespa {
        pub version: String,
    }

    #[derive(Default, Deserialize)]
    pub(super) struct Meta {
        pub name: Option<String>,
        pub generation: Option<u64>,
        pub timestamp: Option<u64>,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn decode(json: &str) -> ApplicationStatus {
        serde_json::from_str::<raw::ApplicationStatus>(json)
            .unwrap()
            .into()
    }

    #[test]
    fn test_decode_application_status() {
        let status = decode(
            r#"{
                "application": {
                    "vespa": {"version": "8.600.35"},
                    "meta": {
                        "name": "default", "user": "unknown", "path": "",
                        "generation": 3, "timestamp": 1700000000000,
                        "date": "Tue Nov 14", "checksum": "abc"
                    },
                    "user": {"version": ""}
                },
                "abstractComponents": [],
                "handlers": []
            }"#,
        );
        assert_eq!(
            status,
            ApplicationStatus {
                vespa_version: "8.600.35".to_string(),
                name: Some("default".to_string()),
                generation: Some(3),
                deployed_at: Some(1_700_000_000_000),
            }
        );
    }

    #[test]
    fn test_decode_application_status_without_meta() {
        let status = decode(r#"{"application": {"vespa": {"version": "8.1.0"}}}"#);
        assert_eq!(status.vespa_version, "8.1.0");
        assert_eq!(status.generation, None);
    }
}
