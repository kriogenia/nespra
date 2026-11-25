//! Overlay with information about the target: its health and the deployment of its
//! application.

use crate::core::health::Deployment;
use ratatui::{
    Frame,
    layout::Rect,
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Wrap},
};

use super::{Model, popup, status_bar};

/// Width of the labels column.
const LABEL_WIDTH: usize = 13;

/// Renders the panel over the area.
pub(super) fn render(model: &Model, frame: &mut Frame, area: Rect) {
    let Some(target) = model.target() else {
        let lines = vec![Line::from(" Not connected to a target, press T to pick one").dim()];
        return render_lines(" Info ", lines, frame, area);
    };
    let mut lines = health_lines(model);
    lines.push(Line::default());
    lines.extend(deployment_lines(
        model.deployment(),
        model.is_fetching_deployment(),
    ));
    render_lines(&format!(" Info of {} ", target.name), lines, frame, area);
}

/// Renders the lines in a bordered popup sized to fit them.
fn render_lines(title: &str, lines: Vec<Line<'static>>, frame: &mut Frame, area: Rect) {
    let width = area.width.saturating_sub(8).clamp(30, 80);
    let height = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_add(2);
    let popup = popup::centered(area, width, height);
    let paragraph = Paragraph::new(lines)
        .block(Block::bordered().title(title.to_string()))
        .wrap(Wrap { trim: false });
    frame.render_widget(Clear, popup);
    frame.render_widget(paragraph, popup);
}

/// Line with a label and its value.
fn row(label: &str, value: impl Into<Vec<Span<'static>>>) -> Line<'static> {
    let mut spans = vec![Span::from(format!(" {label:LABEL_WIDTH$}")).bold()];
    spans.extend(value.into());
    Line::from(spans)
}

/// Status of the last health check, with its message or error.
fn health_lines(model: &Model) -> Vec<Line<'static>> {
    let health = model.health();
    let (color, status) = status_bar::health_status(health);
    let mut lines = vec![row(
        "Status",
        vec![Span::from("● ").fg(color), status.into()],
    )];
    match health {
        Some(Ok(check)) => {
            if let Some(message) = &check.message {
                lines.push(row("Message", vec![message.clone().into()]));
            }
            let elapsed = format!("{} ms", check.elapsed.as_millis());
            lines.push(row("Checked in", vec![elapsed.into()]));
        }
        Some(Err(error)) => lines.push(row("Error", vec![Span::from(error.clone()).red()])),
        None => {}
    }
    lines
}

/// Status of the application, or why it is not known.
fn deployment_lines(
    application: Option<&Result<Deployment, String>>,
    fetching: bool,
) -> Vec<Line<'static>> {
    let deployment = match application {
        Some(Ok(deployment)) => deployment,
        Some(Err(error)) => {
            return vec![row(
                "Application",
                vec![Span::from(format!("unavailable: {error}")).dim()],
            )];
        }
        None if fetching => return vec![row("Application", vec![Span::from("loading…").dim()])],
        None => {
            return vec![row(
                "Application",
                vec![Span::from("unknown until the target is up").dim()],
            )];
        }
    };
    let mut lines = vec![row("Vespa", vec![deployment.vespa_version.clone().into()])];
    if let Some(name) = &deployment.application {
        lines.push(row("Application", vec![name.clone().into()]));
    }
    if let Some(generation) = deployment.generation {
        lines.push(row("Generation", vec![generation.to_string().into()]));
    }
    if let Some(deployed_at) = deployment.deployed_at {
        let deployed_at = deployed_at.strftime("%Y-%m-%d %H:%M:%S UTC").to_string();
        lines.push(row("Deployed", vec![deployed_at.into()]));
    }
    lines
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use insta::assert_snapshot;
    use nespra::admin::state::HealthCode;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::{
        core::{config::Target, connection::ActiveTarget, health::HealthCheck},
        tui::{Message, update},
    };

    fn text(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn test_application_unknown() {
        assert_eq!(
            text(&deployment_lines(None, true)),
            [" Application  loading…"]
        );
        assert_eq!(
            text(&deployment_lines(None, false)),
            [" Application  unknown until the target is up"]
        );
        let error = Err("Unexpected HTTP status 403 Forbidden".to_string());
        assert_eq!(
            text(&deployment_lines(Some(&error), false)),
            [" Application  unavailable: Unexpected HTTP status 403 Forbidden"]
        );
    }

    fn render(model: &Model) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(70, 12)).unwrap();
        terminal
            .draw(|frame| super::render(model, frame, frame.area()))
            .unwrap();
        terminal
    }

    /// Model connected to a target that answered the health check and the request of
    /// the status of the application.
    fn checked(
        health: Result<HealthCheck, String>,
        deployment: Option<Result<Deployment, String>>,
    ) -> Model {
        let target = ActiveTarget::open("local", Target::self_hosted("http://localhost:8080"));
        let mut model = Model::new(Default::default(), Some(target.unwrap()));
        let _ = update(&mut model, Message::Tick);
        let _ = update(
            &mut model,
            Message::HealthChecked {
                id: 1,
                result: health,
            },
        );
        if let Some(result) = deployment {
            let _ = update(&mut model, Message::DeploymentFetched { id: 1, result });
        }
        model
    }

    fn deployment() -> Deployment {
        Deployment {
            vespa_version: "8.512.1".to_string(),
            application: Some("default".to_string()),
            generation: Some(2),
            deployed_at: Some("2026-11-25T17:19:22.436Z".parse().unwrap()),
        }
    }

    #[test]
    fn test_render_up() {
        let health = Ok(HealthCheck {
            code: HealthCode::Up,
            message: None,
            elapsed: Duration::from_millis(3),
        });
        let model = checked(health, Some(Ok(deployment())));
        assert_snapshot!(render(&model).backend());
    }

    #[test]
    fn test_render_unreachable() {
        let model = checked(Err("error sending request".to_string()), None);
        assert_snapshot!(render(&model).backend());
    }

    #[test]
    fn test_render_without_target() {
        assert_snapshot!(render(&Model::default()).backend());
    }
}
