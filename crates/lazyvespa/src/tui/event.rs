//! Translation of terminal events into [`Message`]s.

use std::io;

use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::StreamExt;

use super::Message;

/// Asynchronous source of the messages of the terminal.
pub(super) struct Events {
    terminal: EventStream,
}

impl Events {
    pub(super) fn new() -> Self {
        Self {
            terminal: EventStream::new(),
        }
    }

    /// Waits for the next terminal event and translates it, `None` when it is not relevant.
    pub(super) async fn next(&mut self) -> io::Result<Option<Message>> {
        match self.terminal.next().await {
            Some(event) => Ok(message(event?)),
            None => Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the terminal event stream was closed",
            )),
        }
    }
}

/// Message triggered by a terminal event, if any.
fn message(event: Event) -> Option<Message> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => Some(Message::Key(key)),
        _ => None,
    }
}

#[cfg(test)]
mod test {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;

    #[test]
    fn test_key_presses() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(message(Event::Key(key)), Some(Message::Key(key)));
    }

    #[test]
    fn test_irrelevant_events() {
        assert_eq!(message(Event::FocusGained), None);
        let mut release = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert_eq!(message(Event::Key(release)), None);
    }
}
