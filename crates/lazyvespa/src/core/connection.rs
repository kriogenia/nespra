//! Clients to talk with a [`Target`].
//!
//! A single HTTP client is configured with the authentication of the target and shared by
//! the search and the admin clients, so they reuse connections and TLS sessions.

use std::{fs, io, path::Path, path::PathBuf, time::Duration};

use nespra::{
    Vespa,
    admin::Admin,
    reqwest::{
        Client, Identity,
        header::{AUTHORIZATION, HeaderMap, HeaderValue, InvalidHeaderValue},
    },
};
use thiserror::Error;

use super::{
    cloud::{self, Credentials},
    config::{Auth, Target},
};

const USER_AGENT: &str = concat!("lazyvespa/", env!("CARGO_PKG_VERSION"));
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const TIMEOUT: Duration = Duration::from_secs(30);

/// A named target with its clients, ready to be used.
#[derive(Debug, Clone)]
pub struct ActiveTarget {
    /// Name of the target.
    pub name: String,
    /// Configuration of the target.
    pub target: Target,
    /// Clients of the target.
    pub connection: Connection,
}

impl ActiveTarget {
    /// Opens the connection to the target.
    ///
    /// # Errors
    ///
    /// Same as [`Connection::open`].
    pub fn open(name: impl Into<String>, target: Target) -> Result<Self, ConnectionError> {
        Ok(ActiveTarget {
            name: name.into(),
            connection: Connection::open(&target)?,
            target,
        })
    }
}

/// Clients connected to a target.
#[derive(Debug, Clone)]
pub struct Connection {
    /// Client of the data APIs, like search.
    pub vespa: Vespa,
    /// Client of the operational APIs, like health.
    pub admin: Admin,
}

impl Connection {
    /// Creates the clients of the target, reading its credentials from the environment
    /// and the file system.
    ///
    /// # Errors
    ///
    /// Fails if the credentials cannot be found or read, or the endpoint is not valid.
    pub fn open(target: &Target) -> Result<Self, ConnectionError> {
        let auth = resolve_auth(target, cloud::cli_home().as_deref(), |name| {
            std::env::var(name).ok()
        })?;
        let client = http_client(&auth)?;
        Ok(Connection {
            vespa: Vespa::with_client(&target.endpoint, client.clone())?,
            admin: Admin::with_client(&target.endpoint, client)?,
        })
    }
}

/// Authentication of a target with its values resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ResolvedAuth {
    None,
    Mtls(Credentials),
    Token(String),
}

/// Finds the credentials of the target: explicit paths, the Vespa CLI ones on Vespa Cloud,
/// or the token in the environment.
fn resolve_auth(
    target: &Target,
    cli_home: Option<&Path>,
    env: impl Fn(&str) -> Option<String>,
) -> Result<ResolvedAuth, ConnectionError> {
    match &target.auth {
        Auth::None => Ok(ResolvedAuth::None),
        Auth::Mtls {
            cert: Some(cert),
            key: Some(key),
        } => Ok(ResolvedAuth::Mtls(Credentials {
            cert: cert.clone(),
            key: key.clone(),
        })),
        Auth::Mtls { cert, key } => {
            let app = target
                .cloud
                .as_ref()
                .ok_or(ConnectionError::MissingMtlsPaths)?;
            let home = cli_home.ok_or(ConnectionError::NoCliHome)?;
            let defaults = app.data_plane_credentials(home);
            Ok(ResolvedAuth::Mtls(Credentials {
                cert: cert.clone().unwrap_or(defaults.cert),
                key: key.clone().unwrap_or(defaults.key),
            }))
        }
        Auth::Token { env: name } => env(name)
            .filter(|token| !token.is_empty())
            .map(ResolvedAuth::Token)
            .ok_or_else(|| ConnectionError::MissingToken(name.clone())),
    }
}

/// HTTP client configured with the authentication.
fn http_client(auth: &ResolvedAuth) -> Result<Client, ConnectionError> {
    let mut builder = Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(TIMEOUT);
    match auth {
        ResolvedAuth::None => {}
        ResolvedAuth::Mtls(credentials) => builder = builder.identity(identity(credentials)?),
        ResolvedAuth::Token(token) => {
            let mut value = HeaderValue::from_str(&format!("Bearer {token}"))?;
            value.set_sensitive(true);
            builder = builder.default_headers(HeaderMap::from_iter([(AUTHORIZATION, value)]));
        }
    }
    builder.build().map_err(ConnectionError::Client)
}

/// Reads the PEM files of the credentials.
fn identity(credentials: &Credentials) -> Result<Identity, ConnectionError> {
    let read = |path: &PathBuf| {
        fs::read(path).map_err(|source| ConnectionError::ReadCredentials {
            path: path.clone(),
            source,
        })
    };
    let mut pem = read(&credentials.cert)?;
    if !pem.ends_with(b"\n") {
        pem.push(b'\n');
    }
    pem.extend(read(&credentials.key)?);
    Identity::from_pem(&pem).map_err(ConnectionError::Identity)
}

