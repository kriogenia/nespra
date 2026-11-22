use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Stylize,
    text::Line,
};

use super::{
    Model, help,
    keymap::Binding,
    model::{Notice, Overlay},
    status_bar,
    update::active_bindings,
};

/// Renders the model. It only reads the state, never changes it.
pub fn view(model: &Model, frame: &mut Frame) {
    let [header, main, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    status_bar::render(model.target(), frame, header);
    let [editor, _results] =
        Layout::vertical([Constraint::Length(8), Constraint::Fill(1)]).areas(main);
    model.editor().render(frame, editor);
    match (model.overlay(), model.notice()) {
        (Some(Overlay::Palette(palette)), _) => {
            let line = format!(":{}", palette.input());
            let cursor_x = footer
                .x
                .saturating_add(u16::try_from(line.len()).unwrap_or(u16::MAX));
            frame.render_widget(Line::from(line), footer);
            frame.set_cursor_position((cursor_x.min(footer.right().saturating_sub(1)), footer.y));
        }
        (_, Some(Notice::Info(text))) => {
            frame.render_widget(Line::from(format!(" {text}")).green(), footer)
        }
        (_, Some(Notice::Error(text))) => {
            frame.render_widget(Line::from(format!(" {text}")).red(), footer)
        }
        _ => frame.render_widget(hints(&active_bindings(model)), footer),
    }

    match model.overlay() {
        Some(Overlay::Help) => help::render(&active_bindings(model), frame, frame.area()),
        Some(Overlay::TargetPicker(picker)) => picker.render(frame, main),
        _ => {}
    }
}

/// Suggested key of each active binding.
fn hints(groups: &[&[Binding]]) -> Line<'static> {
    let hints = groups
        .iter()
        .flat_map(|bindings| bindings.iter())
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

    use crate::core::{
        config::{CloudApplication, Config, Target},
        connection::ActiveTarget,
    };

    fn render(model: &Model) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(70, 12)).unwrap();
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
        let _ = crate::tui::update(&mut model, crate::tui::Message::ToggleHelp);
        assert_snapshot!(render(&model).backend());
    }

    #[test]
    fn test_view_with_palette() {
        let mut model = Model::default();
        let _ = crate::tui::update(&mut model, crate::tui::Message::OpenPalette);
        for c in "ctx".chars() {
            let message = crate::tui::palette::PaletteMsg::Insert(c);
            let _ = crate::tui::update(&mut model, crate::tui::Message::Palette(message));
        }
        assert_snapshot!(render(&model).backend());
    }

    #[test]
    fn test_view_with_target_picker() {
        let config = Config::from_toml(
            "[targets.local]\nendpoint = \"http://localhost:8080\"\n\
             [targets.prod]\nendpoint = \"https://prod:443\"",
        )
        .unwrap();
        let target = ActiveTarget::open("local", Target::self_hosted("http://localhost:8080"));
        let mut model = Model::new(config, Some(target.unwrap()));
        let _ = crate::tui::update(&mut model, crate::tui::Message::OpenTargetPicker);
        assert_snapshot!(render(&model).backend());
    }

    #[test]
    fn test_view_while_editing() {
        let mut model = Model::default();
        let _ = crate::tui::update(
            &mut model,
            crate::tui::Message::Editor(crate::tui::query_editor::EditorMsg::StartEditing),
        );
        assert_snapshot!(render(&model).backend());
    }

    #[test]
    fn test_view_with_target() {
        let target = ActiveTarget::open("local", Target::self_hosted("http://localhost:8080"));
        let model = Model::new(Config::default(), Some(target.unwrap()));
        assert_snapshot!(render(&model).backend());
    }

    #[test]
    fn test_view_with_cloud_target() {
        let mut target = Target::self_hosted("https://abc.vespa-app.cloud");
        target.cloud = Some(CloudApplication {
            tenant: "acme".to_string(),
            application: "search".to_string(),
            instance: "default".to_string(),
        });
        let model = Model::new(
            Config::default(),
            Some(ActiveTarget::open("prod", target).unwrap()),
        );
        assert_snapshot!(render(&model).backend());
    }
}
