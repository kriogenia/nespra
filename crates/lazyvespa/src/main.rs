use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let result = lazyvespa::tui::run(&mut terminal).await;
    ratatui::restore();
    Ok(result?)
}
