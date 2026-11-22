//! Editor of the YQL query.
//!
//! It has an explicit edit mode, like vim or lazygit: keys are text only while editing, so
//! the single key shortcuts keep working the rest of the time.

use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::Block,
};
use ratatui_textarea::{CursorMove, TextArea};

/// Query shown when the application starts.
const DEFAULT_YQL: &str = "select * from sources * where true";

/// State of the query editor.
#[derive(Debug, Clone)]
pub struct QueryEditor {
    yql: TextArea<'static>,
    editing: bool,
}

/// Messages of the [`QueryEditor`].
#[derive(Debug, Clone)]
pub enum EditorMsg {
    /// Start typing in the editor.
    StartEditing,
    /// Stop typing in the editor.
    StopEditing,
    /// Key typed while editing.
    Input(KeyEvent),
}

impl Default for QueryEditor {
    fn default() -> Self {
        let mut editor = QueryEditor {
            yql: TextArea::from([DEFAULT_YQL]),
            editing: false,
        };
        editor.yql.move_cursor(CursorMove::Bottom);
        editor.yql.move_cursor(CursorMove::End);
        editor.set_editing(false);
        editor
    }
}

impl QueryEditor {
    /// YQL typed in the editor.
    pub fn yql(&self) -> String {
        self.yql.lines().join("\n")
    }

    /// Whether the keys are typed in the editor.
    pub fn is_editing(&self) -> bool {
        self.editing
    }

    /// Applies a message.
    pub fn update(&mut self, message: EditorMsg) {
        match message {
            EditorMsg::StartEditing => self.set_editing(true),
            EditorMsg::StopEditing => self.set_editing(false),
            EditorMsg::Input(key) if self.editing => {
                self.yql.input(key);
            }
            EditorMsg::Input(_) => {}
        }
    }

    /// Switches the edit mode, showing the cursor only while editing.
    fn set_editing(&mut self, editing: bool) {
        self.editing = editing;
        let cursor = if editing {
            Style::new().reversed()
        } else {
            Style::new()
        };
        self.yql.set_cursor_style(cursor);
        self.yql.set_cursor_line_style(Style::new());
    }

    /// Renders the editor in the area.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = if self.editing {
            Block::bordered()
                .title(" Query · editing ")
                .border_style(Style::new().fg(Color::Yellow))
        } else {
            Block::bordered().title(" Query ")
        };
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(&self.yql, inner);
        if !self.editing {
            // Dims the text to show it is not being edited
            frame.buffer_mut().set_style(inner, Style::new().dim());
        }
    }
}

#[cfg(test)]
mod test {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;

    fn key(code: KeyCode) -> EditorMsg {
        EditorMsg::Input(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn test_default_query() {
        let editor = QueryEditor::default();
        assert_eq!(editor.yql(), DEFAULT_YQL);
        assert!(!editor.is_editing());
    }

    #[test]
    fn test_keys_are_ignored_out_of_the_edit_mode() {
        let mut editor = QueryEditor::default();
        editor.update(key(KeyCode::Char('x')));
        assert_eq!(editor.yql(), DEFAULT_YQL);
    }

    #[test]
    fn test_editing() {
        let mut editor = QueryEditor::default();
        editor.update(EditorMsg::StartEditing);
        editor.update(key(KeyCode::Enter));
        "limit 5"
            .chars()
            .for_each(|c| editor.update(key(KeyCode::Char(c))));
        editor.update(EditorMsg::StopEditing);
        assert_eq!(editor.yql(), format!("{DEFAULT_YQL}\nlimit 5"));
        assert!(!editor.is_editing());
    }
}
