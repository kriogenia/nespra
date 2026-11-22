//! Side effects requested by [`update`](super::update), run outside of it so it stays pure.
//! Their results come back as [`Message`]s.

use tokio::sync::mpsc::UnboundedSender;

use super::Message;
use crate::core::{config::Target, connection::ActiveTarget};

/// A side effect to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Connect to a target, answered with [`Message::TargetOpened`].
    OpenTarget {
        /// Name of the target.
        name: String,
        /// Configuration of the target.
        target: Target,
    },
}

/// Runs the effects in the background and sends their results as messages.
pub(super) struct EffectRunner {
    messages: UnboundedSender<Message>,
}

impl EffectRunner {
    pub(super) fn new(messages: UnboundedSender<Message>) -> Self {
        Self { messages }
    }

    pub(super) fn run(&self, effect: Effect) {
        let messages = self.messages.clone();
        match effect {
            Effect::OpenTarget { name, target } => {
                tokio::task::spawn_blocking(move || {
                    let result = ActiveTarget::open(&name, target)
                        .map(Box::new)
                        .map_err(|e| format!("Unable to connect to the target {name:?}: {e}"));
                    // The receiver is only dropped when the application is closing
                    let _ = messages.send(Message::TargetOpened(result));
                });
            }
        }
    }
}
