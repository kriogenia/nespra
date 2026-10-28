use std::time::Duration;

use serde::{Serialize, Serializer};

/// Helper model to build requests.
///
/// While the yql is actually a required parameter it can still be built without providing
/// one to allow Default implementation and flexible building, but requests created without
/// YQL will contain an empty string that will result in a failed request.
///
/// The struct is generic over a serializable type that will be flattened in the root of the
/// body allowing for extension of the default fields. In case that you no extension is needed
/// the `SimpleRequest` type helps to evade generic boilerplate.
///
/// It can be built with [`Request::new`] and the chained setters or filling the public fields.
///
/// ```
/// # use std::time::Duration;
/// # use serde_json::json;
/// use nespra::search::SimpleRequest;
///
/// let request = SimpleRequest::new("select * from doc where true")
///     .hits(5)
///     .search_chain("fast")
///     .timeout(Duration::from_millis(1500))
///     .extra_fields(json!({"query": "Is statin use connected to breast cancer?"}));
///
/// assert_eq!(request.yql, "select * from doc where true");
/// assert_eq!(request.hits.unwrap(), 5);
/// assert!(request.offset.is_none());
///
/// let value = serde_json::to_value(request).unwrap();
/// assert_eq!(value["searchChain"], json!("fast"));
/// assert_eq!(value["timeout"], json!("1500ms"));
/// assert_eq!(value["query"], json!("Is statin use connected to breast cancer?"));
/// assert!(value.get("queryProfile").is_none());
/// ```
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request<E> {
    pub yql: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hits: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grouping_session_cache: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_chain: Option<String>,
    /// Time Vespa is allowed to spend on the query, sent in milliseconds.
    #[serde(
        serialize_with = "serialize_timeout",
        skip_serializing_if = "Option::is_none"
    )]
    pub timeout: Option<Duration>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation: Option<Presentation>,

    #[serde(flatten)]
    pub extra_fields: E,
}

pub type SimpleRequest = Request<()>;

impl<E: Default> Request<E> {
    /// Creates a request with the given YQL and no other fields.
    pub fn new(yql: impl Into<String>) -> Self {
        Self {
            yql: yql.into(),
            ..Default::default()
        }
    }
}

impl<E> Request<E> {
    pub fn hits(mut self, hits: u32) -> Self {
        self.hits = Some(hits);
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn query_profile(mut self, query_profile: impl Into<String>) -> Self {
        self.query_profile = Some(query_profile.into());
        self
    }

    pub fn grouping_session_cache(mut self, grouping_session_cache: bool) -> Self {
        self.grouping_session_cache = Some(grouping_session_cache);
        self
    }

    pub fn search_chain(mut self, search_chain: impl Into<String>) -> Self {
        self.search_chain = Some(search_chain.into());
        self
    }

    /// Time Vespa is allowed to spend on the query, which is not the client timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn presentation(mut self, presentation: Presentation) -> Self {
        self.presentation = Some(presentation);
        self
    }

    /// Sets the fields flattened in the root of the body, changing the type of the request.
    pub fn extra_fields<F>(self, extra_fields: F) -> Request<F> {
        Request {
            yql: self.yql,
            hits: self.hits,
            offset: self.offset,
            query_profile: self.query_profile,
            grouping_session_cache: self.grouping_session_cache,
            search_chain: self.search_chain,
            timeout: self.timeout,
            presentation: self.presentation,
            extra_fields,
        }
    }
}

fn serialize_timeout<S: Serializer>(
    timeout: &Option<Duration>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match timeout {
        Some(timeout) => serializer.serialize_str(&format!("{}ms", timeout.as_millis())),
        None => serializer.serialize_none(),
    }
}

/// Helper model to presentation options in search requests.
///
/// Knowingly evades the `format` option as it would mess with the format specified in the client or
/// the search call. Use those instead.
#[derive(Debug, Default, Serialize)]
pub struct Presentation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bolding: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<bool>,
    #[serde(rename = "format.tensors", skip_serializing_if = "Option::is_none")]
    pub format_tensors: Option<TensorFormat>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum TensorFormat {
    Short,
    Long,
    ShortValue,
    LongValue,
    Hex,
    HexValue,
}

// TODO: other models

#[cfg(test)]
mod test {
    use serde_json::json;

    use std::time::Duration;

    use crate::search::{Presentation, SimpleRequest, TensorFormat};

    #[test]
    fn serialize_presentation() {
        let presentation = Presentation {
            bolding: Some(true),
            summary: Some("default".to_string()),
            format_tensors: Some(TensorFormat::ShortValue),
            ..Default::default()
        };

        let value = serde_json::to_value(presentation).unwrap();

        assert_eq!(*value.get("bolding").unwrap(), json!(true));
        assert_eq!(*value.get("summary").unwrap(), json!("default"));
        assert_eq!(*value.get("format.tensors").unwrap(), json!("short-value"));
        assert!(value.get("template").is_none());
        assert!(value.get("timing").is_none());
    }

    #[test]
    fn serialize_request() {
        let request = SimpleRequest::new("select * from doc where true")
            .hits(10)
            .offset(20)
            .timeout(Duration::from_secs(2));

        let value = serde_json::to_value(request).unwrap();

        assert_eq!(value["yql"], json!("select * from doc where true"));
        assert_eq!(value["hits"], json!(10));
        assert_eq!(value["offset"], json!(20));
        assert_eq!(value["timeout"], json!("2000ms"));
        assert!(value.get("presentation").is_none());
    }

    #[test]
    fn serialize_request_without_timeout() {
        let value = serde_json::to_value(SimpleRequest::new("yql")).unwrap();
        assert!(value.get("timeout").is_none());
    }
}
