//! Results of the last search: the hits with the detail of the selected one, or the whole
//! response.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Paragraph, Row, Table, TableState, Wrap},
};
use serde_json::Value;

use super::model::SearchState;
use crate::core::search::SearchOutcome;

/// Lines scrolled at once in the detail.
const SCROLL_STEP: u16 = 10;

/// State of the results view.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResultsView {
    selected: usize,
    scroll: u16,
    raw: bool,
}

/// Messages of the [`ResultsView`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultsMsg {
    /// Select the next hit, wrapping around.
    Next,
    /// Select the previous hit, wrapping around.
    Previous,
    /// Scroll the detail down.
    ScrollDown,
    /// Scroll the detail up.
    ScrollUp,
    /// Switch between the hits and the whole response.
    ToggleRaw,
}

impl ResultsView {
    /// Index of the selected hit.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Whether the whole response is shown instead of the hits.
    pub fn is_raw(&self) -> bool {
        self.raw
    }

    /// Applies a message to the view of the outcome.
    pub fn update(&mut self, message: ResultsMsg, outcome: Option<&SearchOutcome>) {
        let hits = outcome.map_or(0, |outcome| outcome.response.hits().len());
        match message {
            ResultsMsg::Next | ResultsMsg::Previous if hits == 0 => {}
            ResultsMsg::Next => self.select((self.selected + 1) % hits),
            ResultsMsg::Previous => self.select((self.selected + hits - 1) % hits),
            ResultsMsg::ScrollDown => {
                let lines = outcome.map_or(0, |outcome| self.detail(outcome).lines().count());
                let last = u16::try_from(lines.saturating_sub(1)).unwrap_or(u16::MAX);
                self.scroll = self.scroll.saturating_add(SCROLL_STEP).min(last);
            }
            ResultsMsg::ScrollUp => self.scroll = self.scroll.saturating_sub(SCROLL_STEP),
            ResultsMsg::ToggleRaw => {
                self.raw = !self.raw;
                self.scroll = 0;
            }
        }
    }

    /// Selects a hit, showing its detail from the start.
    fn select(&mut self, index: usize) {
        self.selected = index;
        self.scroll = 0;
    }

    /// Text of the detail: the selected hit, or the whole response when raw.
    fn detail(&self, outcome: &SearchOutcome) -> String {
        let pretty = if self.raw {
            serde_json::to_string_pretty(&outcome.response)
        } else {
            let hit = outcome.response.hits().get(self.selected);
            serde_json::to_string_pretty(&hit)
        };
        pretty.unwrap_or_else(|e| format!("Unable to show the JSON: {e}"))
    }
}

/// Renders the state of the last search in the area, highlighted when focused.
pub(super) fn render(
    search: &SearchState,
    view: &ResultsView,
    frame: &mut Frame,
    area: Rect,
    focused: bool,
) {
    let mut block = Block::bordered().title(" Results ");
    if focused {
        block = block.border_style(Style::new().fg(Color::Cyan));
    }
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
            let [header, body] =
                Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(inner);
            frame.render_widget(summary(outcome), header);
            let detail = Paragraph::new(view.detail(outcome)).scroll((view.scroll, 0));
            if view.raw {
                frame.render_widget(detail, body);
                return;
            }
            let [hits, detail_area] =
                Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(body);
            let highlight = if focused {
                Style::new().reversed()
            } else {
                Style::new().bold()
            };
            let table = hits_table(outcome.response.hits()).row_highlight_style(highlight);
            let mut state = TableState::default().with_selected(Some(view.selected));
            frame.render_stateful_widget(table, hits, &mut state);
            let border = Block::new()
                .borders(ratatui::widgets::Borders::LEFT)
                .border_style(Style::new().dim());
            frame.render_widget(detail.block(border), detail_area);
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
            .draw(|frame| render(search, &ResultsView::default(), frame, frame.area(), false))
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

    fn outcome() -> SearchOutcome {
        let hits: Vec<_> = (1..=3)
            .map(|i| json!({"id": format!("id:doc:doc::{i}"), "relevance": 1.0 / f64::from(i)}))
            .collect();
        SearchOutcome {
            response: serde_json::from_value(json!({
                "root": {"relevance": 1.0, "fields": {"totalCount": 3}, "children": hits}
            }))
            .unwrap(),
            elapsed: Duration::from_millis(5),
        }
    }

    #[test]
    fn test_selection_wraps_around_and_resets_the_scroll() {
        let outcome = outcome();
        let mut view = ResultsView::default();
        view.update(ResultsMsg::ScrollDown, Some(&outcome));
        assert!(view.scroll > 0);
        view.update(ResultsMsg::Previous, Some(&outcome));
        assert_eq!((view.selected(), view.scroll), (2, 0));
        view.update(ResultsMsg::Next, Some(&outcome));
        assert_eq!(view.selected(), 0);
    }

    #[test]
    fn test_scroll_stays_in_the_detail() {
        let outcome = outcome();
        let mut view = ResultsView::default();
        let lines = view.detail(&outcome).lines().count();
        (0..10).for_each(|_| view.update(ResultsMsg::ScrollDown, Some(&outcome)));
        assert_eq!(usize::from(view.scroll), lines - 1);
        view.update(ResultsMsg::ScrollUp, Some(&outcome));
        assert_eq!(usize::from(view.scroll), (lines - 1).saturating_sub(10));
    }

    #[test]
    fn test_raw_shows_the_whole_response() {
        let outcome = outcome();
        let mut view = ResultsView::default();
        assert!(view.detail(&outcome).contains("id:doc:doc::1"));
        assert!(!view.detail(&outcome).contains("totalCount"));
        view.update(ResultsMsg::ToggleRaw, Some(&outcome));
        assert!(view.is_raw());
        assert!(view.detail(&outcome).contains("totalCount"));
    }

    #[test]
    fn test_without_hits() {
        let mut view = ResultsView::default();
        view.update(ResultsMsg::Next, None);
        view.update(ResultsMsg::ScrollDown, None);
        assert_eq!(view, ResultsView::default());
    }

    #[test]
    fn test_render_the_selected_hit() {
        let search = SearchState::Done(Arc::new(outcome()));
        let mut view = ResultsView::default();
        view.update(ResultsMsg::Next, Some(&outcome()));
        let mut terminal = Terminal::new(TestBackend::new(80, 9)).unwrap();
        terminal
            .draw(|frame| render(&search, &view, frame, frame.area(), true))
            .unwrap();
        assert_snapshot!(terminal.backend());
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
