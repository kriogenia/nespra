use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};

use super::Model;

/// Renders the model. It only reads the state, never changes it.
pub fn view(_model: &Model, frame: &mut Frame) {
    let [main, footer] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

    let title = Paragraph::new("lazyvespa")
        .bold()
        .alignment(Alignment::Center)
        .block(Block::bordered());
    frame.render_widget(title, main);
    frame.render_widget(Line::from(" q quit").dim(), footer);
}

#[cfg(test)]
mod test {
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn test_initial_view() {
        let mut terminal = Terminal::new(TestBackend::new(40, 6)).unwrap();
        terminal
            .draw(|frame| view(&Model::default(), frame))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
