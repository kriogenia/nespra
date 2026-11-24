//! Queries being written by the user, turned into [`nespra`] search requests.

use std::{collections::BTreeSet, num::ParseIntError};

use nespra::{
    error::ParseError,
    search::{Request, parse_timeout},
};
use serde_json::{Map, Number, Value};
use thiserror::Error;

/// Search request built from a [`QueryDraft`]: the parameters known by nespra in their typed
/// fields and the rest flattened in the body.
pub type QueryRequest = Request<Map<String, Value>>;

/// A query: the YQL and the request parameters, as typed by the user.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryDraft {
    /// YQL of the query.
    pub yql: String,
    /// Request parameters, like `hits` or `ranking.profile`, in the typed order.
    pub params: Vec<(String, String)>,
}

impl QueryDraft {
    /// Draft with the YQL and no parameters.
    pub fn new(yql: impl Into<String>) -> Self {
        QueryDraft {
            yql: yql.into(),
            params: Vec::new(),
        }
    }

    /// Adds a parameter.
    pub fn param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.push((name.into(), value.into()));
        self
    }

    /// Search request of the draft.
    ///
    /// `hits`, `offset` and `timeout` go to their typed fields. The other parameters are sent as
    /// written, except values that look like booleans or numbers, sent as such. Parameters
    /// without value are not set, so they are skipped.
    ///
    /// # Errors
    ///
    /// Fails if the YQL is blank, or a parameter has no name, is the YQL, is repeated or
    /// has an invalid value.
    pub fn request(&self) -> Result<QueryRequest, QueryError> {
        let yql = self.yql.trim();
        if yql.is_empty() {
            return Err(QueryError::EmptyYql);
        }
        let mut request = QueryRequest::new(yql);

        let mut seen = BTreeSet::new();
        for (name, value) in &self.params {
            let (name, value) = (name.trim(), value.trim());
            match (name, value) {
                (_, "") => continue,
                ("", value) => return Err(QueryError::MissingName(value.to_string())),
                ("yql", _) => return Err(QueryError::YqlParam),
                (name, _) if !seen.insert(name) => {
                    return Err(QueryError::Repeated(name.to_string()));
                }
                ("hits", value) => request.hits = Some(count(name, value)?),
                ("offset", value) => request.offset = Some(count(name, value)?),
                ("timeout", value) => {
                    request.timeout =
                        Some(parse_timeout(value).map_err(QueryError::InvalidTimeout)?);
                }
                (name, value) => {
                    request.extra_fields.insert(name.to_string(), typed(value));
                }
            }
        }
        Ok(request)
    }
}

/// Value of a parameter that is a count, like `hits`.
fn count(name: &str, value: &str) -> Result<u32, QueryError> {
    value.parse().map_err(|source| QueryError::InvalidCount {
        name: name.to_string(),
        source,
    })
}

/// JSON value of a typed parameter.
fn typed(value: &str) -> Value {
    if let Ok(boolean) = value.parse() {
        return Value::Bool(boolean);
    }
    if let Ok(integer) = value.parse::<i64>() {
        return Value::Number(integer.into());
    }
    match value.parse::<f64>().ok().and_then(Number::from_f64) {
        // Only plain decimals, values like "1e5" or "inf" are sent as written
        Some(number) if value.contains('.') => Value::Number(number),
        _ => Value::String(value.to_string()),
    }
}

/// Errors of an invalid [`QueryDraft`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum QueryError {
    /// There is no YQL.
    #[error("The query has no YQL")]
    EmptyYql,
    /// A parameter has a value but no name.
    #[error("The parameter with value {0:?} has no name")]
    MissingName(String),
    /// The YQL is set as a parameter instead of in the editor.
    #[error("The YQL is set in the editor, not as a parameter")]
    YqlParam,
    /// A parameter is set more than once.
    #[error("The parameter {0:?} is set more than once")]
    Repeated(String),
    /// A count, like `hits`, is not a positive integer.
    #[error("The parameter {name:?} must be a positive integer: {source}")]
    InvalidCount {
        /// Name of the parameter.
        name: String,
        /// Why the value is not valid.
        source: ParseIntError,
    },
    /// The timeout is not a duration Vespa understands.
    #[error("Invalid timeout: {0}")]
    InvalidTimeout(ParseError),
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use serde_json::json;

    use super::*;

    fn body(draft: &QueryDraft) -> Value {
        serde_json::to_value(draft.request().unwrap()).unwrap()
    }

    #[test]
    fn test_request_with_params() {
        let draft = QueryDraft::new("  select * from doc where userQuery()\n")
            .param("query", "vespa search")
            .param("hits", "5")
            .param("offset", " 10 ")
            .param("ranking.profile", "bm25")
            .param("presentation.timing", "true")
            .param("ranking.matching.approximateThreshold", "0.05")
            .param("timeout", "500ms");
        let request = draft.request().unwrap();
        assert_eq!(request.yql, "select * from doc where userQuery()");
        assert_eq!(request.hits, Some(5));
        assert_eq!(request.offset, Some(10));
        assert_eq!(request.timeout, Some(Duration::from_millis(500)));
        assert_eq!(
            body(&draft),
            json!({
                "yql": "select * from doc where userQuery()",
                "hits": 5,
                "offset": 10,
                "query": "vespa search",
                "ranking.profile": "bm25",
                "presentation.timing": true,
                "ranking.matching.approximateThreshold": 0.05,
                "timeout": "500ms",
            })
        );
    }

    #[test]
    fn test_params_without_value_are_skipped() {
        let draft = QueryDraft::new("select * from doc where true")
            .param(" ", "")
            .param("hits", " ")
            .param("hits", "");
        assert_eq!(body(&draft), json!({"yql": "select * from doc where true"}));
    }

    #[test]
    fn test_numbers_without_decimals_are_kept_as_written() {
        assert_eq!(typed("1e5"), json!("1e5"));
        assert_eq!(typed("inf"), json!("inf"));
        assert_eq!(typed("-3"), json!(-3));
    }

    #[test]
    fn test_invalid_counts() {
        for value in ["-1", "ten", "1.5"] {
            let draft = QueryDraft::new("select * from doc where true").param("hits", value);
            assert!(
                matches!(draft.request(), Err(QueryError::InvalidCount { name, .. }) if name == "hits"),
                "{value} must be rejected"
            );
        }
    }

    #[test]
    fn test_invalid_timeout() {
        let draft = QueryDraft::new("select * from doc where true").param("timeout", "soon");
        assert!(matches!(
            draft.request(),
            Err(QueryError::InvalidTimeout(_))
        ));
    }

    #[test]
    fn test_invalid_drafts() {
        let yql = "select * from doc where true";
        for (draft, error) in [
            (QueryDraft::new(" \n "), QueryError::EmptyYql),
            (
                QueryDraft::new(yql).param("", "5"),
                QueryError::MissingName("5".to_string()),
            ),
            (QueryDraft::new(yql).param("yql", "x"), QueryError::YqlParam),
            (
                QueryDraft::new(yql).param("hits", "1").param(" hits", "2"),
                QueryError::Repeated("hits".to_string()),
            ),
        ] {
            assert_eq!(draft.request().err(), Some(error));
        }
    }
}
