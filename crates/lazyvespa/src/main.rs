mod cli;

use clap::Parser;
use color_eyre::Result;
use lazyvespa::tui::{self, Model};

use crate::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    install_error_hooks()?;
    let (config, target) = Cli::parse().load()?;

    let mut terminal = ratatui::init();
    let result = tui::run(&mut terminal, Model::new(config, target)).await;
    ratatui::restore();
    Ok(result?)
}

/// Reports errors and panics without the hints to enable backtraces, and without the
/// location in the source code in release builds, as they are noise for the users.
fn install_error_hooks() -> Result<()> {
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .display_location_section(cfg!(debug_assertions))
        .install()
}
