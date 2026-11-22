use crossterm::event::KeyEvent;

use super::{
    Message, Model,
    keymap::{self, Binding},
    model::Overlay,
    palette::{self, Command},
};

/// Applies a message to the model. It is the only place where the state changes.
pub fn update(model: &mut Model, message: Message) {
    match message {
        Message::Key(key) => {
            model.clear_notice();
            if let Some(message) = key_message(model, &key) {
                update(model, message);
            }
        }
        Message::ToggleHelp => model.toggle_help(),
        Message::OpenPalette => model.open_palette(),
        Message::Palette(message) => {
            let Some(result) = model.palette_mut().and_then(|p| p.update(message)) else {
                return;
            };
            model.close_overlay();
            match result {
                Ok(command) => update(model, command_message(command)),
                Err(error) => model.set_notice(error.to_string()),
            }
        }
        Message::CloseOverlay => model.close_overlay(),
        Message::Quit => model.quit(),
    }
}

/// Message running the command of the palette.
fn command_message(command: Command) -> Message {
    match command {
        Command::Help => Message::ToggleHelp,
        Command::Quit => Message::Quit,
    }
}

/// Bindings active in the current state, the most specific first.
pub(super) fn active_bindings(model: &Model) -> Vec<&'static [Binding]> {
    match model.overlay() {
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

    fn press(model: &mut Model, code: KeyCode) {
        update(model, Message::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    }

    fn type_text(model: &mut Model, text: &str) {
        text.chars().for_each(|c| press(model, KeyCode::Char(c)));
    }

    #[test]
    fn test_quit() {
        let mut model = Model::default();
        assert!(model.is_running());
        update(&mut model, Message::Quit);
        assert!(!model.is_running());
    }

    #[test]
    fn test_keys_trigger_their_message() {
        let mut model = Model::default();
        press(&mut model, KeyCode::Char('q'));
        assert!(!model.is_running());
    }

    #[test]
    fn test_toggle_help() {
        let mut model = Model::default();
        press(&mut model, KeyCode::Char('?'));
        assert_eq!(model.overlay(), Some(&Overlay::Help));
        press(&mut model, KeyCode::Char('?'));
        assert_eq!(model.overlay(), None);
    }

    #[test]
    fn test_escape_closes_the_help() {
        let mut model = Model::default();
        press(&mut model, KeyCode::Esc);
        assert_eq!(model.overlay(), None);
        press(&mut model, KeyCode::Char('?'));
        press(&mut model, KeyCode::Esc);
        assert_eq!(model.overlay(), None);
        assert!(model.is_running());
    }

    #[test]
    fn test_keys_are_text_in_the_palette() {
        let mut model = Model::default();
        type_text(&mut model, ":q?");
        let Some(Overlay::Palette(palette)) = model.overlay() else {
            panic!("the palette must be open");
        };
        assert_eq!(palette.input(), "q?");
        assert!(model.is_running());
    }

    #[test]
    fn test_run_a_command() {
        let mut model = Model::default();
        type_text(&mut model, ":quit");
        press(&mut model, KeyCode::Enter);
        assert!(!model.is_running());
    }

    #[test]
    fn test_unknown_command_is_noticed_until_the_next_key() {
        let mut model = Model::default();
        type_text(&mut model, ":nope");
        press(&mut model, KeyCode::Enter);
        assert_eq!(model.overlay(), None);
        assert_eq!(model.notice(), Some("Unknown command \"nope\""));
        press(&mut model, KeyCode::Char('x'));
        assert_eq!(model.notice(), None);
    }

    #[test]
    fn test_escape_closes_the_palette() {
        let mut model = Model::default();
        type_text(&mut model, ":qui");
        press(&mut model, KeyCode::Esc);
        assert_eq!(model.overlay(), None);
        assert!(model.is_running());
    }
}
