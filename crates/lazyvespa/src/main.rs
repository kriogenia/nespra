mod cli;

use clap::Parser;
use color_eyre::Result;
use lazyvespa::{
    core::{
        history::{self, Histories},
        saved::{self, SavedQueries},
    },
    tui::{self, Model},
};

use crate::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    install_error_hooks()?;
    let (config, target) = Cli::parse().load()?;

    let mut model = Model::new(config, target);
    if let Some(path) = history::default_path() {
        let loaded = Histories::load(&path);
        model = model.with_history(path, loaded);
    }
    if let Some(path) = saved::default_path() {
        let loaded = SavedQueries::load(&path);
        model = model.with_saved_queries(path, loaded);
    }

    let mut terminal = ratatui::init();
    let result = tui::run(&mut terminal, model).await;
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
