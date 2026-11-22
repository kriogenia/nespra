use crossterm::event::KeyEvent;

use super::{
    Effect, Message, Model,
    keymap::{self, Binding},
    model::{Notice, Overlay},
    palette::{self, Command},
    query_editor::{EditorMsg, Focus},
};

/// Applies a message to the model. It is the only place where the state changes, and it
/// does not run side effects: it returns them to be run by the caller.
#[must_use = "the effects must be run"]
pub fn update(model: &mut Model, message: Message) -> Vec<Effect> {
    match message {
        Message::Key(key) => {
            model.clear_notice();
            if let Some(message) = key_message(model, &key) {
                return update(model, message);
            }
        }
        Message::ToggleHelp => model.toggle_help(),
        Message::OpenPalette => model.open_palette(),
        Message::Palette(message) => {
            let Some(result) = model.palette_mut().and_then(|p| p.update(message)) else {
                return Vec::new();
            };
            model.close_overlay();
            match result {
                Ok(command) => return update(model, command_message(command)),
                Err(error) => model.set_notice(Notice::Error(error.to_string())),
            }
        }
        Message::Editor(message) => model.editor_mut().update(message),
        Message::OpenTargetPicker => model.open_target_picker(),
        Message::Picker(message) => {
            let Some(name) = model.target_picker_mut().and_then(|p| p.update(message)) else {
                return Vec::new();
            };
            model.close_overlay();
            return update(model, Message::SwitchTarget(name));
        }
        Message::SwitchTarget(name) => match model.config().target(&name) {
            Some(target) => {
                let target = target.clone();
                return vec![Effect::OpenTarget { name, target }];
            }
            None => model.set_notice(Notice::Error(format!("There is no target named {name:?}"))),
        },
        Message::TargetOpened(Ok(target)) => {
            model.set_notice(Notice::Info(format!("Connected to {}", target.name)));
            model.set_target(*target);
        }
        Message::TargetOpened(Err(error)) => model.set_notice(Notice::Error(error)),
        Message::RunQuery => return run_query(model),
        Message::SearchDone { id, result } => {
            let notice = match &result {
                Ok(outcome) => Notice::Info(format!(
                    "{} hits · {} ms",
                    outcome.response.total_count(),
                    outcome.elapsed.as_millis()
                )),
                Err(error) => Notice::Error(error.clone()),
            };
            if model.finish_search(id, result) {
                model.set_notice(notice);
            }
        }
        Message::CloseOverlay => model.close_overlay(),
        Message::Quit => model.quit(),
    }
    Vec::new()
}

/// Searches the query of the editor in the target.
fn run_query(model: &mut Model) -> Vec<Effect> {
    let Some(vespa) = model.target().map(|active| active.connection.vespa.clone()) else {
        model.set_notice(Notice::Error(
            "Not connected to a target, press T to pick one".to_string(),
        ));
        return Vec::new();
    };
    match model.editor().draft().request() {
        Ok(request) => {
            let id = model.start_search();
            model.set_notice(Notice::Info("Searching…".to_string()));
            vec![Effect::Search { id, vespa, request }]
        }
        Err(error) => {
            model.set_notice(Notice::Error(error.to_string()));
            Vec::new()
        }
    }
}

/// Message running the command of the palette.
fn command_message(command: Command) -> Message {
    match command {
        Command::Help => Message::ToggleHelp,
        Command::Quit => Message::Quit,
        Command::Context(Some(name)) => Message::SwitchTarget(name),
        Command::Context(None) => Message::OpenTargetPicker,
    }
}

