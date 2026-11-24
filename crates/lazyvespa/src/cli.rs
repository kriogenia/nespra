//! Command line arguments.

use std::path::PathBuf;

use clap::Parser;
use color_eyre::{Result, eyre::WrapErr};
use lazyvespa::core::{
    config::{self, Config, Target},
    connection::ActiveTarget,
};

/// Terminal UI to query and operate Vespa deployments.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    /// Configuration file [default: ~/.config/lazyvespa/config.toml]
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Name of the target to connect to, the default target of the configuration if not set
    #[arg(short, long, value_name = "NAME")]
    target: Option<String>,

    /// Connects to this endpoint instead of a configured target
    #[arg(short, long, value_name = "URL", conflicts_with = "target")]
    url: Option<String>,
}

impl Cli {
    /// Loads the configuration and opens the requested target, `None` when no target is
    /// requested nor configured by default.
    pub fn load(&self) -> Result<(Config, Option<ActiveTarget>)> {
        let config = self.load_config()?;
        let selected = match &self.url {
            Some(url) => Some(("url".to_string(), Target::self_hosted(url))),
            None => config
                .select(self.target.as_deref())?
                .map(|(name, target)| (name.to_string(), target.clone())),
        };
        let target = selected
            .map(|(name, target)| {
                ActiveTarget::open(&name, target)
                    .wrap_err_with(|| format!("Unable to connect to the target {name:?}"))
            })
            .transpose()?;
        Ok((config, target))
    }

    /// Loads the requested configuration file, or the default one if it exists.
    fn load_config(&self) -> Result<Config> {
        let loaded = match (&self.config, config::default_path()) {
            (Some(path), _) => Config::load(path),
            (None, Some(path)) => Config::load_or_default(&path),
            (None, None) => Ok(Config::default()),
        };
        Ok(loaded?)
    }
}

#[cfg(test)]
mod test {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn test_cli_definition() {
        Cli::command().debug_assert();
    }

    #[test]
    fn test_target_and_url_conflict() {
        let result = Cli::try_parse_from(["lazyvespa", "--target", "a", "--url", "http://b"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_url_target() {
        let cli = Cli::try_parse_from(["lazyvespa", "--url", "http://localhost:8080"]).unwrap();
        let (_, active) = cli.load().unwrap();
        let active = active.unwrap();
        assert_eq!(active.name, "url");
        assert_eq!(active.target, Target::self_hosted("http://localhost:8080"));
    }

    #[test]
    fn test_missing_config_file() {
        let cli = Cli::try_parse_from(["lazyvespa", "--config", "/does/not/exist.toml"]).unwrap();
        assert!(cli.load().is_err());
    }
}
