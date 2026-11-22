use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};

use super::{Model, help, keymap, model::Overlay, update::active_bindings};

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
    match (model.overlay(), model.notice()) {
        (Some(Overlay::Palette(palette)), _) => {
            let line = format!(":{}", palette.input());
            let cursor_x = footer
                .x
                .saturating_add(u16::try_from(line.len()).unwrap_or(u16::MAX));
            frame.render_widget(Line::from(line), footer);
            frame.set_cursor_position((cursor_x.min(footer.right().saturating_sub(1)), footer.y));
        }
        (_, Some(notice)) => frame.render_widget(Line::from(format!(" {notice}")).red(), footer),
        _ => frame.render_widget(hints(), footer),
    }

    if let Some(Overlay::Help) = model.overlay() {
        help::render(&active_bindings(model), frame, frame.area());
    }
}

/// Suggested key of each global binding.
fn hints() -> Line<'static> {
    let hints = keymap::GLOBAL
        .iter()
        .filter_map(|binding| {
            let key = binding.keys.first()?;
            Some(format!("{key} {}", binding.description))
        })
        .collect::<Vec<_>>()
        .join(" · ");
    Line::from(format!(" {hints}")).dim()
}

#[cfg(test)]
mod test {
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    use crate::core::{config::Target, connection::ActiveTarget};

    fn render(model: &Model) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(50, 8)).unwrap();
        terminal.draw(|frame| view(model, frame)).unwrap();
        terminal
    }

    #[test]
    fn test_view_without_target() {
        assert_snapshot!(render(&Model::default()).backend());
    }

    #[test]
    fn test_view_with_help() {
        let mut model = Model::default();
        crate::tui::update(&mut model, crate::tui::Message::ToggleHelp);
        assert_snapshot!(render(&model).backend());
    }

    #[test]
    fn test_view_with_palette() {
        let mut model = Model::default();
        crate::tui::update(&mut model, crate::tui::Message::OpenPalette);
        for c in "ctx".chars() {
            let message = crate::tui::palette::PaletteMsg::Insert(c);
            crate::tui::update(&mut model, crate::tui::Message::Palette(message));
        }
        assert_snapshot!(render(&model).backend());
    }

    #[test]
    fn test_view_with_target() {
        let target = ActiveTarget::open("local", Target::self_hosted("http://localhost:8080"));
        let model = Model::new(Some(target.unwrap()));
        assert_snapshot!(render(&model).backend());
    }
}
