use super::{
    Message, Model,
    keymap::{self, Binding},
    model::Overlay,
};

/// Applies a message to the model. It is the only place where the state changes.
pub fn update(model: &mut Model, message: Message) {
    match message {
        Message::Key(key) => {
            if let Some(message) = keymap::message(&active_bindings(model), &key) {
                update(model, message);
            }
        }
        Message::ToggleHelp => model.toggle_overlay(Overlay::Help),
        Message::CloseOverlay => model.close_overlay(),
        Message::Quit => model.quit(),
    }
}

/// Bindings active in the current state, the most specific first.
pub(super) fn active_bindings(model: &Model) -> Vec<&'static [Binding]> {
    match model.overlay() {
        Some(_) => vec![keymap::OVERLAY, keymap::GLOBAL],
        None => vec![keymap::GLOBAL],
    }
}

#[cfg(test)]
mod test {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;

    fn press(model: &mut Model, code: KeyCode) {
        update(model, Message::Key(KeyEvent::new(code, KeyModifiers::NONE)));
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
        assert_eq!(model.overlay(), Some(Overlay::Help));
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
}
