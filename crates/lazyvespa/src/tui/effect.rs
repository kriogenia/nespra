//! Side effects requested by [`update`](super::update), run outside of it so it stays pure.
//! Their results come back as [`Message`]s.

use std::{path::PathBuf, sync::Arc};

use nespra::Vespa;
use tokio::sync::{mpsc::UnboundedSender, watch};

use super::Message;
use crate::core::{
    config::Target, connection::ActiveTarget, history::Histories, query::QueryRequest, search,
};

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
    /// Save the histories in the file, answered with [`Message::HistoryNotSaved`] only
    /// when it fails.
    SaveHistory {
        /// Path of the file.
        path: PathBuf,
        /// Histories to save.
        histories: Histories,
    },
}

/// Histories waiting to be saved, and where.
type PendingSave = Option<(PathBuf, Histories)>;

/// Runs the effects in the background and sends their results as messages.
pub(super) struct EffectRunner {
    messages: UnboundedSender<Message>,
    history: watch::Sender<PendingSave>,
}

impl EffectRunner {
    /// Creates the runner, starting the task that saves the history.
    pub(super) fn new(messages: UnboundedSender<Message>) -> Self {
        let (history, pending) = watch::channel(None);
        tokio::spawn(save_histories(pending, messages.clone()));
        Self { messages, history }
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
            Effect::SaveHistory { path, histories } => {
                self.history.send_replace(Some((path, histories)));
            }
        }
    }
}

/// Saves the histories one at a time, so the writes never overlap. When several are
/// requested during a write, only the latest one is saved after it.
async fn save_histories(
    mut pending: watch::Receiver<PendingSave>,
    messages: UnboundedSender<Message>,
) {
    while pending.changed().await.is_ok() {
        let Some((path, histories)) = pending.borrow_and_update().clone() else {
            continue;
        };
        let saved = tokio::task::spawn_blocking(move || histories.save(&path)).await;
        if let Ok(Err(error)) = saved {
            let _ = messages.send(Message::HistoryNotSaved(error.to_string()));
        }
    }
}

#[cfg(test)]
mod test {
    use tokio::sync::mpsc;

    use super::*;
    use crate::core::query::QueryDraft;

    #[tokio::test]
    async fn test_save_the_history() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let runner = EffectRunner::new(sender);

        let mut histories = Histories::default();
        for yql in ["a", "b"] {
            histories.record("local", QueryDraft::new(yql));
            runner.run(Effect::SaveHistory {
                path: path.clone(),
                histories: histories.clone(),
            });
        }
        drop(runner);

        // The saver ends once the runner is dropped and the last save is done
        assert!(receiver.recv().await.is_none());
        assert_eq!(Histories::load(&path).unwrap(), histories);
    }

    #[tokio::test]
    async fn test_failure_to_save_the_history() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let runner = EffectRunner::new(sender);
        runner.run(Effect::SaveHistory {
            // A directory cannot be created inside a file
            path: file.path().join("history.json"),
            histories: Histories::default(),
        });
        let message = receiver.recv().await;
        assert!(
            matches!(message, Some(Message::HistoryNotSaved(_))),
            "{message:?}"
        );
    }
}
