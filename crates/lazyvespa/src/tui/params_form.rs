//! Form with the request parameters of the query, as `name = value` rows.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Cell, Row, Table, TableState},
};

/// Parameters shown when the application starts, without values.
const DEFAULT_PARAMS: [&str; 3] = ["query", "hits", "ranking.profile"];

/// Width of the column of the names.
const NAME_WIDTH: u16 = 16;

/// State of the parameters form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamsForm {
    rows: Vec<Param>,
    selected: usize,
    field: Field,
}

/// A request parameter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Param {
    /// Name, like `ranking.profile`.
    pub name: String,
    /// Value, unset when blank.
    pub value: String,
}

/// Part of a row being edited.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Field {
    /// The name of the parameter.
    Name,
    /// The value of the parameter.
    #[default]
    Value,
}

/// Messages of the [`ParamsForm`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamsMsg {
    /// Select the next row, wrapping around.
    Next,
    /// Select the previous row, wrapping around.
    Previous,
    /// Delete the selected row.
    Delete,
    /// Switch between editing the name and the value of the row.
    SwitchField,
}

impl Default for ParamsForm {
    fn default() -> Self {
        ParamsForm {
            rows: DEFAULT_PARAMS
                .map(|name| Param {
                    name: name.to_string(),
                    value: String::new(),
                })
                .to_vec(),
            selected: 0,
            field: Field::default(),
        }
    }
}

impl ParamsForm {
    /// Rows of the form, in order.
    pub fn params(&self) -> &[Param] {
        &self.rows
    }

    /// Index of the selected row, `None` when there are no rows.
    pub fn selected(&self) -> Option<usize> {
        (!self.rows.is_empty()).then_some(self.selected)
    }

    /// Field of the selected row that is edited.
    pub fn field(&self) -> Field {
        self.field
    }

    /// Applies a message.
    pub fn update(&mut self, message: ParamsMsg) {
        let len = self.rows.len();
        match message {
            _ if len == 0 => {}
            ParamsMsg::Next => self.selected = (self.selected + 1) % len,
            ParamsMsg::Previous => self.selected = (self.selected + len - 1) % len,
            ParamsMsg::Delete => {
                self.rows.remove(self.selected);
                self.selected = self.selected.min(len.saturating_sub(2));
            }
            ParamsMsg::SwitchField => {
                self.field = match self.field {
                    Field::Name => Field::Value,
                    Field::Value => Field::Name,
                }
            }
        }
    }

    /// Adds an empty row after the others and selects its name to be edited.
    pub fn add(&mut self) {
        self.rows.push(Param::default());
        self.selected = self.rows.len() - 1;
        self.field = Field::Name;
    }

    /// Starts editing the value of the selected row, adding one if there is none.
    pub fn start_editing(&mut self) {
        if self.rows.is_empty() {
            self.add();
        } else {
            self.field = Field::Value;
        }
    }

    /// Types a key in the edited field of the selected row.
    pub fn input(&mut self, key: &KeyEvent) {
        let Some(row) = self.rows.get_mut(self.selected) else {
            return;
        };
        let text = match self.field {
            Field::Name => &mut row.name,
            Field::Value => &mut row.value,
        };
        match key.code {
            KeyCode::Char(c) if key.modifiers - KeyModifiers::SHIFT == KeyModifiers::NONE => {
                text.push(c);
            }
            KeyCode::Backspace => {
                text.pop();
            }
            _ => {}
        }
    }

