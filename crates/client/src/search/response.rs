use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use serde_json::Value;

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

    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
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
