use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    ratatui::run(lazyvespa::tui::run)?;
    Ok(())
}
