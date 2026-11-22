//! Sources of [`Message`]s: the terminal and the results of the effects.

use std::io;

use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::StreamExt;
use tokio::sync::mpsc::UnboundedReceiver;

use super::Message;

/// Asynchronous source of the messages.
pub(super) struct Events {
    terminal: EventStream,
    effects: UnboundedReceiver<Message>,
}

impl Events {
    /// Listens to the terminal and to the messages sent by the effects.
    pub(super) fn new(effects: UnboundedReceiver<Message>) -> Self {
        Self {
            terminal: EventStream::new(),
            effects,
        }
    }

    /// Waits for the next message, `None` when the event is not relevant.
    pub(super) async fn next(&mut self) -> io::Result<Option<Message>> {
        tokio::select! {
            event = self.terminal.next() => match event {
                Some(event) => Ok(message(event?)),
                None => Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "the terminal event stream was closed",
                )),
            },
            // The sender lives in the effect runner as long as the application runs
            Some(message) = self.effects.recv() => Ok(Some(message)),
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
        assert!(matches!(message(Event::Key(key)), Some(Message::Key(k)) if k == key));
    }

    #[test]
    fn test_irrelevant_events() {
        assert!(message(Event::FocusGained).is_none());
        let mut release = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert!(message(Event::Key(release)).is_none());
    }
}
