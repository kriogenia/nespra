//! Bar at the top with the state of the connection.

use nespra::admin::state::HealthCode;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Stylize},
    text::{Line, Span},
};

use crate::core::{connection::ActiveTarget, health::HealthCheck};

/// Renders the target the application is connected to, with its health on the right.
pub(super) fn render(
    target: Option<&ActiveTarget>,
    health: Option<&Result<HealthCheck, String>>,
    frame: &mut Frame,
    area: Rect,
) {
    let mut spans = vec![Span::from(" lazyvespa ").bold().black().on_light_green()];
    let Some(active) = target else {
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

    let health = health_line(health);
    let width = u16::try_from(health.width()).unwrap_or(u16::MAX);
    let [target_area, health_area] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(width)]).areas(area);
    frame.render_widget(Line::from(spans), target_area);
    frame.render_widget(health, health_area);
}

/// Health of the target: a coloured dot, the status and how long the check took.
fn health_line(health: Option<&Result<HealthCheck, String>>) -> Line<'static> {
    let (color, status, elapsed) = match health {
        None => (Color::DarkGray, "checking".to_string(), None),
        Some(Err(_)) => (Color::Red, "unreachable".to_string(), None),
        Some(Ok(check)) => {
            let color = match check.code {
                HealthCode::Up => Color::Green,
                HealthCode::Initializing => Color::Yellow,
                _ => Color::Red,
            };
            (color, check.code.to_string(), Some(check.elapsed))
        }
    };
    let mut spans = vec![Span::from(" ● ").fg(color), Span::from(status)];
    if let Some(elapsed) = elapsed {
        spans.push(Span::from(format!(" {} ms", elapsed.as_millis())).dim());
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
        let line = health_line(None);
        assert_eq!(text(&line), " ● checking ");
        assert_eq!(line.spans[0].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_health_up() {
        let line = health_line(Some(&check(HealthCode::Up)));
        assert_eq!(text(&line), " ● up 12 ms ");
        assert_eq!(line.spans[0].style.fg, Some(Color::Green));
    }

    #[test]
    fn test_health_initializing() {
        let line = health_line(Some(&check(HealthCode::Initializing)));
        assert_eq!(text(&line), " ● initializing 12 ms ");
        assert_eq!(line.spans[0].style.fg, Some(Color::Yellow));
    }

    #[test]
    fn test_health_down() {
        let line = health_line(Some(&check(HealthCode::Down)));
        assert_eq!(text(&line), " ● down 12 ms ");
        assert_eq!(line.spans[0].style.fg, Some(Color::Red));
    }

    #[test]
    fn test_health_unreachable() {
        let line = health_line(Some(&Err("connection refused".to_string())));
        assert_eq!(text(&line), " ● unreachable ");
        assert_eq!(line.spans[0].style.fg, Some(Color::Red));
    }
}
