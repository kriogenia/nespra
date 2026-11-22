//! Command palette, opened with `:` to run commands by name like in k9s or vim.

use std::str::FromStr;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use thiserror::Error;

/// State of the command palette: the command being typed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Palette {
    input: String,
}

/// Messages of the [`Palette`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteMsg {
    /// Type a character.
    Insert(char),
    /// Delete the last character.
    DeleteBack,
    /// Run the typed command.
    Submit,
}

/// Commands of the palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Show the key bindings.
    Help,
    /// Close the application.
    Quit,
}

/// The typed text is not a command.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Unknown command {0:?}")]
pub struct UnknownCommand(String);

impl FromStr for Command {
    type Err = UnknownCommand;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "help" | "h" | "?" => Ok(Command::Help),
            "quit" | "q" | "q!" => Ok(Command::Quit),
            other => Err(UnknownCommand(other.to_string())),
        }
    }
}

impl Palette {
    /// Text typed so far.
    pub fn input(&self) -> &str {
        &self.input
    }

    /// Applies a message. Submitting returns the parsed command, `None` if nothing was
    /// typed.
    pub fn update(&mut self, message: PaletteMsg) -> Option<Result<Command, UnknownCommand>> {
        match message {
            PaletteMsg::Insert(c) => self.input.push(c),
            PaletteMsg::DeleteBack => {
                self.input.pop();
            }
            PaletteMsg::Submit if self.input.trim().is_empty() => {}
            PaletteMsg::Submit => return Some(self.input.parse()),
        }
        None
    }
}

/// Message of the key while typing in the palette, if any.
pub fn key_message(key: &KeyEvent) -> Option<PaletteMsg> {
    let typing = key.modifiers - KeyModifiers::SHIFT == KeyModifiers::NONE;
    match key.code {
        KeyCode::Char(c) if typing => Some(PaletteMsg::Insert(c)),
        KeyCode::Backspace => Some(PaletteMsg::DeleteBack),
        KeyCode::Enter => Some(PaletteMsg::Submit),
        _ => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn typed(text: &str) -> Palette {
        let mut palette = Palette::default();
        for c in text.chars() {
            palette.update(PaletteMsg::Insert(c));
        }
        palette
    }

    #[test]
    fn test_typing() {
        let mut palette = typed("quix");
        palette.update(PaletteMsg::DeleteBack);
        palette.update(PaletteMsg::Insert('t'));
        assert_eq!(palette.input(), "quit");
        assert_eq!(palette.update(PaletteMsg::Submit), Some(Ok(Command::Quit)));
    }

    #[test]
    fn test_submit_nothing() {
        assert_eq!(typed("  ").update(PaletteMsg::Submit), None);
    }

    #[test]
    fn test_unknown_command() {
        assert_eq!(
            typed("deploy").update(PaletteMsg::Submit),
            Some(Err(UnknownCommand("deploy".to_string())))
        );
    }

    #[test]
    fn test_commands() {
        for (text, command) in [
            ("q", Command::Quit),
            ("quit", Command::Quit),
            (" help ", Command::Help),
            ("?", Command::Help),
        ] {
            assert_eq!(text.parse(), Ok(command));
        }
    }

    #[test]
    fn test_keys() {
        let key = |code, modifiers| key_message(&KeyEvent::new(code, modifiers));
        assert_eq!(
            key(KeyCode::Char('Q'), KeyModifiers::SHIFT),
            Some(PaletteMsg::Insert('Q'))
        );
        assert_eq!(
            key(KeyCode::Backspace, KeyModifiers::NONE),
            Some(PaletteMsg::DeleteBack)
        );
        assert_eq!(
            key(KeyCode::Enter, KeyModifiers::NONE),
            Some(PaletteMsg::Submit)
        );
        assert_eq!(key(KeyCode::Char('c'), KeyModifiers::CONTROL), None);
    }
}
