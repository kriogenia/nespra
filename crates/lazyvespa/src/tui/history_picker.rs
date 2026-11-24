//! Overlay to pick one of the queries run against the target, to load it in the editor.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListState, Paragraph},
};

use super::popup;
use crate::core::{history::History, query::QueryDraft};

/// State of the history picker: the queries, the most recent first, and the highlighted one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HistoryPicker {
    queries: Vec<QueryDraft>,
    selected: usize,
}

/// Messages of the [`HistoryPicker`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryMsg {
    /// Highlight the next query, wrapping around.
    Next,
    /// Highlight the previous query, wrapping around.
    Previous,
    /// Pick the highlighted query.
    Pick,
}

impl HistoryPicker {
    /// Creates a picker of the queries of the history, with the most recent highlighted.
    pub fn new(history: &History) -> Self {
        HistoryPicker {
            queries: history.entries().cloned().collect(),
            selected: 0,
        }
    }

    /// The highlighted query.
    pub fn selected(&self) -> Option<&QueryDraft> {
        self.queries.get(self.selected)
    }

    /// Applies a message. Picking returns the highlighted query.
    pub fn update(&mut self, message: HistoryMsg) -> Option<QueryDraft> {
        let len = self.queries.len();
        match message {
            _ if len == 0 => {}
            HistoryMsg::Next => self.selected = (self.selected + 1) % len,
            HistoryMsg::Previous => self.selected = (self.selected + len - 1) % len,
            HistoryMsg::Pick => return self.selected().cloned(),
        }
        None
    }

    /// Renders the picker over most of the area.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().title(" History ");
        let height = u16::try_from(self.queries.len().max(1))
            .unwrap_or(u16::MAX)
            .saturating_add(2);
        let width = area.width.saturating_sub(8).max(30);
        let popup = popup::centered(area, width, height);
        frame.render_widget(Clear, popup);

        if self.queries.is_empty() {
            let empty = Paragraph::new(Line::from(" No queries run against this target yet").dim());
            frame.render_widget(empty.block(block), popup);
            return;
        }
        let list = List::new(self.queries.iter().map(summary))
            .block(block)
            .highlight_symbol("> ")
            .highlight_style(Style::new().reversed());
        let mut state = ListState::default().with_selected(Some(self.selected));
        frame.render_stateful_widget(list, popup, &mut state);
    }
}

/// The query in one line: the YQL, and its parameters dimmed.
fn summary(query: &QueryDraft) -> Line<'_> {
    let yql = query.yql.split_whitespace().collect::<Vec<_>>().join(" ");
    let params = query
        .params
        .iter()
        .map(|(name, value)| format!(" {name}={value}"))
        .collect::<String>();
    Line::from(vec![Span::raw(yql), Span::raw(params).dim()])
}

#[cfg(test)]
mod test {
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn picker() -> HistoryPicker {
        let mut history = History::default();
        history.record(QueryDraft::new("select * from doc where true"));
        history.record(QueryDraft::new("select *\nfrom music\nwhere true").param("hits", "5"));
        HistoryPicker::new(&history)
    }

    #[test]
    fn test_most_recent_is_highlighted() {
        assert_eq!(picker().selected().unwrap().params.len(), 1);
    }

    #[test]
    fn test_navigation_wraps_around() {
        let mut picker = picker();
        picker.update(HistoryMsg::Previous);
        assert_eq!(
            picker.selected().unwrap().yql,
            "select * from doc where true"
        );
        picker.update(HistoryMsg::Next);
        assert_eq!(picker.selected().unwrap().params.len(), 1);
    }

    #[test]
    fn test_pick() {
        let mut picker = picker();
        picker.update(HistoryMsg::Next);
        let picked = picker.update(HistoryMsg::Pick);
        assert_eq!(
            picked,
            Some(QueryDraft::new("select * from doc where true"))
        );
    }

    #[test]
    fn test_empty_picker() {
        let mut picker = HistoryPicker::new(&History::default());
        assert_eq!(picker.update(HistoryMsg::Next), None);
        assert_eq!(picker.update(HistoryMsg::Pick), None);
    }

    fn render(picker: &HistoryPicker) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(60, 8)).unwrap();
        terminal
            .draw(|frame| picker.render(frame, frame.area()))
            .unwrap();
        terminal
    }

    #[test]
    fn test_render() {
        assert_snapshot!(render(&picker()).backend());
    }

    #[test]
    fn test_render_empty() {
        assert_snapshot!(render(&HistoryPicker::default()).backend());
    }
}
