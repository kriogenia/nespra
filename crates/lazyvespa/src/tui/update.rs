use crossterm::event::KeyEvent;

use super::{
    Effect, Message, Model,
    keymap::{self, Binding},
    model::{Notice, Overlay},
    palette::{self, Command},
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
        Message::CloseOverlay => model.close_overlay(),
        Message::Quit => model.quit(),
    }
    Vec::new()
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
        None => vec![keymap::GLOBAL],
    }
}

/// Message of the key in the current state. While typing in the palette, the keys that
/// are not bound by the overlay are text.
fn key_message(model: &Model, key: &KeyEvent) -> Option<Message> {
    if let Some(Overlay::Palette(_)) = model.overlay() {
        keymap::message(&[keymap::OVERLAY], key)
            .or_else(|| palette::key_message(key).map(Message::Palette))
            .or_else(|| keymap::message(&[keymap::GLOBAL], key))
    } else {
        keymap::message(&active_bindings(model), key)
    }
}

#[cfg(test)]
mod test {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;
    use crate::core::{
        config::{Config, Target},
        connection::ActiveTarget,
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
        assert_eq!(
            effects,
            vec![Effect::OpenTarget {
                name: "local".to_string(),
                target: Target::self_hosted("http://localhost:8080"),
            }]
        );
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
