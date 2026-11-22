//! Vespa Cloud helpers, compatible with the files created by the
//! [Vespa CLI](https://docs.vespa.ai/en/vespa-cli.html).

use std::{
    env, fmt,
    path::{Path, PathBuf},
};

use super::config::CloudApplication;

/// Environment variable overriding the home directory of the Vespa CLI.
const CLI_HOME_ENV: &str = "VESPA_CLI_HOME";

/// Home directory of the Vespa CLI: `$VESPA_CLI_HOME`, or `~/.vespa` by default.
pub fn cli_home() -> Option<PathBuf> {
    match env::var_os(CLI_HOME_ENV) {
        Some(home) if !home.is_empty() => Some(PathBuf::from(home)),
        _ => env::home_dir().map(|home| home.join(".vespa")),
    }
}

/// Paths of an mTLS certificate and its private key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credentials {
    /// PEM certificate.
    pub cert: PathBuf,
    /// PEM private key.
    pub key: PathBuf,
}

impl CloudApplication {
    /// Data plane credentials created by `vespa auth cert` inside the Vespa CLI home.
    pub fn data_plane_credentials(&self, cli_home: &Path) -> Credentials {
        let dir = cli_home.join(self.to_string());
        Credentials {
            cert: dir.join("data-plane-public-cert.pem"),
            key: dir.join("data-plane-private-key.pem"),
        }
    }
}

/// Formats the application as the Vespa CLI does: `<tenant>.<application>.<instance>`.
impl fmt::Display for CloudApplication {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.tenant, self.application, self.instance)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn app() -> CloudApplication {
        CloudApplication {
            tenant: "acme".to_string(),
            application: "search".to_string(),
            instance: "prod".to_string(),
        }
    }

    #[test]
    fn test_display() {
        assert_eq!(app().to_string(), "acme.search.prod");
    }

    #[test]
    fn test_data_plane_credentials() {
        let credentials = app().data_plane_credentials(Path::new("/home/me/.vespa"));
        assert_eq!(
            credentials,
            Credentials {
                cert: "/home/me/.vespa/acme.search.prod/data-plane-public-cert.pem".into(),
                key: "/home/me/.vespa/acme.search.prod/data-plane-private-key.pem".into(),
            }
        );
    }
}
