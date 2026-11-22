//! Configuration of lazyvespa: the Vespa deployments it can connect to.
//!
//! ```toml
//! default_target = "local"
//!
//! [targets.local]
//! endpoint = "http://localhost:8080"
//!
//! [targets.prod]
//! endpoint = "https://abcdef.z.vespa-app.cloud"
//! cloud = { tenant = "acme", application = "search" }
//! auth = { mtls = {} }
//! ```

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use serde::Deserialize;
use thiserror::Error;

/// Configuration of the application.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Name of the target used when none is requested.
    pub default_target: Option<String>,
    /// Vespa deployments, by name.
    #[serde(default)]
    pub targets: BTreeMap<String, Target>,
}

/// Default location of the configuration file, like `~/.config/lazyvespa/config.toml` on
/// Linux. `None` when the home directory cannot be found.
pub fn default_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "lazyvespa").map(|dirs| dirs.config_dir().join("config.toml"))
}

impl Config {
    /// Loads the configuration file.
    ///
    /// # Errors
    ///
    /// Fails if the file cannot be read or is not a valid configuration.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let toml = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Config::from_toml(&toml)
    }

    /// Loads the configuration file, or an empty configuration if it does not exist.
    ///
    /// # Errors
    ///
    /// Fails if the file exists but cannot be read or is not a valid configuration.
    pub fn load_or_default(path: &Path) -> Result<Self, ConfigError> {
        match Config::load(path) {
            Err(ConfigError::Read { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                Ok(Config::default())
            }
            result => result,
        }
    }

    /// Parses and validates a configuration in TOML.
    ///
    /// # Errors
    ///
    /// Fails if the TOML is not a valid configuration or the default target is not one of
    /// the targets.
    pub fn from_toml(toml: &str) -> Result<Self, ConfigError> {
        let config: Config = toml::from_str(toml)?;
        match &config.default_target {
            Some(name) if !config.targets.contains_key(name) => {
                Err(ConfigError::UnknownDefaultTarget(name.clone()))
            }
            _ => Ok(config),
        }
    }

    /// Target to use when none is requested: the default one, or the only one there is.
    pub fn default_target(&self) -> Option<(&str, &Target)> {
        match &self.default_target {
            Some(name) => self.target(name).map(|target| (name.as_str(), target)),
            None if self.targets.len() == 1 => self
                .targets
                .iter()
                .next()
                .map(|(name, target)| (name.as_str(), target)),
            None => None,
        }
    }

    /// Target with the given name.
    pub fn target(&self, name: &str) -> Option<&Target> {
        self.targets.get(name)
    }
}

/// A Vespa deployment lazyvespa can connect to.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// URL of the container endpoint, such as `http://localhost:8080`.
    pub endpoint: String,
    /// Vespa Cloud application of the deployment, `None` when it is self-hosted.
    pub cloud: Option<CloudApplication>,
    /// How to authenticate the requests.
    #[serde(default)]
    pub auth: Auth,
}

/// Instance of an application deployed on [Vespa Cloud](https://cloud.vespa.ai).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloudApplication {
    /// Tenant owning the application.
    pub tenant: String,
    /// Name of the application.
    pub application: String,
    /// Instance of the application, `default` if not set.
    #[serde(default = "default_instance")]
    pub instance: String,
}

fn default_instance() -> String {
    "default".to_string()
}

/// Authentication of the requests to a target.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Auth {
    /// No authentication.
    #[default]
    None,
    /// Mutual TLS with a client certificate. On Vespa Cloud the paths default to the data
    /// plane credentials created by the Vespa CLI.
    Mtls {
        /// Path of the PEM certificate.
        cert: Option<PathBuf>,
        /// Path of the PEM private key.
        key: Option<PathBuf>,
    },
    /// Bearer token, read from an environment variable so it is not stored in the file.
    Token {
        /// Name of the environment variable holding the token.
        env: String,
    },
}

/// Errors of an invalid [`Config`].
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The configuration file could not be read.
    #[error("Unable to read {}: {source}", path.display())]
    Read {
        /// Path of the file.
        path: PathBuf,
        /// Cause of the failure.
        source: io::Error,
    },
    /// The TOML is not valid or does not match the configuration format.
    #[error("Invalid configuration: {0}")]
    Parse(#[from] toml::de::Error),
    /// The default target is not one of the targets.
    #[error("The default target {0:?} is not defined in the targets")]
    UnknownDefaultTarget(String),
}

#[cfg(test)]
mod test {
    use std::io::Write;

    use super::*;

