//! Key bindings of the application, declared once to handle the keys and to document them
//! in the help.

use std::fmt;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::Message;

/// A key, optionally pressed together with modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl Key {
    /// Key pressed alone.
    const fn new(code: KeyCode) -> Self {
        Key {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    /// Character pressed alone.
    const fn char(c: char) -> Self {
        Key::new(KeyCode::Char(c))
    }

    /// Character pressed with `Ctrl`.
    const fn ctrl(c: char) -> Self {
        Key {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::CONTROL,
        }
    }

    /// Whether the event is this key. `Shift` is ignored on characters, as it is already
    /// part of the character (`?`, `T`...) and terminals do not agree on reporting it.
    fn matches(&self, event: &KeyEvent) -> bool {
        let modifiers = match event.code {
            KeyCode::Char(_) => event.modifiers - KeyModifiers::SHIFT,
            _ => event.modifiers,
        };
        self.code == event.code && self.modifiers == modifiers
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            f.write_str("Ctrl+")?;
        }
        match self.code {
            KeyCode::Char(c) if self.modifiers.contains(KeyModifiers::CONTROL) => {
                write!(f, "{}", c.to_ascii_uppercase())
            }
            KeyCode::Char(c) => write!(f, "{c}"),
            KeyCode::Esc => f.write_str("Esc"),
            KeyCode::Enter => f.write_str("Enter"),
            code => write!(f, "{code}"),
        }
    }
}

/// Keys triggering a message.
#[derive(Debug)]
pub struct Binding {
    /// Keys triggering the message, the first one is the one suggested.
    pub keys: &'static [Key],
    /// Message triggered.
    pub message: Message,
    /// What the binding does, for the help.
    pub description: &'static str,
}

/// Bindings available everywhere.
pub const GLOBAL: &[Binding] = &[
    Binding {
        keys: &[Key::char(':')],
        message: Message::OpenPalette,
        description: "command",
    },
    Binding {
        keys: &[Key::char('?')],
        message: Message::ToggleHelp,
        description: "help",
    },
    Binding {
        keys: &[Key::char('q'), Key::ctrl('c')],
        message: Message::Quit,
        description: "quit",
    },
];

/// Bindings available while an overlay is open, on top of the global ones.
pub const OVERLAY: &[Binding] = &[Binding {
    keys: &[Key::new(KeyCode::Esc)],
    message: Message::CloseOverlay,
    description: "close",
}];

/// Message bound to the key in the first group of bindings having it.
pub fn message(groups: &[&[Binding]], event: &KeyEvent) -> Option<Message> {
    groups
        .iter()
        .flat_map(|bindings| bindings.iter())
        .find(|binding| binding.keys.iter().any(|key| key.matches(event)))
        .map(|binding| binding.message.clone())
}

#[cfg(test)]
mod test {
    use super::*;

    fn press(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn test_global_bindings() {
        for (code, modifiers, message) in [
            (KeyCode::Char('q'), KeyModifiers::NONE, Message::Quit),
            (KeyCode::Char('c'), KeyModifiers::CONTROL, Message::Quit),
            (KeyCode::Char('?'), KeyModifiers::NONE, Message::ToggleHelp),
            (KeyCode::Char('?'), KeyModifiers::SHIFT, Message::ToggleHelp),
        ] {
            let found = message_for(&[GLOBAL], press(code, modifiers));
            assert_eq!(format!("{found:?}"), format!("{:?}", Some(message)));
        }
    }

    #[test]
    fn test_unbound_keys() {
        assert!(message_for(&[GLOBAL], press(KeyCode::Char('c'), KeyModifiers::NONE)).is_none());
        assert!(message_for(&[GLOBAL], press(KeyCode::Esc, KeyModifiers::NONE)).is_none());
    }

    #[test]
    fn test_first_group_wins() {
        let esc = press(KeyCode::Esc, KeyModifiers::NONE);
        assert!(matches!(
            message_for(&[OVERLAY, GLOBAL], esc),
            Some(Message::CloseOverlay)
        ));
    }

    #[test]
    fn test_display_keys() {
        assert_eq!(Key::char('q').to_string(), "q");
        assert_eq!(Key::ctrl('c').to_string(), "Ctrl+C");
        assert_eq!(Key::new(KeyCode::Esc).to_string(), "Esc");
        assert_eq!(Key::new(KeyCode::F(5)).to_string(), "F5");
    }

    fn message_for(groups: &[&[Binding]], event: KeyEvent) -> Option<Message> {
        message(groups, &event)
    }
}