/// Bindings active in the current state, the most specific first.
pub(super) fn active_bindings(model: &Model) -> Vec<&'static [Binding]> {
    match model.overlay() {
        Some(Overlay::TargetPicker(_)) => {
            vec![keymap::TARGET_PICKER, keymap::OVERLAY, keymap::GLOBAL]
        }
        Some(_) => vec![keymap::OVERLAY, keymap::GLOBAL],
        None => {
            let editor = model.editor();
            match (editor.focus(), editor.is_editing()) {
                (Focus::Yql, true) => vec![keymap::EDITING],
                (Focus::Params, true) => vec![keymap::EDITING_PARAM, keymap::EDITING],
                (Focus::Yql, false) => vec![keymap::EDITOR_YQL, keymap::GLOBAL],
                (Focus::Params, false) => vec![keymap::EDITOR_PARAMS, keymap::GLOBAL],
            }
        }
    }
}

/// Message of the key in the current state. While typing in the palette or the editor,
/// the keys that are not bound are text.
fn key_message(model: &Model, key: &KeyEvent) -> Option<Message> {
    match model.overlay() {
        Some(Overlay::Palette(_)) => keymap::message(&[keymap::OVERLAY], key)
            .or_else(|| palette::key_message(key).map(Message::Palette))
            .or_else(|| keymap::message(&[keymap::GLOBAL], key)),
        None if model.editor().is_editing() => keymap::message(&active_bindings(model), key)
            .or(Some(Message::Editor(EditorMsg::Input(*key)))),
        _ => keymap::message(&active_bindings(model), key),
    }
}

#[cfg(test)]
mod test {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;
    use std::{sync::Arc, time::Duration};

    use crate::tui::model::SearchState;

    use crate::core::{
        config::{Config, Target},
        connection::ActiveTarget,
        search::SearchOutcome,
    };

    fn press(model: &mut Model, code: KeyCode) -> Vec<Effect> {
        update(model, Message::Key(KeyEvent::new(code, KeyModifiers::NONE)))
    }

    fn type_text(model: &mut Model, text: &str) -> Vec<Effect> {
        text.chars()
            .flat_map(|c| press(model, KeyCode::Char(c)))
            .collect()
    }

    fn run_command(model: &mut Model, command: &str) -> Vec<Effect> {
        let mut effects = type_text(model, &format!(":{command}"));
        effects.extend(press(model, KeyCode::Enter));
        effects
    }

    fn config() -> Config {
        Config::from_toml(
            "[targets.local]\nendpoint = \"http://localhost:8080\"\n\
             [targets.prod]\nendpoint = \"https://prod:443\"",
        )
        .unwrap()
    }

    #[test]
    fn test_quit() {
        let mut model = Model::default();
        assert!(model.is_running());
        assert!(update(&mut model, Message::Quit).is_empty());
        assert!(!model.is_running());
    }

    #[test]
    fn test_keys_trigger_their_message() {
        let mut model = Model::default();
        let _ = press(&mut model, KeyCode::Char('q'));
        assert!(!model.is_running());
    }

    #[test]
    fn test_toggle_help() {
        let mut model = Model::default();
        let _ = press(&mut model, KeyCode::Char('?'));
        assert_eq!(model.overlay(), Some(&Overlay::Help));
        let _ = press(&mut model, KeyCode::Char('?'));
        assert_eq!(model.overlay(), None);
    }

    #[test]
    fn test_escape_closes_the_help() {
        let mut model = Model::default();
        let _ = press(&mut model, KeyCode::Esc);
        assert_eq!(model.overlay(), None);
        let _ = press(&mut model, KeyCode::Char('?'));
        let _ = press(&mut model, KeyCode::Esc);
        assert_eq!(model.overlay(), None);
        assert!(model.is_running());
    }

    #[test]
    fn test_keys_are_text_in_the_palette() {
        let mut model = Model::default();
        let _ = type_text(&mut model, ":q?");
        let Some(Overlay::Palette(palette)) = model.overlay() else {
            panic!("the palette must be open");
        };
        assert_eq!(palette.input(), "q?");
        assert!(model.is_running());
    }

    #[test]
    fn test_run_a_command() {
        let mut model = Model::default();
        let _ = run_command(&mut model, "quit");
        assert!(!model.is_running());
    }