    /// Renders the form. The selected row is highlighted when focused, and its edited field
    /// gets the cursor while editing.
    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        block: Block,
        focused: bool,
        editing: bool,
    ) {
        let rows = self.rows.iter().enumerate().map(|(index, param)| {
            let mut name = Cell::from(param.name.as_str()).bold();
            let mut value = Cell::from(param.value.as_str());
            if editing && index == self.selected {
                let edited = Style::new().reversed();
                match self.field {
                    Field::Name => name = name.style(edited),
                    Field::Value => value = value.style(edited),
                }
            }
            Row::new([name, value])
        });
        let highlight = if focused && !editing {
            Style::new().reversed()
        } else {
            Style::new()
        };
        let table = Table::new(rows, [Constraint::Length(NAME_WIDTH), Constraint::Fill(1)])
            .block(block)
            .row_highlight_style(highlight);

        if self.rows.is_empty() {
            let empty = Line::from(" a to add a parameter").dim();
            frame.render_widget(table, area);
            let inner = Rect {
                x: area.x + 1,
                y: area.y + 1,
                width: area.width.saturating_sub(2),
                height: 1,
            };
            frame.render_widget(empty, inner);
            return;
        }
        let mut state = TableState::default().with_selected(Some(self.selected));
        frame.render_stateful_widget(table, area, &mut state);
        if editing {
            self.place_cursor(frame, area, state.offset());
        }
    }

    /// Shows the terminal cursor after the text of the edited field.
    fn place_cursor(&self, frame: &mut Frame, area: Rect, offset: usize) {
        let Some(row) = self.rows.get(self.selected) else {
            return;
        };
        let (column, text) = match self.field {
            Field::Name => (0, &row.name),
            // The columns are separated by a space
            Field::Value => (NAME_WIDTH + 1, &row.value),
        };
        let width = u16::try_from(text.chars().count()).unwrap_or(u16::MAX);
        let line = u16::try_from(self.selected.saturating_sub(offset)).unwrap_or(u16::MAX);
        // Inside the borders
        let x = area.x.saturating_add(1 + column).saturating_add(width);
        let y = area.y.saturating_add(1).saturating_add(line);
        frame.set_cursor_position((x.min(area.right().saturating_sub(2)), y));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn type_text(form: &mut ParamsForm, text: &str) {
        for c in text.chars() {
            form.input(&KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
    }

    fn values(form: &ParamsForm) -> Vec<(&str, &str)> {
        form.params()
            .iter()
            .map(|p| (p.name.as_str(), p.value.as_str()))
            .collect()
    }

    #[test]
    fn test_default_rows() {
        let form = ParamsForm::default();
        assert_eq!(
            values(&form),
            [("query", ""), ("hits", ""), ("ranking.profile", "")]
        );
        assert_eq!(form.selected(), Some(0));
    }

    #[test]
    fn test_edit_the_value_of_the_selected_row() {
        let mut form = ParamsForm::default();
        form.update(ParamsMsg::Next);
        form.start_editing();
        type_text(&mut form, "50");
        form.input(&KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(values(&form)[1], ("hits", "5"));
    }

    #[test]
    fn test_add_a_row_editing_its_name() {
        let mut form = ParamsForm::default();
        form.add();
        type_text(&mut form, "timeout");
        form.update(ParamsMsg::SwitchField);
        type_text(&mut form, "1s");
        assert_eq!(form.selected(), Some(3));
        assert_eq!(values(&form)[3], ("timeout", "1s"));
    }

    #[test]
    fn test_navigation_wraps_around() {
        let mut form = ParamsForm::default();
        form.update(ParamsMsg::Previous);
        assert_eq!(form.selected(), Some(2));
        form.update(ParamsMsg::Next);
        assert_eq!(form.selected(), Some(0));
    }

    #[test]
    fn test_delete_rows() {
        let mut form = ParamsForm::default();
        form.update(ParamsMsg::Previous);
        form.update(ParamsMsg::Delete);
        assert_eq!(form.selected(), Some(1));
        form.update(ParamsMsg::Delete);
        form.update(ParamsMsg::Delete);
        assert_eq!(form.selected(), None);
        form.update(ParamsMsg::Delete);
        form.update(ParamsMsg::Next);
        assert!(form.params().is_empty());
    }

    #[test]
    fn test_editing_without_rows_adds_one() {
        let mut form = ParamsForm::default();
        (0..3).for_each(|_| form.update(ParamsMsg::Delete));
        form.start_editing();
        assert_eq!(form.params().len(), 1);
        assert_eq!(form.field(), Field::Name);
    }
}