    #[test]
    fn test_load_a_file() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, "[targets.a]\nendpoint = \"http://a:8080\"").unwrap();
        let config = Config::load(file.path()).unwrap();
        assert!(config.target("a").is_some());
        assert_eq!(Config::load_or_default(file.path()).unwrap(), config);
    }

    #[test]
    fn test_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        assert!(matches!(Config::load(&path), Err(ConfigError::Read { .. })));
        assert_eq!(Config::load_or_default(&path).unwrap(), Config::default());
    }

    #[test]
    fn test_invalid_file_is_not_replaced_by_the_default() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, "targets = 1").unwrap();
        assert!(matches!(
            Config::load_or_default(file.path()),
            Err(ConfigError::Parse(_))
        ));
    }

    #[test]
    fn test_default_path() {
        if let Some(path) = default_path() {
            assert!(path.ends_with("config.toml"));
            assert!(path.parent().unwrap().ends_with("lazyvespa"));
        }
    }

    #[test]
    fn test_empty_config() {
        let config = Config::from_toml("").unwrap();
        assert_eq!(config, Config::default());
        assert!(config.default_target().is_none());
    }

    #[test]
    fn test_self_hosted_target_by_default() {
        let config = Config::from_toml(
            r#"
            [targets.local]
            endpoint = "http://localhost:8080"
            "#,
        )
        .unwrap();
        let target = config.target("local").unwrap();
        assert_eq!(target.endpoint, "http://localhost:8080");
        assert_eq!(target.cloud, None);
        assert_eq!(target.auth, Auth::None);
    }

    #[test]
    fn test_cloud_target() {
        let config = Config::from_toml(
            r#"
            [targets.prod]
            endpoint = "https://abc.z.vespa-app.cloud"
            cloud = { tenant = "acme", application = "search" }
            auth = { mtls = {} }
            "#,
        )
        .unwrap();
        let target = config.target("prod").unwrap();
        assert_eq!(
            target.cloud,
            Some(CloudApplication {
                tenant: "acme".to_string(),
                application: "search".to_string(),
                instance: "default".to_string(),
            })
        );
        assert_eq!(
            target.auth,
            Auth::Mtls {
                cert: None,
                key: None
            }
        );
    }

    #[test]
    fn test_auth_variants() {
        let config = Config::from_toml(
            r#"
            [targets.tls]
            endpoint = "https://vespa:8443"
            auth = { mtls = { cert = "/certs/client.pem", key = "/certs/client.key" } }

            [targets.token]
            endpoint = "https://vespa:8443"
            auth = { token = { env = "VESPA_TOKEN" } }
            "#,
        )
        .unwrap();
        assert_eq!(
            config.target("tls").unwrap().auth,
            Auth::Mtls {
                cert: Some("/certs/client.pem".into()),
                key: Some("/certs/client.key".into()),
            }
        );
        assert_eq!(
            config.target("token").unwrap().auth,
            Auth::Token {
                env: "VESPA_TOKEN".to_string()
            }
        );
    }

    #[test]
    fn test_default_target() {
        let config = Config::from_toml(
            r#"
            default_target = "b"
            [targets.a]
            endpoint = "http://a:8080"
            [targets.b]
            endpoint = "http://b:8080"
            "#,
        )
        .unwrap();
        assert_eq!(config.default_target().unwrap().0, "b");
    }

    #[test]
    fn test_the_only_target_is_the_default() {
        let config = Config::from_toml("[targets.a]\nendpoint = \"http://a:8080\"").unwrap();
        assert_eq!(config.default_target().unwrap().0, "a");
    }

    #[test]
    fn test_no_default_among_several_targets() {
        let config = Config::from_toml(
            "[targets.a]\nendpoint = \"http://a\"\n[targets.b]\nendpoint = \"http://b\"",
        )
        .unwrap();
        assert!(config.default_target().is_none());
    }

    #[test]
    fn test_unknown_default_target() {
        let error =
            Config::from_toml("default_target = \"nope\"\n[targets.a]\nendpoint = \"http://a\"")
                .unwrap_err();
        assert!(matches!(error, ConfigError::UnknownDefaultTarget(name) if name == "nope"));
    }

    #[test]
    fn test_invalid_configs() {
        for toml in [
            "[targets.a]",                                               // no endpoint
            "[targets.a]\nendpoint = \"x\"\ncloud = { tenant = \"t\" }", // cloud without app
            "[targets.a]\nendpoint = \"x\"\nkind = \"cloud\"",           // unknown target field
            "[targets.a]\nendpoint = \"x\"\nauth = { basic = {} }",      // unknown auth
            "unknown = true",                                            // unknown field
        ] {
            assert!(
                matches!(Config::from_toml(toml), Err(ConfigError::Parse(_))),
                "{toml} must be rejected"
            );
        }
    }
}
