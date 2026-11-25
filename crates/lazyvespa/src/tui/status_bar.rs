//! Bar at the top with the state of the connection.

use nespra::admin::state::HealthCode;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Stylize},
    text::{Line, Span},
};

use super::Model;
use crate::core::health::HealthCheck;

/// Renders the target the application is connected to, with its health and Vespa
/// version on the right.
pub(super) fn render(model: &Model, frame: &mut Frame, area: Rect) {
    let mut spans = vec![Span::from(" lazyvespa ").bold().black().on_light_green()];
    let Some(active) = model.target() else {
        spans.push(Span::from(" no target, use --url or --target").dim());
        frame.render_widget(Line::from(spans), area);
        return;
    };
    let deployment = match &active.target.cloud {
        Some(app) => format!(" cloud {app} "),
        None => " self-hosted ".to_string(),
    };
    spans.extend([
        Span::from(format!(" {} ", active.name)).bold(),
        Span::from(deployment).fg(Color::Black).bg(Color::Cyan),
        Span::from(format!(" {}", active.target.endpoint)).dim(),
    ]);

    let version = match model.deployment() {
        Some(Ok(deployment)) => Some(deployment.vespa_version.as_str()),
        _ => None,
    };
    let health = health_line(model.health(), version);
    let width = u16::try_from(health.width()).unwrap_or(u16::MAX);
    let [target_area, health_area] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(width)]).areas(area);
    frame.render_widget(Line::from(spans), target_area);
    frame.render_widget(health, health_area);
}

/// Colour and text of the health of the target.
pub(super) fn health_status(health: Option<&Result<HealthCheck, String>>) -> (Color, String) {
    match health {
        None => (Color::DarkGray, "checking".to_string()),
        Some(Err(_)) => (Color::Red, "unreachable".to_string()),
        Some(Ok(check)) => {
            let color = match check.code {
                HealthCode::Up => Color::Green,
                HealthCode::Initializing => Color::Yellow,
                _ => Color::Red,
            };
            (color, check.code.to_string())
        }
    }
}

/// Vespa version of the target, then its health: a coloured dot, the status and how long
/// the check took.
fn health_line(
    health: Option<&Result<HealthCheck, String>>,
    version: Option<&str>,
) -> Line<'static> {
    let mut spans = Vec::new();
    if let Some(version) = version {
        spans.push(Span::from(format!(" Vespa {version} ")).dim());
    }
    let (color, status) = health_status(health);
    spans.extend([Span::from(" ● ").fg(color), Span::from(status)]);
    if let Some(Ok(check)) = health {
        spans.push(Span::from(format!(" {} ms", check.elapsed.as_millis())).dim());
    }
    spans.push(Span::from(" "));
    Line::from(spans)
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use super::*;

    fn text(line: &Line) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    fn check(code: HealthCode) -> Result<HealthCheck, String> {
        Ok(HealthCheck {
            code,
            message: None,
            elapsed: Duration::from_millis(12),
        })
    }

    #[test]
    fn test_health_before_the_first_check() {
        let line = health_line(None, None);
        assert_eq!(text(&line), " ● checking ");
        assert_eq!(line.spans[0].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_health_up() {
        let line = health_line(Some(&check(HealthCode::Up)), None);
        assert_eq!(text(&line), " ● up 12 ms ");
        assert_eq!(line.spans[0].style.fg, Some(Color::Green));
    }

    #[test]
    fn test_health_initializing() {
        let line = health_line(Some(&check(HealthCode::Initializing)), None);
        assert_eq!(text(&line), " ● initializing 12 ms ");
        assert_eq!(line.spans[0].style.fg, Some(Color::Yellow));
    }

    #[test]
    fn test_health_down() {
        let line = health_line(Some(&check(HealthCode::Down)), None);
        assert_eq!(text(&line), " ● down 12 ms ");
        assert_eq!(line.spans[0].style.fg, Some(Color::Red));
    }

    #[test]
    fn test_version() {
        let line = health_line(Some(&check(HealthCode::Up)), Some("8.512.1"));
        assert_eq!(text(&line), " Vespa 8.512.1  ● up 12 ms ");
    }

    #[test]
    fn test_health_unreachable() {
        let line = health_line(Some(&Err("connection refused".to_string())), None);
        assert_eq!(text(&line), " ● unreachable ");
        assert_eq!(line.spans[0].style.fg, Some(Color::Red));
    }
}