    #[test]
    fn test_invalid_command_is_noticed_until_the_next_key() {
        let mut model = Model::default();
        let _ = run_command(&mut model, "nope");
        assert_eq!(model.overlay(), None);
        assert_eq!(
            model.notice(),
            Some(&Notice::Error("Unknown command \"nope\"".to_string()))
        );
        let _ = press(&mut model, KeyCode::Char('x'));
        assert_eq!(model.notice(), None);
    }

    #[test]
    fn test_escape_closes_the_palette() {
        let mut model = Model::default();
        let _ = type_text(&mut model, ":qui");
        let _ = press(&mut model, KeyCode::Esc);
        assert_eq!(model.overlay(), None);
        assert!(model.is_running());
    }

    #[test]
    fn test_switch_to_a_target_opens_it() {
        let mut model = Model::new(config(), None);
        let effects = run_command(&mut model, "ctx local");
        assert!(matches!(
            effects.as_slice(),
            [Effect::OpenTarget { name, target }]
                if name == "local" && *target == Target::self_hosted("http://localhost:8080")
        ));
        assert!(model.target().is_none(), "connected only once it is opened");
    }

    #[test]
    fn test_switch_to_an_unknown_target() {
        let mut model = Model::new(config(), None);
        assert!(run_command(&mut model, "ctx staging").is_empty());
        assert!(matches!(model.notice(), Some(Notice::Error(_))));
    }

    #[test]
    fn test_pick_a_target() {
        let mut model = Model::new(config(), None);
        let _ = press(&mut model, KeyCode::Char('T'));
        assert!(matches!(model.overlay(), Some(Overlay::TargetPicker(_))));
        let _ = press(&mut model, KeyCode::Char('j'));
        let effects = press(&mut model, KeyCode::Enter);
        assert_eq!(model.overlay(), None);
        assert!(matches!(
            effects.as_slice(),
            [Effect::OpenTarget { name, .. }] if name == "prod"
        ));
    }

    #[test]
    fn test_ctx_without_name_opens_the_picker() {
        let mut model = Model::new(config(), None);
        assert!(run_command(&mut model, "ctx").is_empty());
        assert!(matches!(model.overlay(), Some(Overlay::TargetPicker(_))));
    }

    #[test]
    fn test_edit_the_query() {
        let mut model = Model::default();
        let _ = press(&mut model, KeyCode::Char('i'));
        assert!(model.editor().is_editing());
        // Shortcuts are text while editing
        let _ = type_text(&mut model, " q?T");
        let _ = press(&mut model, KeyCode::Esc);
        assert!(!model.editor().is_editing());
        assert!(model.is_running());
        assert!(
            model.editor().yql().ends_with(" q?T"),
            "{}",
            model.editor().yql()
        );
        // and shortcuts again after editing
        let _ = press(&mut model, KeyCode::Char('q'));
        assert!(!model.is_running());
    }

    #[test]
    fn test_edit_a_param() {
        let mut model = Model::default();
        let _ = press(&mut model, KeyCode::Tab);
        let _ = type_text(&mut model, "j");
        let _ = press(&mut model, KeyCode::Enter);
        let _ = type_text(&mut model, "7");
        let _ = press(&mut model, KeyCode::Esc);
        let _ = type_text(&mut model, "a");
        let _ = type_text(&mut model, "offset");
        let _ = press(&mut model, KeyCode::Tab);
        let _ = type_text(&mut model, "2");
        let _ = press(&mut model, KeyCode::Esc);
        let request = model.editor().draft().request().unwrap();
        assert_eq!((request.hits, request.offset), (Some(7), Some(2)));
        assert!(model.is_running());
    }

    #[test]
    fn test_ctrl_c_quits_while_editing() {
        let mut model = Model::default();
        let _ = press(&mut model, KeyCode::Enter);
        let _ = update(
            &mut model,
            Message::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        );
        assert!(!model.is_running());
    }

    fn connected() -> Model {
        let target = ActiveTarget::open("local", Target::self_hosted("http://localhost:8080"));
        Model::new(config(), Some(target.unwrap()))
    }

