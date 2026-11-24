//! History of the queries run against each target, kept between runs.

use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::core::{
    dirs,
    query::QueryDraft,
    store::{self, StoreError},
};

/// Maximum number of queries kept in a [`History`].
pub const LIMIT: usize = 500;

/// Queries run against a target, the most recent first and without duplicates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct History {
    entries: VecDeque<QueryDraft>,
}

/// History of a target without queries.
static EMPTY: History = History {
    entries: VecDeque::new(),
};

impl History {
    /// Records a query as the most recent one.
    ///
    /// A query already in the history is moved to the top instead of repeated, and the
    /// oldest one is dropped once there are more than [`LIMIT`].
    pub fn record(&mut self, query: QueryDraft) {
        self.entries.retain(|entry| *entry != query);
        self.entries.push_front(query);
        self.entries.truncate(LIMIT);
    }

    /// The queries, the most recent first.
    pub fn entries(&self) -> impl ExactSizeIterator<Item = &QueryDraft> {
        self.entries.iter()
    }

    /// The query at the position, `0` being the most recent.
    pub fn get(&self, index: usize) -> Option<&QueryDraft> {
        self.entries.get(index)
    }

    /// Number of queries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no query was recorded.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Histories of all the targets, by target name, stored together in one JSON file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Histories {
    targets: BTreeMap<String, History>,
}

/// Default location of the histories file, `history.json` in the [data directory](dirs::data_dir).
pub fn default_path() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("history.json"))
}

impl Histories {
    /// Loads the histories file, or empty histories if it does not exist.
    ///
    /// # Errors
    ///
    /// Fails if the file exists but cannot be read or is not valid.
    pub fn load(path: &Path) -> Result<Self, StoreError> {
        store::load(path)
    }

    /// Saves the histories to the file, see [`store::save`].
    ///
    /// # Errors
    ///
    /// Fails if the directory or the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), StoreError> {
        store::save(path, self)
    }

    /// History of the target, empty if no query was recorded for it.
    pub fn target(&self, name: &str) -> &History {
        self.targets.get(name).unwrap_or(&EMPTY)
    }

    /// Records a query run against the target, see [`History::record`].
    pub fn record(&mut self, target: &str, query: QueryDraft) {
        self.targets
            .entry(target.to_string())
            .or_default()
            .record(query);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn yqls(history: &History) -> Vec<&str> {
        history.entries().map(|query| query.yql.as_str()).collect()
    }

    #[test]
    fn test_most_recent_first() {
        let mut history = History::default();
        assert!(history.is_empty());
        history.record(QueryDraft::new("first"));
        history.record(QueryDraft::new("second"));
        assert_eq!(yqls(&history), ["second", "first"]);
        assert_eq!(history.get(0), Some(&QueryDraft::new("second")));
        assert_eq!(history.get(2), None);
    }

    #[test]
    fn test_repeated_queries_move_to_the_top() {
        let mut history = History::default();
        for yql in ["a", "b", "c", "a"] {
            history.record(QueryDraft::new(yql));
        }
        assert_eq!(yqls(&history), ["a", "c", "b"]);
    }

    #[test]
    fn test_parameters_make_a_query_different() {
        let mut history = History::default();
        history.record(QueryDraft::new("q").param("hits", "5"));
        history.record(QueryDraft::new("q").param("hits", "10"));
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_the_oldest_queries_are_dropped() {
        let mut history = History::default();
        for i in 0..=LIMIT {
            history.record(QueryDraft::new(i.to_string()));
        }
        assert_eq!(history.len(), LIMIT);
        assert_eq!(history.get(0).unwrap().yql, LIMIT.to_string());
        assert_eq!(history.get(LIMIT - 1).unwrap().yql, "1");
    }

    #[test]
    fn test_histories_by_target() {
        let mut histories = Histories::default();
        histories.record("local", QueryDraft::new("a"));
        histories.record("prod", QueryDraft::new("b"));
        histories.record("local", QueryDraft::new("c"));
        assert_eq!(yqls(histories.target("local")), ["c", "a"]);
        assert_eq!(yqls(histories.target("prod")), ["b"]);
        assert!(histories.target("unknown").is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("history.json");
        let mut histories = Histories::default();
        histories.record("local", QueryDraft::new("a").param("hits", "5"));
        histories.record("prod", QueryDraft::new("b"));

        histories.save(&path).unwrap();

        assert_eq!(Histories::load(&path).unwrap(), histories);
    }

    #[test]
    fn test_saved_format() {
        let mut histories = Histories::default();
        histories.record("local", QueryDraft::new("a").param("hits", "5"));
        assert_eq!(
            serde_json::to_value(&histories).unwrap(),
            serde_json::json!({"local": [{"yql": "a", "params": [["hits", "5"]]}]})
        );
    }

    #[test]
    fn test_default_path() {
        let path = default_path().unwrap();
        assert!(path.ends_with("lazyvespa/history.json"), "{path:?}");
    }
}
