//! Queries saved by the user under a name, shared by all the targets and kept between runs.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::core::{
    dirs,
    query::QueryDraft,
    store::{self, StoreError},
};

/// Queries saved by name, sorted by name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SavedQueries {
    queries: BTreeMap<String, QueryDraft>,
}

/// Default location of the saved queries file, `saved.json` in the
/// [data directory](dirs::data_dir).
pub fn default_path() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("saved.json"))
}

impl SavedQueries {
    /// Loads the saved queries file, or no queries if it does not exist.
    ///
    /// # Errors
    ///
    /// Fails if the file exists but cannot be read or is not valid.
    pub fn load(path: &Path) -> Result<Self, StoreError> {
        store::load(path)
    }

    /// Saves the queries to the file, see [`store::save`].
    ///
    /// # Errors
    ///
    /// Fails if the directory or the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), StoreError> {
        store::save(path, self)
    }

    /// Saves a query under the name, returning the query it replaces, if any.
    pub fn insert(&mut self, name: impl Into<String>, query: QueryDraft) -> Option<QueryDraft> {
        self.queries.insert(name.into(), query)
    }

    /// Removes the query with the name, returning it.
    pub fn remove(&mut self, name: &str) -> Option<QueryDraft> {
        self.queries.remove(name)
    }

    /// The query with the name.
    pub fn get(&self, name: &str) -> Option<&QueryDraft> {
        self.queries.get(name)
    }

    /// The queries with their names, sorted by name.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&str, &QueryDraft)> {
        self.queries
            .iter()
            .map(|(name, query)| (name.as_str(), query))
    }

    /// Number of queries.
    pub fn len(&self) -> usize {
        self.queries.len()
    }

    /// Whether no query is saved.
    pub fn is_empty(&self) -> bool {
        self.queries.is_empty()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_queries_sorted_by_name() {
        let mut saved = SavedQueries::default();
        assert!(saved.is_empty());
        saved.insert("songs", QueryDraft::new("select * from music where true"));
        saved.insert("all", QueryDraft::new("select * from sources * where true"));
        let names: Vec<_> = saved.iter().map(|(name, _)| name).collect();
        assert_eq!(names, ["all", "songs"]);
        assert_eq!(saved.len(), 2);
    }

    #[test]
    fn test_save_under_an_existing_name_replaces_it() {
        let mut saved = SavedQueries::default();
        assert_eq!(saved.insert("q", QueryDraft::new("old")), None);
        assert_eq!(
            saved.insert("q", QueryDraft::new("new")),
            Some(QueryDraft::new("old"))
        );
        assert_eq!(saved.get("q"), Some(&QueryDraft::new("new")));
    }

    #[test]
    fn test_remove() {
        let mut saved = SavedQueries::default();
        saved.insert("q", QueryDraft::new("yql"));
        assert_eq!(saved.remove("q"), Some(QueryDraft::new("yql")));
        assert_eq!(saved.remove("q"), None);
        assert!(saved.is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("saved.json");
        let mut saved = SavedQueries::default();
        saved.insert(
            "songs",
            QueryDraft::new("select * from music where true").param("hits", "5"),
        );

        saved.save(&path).unwrap();

        assert_eq!(SavedQueries::load(&path).unwrap(), saved);
    }

    #[test]
    fn test_saved_format() {
        let mut saved = SavedQueries::default();
        saved.insert("q", QueryDraft::new("yql").param("hits", "5"));
        assert_eq!(
            serde_json::to_value(&saved).unwrap(),
            serde_json::json!({"q": {"yql": "yql", "params": [["hits", "5"]]}})
        );
    }

    #[test]
    fn test_default_path() {
        let path = default_path().unwrap();
        assert!(path.ends_with("lazyvespa/saved.json"), "{path:?}");
    }
}
