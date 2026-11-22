//! Bar at the top with the state of the connection.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Stylize},
    text::{Line, Span},
};

use crate::core::connection::ActiveTarget;

/// Renders the target the application is connected to.
pub(super) fn render(target: Option<&ActiveTarget>, frame: &mut Frame, area: Rect) {
    let mut spans = vec![Span::from(" lazyvespa ").bold().black().on_light_green()];
    match target {
        Some(active) => {
            let deployment = match &active.target.cloud {
                Some(app) => format!(" cloud {app} "),
                None => " self-hosted ".to_string(),
            };
            spans.extend([
                Span::from(format!(" {} ", active.name)).bold(),
                Span::from(deployment).fg(Color::Black).bg(Color::Cyan),
                Span::from(format!(" {}", active.target.endpoint)).dim(),
            ]);
        }
        None => spans.push(Span::from(" no target, use --url or --target").dim()),
    }
    frame.render_widget(Line::from(spans), area);
}
