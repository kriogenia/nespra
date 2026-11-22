use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};

use super::Model;

/// Renders the model. It only reads the state, never changes it.
pub fn view(model: &Model, frame: &mut Frame) {
    let [main, footer] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

    let target = match model.target() {
        Some(active) => Line::from(format!(" {} · {} ", active.name, active.target.endpoint)),
        None => Line::from(" no target ").dim(),
    };
    let title = Paragraph::new("lazyvespa")
        .bold()
        .alignment(Alignment::Center)
        .block(Block::bordered().title_top(target.right_aligned()));
    frame.render_widget(title, main);
    frame.render_widget(Line::from(" q quit").dim(), footer);
}

#[cfg(test)]
mod test {
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    use crate::core::{config::Target, connection::ActiveTarget};

    fn render(model: &Model) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(50, 6)).unwrap();
        terminal.draw(|frame| view(model, frame)).unwrap();
        terminal
    }

    #[test]
    fn test_view_without_target() {
        assert_snapshot!(render(&Model::default()).backend());
    }

    #[test]
    fn test_view_with_target() {
        let target = ActiveTarget::open("local", Target::self_hosted("http://localhost:8080"));
        let model = Model::new(Some(target.unwrap()));
        assert_snapshot!(render(&model).backend());
    }
}
