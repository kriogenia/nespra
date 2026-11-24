//! Side effects requested by [`update`](super::update), run outside of it so it stays pure.
//! Their results come back as [`Message`]s.

use std::{path::PathBuf, sync::Arc};

use nespra::Vespa;
use serde::Serialize;
use tokio::sync::{mpsc::UnboundedSender, watch};

use super::Message;
use crate::core::{
    config::Target, connection::ActiveTarget, history::Histories, query::QueryRequest,
    saved::SavedQueries, search, store,
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
    /// Save the histories in the file, answered with [`Message::NotSaved`] only when it
    /// fails.
    SaveHistory {
        /// Path of the file.
        path: PathBuf,
        /// Histories to save.
        histories: Histories,
    },
    /// Save the saved queries in the file, answered with [`Message::NotSaved`] only when
    /// it fails.
    SaveQueries {
        /// Path of the file.
        path: PathBuf,
        /// Queries to save.
        queries: SavedQueries,
    },
}

/// Data waiting to be saved, and where.
type PendingSave<T> = Option<(PathBuf, T)>;

/// Runs the effects in the background and sends their results as messages.
pub(super) struct EffectRunner {
    messages: UnboundedSender<Message>,
    history: watch::Sender<PendingSave<Histories>>,
    queries: watch::Sender<PendingSave<SavedQueries>>,
}

impl EffectRunner {
    /// Creates the runner, starting the tasks that save the data files.
    pub(super) fn new(messages: UnboundedSender<Message>) -> Self {
        Self {
            history: saver(messages.clone()),
            queries: saver(messages.clone()),
            messages,
        }
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
            Effect::SaveQueries { path, queries } => {
                self.queries.send_replace(Some((path, queries)));
            }
        }
    }
}

/// Starts a task saving the data sent to it, returning where to send it.
fn saver<T>(messages: UnboundedSender<Message>) -> watch::Sender<PendingSave<T>>
where
    T: Serialize + Clone + Send + Sync + 'static,
{
    let (sender, pending) = watch::channel(None);
    tokio::spawn(save_latest(pending, messages));
    sender
}

/// Saves the data one at a time, so the writes of a file never overlap. When it is sent
/// several times during a write, only the latest one is saved after it.
async fn save_latest<T>(
    mut pending: watch::Receiver<PendingSave<T>>,
    messages: UnboundedSender<Message>,
) where
    T: Serialize + Clone + Send + Sync + 'static,
{
    while pending.changed().await.is_ok() {
        let Some((path, value)) = pending.borrow_and_update().clone() else {
            continue;
        };
        let saved = tokio::task::spawn_blocking(move || store::save(&path, &value)).await;
        if let Ok(Err(error)) = saved {
            let _ = messages.send(Message::NotSaved(error.to_string()));
        }
    }
}

#[cfg(test)]
mod test {
    use tokio::sync::mpsc;

    use super::*;
    use crate::core::query::QueryDraft;

    #[tokio::test]
    async fn test_save_the_queries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("saved.json");
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let runner = EffectRunner::new(sender);

        let mut queries = SavedQueries::default();
        queries.insert("q", QueryDraft::new("yql"));
        runner.run(Effect::SaveQueries {
            path: path.clone(),
            queries: queries.clone(),
        });
        drop(runner);

        assert!(receiver.recv().await.is_none());
        assert_eq!(SavedQueries::load(&path).unwrap(), queries);
    }

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
        assert!(matches!(message, Some(Message::NotSaved(_))), "{message:?}");
    }
}
