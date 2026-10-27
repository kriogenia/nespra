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
/// let request = nespra::search::Request::<serde_json::Value> {
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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation: Option<Presentation<'a>>,

    #[serde(flatten)]
    pub extra_fields: E,
}

pub type SimpleRequest<'a> = Request<'a, ()>;

/// Helper model to presentation options in search requests.
///
/// Knowingly evades the `format` option as the client is not designed for any
/// other format than JSON (for now) and CBOR in the future.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Presentation<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bolding: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<bool>,
    #[serde(rename = "format.tensors", skip_serializing_if = "Option::is_none")]
    pub format_tensors: Option<TensorFormat>,
}

#[derive(Debug, Deserialize, Serialize)]
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

    use crate::search::{Presentation, TensorFormat};

    #[test]
    fn serialize_presentation() {
        let presentation = Presentation {
            bolding: Some(true),
            summary: Some("default"),
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
}
