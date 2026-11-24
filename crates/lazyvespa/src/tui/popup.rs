//! Helpers to place windows over the rest of the UI.

use ratatui::layout::{Constraint, Flex, Layout, Rect};

/// Area of the given size centered in another one, shrunk if it does not fit.
pub(super) fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let [area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    area
}
