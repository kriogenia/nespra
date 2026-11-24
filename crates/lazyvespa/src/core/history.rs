//! History of the queries run against a target.

use std::collections::VecDeque;

use crate::core::query::QueryDraft;

/// Maximum number of queries kept in a [`History`].
pub const LIMIT: usize = 500;

/// Queries run against a target, the most recent first and without duplicates.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct History {
    entries: VecDeque<QueryDraft>,
}

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
}
