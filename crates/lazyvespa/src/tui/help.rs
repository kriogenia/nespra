//! Overlay listing the key bindings.

use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

use super::keymap::Binding;

/// Renders the bindings over the area, the most specific group first.
pub(super) fn render(groups: &[&[Binding]], frame: &mut Frame, area: Rect) {
    let lines: Vec<Line> = groups
        .iter()
        .flat_map(|bindings| bindings.iter())
        .map(|binding| {
            let keys = binding
                .keys
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            Line::from(vec![
                Span::from(format!("{keys:>10}")).bold(),
                Span::from("  "),
                Span::from(binding.description),
            ])
        })
        .collect();

    let height = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_add(2);
    let popup = centered(area, 36, height);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(" Keys ")),
        popup,
    );
}

/// Area of the given size centered in another one, shrunk if it does not fit.
fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let [area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    area
}