    fn outcome(total: i32) -> Arc<SearchOutcome> {
        let response = serde_json::from_value(serde_json::json!({
            "root": {"relevance": 1.0, "fields": {"totalCount": total}}
        }));
        Arc::new(SearchOutcome {
            response: response.unwrap(),
            elapsed: Duration::from_millis(12),
        })
    }

    #[test]
    fn test_run_the_query() {
        let mut model = connected();
        let effects = update(&mut model, Message::RunQuery);
        let [Effect::Search { id, request, .. }] = effects.as_slice() else {
            panic!("a search must be run, got {effects:?}");
        };
        assert_eq!(request.yql, model.editor().yql());
        assert!(matches!(model.search(), SearchState::Running(running) if running == id));

        let id = *id;
        let _ = update(
            &mut model,
            Message::SearchDone {
                id,
                result: Ok(outcome(3)),
            },
        );
        assert!(matches!(model.search(), SearchState::Done(_)));
        assert_eq!(
            model.notice(),
            Some(&Notice::Info("3 hits · 12 ms".to_string()))
        );
    }

    #[test]
    fn test_run_with_ctrl_r_while_editing() {
        let mut model = connected();
        let _ = press(&mut model, KeyCode::Char('i'));
        let effects = update(
            &mut model,
            Message::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
        );
        assert!(matches!(effects.as_slice(), [Effect::Search { .. }]));
        assert!(model.editor().is_editing());
    }

    #[test]
    fn test_answers_of_older_searches_are_ignored() {
        let mut model = connected();
        let _ = update(&mut model, Message::RunQuery);
        let _ = update(&mut model, Message::RunQuery);
        let _ = update(
            &mut model,
            Message::SearchDone {
                id: 1,
                result: Ok(outcome(1)),
            },
        );
        assert!(matches!(model.search(), SearchState::Running(2)));
        let error = Err("Search failed: timeout".to_string());
        let _ = update(
            &mut model,
            Message::SearchDone {
                id: 2,
                result: error,
            },
        );
        assert!(matches!(model.search(), SearchState::Failed(_)));
        assert!(matches!(model.notice(), Some(Notice::Error(_))));
    }

    #[test]
    fn test_run_without_target() {
        let mut model = Model::new(config(), None);
        assert!(update(&mut model, Message::RunQuery).is_empty());
        assert!(matches!(model.notice(), Some(Notice::Error(_))));
    }

    #[test]
    fn test_run_an_invalid_query() {
        let mut model = connected();
        let _ = press(&mut model, KeyCode::Tab);
        let _ = press(&mut model, KeyCode::Char('j'));
        let _ = press(&mut model, KeyCode::Enter);
        let _ = type_text(&mut model, "many");
        assert!(update(&mut model, Message::RunQuery).is_empty());
        assert!(matches!(model.search(), SearchState::Idle));
        assert!(matches!(model.notice(), Some(Notice::Error(e)) if e.contains("hits")));
    }

    #[test]
    fn test_target_opened() {
        let mut model = Model::new(config(), None);
        let target = ActiveTarget::open("local", Target::self_hosted("http://localhost:8080"));
        let target = Box::new(target.unwrap());
        let _ = update(&mut model, Message::TargetOpened(Ok(target)));
        assert_eq!(model.target().unwrap().name, "local");
        assert_eq!(
            model.notice(),
            Some(&Notice::Info("Connected to local".to_string()))
        );
    }

    #[test]
    fn test_target_failed_to_open_keeps_the_current_one() {
        let current = ActiveTarget::open("local", Target::self_hosted("http://localhost:8080"));
        let mut model = Model::new(config(), Some(current.unwrap()));
        let _ = update(
            &mut model,
            Message::TargetOpened(Err("no cert".to_string())),
        );
        assert_eq!(model.target().unwrap().name, "local");
        assert_eq!(model.notice(), Some(&Notice::Error("no cert".to_string())));
    }
}
