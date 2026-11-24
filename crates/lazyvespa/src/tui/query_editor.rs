//! Editor of the query: the YQL and the request parameters.
//!
//! It has an explicit edit mode, like vim or lazygit: keys are text only while editing, so
//! the single key shortcuts keep working the rest of the time.

use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::Block,
};
use ratatui_textarea::{CursorMove, TextArea};

use super::params_form::{ParamsForm, ParamsMsg};
use crate::core::query::QueryDraft;

/// Width of the parameters form.
const PARAMS_WIDTH: u16 = 40;

/// Query shown when the application starts.
const DEFAULT_YQL: &str = "select * from sources * where true";

/// State of the query editor.
#[derive(Debug, Clone)]
pub struct QueryEditor {
    yql: TextArea<'static>,
    params: ParamsForm,
    focus: Focus,
    editing: bool,
}

/// Part of the editor receiving the keys.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Focus {
    /// The YQL.
    #[default]
    Yql,
    /// The parameters form.
    Params,
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
    /// Move the focus to a part of the editor.
    Focus(Focus),
    /// Add a parameter and start editing its name.
    AddParam,
    /// Message of the parameters form.
    Params(ParamsMsg),
}

impl Default for QueryEditor {
    fn default() -> Self {
        let mut editor = QueryEditor {
            yql: TextArea::from([DEFAULT_YQL]),
            params: ParamsForm::default(),
            focus: Focus::default(),
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

    /// Query of the editor, to be sent.
    pub fn draft(&self) -> QueryDraft {
        QueryDraft {
            yql: self.yql(),
            params: self
                .params
                .params()
                .iter()
                .map(|param| (param.name.clone(), param.value.clone()))
                .collect(),
        }
    }

    /// Replaces the query of the editor, leaving the edit mode.
    pub fn load(&mut self, query: &QueryDraft) {
        self.yql = TextArea::from(query.yql.lines());
        self.yql.move_cursor(CursorMove::Bottom);
        self.yql.move_cursor(CursorMove::End);
        self.params = ParamsForm::with_params(
            query
                .params
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_str())),
        );
        self.focus = Focus::Yql;
        self.set_editing(false);
    }

    /// Parameters form.
    pub fn params(&self) -> &ParamsForm {
        &self.params
    }

    /// Part of the editor receiving the keys.
    pub fn focus(&self) -> Focus {
        self.focus
    }

    /// Whether the keys are typed in the editor.
    pub fn is_editing(&self) -> bool {
        self.editing
    }

    /// Applies a message.
    pub fn update(&mut self, message: EditorMsg) {
        match message {
            EditorMsg::StartEditing => {
                if self.focus == Focus::Params {
                    self.params.start_editing();
                }
                self.set_editing(true);
            }
            EditorMsg::StopEditing => self.set_editing(false),
            EditorMsg::Input(key) if self.editing => match self.focus {
                Focus::Yql => {
                    self.yql.input(key);
                }
                Focus::Params => self.params.input(&key),
            },
            EditorMsg::Input(_) => {}
            EditorMsg::Focus(focus) if !self.editing => self.focus = focus,
            EditorMsg::Focus(_) => {}
            EditorMsg::AddParam => {
                self.focus = Focus::Params;
                self.params.add();
                self.set_editing(true);
            }
            EditorMsg::Params(message) => self.params.update(message),
        }
    }

    /// Switches the edit mode, showing the cursor only while editing.
    fn set_editing(&mut self, editing: bool) {
        self.editing = editing;
        let cursor = if editing && self.focus == Focus::Yql {
            Style::new().reversed()
        } else {
            Style::new()
        };
        self.yql.set_cursor_style(cursor);
        self.yql.set_cursor_line_style(Style::new());
    }

    /// Renders the editor in the area: the YQL and, at its right, the parameters. The
    /// part with the focus is highlighted when the editor is focused.
    pub fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let [yql, params] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(PARAMS_WIDTH)]).areas(area);

        let block = self.block(Focus::Yql, " Query ", focused);
        let inner = block.inner(yql);
        frame.render_widget(block, yql);
        frame.render_widget(&self.yql, inner);
        if !(self.editing && self.focus == Focus::Yql) {
            // Dims the text to show it is not being edited
            frame.buffer_mut().set_style(inner, Style::new().dim());
        }

        let focused = focused && self.focus == Focus::Params;
        let block = self.block(Focus::Params, " Parameters ", focused);
        self.params
            .render(frame, params, block, focused, focused && self.editing);
    }

    /// Border of a part of the editor, highlighted when it has the focus.
    fn block(&self, part: Focus, title: &'static str, focused: bool) -> Block<'static> {
        match (focused && self.focus == part, self.editing) {
            (true, true) => Block::bordered()
                .title(format!("{title}· editing "))
                .border_style(Style::new().fg(Color::Yellow)),
            (true, false) => Block::bordered()
                .title(title)
                .border_style(Style::new().fg(Color::Cyan)),
            (false, _) => Block::bordered().title(title),
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
    fn test_load_a_query() {
        let mut editor = QueryEditor::default();
        editor.update(EditorMsg::Focus(Focus::Params));
        editor.update(EditorMsg::StartEditing);
        let query = QueryDraft::new("select *\nfrom music")
            .param("hits", "5")
            .param("timeout", "1s");

        editor.load(&query);

        assert_eq!(editor.yql(), "select *\nfrom music");
        assert_eq!(editor.draft().trimmed(), query);
        assert_eq!((editor.focus(), editor.is_editing()), (Focus::Yql, false));
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
    fn test_draft() {
        let mut editor = QueryEditor::default();
        editor.update(EditorMsg::Focus(Focus::Params));
        editor.update(EditorMsg::Params(ParamsMsg::Next));
        editor.update(EditorMsg::StartEditing);
        editor.update(key(KeyCode::Char('3')));
        editor.update(EditorMsg::StopEditing);
        let draft = editor.draft();
        assert_eq!(draft.yql, DEFAULT_YQL);
        assert_eq!(draft.params[1], ("hits".to_string(), "3".to_string()));
        assert_eq!(draft.request().unwrap().hits, Some(3));
    }

    #[test]
    fn test_focus_does_not_move_while_editing() {
        let mut editor = QueryEditor::default();
        editor.update(EditorMsg::StartEditing);
        editor.update(EditorMsg::Focus(Focus::Params));
        assert_eq!(editor.focus(), Focus::Yql);
        editor.update(EditorMsg::StopEditing);
        editor.update(EditorMsg::Focus(Focus::Params));
        assert_eq!(editor.focus(), Focus::Params);
    }

    #[test]
    fn test_add_a_param() {
        let mut editor = QueryEditor::default();
        editor.update(EditorMsg::AddParam);
        assert_eq!(editor.focus(), Focus::Params);
        assert!(editor.is_editing());
        "offset"
            .chars()
            .for_each(|c| editor.update(key(KeyCode::Char(c))));
        assert_eq!(editor.params().params()[3].name, "offset");
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
