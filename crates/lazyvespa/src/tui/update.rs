use crossterm::event::KeyEvent;

use super::{
    Effect, Message, Model,
    keymap::{self, Binding},
    model::{Notice, Overlay, Pane},
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
        Message::OpenPalette(input) => model.open_palette(input),
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
        Message::Results(message) => model.update_results(message),
        Message::FocusNext => focus_next(model),
        Message::OpenTargetPicker => model.open_target_picker(),
        Message::Picker(message) => {
            let Some(name) = model.target_picker_mut().and_then(|p| p.update(message)) else {
                return Vec::new();
            };
            model.close_overlay();
            return update(model, Message::SwitchTarget(name));
        }
        Message::OpenHistory => model.open_history_picker(),
        Message::History(message) => {
            let Some(query) = model.history_picker_mut().and_then(|p| p.update(message)) else {
                return Vec::new();
            };
            model.close_overlay();
            model.editor_mut().load(&query);
            model.focus_pane(Pane::Editor);
            model.set_notice(Notice::Info("Query loaded from the history".to_string()));
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
            let succeeded = result.is_ok();
            if model.finish_search(id, result) {
                model.set_notice(notice);
                if succeeded && !model.editor().is_editing() {
                    model.focus_pane(Pane::Results);
                }
            }
        }
        Message::SaveQuery(name) => return save_query(model, name),
        Message::NotSaved(error) => model.set_notice(Notice::Error(error)),
        Message::CloseOverlay => model.close_overlay(),
        Message::Quit => model.quit(),
    }
    Vec::new()
}

/// Moves the focus in a cycle: YQL, parameters, results.
fn focus_next(model: &mut Model) {
    match (model.pane(), model.editor().focus()) {
        (Pane::Editor, Focus::Yql) => model.editor_mut().update(EditorMsg::Focus(Focus::Params)),
        (Pane::Editor, Focus::Params) => model.focus_pane(Pane::Results),
        (Pane::Results, _) => {
            model.focus_pane(Pane::Editor);
            model.editor_mut().update(EditorMsg::Focus(Focus::Yql));
        }
    }
}

/// Searches the query of the editor in the target.
fn run_query(model: &mut Model) -> Vec<Effect> {
    let Some(vespa) = model.target().map(|active| active.connection.vespa.clone()) else {
        model.set_notice(Notice::Error(
            "Not connected to a target, press T to pick one".to_string(),
        ));
        return Vec::new();
    };
    let draft = model.editor().draft();
    match draft.request() {
        Ok(request) => {
            let id = model.start_search();
            model.set_notice(Notice::Info("Searching…".to_string()));
            let save = model
                .record_query(draft.trimmed())
                .map(|(path, histories)| Effect::SaveHistory { path, histories });
            [Effect::Search { id, vespa, request }]
                .into_iter()
                .chain(save)
                .collect()
        }
        Err(error) => {
            model.set_notice(Notice::Error(error.to_string()));
            Vec::new()
        }
    }
}

/// Saves the query of the editor under the name.
fn save_query(model: &mut Model, name: String) -> Vec<Effect> {
    let query = model.editor().draft().trimmed();
    if query.yql.is_empty() {
        model.set_notice(Notice::Error("There is no query to save".to_string()));
        return Vec::new();
    }
    let notice = format!("Saved the query as {name:?}");
    let (replaced, save) = model.save_query(name, query);
    let notice = if replaced {
        format!("{notice}, replacing the previous one")
    } else {
        notice
    };
    model.set_notice(Notice::Info(notice));
    save.map(|(path, queries)| Effect::SaveQueries { path, queries })
        .into_iter()
        .collect()
}

/// Message running the command of the palette.
fn command_message(command: Command) -> Message {
    match command {
        Command::Help => Message::ToggleHelp,
        Command::Quit => Message::Quit,
        Command::Context(Some(name)) => Message::SwitchTarget(name),
        Command::Context(None) => Message::OpenTargetPicker,
        Command::History => Message::OpenHistory,
        Command::Save(name) => Message::SaveQuery(name),
    }
}

