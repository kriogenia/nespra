use serde::{Deserialize, Serialize};

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
/// ```
/// # use serde_json;
///
/// let request = vespa_client::search::Request::<serde_json::Value> {
///     yql: "select * from doc where true",
///     hits: Some(5),
///     search_chain: Some("fast"),
///     extra_fields: serde_json::json!({"query": "Is statin use connected to breast cancer?"}),
///     ..Default::default()
/// };
///
/// assert_eq!(request.yql, "select * from doc where true");
/// assert_eq!(request.hits.unwrap(), 5);
/// assert!(request.offset.is_none());
///
/// let mut value = serde_json::to_value(request).unwrap();
/// assert_eq!(*value.get("searchChain").unwrap(), serde_json::json!("fast"));
/// assert_eq!(*value.get("query").unwrap(), serde_json::json!("Is statin use connected to breast cancer?"));
/// assert!(value.get("queryProfile").is_none());
/// ```
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request<'a, E> {
    pub yql: &'a str,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hits: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_profile: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grouping_session_cache: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_chain: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<&'a str>,

    #[serde(flatten)]
    pub extra_fields: E,
}

pub type SimpleRequest<'a> = Request<'a, ()>;

// TODO: rest of query API: https://docs.vespa.ai/en/reference/api/query.html
