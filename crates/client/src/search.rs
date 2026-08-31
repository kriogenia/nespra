use std::collections::BTreeMap;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{Vespa, error::HttpError};

#[derive(Debug, Deserialize, Serialize)]
pub struct Response<T> {
    pub root: Root<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<Timing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Value>, // TODO: model
}

impl<T> Response<T> {
    pub fn hits(&self) -> &Vec<T> {
        &self.root.children
    }

    pub fn total_count(&self) -> i32 {
        self.root.fields.as_ref().map_or_default(|f| f.total_count)
    }

    pub fn indexed_docs_count(&self) -> i64 {
        self.root.coverage.as_ref().map_or_default(|c| c.documents)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Root<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub relevance: f64,
    pub children: Vec<T>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub continuation: BTreeMap<String, String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<Limits>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Fields>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage: Option<Coverage>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<Error>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Limits {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fields {
    pub total_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_group: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    pub coverage: u8,
    pub documents: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degraded: Option<Degraded>,
    pub full: bool,
    pub nodes: i32,
    pub results: i32,
    pub results_full: i32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Degraded {
    pub match_phase: bool,
    pub timeout: bool,
    pub adaptive_timeout: bool,
    #[serde(rename = "anntimeout")]
    pub ann_timeout: bool,
    pub non_ideal_state: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub code: i32,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Timing {
    #[serde(rename = "querytime", skip_serializing_if = "Option::is_none")]
    pub query_time: Option<f64>,
    #[serde(rename = "summaryfetchtime", skip_serializing_if = "Option::is_none")]
    pub summary_fetch_time: Option<f64>,
    #[serde(rename = "searchtime", skip_serializing_if = "Option::is_none")]
    pub search_time: Option<f64>,
}

impl Vespa {
    pub async fn search<B: Serialize + ?Sized, H: DeserializeOwned>(
        &self,
        body: &B,
    ) -> Result<Response<H>, HttpError> {
        let response = self
            .client
            .post(&self.search_url)
            // TODO: .version(Version::HTTP_2)
            .json(body)
            .send()
            .await
            .map_err(HttpError::Request)?
            .json()
            .await
            .map_err(HttpError::Response)?;

        // TODO: check if Vespa responses can have both results and errors
        // to evaluate if we should transform the responses with errors into
        // a Error and return that as part of the query

        Ok(response)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::assert_matches;

    #[test]
    fn test_response_deserialize_all() {
        let json = r#"
        {
          "root": {
            "id": "toplevel",
            "relevance": 0.96,
            "children": [
              {
                "fields": {
                  "title": "Statin use",
                  "body": "Recent studies"
                }
              }
            ],
            "types": [ "type" ],
            "source": "somewhere",
            "continuation": {
              "key": "value"
            },
            "label": "name",
            "value": "a lot",
            "limits": {
              "from": "start",
              "to": "end"
            },
            "fields": {
              "totalCount": 97,
              "searchGroup": 2
            },
            "coverage": {
              "coverage": 100,
              "documents": 100,
              "full": true,
              "nodes": 1,
              "results": 1,
              "resultsFull": 1,
              "degraded": {
                "match-phase": true,
                "timeout": true,
                "adaptive-timeout": true,
                "anntimeout": true,
                "non-ideal-state": true
              }
            },
            "errors": [
              {
                "code": 314,
                "summary": "pi",
                "source": "a dream",
                "message": "detailed",
                "stackTrace": "NullPointer"
              }
            ]
          },
          "timing": {
            "querytime": 0.1,
            "summaryfetchtime": 0.2,
            "searchtime": 0.3
          },
          "trace": {
            "children": [
              {
                "message": "tracing message"
              }
            ]
          }
        }
        "#;
        let mut deserializer = serde_json::Deserializer::from_str(json);

        let response = Response::<Value>::deserialize(&mut deserializer).unwrap();
        assert!(matches!(
            response,
            Response {
                root: Root {
                    relevance: 0.96,
                    fields: Some(Fields {
                        total_count: 97,
                        search_group: Some(2)
                    }),
                    coverage: Some(Coverage {
                        coverage: 100,
                        documents: 100,
                        degraded: Some(Degraded {
                            match_phase: true,
                            timeout: true,
                            adaptive_timeout: true,
                            ann_timeout: true,
                            non_ideal_state: true
                        }),
                        full: true,
                        nodes: 1,
                        results: 1,
                        results_full: 1
                    }),
                    ..
                },
                timing: Some(Timing {
                    query_time: Some(0.1),
                    summary_fetch_time: Some(0.2),
                    search_time: Some(0.3),
                }),
                trace: Some(Value::Object(_)),
            },
        ));

        let root = response.root;
        assert_eq!(root.id.unwrap(), "toplevel");

        assert_eq!(root.children.len(), 1);
        let doc = root.children[0].as_object().unwrap();
        let doc = doc.get("fields").unwrap().as_object().unwrap();
        assert_eq!(doc.get("title").unwrap().as_str().unwrap(), "Statin use");
        assert_eq!(doc.get("body").unwrap().as_str().unwrap(), "Recent studies");

        assert_eq!(root.types, vec!["type"]);
        assert_eq!(root.source.unwrap(), "somewhere");
        assert_eq!(root.label.unwrap(), "name");
        assert_eq!(root.value.unwrap(), "a lot");

        assert_eq!(root.continuation.len(), 1);
        assert_eq!(root.continuation["key"], "value");

        let limits = root.limits.unwrap();
        assert_eq!(limits.from, "start");
        assert_eq!(limits.to, "end");

        assert_eq!(root.errors.len(), 1);
        assert_eq!(root.errors[0].code, 314);
        assert_eq!(root.errors[0].summary, "pi");
        assert_eq!(root.errors[0].source.as_ref().unwrap(), "a dream");
        assert_eq!(root.errors[0].message.as_ref().unwrap(), "detailed");
        assert_eq!(root.errors[0].stack_trace.as_ref().unwrap(), "NullPointer");

        let trace = response.trace.unwrap();
        let children = trace.get("children").unwrap().as_array().unwrap();
        assert_eq!(children.len(), 1);
        assert!(children[0].is_object());
    }

    #[test]
    fn test_response_deserialize_minimal() {
        let json = r#"
        {
          "root": {
            "relevance": 0.56,
            "children": []
          }
        }
        "#;
        let mut deserializer = serde_json::Deserializer::from_str(json);

        let root: Root<Value> = Response::deserialize(&mut deserializer).unwrap().root;
        assert_matches!(
            root,
            Root {
                relevance: 0.56,
                ..
            }
        );

        assert!(root.children.is_empty());
        assert!(root.types.is_empty());
        assert!(root.continuation.is_empty());
        assert!(root.errors.is_empty());
    }
}
