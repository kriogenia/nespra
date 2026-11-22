//! Results of the last search.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Paragraph, Row, Table, Wrap},
};
use serde_json::Value;

use super::model::SearchState;
use crate::core::search::SearchOutcome;

/// Renders the state of the last search in the area.
pub(super) fn render(search: &SearchState, frame: &mut Frame, area: Rect) {
    let block = Block::bordered().title(" Results ");
    match search {
        SearchState::Idle => {
            let hint = Paragraph::new(Line::from(" Ctrl+R to run the query").dim());
            frame.render_widget(hint.block(block), area);
        }
        SearchState::Running(_) => {
            let running = Paragraph::new(Line::from(" Searching…").dim());
            frame.render_widget(running.block(block), area);
        }
        SearchState::Failed(error) => {
            let error = Paragraph::new(error.as_str())
                .red()
                .wrap(Wrap { trim: false });
            frame.render_widget(error.block(block), area);
        }
        SearchState::Done(outcome) => {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let [header, hits] =
                Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(inner);
            frame.render_widget(summary(outcome), header);
            frame.render_widget(hits_table(outcome.response.hits()), hits);
        }
    }
}

/// Line with the totals and the times of the search.
fn summary(outcome: &SearchOutcome) -> Line<'static> {
    let response = &outcome.response;
    let mut spans = vec![Span::from(format!("{} hits", response.total_count())).bold()];
    if let Some(coverage) = &response.root.coverage {
        spans.push(Span::from(format!(" · coverage {}%", coverage.coverage)));
        if coverage.degraded.is_some() {
            spans.push(Span::from(" · degraded").yellow());
        }
    }
    if let Some(search_time) = response.timing.as_ref().and_then(|t| t.search_time) {
        let millis = search_time * 1000.0;
        spans.push(Span::from(format!(" · {millis:.0} ms in Vespa")));
    }
    spans.push(Span::from(format!(
        " · {} ms round trip",
        outcome.elapsed.as_millis()
    )));
    Line::from(spans)
}

/// Table with the relevance, id and source of the hits.
fn hits_table(hits: &[Value]) -> Table<'_> {
    let rows = hits.iter().enumerate().map(|(index, hit)| {
        let relevance = hit["relevance"]
            .as_f64()
            .map(|relevance| format!("{relevance:.4}"))
            .unwrap_or_default();
        Row::new([
            format!("{}", index + 1),
            relevance,
            text(&hit["id"]),
            text(&hit["source"]),
        ])
    });
    Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Length(10),
            Constraint::Fill(3),
            Constraint::Fill(1),
        ],
    )
    .header(Row::new(["#", "relevance", "id", "source"]).style(Style::new().dim()))
}

/// Text of a JSON value, without quotes for strings.
fn text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod test {
    use std::{sync::Arc, time::Duration};

    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};
    use serde_json::json;

    use super::*;

    fn render_state(search: &SearchState) -> String {
        let mut terminal = Terminal::new(TestBackend::new(80, 7)).unwrap();
        terminal
            .draw(|frame| render(search, frame, frame.area()))
            .unwrap();
        terminal.backend().to_string()
    }

    fn done(response: Value) -> SearchState {
        SearchState::Done(Arc::new(SearchOutcome {
            response: serde_json::from_value(response).unwrap(),
            elapsed: Duration::from_millis(23),
        }))
    }

    #[test]
    fn test_results() {
        let search = done(json!({
            "timing": {"searchtime": 0.0081},
            "root": {
                "relevance": 1.0,
                "fields": {"totalCount": 120},
                "coverage": {
                    "coverage": 100, "documents": 500, "full": true,
                    "nodes": 1, "results": 1, "resultsFull": 1
                },
                "children": [
                    {"id": "id:doc:doc::1", "relevance": 0.91234, "source": "content"},
                    {"id": "index:content/0/abc", "relevance": 0.5, "source": "content"}
                ]
            }
        }));
        assert_snapshot!(render_state(&search));
    }

    #[test]
    fn test_degraded_results_without_timing() {
        let search = done(json!({
            "root": {
                "relevance": 1.0,
                "fields": {"totalCount": 0},
                "coverage": {
                    "coverage": 50, "documents": 10, "full": false,
                    "nodes": 1, "results": 1, "resultsFull": 0,
                    "degraded": {
                        "match-phase": false, "timeout": true, "adaptive-timeout": false,
                        "anntimeout": false, "non-ideal-state": false
                    }
                }
            }
        }));
        assert_snapshot!(render_state(&search));
    }

    #[test]
    fn test_other_states() {
        let states = [
            SearchState::Idle,
            SearchState::Running(1),
            SearchState::Failed("Search failed: Unexpected HTTP status 502".to_string()),
        ];
        let rendered: Vec<_> = states.iter().map(render_state).collect();
        assert_snapshot!(rendered.join("\n"));
    }
}
