use serde::{Serialize, de::DeserializeOwned};

use crate::error::CodecError;

/// Format of the response bodies returned by Vespa.
///
/// CBOR is the default as it is smaller and faster than JSON, but requires
/// Vespa 8.623.5 or later. Request bodies are always sent as JSON, as Vespa
/// does not accept them in CBOR.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Format {
    /// Binary CBOR encoding.
    #[default]
    Cbor,
    /// JSON encoding.
    Json,
}

impl Format {
    /// MIME type to request in the `Accept` header.
    pub(crate) fn mime(&self) -> &'static str {
        match self {
            Format::Cbor => "application/cbor",
            Format::Json => "application/json",
        }
    }

    /// Decodes the sequence of bytes using the specific format.
    pub(crate) fn decode<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T, CodecError> {
        match self {
            Format::Cbor => ciborium::from_reader(bytes).map_err(|e| CodecError::Decode(e.into())),
            Format::Json => serde_json::from_slice(bytes).map_err(|e| CodecError::Decode(e.into())),
        }
    }
}

/// Encodes a request body. Vespa only understands JSON requests, whatever
/// the `Format` used for the response.
pub(crate) fn encode_request<B: Serialize + ?Sized>(body: &B) -> Result<Vec<u8>, CodecError> {
    serde_json::to_vec(body).map_err(|e| CodecError::Encode(e.into()))
}

#[cfg(test)]
mod test {
    use serde_json::{Value, json};

    use super::*;
    use crate::search::Response;

    fn sample() -> Value {
        json!({
            "root": {
                "id": "toplevel",
                "relevance": 0.96,
                "children": [{ "fields": { "title": "Statin use", "body": "Recent studies" } }],
                "fields": { "totalCount": 97 },
                "coverage": {
                    "coverage": 100, "documents": 5_000_000_000_i64, "full": true,
                    "nodes": 1, "results": 1, "resultsFull": 1
                },
                "errors": [{ "code": 314, "summary": "pi" }]
            },
            "timing": { "querytime": 0.1 },
            "trace": { "children": [{ "message": "tracing message" }] }
        })
    }

    fn assert_decoded(format: Format, bytes: &[u8]) {
        let response: Response<Value> = format.decode(bytes).unwrap();

        assert_eq!(response.root.relevance, 0.96);
        assert_eq!(response.total_count(), 97);
        assert_eq!(response.indexed_docs_count(), 5_000_000_000);
        assert_eq!(response.root.errors[0].code, 314);
        assert_eq!(serde_json::to_value(&response).unwrap(), sample());
    }

    #[test]
    fn test_cbor_decode() {
        let mut bytes = Vec::new();
        ciborium::into_writer(&sample(), &mut bytes).unwrap();
        assert_decoded(Format::Cbor, &bytes);
    }

    #[test]
    fn test_json_decode() {
        assert_decoded(Format::Json, &serde_json::to_vec(&sample()).unwrap());
    }

    #[test]
    fn test_requests_are_encoded_as_json() {
        let bytes = encode_request(&json!({"yql": "select * from doc where true"})).unwrap();
        assert_eq!(bytes, br#"{"yql":"select * from doc where true"}"#);
    }

    #[test]
    fn test_decode_garbage_fails() {
        let result = Format::default().decode::<Response<Value>>(&[0xff, 0x00, 0x13]);
        assert!(matches!(result, Err(CodecError::Decode(_))));
    }

    #[test]
    fn test_default_and_mime() {
        assert_eq!(Format::default(), Format::Cbor);
        assert_eq!(Format::Cbor.mime(), "application/cbor");
    }

    #[test]
    fn test_json_mime() {
        assert_eq!(Format::Json.mime(), "application/json");
    }
}
