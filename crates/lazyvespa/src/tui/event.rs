//! Translation of terminal events into [`Message`]s.

use std::io;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::Message;

/// Waits for the next terminal event and translates it, `None` when it is not relevant.
pub(super) fn next() -> io::Result<Option<Message>> {
    Ok(message(event::read()?))
}

/// Message triggered by a terminal event, if any.
fn message(event: Event) -> Option<Message> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => key_message(key),
        _ => None,
    }
}

fn key_message(key: KeyEvent) -> Option<Message> {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            Some(Message::Quit)
        }
        _ => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    #[test]
    fn test_quit_keys() {
        assert_eq!(
            message(press(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(Message::Quit)
        );
        assert_eq!(
            message(press(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Message::Quit)
        );
    }

    #[test]
    fn test_irrelevant_events() {
        assert_eq!(message(press(KeyCode::Char('c'), KeyModifiers::NONE)), None);
        assert_eq!(message(Event::FocusGained), None);
        let mut release = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert_eq!(message(Event::Key(release)), None);
    }
}
