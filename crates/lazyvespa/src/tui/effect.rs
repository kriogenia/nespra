//! Side effects requested by [`update`](super::update), run outside of it so it stays pure.
//! Their results come back as [`Message`]s.

use std::sync::Arc;

use nespra::Vespa;
use tokio::sync::mpsc::UnboundedSender;

use super::Message;
use crate::core::{config::Target, connection::ActiveTarget, query::QueryRequest, search};

/// A side effect to run.
#[derive(Debug, Clone)]
pub enum Effect {
    /// Connect to a target, answered with [`Message::TargetOpened`].
    OpenTarget {
        /// Name of the target.
        name: String,
        /// Configuration of the target.
        target: Target,
    },
    /// Run a search, answered with [`Message::SearchDone`].
    Search {
        /// Identifier of the search, to tell apart the answers of older searches.
        id: u64,
        /// Client of the target.
        vespa: Vespa,
        /// Request to send.
        request: QueryRequest,
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
            Effect::Search { id, vespa, request } => {
                tokio::spawn(async move {
                    let result = search::search(&vespa, &request)
                        .await
                        .map(Arc::new)
                        .map_err(|e| format!("Search failed: {e}"));
                    let _ = messages.send(Message::SearchDone { id, result });
                });
            }
        }
    }
}
