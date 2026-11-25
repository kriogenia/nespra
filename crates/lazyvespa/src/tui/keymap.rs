//! Key bindings of the application, declared once to handle the keys and to document them
//! in the help.

use std::fmt;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{
    Message, history_picker::HistoryMsg, params_form::ParamsMsg, query_editor::EditorMsg,
    results::ResultsMsg, saved_picker::SavedMsg, target_picker::PickerMsg,
};

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
            KeyCode::Tab => f.write_str("Tab"),
            KeyCode::Up => f.write_str("↑"),
            KeyCode::Down => f.write_str("↓"),
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
        keys: &[Key::ctrl('r'), Key::new(KeyCode::F(5))],
        message: Message::RunQuery,
        description: "run",
    },
    Binding {
        keys: &[Key::char(':')],
        message: Message::OpenPalette(""),
        description: "command",
    },
    Binding {
        keys: &[Key::char('T')],
        message: Message::OpenTargetPicker,
        description: "targets",
    },
    Binding {
        keys: &[Key::char('H')],
        message: Message::OpenHistory,
        description: "history",
    },
    Binding {
        keys: &[Key::char('s')],
        message: Message::OpenPalette("save "),
        description: "save",
    },
    Binding {
        keys: &[Key::char('S')],
        message: Message::OpenSaved,
        description: "saved",
    },
    Binding {
        keys: &[Key::char('I')],
        message: Message::OpenInfo,
        description: "info",
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

/// Bindings of the YQL out of the edit mode.
pub const EDITOR_YQL: &[Binding] = &[
    Binding {
        keys: &[Key::char('i'), Key::new(KeyCode::Enter)],
        message: Message::Editor(EditorMsg::StartEditing),
        description: "edit",
    },
    Binding {
        keys: &[Key::new(KeyCode::Tab)],
        message: Message::FocusNext,
        description: "parameters",
    },
];

/// Bindings of the parameters out of the edit mode.
pub const EDITOR_PARAMS: &[Binding] = &[
    Binding {
        keys: &[Key::char('i'), Key::new(KeyCode::Enter)],
        message: Message::Editor(EditorMsg::StartEditing),
        description: "edit",
    },
    Binding {
        keys: &[Key::char('a')],
        message: Message::Editor(EditorMsg::AddParam),
        description: "add",
    },
    Binding {
        keys: &[Key::char('d')],
        message: Message::Editor(EditorMsg::Params(ParamsMsg::Delete)),
        description: "delete",
    },
    Binding {
        keys: &[Key::char('j'), Key::new(KeyCode::Down)],
        message: Message::Editor(EditorMsg::Params(ParamsMsg::Next)),
        description: "next",
    },
    Binding {
        keys: &[Key::char('k'), Key::new(KeyCode::Up)],
        message: Message::Editor(EditorMsg::Params(ParamsMsg::Previous)),
        description: "previous",
    },
    Binding {
        keys: &[Key::new(KeyCode::Tab)],
        message: Message::FocusNext,
        description: "results",
    },
];

/// Bindings of the results.
pub const RESULTS: &[Binding] = &[
    Binding {
        keys: &[Key::char('j'), Key::new(KeyCode::Down)],
        message: Message::Results(ResultsMsg::Next),
        description: "next",
    },
    Binding {
        keys: &[Key::char('k'), Key::new(KeyCode::Up)],
        message: Message::Results(ResultsMsg::Previous),
        description: "previous",
    },
    Binding {
        keys: &[Key::ctrl('d'), Key::new(KeyCode::PageDown)],
        message: Message::Results(ResultsMsg::ScrollDown),
        description: "scroll down",
    },
    Binding {
        keys: &[Key::ctrl('u'), Key::new(KeyCode::PageUp)],
        message: Message::Results(ResultsMsg::ScrollUp),
        description: "scroll up",
    },
    Binding {
        keys: &[Key::char('r')],
        message: Message::Results(ResultsMsg::ToggleRaw),
        description: "raw",
    },
    Binding {
        keys: &[Key::new(KeyCode::Tab)],
        message: Message::FocusNext,
        description: "query",
    },
];

/// Bindings while typing a parameter, on top of the ones of the edit mode.
pub const EDITING_PARAM: &[Binding] = &[Binding {
    keys: &[Key::new(KeyCode::Tab)],
    message: Message::Editor(EditorMsg::Params(ParamsMsg::SwitchField)),
    description: "name/value",
}];

/// Bindings while typing in the query editor. Any other key is text.
pub const EDITING: &[Binding] = &[
    Binding {
        keys: &[Key::ctrl('r'), Key::new(KeyCode::F(5))],
        message: Message::RunQuery,
        description: "run",
    },
    Binding {
        keys: &[Key::new(KeyCode::Esc)],
        message: Message::Editor(EditorMsg::StopEditing),
        description: "stop editing",
    },
    Binding {
        keys: &[Key::ctrl('c')],
        message: Message::Quit,
        description: "quit",
    },
];

/// Bindings of the target picker.
pub const TARGET_PICKER: &[Binding] = &[
    Binding {
        keys: &[Key::char('j'), Key::new(KeyCode::Down)],
        message: Message::Picker(PickerMsg::Next),
        description: "next",
    },
    Binding {
        keys: &[Key::char('k'), Key::new(KeyCode::Up)],
        message: Message::Picker(PickerMsg::Previous),
        description: "previous",
    },
    Binding {
        keys: &[Key::new(KeyCode::Enter)],
        message: Message::Picker(PickerMsg::Pick),
        description: "connect",
    },
];

/// Bindings of the history picker.
pub const HISTORY_PICKER: &[Binding] = &[
    Binding {
        keys: &[Key::char('j'), Key::new(KeyCode::Down)],
        message: Message::History(HistoryMsg::Next),
        description: "next",
    },
    Binding {
        keys: &[Key::char('k'), Key::new(KeyCode::Up)],
        message: Message::History(HistoryMsg::Previous),
        description: "previous",
    },
    Binding {
        keys: &[Key::new(KeyCode::Enter)],
        message: Message::History(HistoryMsg::Pick),
        description: "load",
    },
];

/// Bindings of the saved queries picker.
pub const SAVED_PICKER: &[Binding] = &[
    Binding {
        keys: &[Key::char('j'), Key::new(KeyCode::Down)],
        message: Message::Saved(SavedMsg::Next),
        description: "next",
    },
    Binding {
        keys: &[Key::char('k'), Key::new(KeyCode::Up)],
        message: Message::Saved(SavedMsg::Previous),
        description: "previous",
    },
    Binding {
        keys: &[Key::new(KeyCode::Enter)],
        message: Message::Saved(SavedMsg::Pick),
        description: "load",
    },
    Binding {
        keys: &[Key::char('d')],
        message: Message::Saved(SavedMsg::Delete),
        description: "delete",
    },
];

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
