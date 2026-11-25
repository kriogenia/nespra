use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Stylize,
    text::Line,
};

use super::{
    Message, Model, help,
    keymap::Binding,
    model::{Notice, Overlay, Pane},
    results, status_bar,
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

    status_bar::render(model.target(), model.health(), frame, header);
    let [editor, results] =
        Layout::vertical([Constraint::Length(8), Constraint::Fill(1)]).areas(main);
    let pane = model.pane();
    model.editor().render(frame, editor, pane == Pane::Editor);
    results::render(
        model.search(),
        model.results(),
        frame,
        results,
        pane == Pane::Results,
    );
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
        _ => frame.render_widget(hints(&active_bindings(model), footer.width), footer),
    }

    match model.overlay() {
        Some(Overlay::Help) => help::render(&active_bindings(model), frame, frame.area()),
        Some(Overlay::TargetPicker(picker)) => picker.render(frame, main),
        Some(Overlay::HistoryPicker(picker)) => picker.render(frame, main),
        Some(Overlay::SavedPicker(picker)) => picker.render(frame, main),
        _ => {}
    }
}

/// Separator between the hints of the footer.
const SEPARATOR: &str = " · ";

/// Suggested key of the active bindings, the most specific first, as many as fit in the
/// width. When some are left out, the last hint is the help, which lists all of them.
fn hints(groups: &[&[Binding]], width: u16) -> Line<'static> {
    let bindings: Vec<&Binding> = groups.iter().flat_map(|bindings| bindings.iter()).collect();
    let all: Vec<String> = bindings
        .iter()
        .filter_map(|binding| hint(binding))
        .collect();
    let available = usize::from(width).saturating_sub(1);
    if all.join(SEPARATOR).chars().count() <= available {
        return hints_line(&all);
    }

    let help = bindings
        .iter()
        .find(|binding| matches!(binding.message, Message::ToggleHelp))
        .and_then(|binding| hint(binding));
    let separator = SEPARATOR.chars().count();
    let reserved = help
        .as_ref()
        .map_or(0, |help| separator + help.chars().count());
    let mut shown = Vec::new();
    let mut used = 0;
    for hint in all.into_iter().filter(|hint| Some(hint) != help.as_ref()) {
        let needed = hint.chars().count() + if shown.is_empty() { 0 } else { separator };
        if used + needed + reserved > available {
            break;
        }
        used += needed;
        shown.push(hint);
    }
    shown.extend(help);
    hints_line(&shown)
}

/// Hint of a binding: its suggested key and what it does.
fn hint(binding: &Binding) -> Option<String> {
    let key = binding.keys.first()?;
    Some(format!("{key} {}", binding.description))
}

fn hints_line(hints: &[String]) -> Line<'static> {
    Line::from(format!(" {}", hints.join(SEPARATOR))).dim()
}

#[cfg(test)]
mod test {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::tui::{params_form::ParamsMsg, query_editor::EditorMsg};

    use crate::core::{
        config::{CloudApplication, Config, Target},
        connection::ActiveTarget,
    };

    fn render(model: &Model) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(90, 12)).unwrap();
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
        let _ = crate::tui::update(&mut model, crate::tui::Message::OpenPalette(""));
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
            crate::tui::Message::Editor(EditorMsg::StartEditing),
        );
        assert_snapshot!(render(&model).backend());
    }

    #[test]
    fn test_view_while_editing_a_param() {
        let mut model = Model::default();
        let _ = crate::tui::update(&mut model, crate::tui::Message::FocusNext);
        for message in [
            EditorMsg::Params(ParamsMsg::Next),
            EditorMsg::StartEditing,
            EditorMsg::Input(KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE)),
        ] {
            let _ = crate::tui::update(&mut model, crate::tui::Message::Editor(message));
        }
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

    #[test]
    fn test_view_with_history_picker() {
        let mut histories = crate::core::history::Histories::default();
        for yql in [
            "select * from doc where true",
            "select * from music where true",
        ] {
            histories.record("local", crate::core::query::QueryDraft::new(yql));
        }
        let target = ActiveTarget::open("local", Target::self_hosted("http://localhost:8080"));
        let mut model = Model::new(Config::default(), Some(target.unwrap()))
            .with_history("history.json".into(), Ok(histories));
        let _ = crate::tui::update(&mut model, crate::tui::Message::OpenHistory);
        assert_snapshot!(render(&model).backend());
    }

    #[test]
    fn test_hints_that_fit() {
        let line = hints(&[crate::tui::keymap::GLOBAL], 200);
        assert_eq!(
            line.to_string(),
            " Ctrl+R run · : command · T targets · H history · s save · S saved · ? help · q quit"
        );
    }

    #[test]
    fn test_hints_end_with_the_help_when_they_do_not_fit() {
        let groups = [crate::tui::keymap::RESULTS, crate::tui::keymap::GLOBAL];
        let line = hints(&groups, 40);
        assert_eq!(line.to_string(), " j next · k previous · ? help");
        assert!(hints(&groups, 10).to_string().ends_with("? help"));
    }

    #[test]
    fn test_view_with_saved_picker() {
        let mut saved = crate::core::saved::SavedQueries::default();
        saved.insert(
            "docs",
            crate::core::query::QueryDraft::new("select * from doc where true"),
        );
        let mut model = Model::default().with_saved_queries("saved.json".into(), Ok(saved));
        let _ = crate::tui::update(&mut model, crate::tui::Message::OpenSaved);
        assert_snapshot!(render(&model).backend());
    }
}