/// Bindings active in the current state, the most specific first.
pub(super) fn active_bindings(model: &Model) -> Vec<&'static [Binding]> {
    match model.overlay() {
        Some(Overlay::TargetPicker(_)) => {
            vec![keymap::TARGET_PICKER, keymap::OVERLAY, keymap::GLOBAL]
        }
        Some(Overlay::HistoryPicker(_)) => {
            vec![keymap::HISTORY_PICKER, keymap::OVERLAY, keymap::GLOBAL]
        }
        Some(_) => vec![keymap::OVERLAY, keymap::GLOBAL],
        None if model.pane() == Pane::Results => vec![keymap::RESULTS, keymap::GLOBAL],
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
        history::Histories,
        query::QueryDraft,
        saved::SavedQueries,
        search::SearchOutcome,
        store::StoreError,
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
    fn test_tab_cycles_the_focus() {
        let mut model = Model::default();
        let mut visited = Vec::new();
        for _ in 0..4 {
            visited.push((model.pane(), model.editor().focus()));
            let _ = press(&mut model, KeyCode::Tab);
        }
        assert_eq!(
            visited,
            [
                (Pane::Editor, Focus::Yql),
                (Pane::Editor, Focus::Params),
                (Pane::Results, Focus::Params),
                (Pane::Editor, Focus::Yql),
            ]
        );
    }

    #[test]
    fn test_results_are_focused_when_the_search_succeeds() {
        let mut model = connected();
        let _ = update(&mut model, Message::RunQuery);
        let _ = update(
            &mut model,
            Message::SearchDone {
                id: 1,
                result: Ok(outcome(1)),
            },
        );
        assert_eq!(model.pane(), Pane::Results);
    }

    #[test]
    fn test_editing_keeps_the_focus_when_the_search_succeeds() {
        let mut model = connected();
        let _ = press(&mut model, KeyCode::Char('i'));
        let _ = update(&mut model, Message::RunQuery);
        let _ = update(
            &mut model,
            Message::SearchDone {
                id: 1,
                result: Ok(outcome(1)),
            },
        );
        assert_eq!(model.pane(), Pane::Editor);
        assert!(model.editor().is_editing());
    }

    #[test]
    fn test_navigate_the_results() {
        let mut model = connected();
        let _ = update(&mut model, Message::RunQuery);
        let _ = update(
            &mut model,
            Message::SearchDone {
                id: 1,
                result: Ok(outcome(1)),
            },
        );
        let _ = press(&mut model, KeyCode::Char('r'));
        assert!(model.results().is_raw());
        // A new search shows the hits again
        let _ = update(&mut model, Message::RunQuery);
        assert!(!model.results().is_raw());
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

    #[test]
    fn test_run_queries_are_recorded_in_the_history() {
        let mut model = connected();
        let _ = update(&mut model, Message::RunQuery);
        let _ = update(&mut model, Message::RunQuery);
        let recorded: Vec<_> = model.history().entries().collect();
        assert_eq!(recorded, [&model.editor().draft().trimmed()]);
    }

    #[test]
    fn test_invalid_queries_are_not_recorded() {
        let mut model = connected();
        // A parameter with value but without name
        let _ = press(&mut model, KeyCode::Tab);
        let _ = type_text(&mut model, "a");
        let _ = press(&mut model, KeyCode::Tab);
        let _ = type_text(&mut model, "5");
        let _ = press(&mut model, KeyCode::Esc);
        let effects = update(&mut model, Message::RunQuery);
        assert!(effects.is_empty());
        assert!(matches!(model.notice(), Some(Notice::Error(_))));
        assert!(model.history().is_empty());
    }

    #[test]
    fn test_the_history_is_saved_after_running() {
        let mut model = connected().with_history("history.json".into(), Ok(Histories::default()));
        let effects = update(&mut model, Message::RunQuery);
        let [
            Effect::Search { .. },
            Effect::SaveHistory { path, histories },
        ] = effects.as_slice()
        else {
            panic!("the history must be saved, got {effects:?}");
        };
        assert_eq!(path.to_str(), Some("history.json"));
        assert_eq!(histories.target("local").len(), 1);
    }

    #[test]
    fn test_the_history_is_by_target() {
        let mut histories = Histories::default();
        histories.record("prod", QueryDraft::new("select * from prod where true"));
        let mut model = connected().with_history("history.json".into(), Ok(histories));
        assert!(model.history().is_empty());

        let prod = ActiveTarget::open("prod", Target::self_hosted("http://prod:8080"));
        let _ = update(
            &mut model,
            Message::TargetOpened(Ok(Box::new(prod.unwrap()))),
        );
        assert_eq!(model.history().len(), 1);
    }

    #[test]
    fn test_an_invalid_history_is_not_saved() {
        let invalid = serde_json::from_str::<serde_json::Value>("nope").unwrap_err();
        let error = StoreError::Parse {
            path: "history.json".into(),
            source: invalid,
        };
        let mut model = connected().with_history("history.json".into(), Err(error));
        assert!(
            matches!(model.notice(), Some(Notice::Error(e)) if e.ends_with("will not be saved."))
        );

        let effects = update(&mut model, Message::RunQuery);
        assert!(matches!(effects.as_slice(), [Effect::Search { .. }]));
        assert_eq!(model.history().len(), 1);
    }

    #[test]
    fn test_history_not_saved() {
        let mut model = connected();
        let _ = update(&mut model, Message::NotSaved("disk full".to_string()));
        assert_eq!(
            model.notice(),
            Some(&Notice::Error("disk full".to_string()))
        );
    }

    #[test]
    fn test_load_a_query_from_the_history() {
        let mut histories = Histories::default();
        histories.record("local", QueryDraft::new("select * from music where true"));
        histories.record("local", QueryDraft::new("select * from doc where true"));
        let mut model = connected().with_history("history.json".into(), Ok(histories));
        model.focus_pane(Pane::Results);

        let _ = type_text(&mut model, "H");
        assert!(matches!(model.overlay(), Some(Overlay::HistoryPicker(_))));
        let _ = type_text(&mut model, "j");
        let _ = press(&mut model, KeyCode::Enter);

        assert_eq!(model.overlay(), None);
        assert_eq!(model.editor().yql(), "select * from music where true");
        assert_eq!(model.pane(), Pane::Editor);
        assert_eq!(
            model.notice(),
            Some(&Notice::Info("Query loaded from the history".to_string()))
        );
    }

    #[test]
    fn test_history_command() {
        let mut model = connected();
        let _ = run_command(&mut model, "history");
        assert!(matches!(model.overlay(), Some(Overlay::HistoryPicker(_))));
        // Nothing to load from an empty history
        let _ = press(&mut model, KeyCode::Enter);
        assert!(matches!(model.overlay(), Some(Overlay::HistoryPicker(_))));
    }

    #[test]
    fn test_h_is_text_while_editing() {
        let mut model = connected();
        let _ = press(&mut model, KeyCode::Char('i'));
        let _ = type_text(&mut model, "H");
        assert_eq!(model.overlay(), None);
        assert!(model.editor().yql().ends_with('H'));
    }

    #[test]
    fn test_save_the_query() {
        let mut model =
            Model::default().with_saved_queries("saved.json".into(), Ok(SavedQueries::default()));
        let _ = type_text(&mut model, "s");
        assert!(matches!(model.overlay(), Some(Overlay::Palette(p)) if p.input() == "save "));

        let _ = type_text(&mut model, "all docs");
        let effects = press(&mut model, KeyCode::Enter);

        let [Effect::SaveQueries { path, queries }] = effects.as_slice() else {
            panic!("the saved queries must be saved, got {effects:?}");
        };
        assert_eq!(path.to_str(), Some("saved.json"));
        let expected = model.editor().draft().trimmed();
        assert_eq!(queries.get("all docs"), Some(&expected));
        assert_eq!(model.saved_queries(), queries);
        assert_eq!(
            model.notice(),
            Some(&Notice::Info("Saved the query as \"all docs\"".to_string()))
        );
    }

    #[test]
    fn test_save_under_an_existing_name() {
        let mut model = Model::default();
        let _ = run_command(&mut model, "save q");
        let effects = run_command(&mut model, "save q");
        // Not saved in a file without one
        assert!(effects.is_empty());
        assert_eq!(model.saved_queries().len(), 1);
        assert_eq!(
            model.notice(),
            Some(&Notice::Info(
                "Saved the query as \"q\", replacing the previous one".to_string()
            ))
        );
    }

    #[test]
    fn test_save_without_name() {
        let mut model = Model::default();
        let _ = run_command(&mut model, "save");
        assert!(model.saved_queries().is_empty());
        assert_eq!(
            model.notice(),
            Some(&Notice::Error(
                "Missing argument, use :save <name>".to_string()
            ))
        );
    }

    #[test]
    fn test_save_an_empty_query() {
        let mut model = Model::default();
        model.editor_mut().load(&QueryDraft::new("  "));
        let _ = run_command(&mut model, "save q");
        assert!(model.saved_queries().is_empty());
        assert!(matches!(model.notice(), Some(Notice::Error(_))));
    }
}
