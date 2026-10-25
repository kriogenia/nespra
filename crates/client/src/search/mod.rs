mod request;
mod response;

pub use request::*;
pub use response::*; // TODO: hide behind feature

use serde::{Serialize, de::DeserializeOwned};

use crate::{Vespa, error::SearchError};

impl Vespa {
    /// Runs a search request with the specified body and returns a results
    /// with the modelled response or the search error.
    ///
    /// # Request
    ///
    /// Can be any JSON serializable object. This means that you can use the
    /// helping `search::Request` struct, a string built `json!` invokation or
    /// use a custom struct implementating Serialize.
    ///
    /// # Response
    ///
    /// Must be deserializable representation of the documents returned in the
    /// response. The rest of fields are already implemented in the the `search::Response`
    /// model. In case that you want a simple object you can just use a `json::Value` or
    /// any other generic type.
    ///
    /// # Errors
    ///
    /// Any of `vespa_client::error::SearchError`.
    pub async fn search<B: Serialize + ?Sized, H: DeserializeOwned>(
        &self,
        body: &B,
    ) -> Result<Response<H>, SearchError> {
        let response: Response<H> = self
            .client
            .post(&self.search_url)
            // TODO: .version(Version::HTTP_2)
            .json(body)
            .send()
            .await
            .map_err(SearchError::Request)?
            .json()
            .await
            .map_err(SearchError::Response)?;

        if response.root.errors.is_empty() {
            Ok(response)
        } else {
            Err(SearchError::Vespa(response.root.errors))
        }
    }
}

#[cfg(test)]
mod test {
    use serde::Deserialize;
    use serde_json::Value;

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
            "relevance": 0.56
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