/// Errors connecting to a target.
#[derive(Debug, Error)]
pub enum ConnectionError {
    /// mTLS without certificate paths on a self-hosted target.
    #[error("mTLS requires the cert and key paths on self-hosted targets")]
    MissingMtlsPaths,
    /// The home directory of the Vespa CLI could not be found.
    #[error("Unable to find the Vespa CLI home, set VESPA_CLI_HOME or the cert and key paths")]
    NoCliHome,
    /// The environment variable of the token is not set.
    #[error("The token environment variable {0} is not set")]
    MissingToken(String),
    /// The token is not a valid header value.
    #[error("Invalid token: {0}")]
    InvalidToken(#[from] InvalidHeaderValue),
    /// A credentials file could not be read.
    #[error("Unable to read {}: {source}", path.display())]
    ReadCredentials {
        /// Path of the file.
        path: PathBuf,
        /// Cause of the failure.
        source: io::Error,
    },
    /// The certificate or the key are not valid PEM.
    #[error("Invalid client certificate: {0}")]
    Identity(#[source] nespra::reqwest::Error),
    /// The HTTP client could not be built.
    #[error("Failed to build the HTTP client: {0}")]
    Client(#[source] nespra::reqwest::Error),
    /// The search client could not be created.
    #[error(transparent)]
    Search(#[from] nespra::error::BuildError),
    /// The admin client could not be created.
    #[error(transparent)]
    Admin(#[from] nespra::admin::error::BuildError),
}

#[cfg(test)]
mod test {
    use std::io::Write;

    use super::*;
    use crate::core::config::CloudApplication;

    fn target(auth: Auth, cloud: Option<CloudApplication>) -> Target {
        Target {
            endpoint: "http://localhost:8080".to_string(),
            cloud,
            auth,
        }
    }

    fn app() -> Option<CloudApplication> {
        Some(CloudApplication {
            tenant: "acme".to_string(),
            application: "search".to_string(),
            instance: "default".to_string(),
        })
    }

    fn no_env(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn test_no_auth() {
        let auth = resolve_auth(&target(Auth::None, None), None, no_env).unwrap();
        assert_eq!(auth, ResolvedAuth::None);
    }

    #[test]
    fn test_explicit_mtls_paths() {
        let mtls = Auth::Mtls {
            cert: Some("/c.pem".into()),
            key: Some("/k.pem".into()),
        };
        let auth = resolve_auth(&target(mtls, None), None, no_env).unwrap();
        assert_eq!(
            auth,
            ResolvedAuth::Mtls(Credentials {
                cert: "/c.pem".into(),
                key: "/k.pem".into()
            })
        );
    }

    #[test]
    fn test_cloud_mtls_defaults_to_the_cli_credentials() {
        let mtls = Auth::Mtls {
            cert: None,
            key: Some("/k.pem".into()),
        };
        let auth = resolve_auth(&target(mtls, app()), Some(Path::new("/v")), no_env).unwrap();
        assert_eq!(
            auth,
            ResolvedAuth::Mtls(Credentials {
                cert: "/v/acme.search.default/data-plane-public-cert.pem".into(),
                key: "/k.pem".into()
            })
        );
    }

    #[test]
    fn test_self_hosted_mtls_requires_the_paths() {
        let mtls = Auth::Mtls {
            cert: None,
            key: None,
        };
        let error = resolve_auth(&target(mtls, None), Some(Path::new("/v")), no_env);
        assert!(matches!(error, Err(ConnectionError::MissingMtlsPaths)));
    }

    #[test]
    fn test_cloud_mtls_without_cli_home() {
        let mtls = Auth::Mtls {
            cert: None,
            key: None,
        };
        let error = resolve_auth(&target(mtls, app()), None, no_env);
        assert!(matches!(error, Err(ConnectionError::NoCliHome)));
    }

    #[test]
    fn test_token_from_the_environment() {
        let token = || Auth::Token {
            env: "TOKEN".to_string(),
        };
        let env = |name: &str| (name == "TOKEN").then(|| "secret".to_string());
        let auth = resolve_auth(&target(token(), None), None, env).unwrap();
        assert_eq!(auth, ResolvedAuth::Token("secret".to_string()));

        let error = resolve_auth(&target(token(), None), None, no_env);
        assert!(matches!(error, Err(ConnectionError::MissingToken(name)) if name == "TOKEN"));
    }

    #[test]
    fn test_missing_credentials_file() {
        let credentials = Credentials {
            cert: "/does/not/exist.pem".into(),
            key: "/does/not/exist.key".into(),
        };
        let error = http_client(&ResolvedAuth::Mtls(credentials)).unwrap_err();
        assert!(
            matches!(error, ConnectionError::ReadCredentials { path, .. } if path == Path::new("/does/not/exist.pem"))
        );
    }

    #[test]
    fn test_invalid_credentials_file() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"not a pem").unwrap();
        let credentials = Credentials {
            cert: file.path().into(),
            key: file.path().into(),
        };
        let error = http_client(&ResolvedAuth::Mtls(credentials)).unwrap_err();
        assert!(matches!(error, ConnectionError::Identity(_)));
    }

    #[test]
    fn test_invalid_token() {
        let error = http_client(&ResolvedAuth::Token("line\nbreak".to_string())).unwrap_err();
        assert!(matches!(error, ConnectionError::InvalidToken(_)));
    }

    #[test]
    fn test_open_a_self_hosted_target() {
        let connection = Connection::open(&target(Auth::None, None)).unwrap();
        assert_eq!(connection.admin.url().as_str(), "http://localhost:8080/");
    }

    #[test]
    fn test_open_with_an_invalid_endpoint() {
        let mut target = target(Auth::None, None);
        target.endpoint = "localhost:8080".to_string();
        assert!(matches!(
            Connection::open(&target),
            Err(ConnectionError::Search(_))
        ));
    }
}
