//! Overlay to pick one of the saved queries, to load it in the editor or delete it.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListState, Paragraph},
};

use super::{history_picker::summary, popup};
use crate::core::{query::QueryDraft, saved::SavedQueries};

/// State of the saved queries picker: the queries with their names, sorted by name, and
/// the highlighted one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SavedPicker {
    queries: Vec<(String, QueryDraft)>,
    selected: usize,
}

/// Messages of the [`SavedPicker`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedMsg {
    /// Highlight the next query, wrapping around.
    Next,
    /// Highlight the previous query, wrapping around.
    Previous,
    /// Pick the highlighted query to load it.
    Pick,
    /// Delete the highlighted query.
    Delete,
}

/// What the user chose in the [`SavedPicker`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedChoice {
    /// Load the query in the editor.
    Load(QueryDraft),
    /// Delete the query with this name.
    Delete(String),
}

impl SavedPicker {
    /// Creates a picker of the saved queries, with the first one highlighted.
    pub fn new(saved: &SavedQueries) -> Self {
        SavedPicker {
            queries: saved
                .iter()
                .map(|(name, query)| (name.to_string(), query.clone()))
                .collect(),
            selected: 0,
        }
    }

    /// Name of the highlighted query.
    pub fn selected(&self) -> Option<&str> {
        self.queries
            .get(self.selected)
            .map(|(name, _)| name.as_str())
    }

    /// Applies a message, returning the choice of the user, if any. A deleted query is
    /// removed from the picker, which stays open.
    pub fn update(&mut self, message: SavedMsg) -> Option<SavedChoice> {
        let len = self.queries.len();
        match message {
            _ if len == 0 => {}
            SavedMsg::Next => self.selected = (self.selected + 1) % len,
            SavedMsg::Previous => self.selected = (self.selected + len - 1) % len,
            SavedMsg::Pick => {
                let (_, query) = &self.queries[self.selected];
                return Some(SavedChoice::Load(query.clone()));
            }
            SavedMsg::Delete => {
                let (name, _) = self.queries.remove(self.selected);
                self.selected = self.selected.min(len.saturating_sub(2));
                return Some(SavedChoice::Delete(name));
            }
        }
        None
    }

    /// Renders the picker over most of the area.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().title(" Saved queries ");
        let height = u16::try_from(self.queries.len().max(1))
            .unwrap_or(u16::MAX)
            .saturating_add(2);
        let width = area.width.saturating_sub(8).max(30);
        let popup = popup::centered(area, width, height);
        frame.render_widget(Clear, popup);

        if self.queries.is_empty() {
            let empty = Paragraph::new(Line::from(" No saved queries, press s to save one").dim());
            frame.render_widget(empty.block(block), popup);
            return;
        }
        let names_width = self
            .queries
            .iter()
            .map(|(name, _)| name.chars().count())
            .max()
            .unwrap_or_default();
        let items = self.queries.iter().map(|(name, query)| {
            let name = format!("{name:names_width$}  ");
            let mut spans = vec![Span::raw(name).bold()];
            spans.extend(summary(query).spans);
            Line::from(spans)
        });
        let list = List::new(items)
            .block(block)
            .highlight_symbol("> ")
            .highlight_style(Style::new().reversed());
        let mut state = ListState::default().with_selected(Some(self.selected));
        frame.render_stateful_widget(list, popup, &mut state);
    }
}

#[cfg(test)]
mod test {
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn picker() -> SavedPicker {
        let mut saved = SavedQueries::default();
        saved.insert(
            "songs",
            QueryDraft::new("select * from music where true").param("hits", "5"),
        );
        saved.insert("all", QueryDraft::new("select * from sources * where true"));
        saved.insert("docs", QueryDraft::new("select * from doc where true"));
        SavedPicker::new(&saved)
    }

    #[test]
    fn test_first_is_highlighted() {
        assert_eq!(picker().selected(), Some("all"));
    }

    #[test]
    fn test_navigation_wraps_around() {
        let mut picker = picker();
        picker.update(SavedMsg::Previous);
        assert_eq!(picker.selected(), Some("songs"));
        picker.update(SavedMsg::Next);
        assert_eq!(picker.selected(), Some("all"));
    }

    #[test]
    fn test_pick() {
        let mut picker = picker();
        picker.update(SavedMsg::Next);
        assert_eq!(
            picker.update(SavedMsg::Pick),
            Some(SavedChoice::Load(QueryDraft::new(
                "select * from doc where true"
            )))
        );
    }

    #[test]
    fn test_delete() {
        let mut picker = picker();
        picker.update(SavedMsg::Previous);
        assert_eq!(
            picker.update(SavedMsg::Delete),
            Some(SavedChoice::Delete("songs".to_string()))
        );
        // The last one was deleted, so the new last one is highlighted
        assert_eq!(picker.selected(), Some("docs"));
        picker.update(SavedMsg::Delete);
        picker.update(SavedMsg::Delete);
        assert_eq!(picker.selected(), None);
        assert_eq!(picker.update(SavedMsg::Delete), None);
    }

    fn render(picker: &SavedPicker) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(70, 8)).unwrap();
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
        assert_snapshot!(render(&SavedPicker::default()).backend());
    }
}
