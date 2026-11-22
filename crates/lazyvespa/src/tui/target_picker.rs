//! Overlay to pick one of the configured targets.

use ratatui::{
    Frame,
    layout::Rect,
    style::Stylize,
    text::Line,
    widgets::{Block, Clear, List, ListState, Paragraph},
};

/// State of the target picker: the names of the targets and the highlighted one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TargetPicker {
    names: Vec<String>,
    current: Option<String>,
    selected: usize,
}

/// Messages of the [`TargetPicker`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerMsg {
    /// Highlight the next target, wrapping around.
    Next,
    /// Highlight the previous target, wrapping around.
    Previous,
    /// Pick the highlighted target.
    Pick,
}

impl TargetPicker {
    /// Creates a picker of the targets with the current one highlighted.
    pub fn new(names: Vec<String>, current: Option<&str>) -> Self {
        let selected = current
            .and_then(|current| names.iter().position(|name| name == current))
            .unwrap_or_default();
        TargetPicker {
            names,
            current: current.map(str::to_string),
            selected,
        }
    }

    /// Name of the highlighted target.
    pub fn selected(&self) -> Option<&str> {
        self.names.get(self.selected).map(String::as_str)
    }

    /// Applies a message. Picking returns the name of the highlighted target.
    pub fn update(&mut self, message: PickerMsg) -> Option<String> {
        let len = self.names.len();
        match message {
            _ if len == 0 => {}
            PickerMsg::Next => self.selected = (self.selected + 1) % len,
            PickerMsg::Previous => self.selected = (self.selected + len - 1) % len,
            PickerMsg::Pick => return self.selected().map(str::to_string),
        }
        None
    }

    /// Renders the picker centered in the area.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().title(" Targets ");
        let height = u16::try_from(self.names.len().max(1))
            .unwrap_or(u16::MAX)
            .saturating_add(2);
        let width = self
            .names
            .iter()
            .map(|name| name.len() + 4)
            .chain([30])
            .max()
            .and_then(|width| u16::try_from(width).ok())
            .unwrap_or(u16::MAX);
        let popup = super::popup::centered(area, width, height);
        frame.render_widget(Clear, popup);

        if self.names.is_empty() {
            let empty = Paragraph::new(Line::from(" No targets configured").dim());
            frame.render_widget(empty.block(block), popup);
            return;
        }
        let items = self.names.iter().map(|name| {
            if self.current.as_ref() == Some(name) {
                Line::from(format!("{name} ●")).bold()
            } else {
                Line::from(name.as_str())
            }
        });
        let list = List::new(items)
            .block(block)
            .highlight_symbol("> ")
            .highlight_style(ratatui::style::Style::new().reversed());
        let mut state = ListState::default().with_selected(Some(self.selected));
        frame.render_stateful_widget(list, popup, &mut state);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn picker(current: Option<&str>) -> TargetPicker {
        let names = ["local", "prod", "staging"].map(String::from).to_vec();
        TargetPicker::new(names, current)
    }

    #[test]
    fn test_current_target_is_highlighted() {
        assert_eq!(picker(Some("prod")).selected(), Some("prod"));
        assert_eq!(picker(None).selected(), Some("local"));
        assert_eq!(picker(Some("url")).selected(), Some("local"));
    }

    #[test]
    fn test_navigation_wraps_around() {
        let mut picker = picker(Some("staging"));
        picker.update(PickerMsg::Next);
        assert_eq!(picker.selected(), Some("local"));
        picker.update(PickerMsg::Previous);
        assert_eq!(picker.selected(), Some("staging"));
        picker.update(PickerMsg::Previous);
        assert_eq!(picker.selected(), Some("prod"));
    }

    #[test]
    fn test_pick() {
        let mut picker = picker(None);
        picker.update(PickerMsg::Next);
        assert_eq!(picker.update(PickerMsg::Pick), Some("prod".to_string()));
    }

    #[test]
    fn test_empty_picker() {
        let mut picker = TargetPicker::new(Vec::new(), None);
        assert_eq!(picker.update(PickerMsg::Next), None);
        assert_eq!(picker.update(PickerMsg::Pick), None);
        assert_eq!(picker.selected(), None);
    }
}
