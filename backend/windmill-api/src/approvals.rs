use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct ResumeSchema {
    pub schema: Schema,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Schema {
    pub order: Vec<String>,
    pub required: Vec<String>,
    pub properties: HashMap<String, ResumeFormField>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Boolean,
    String,
    Number,
    Integer,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResumeFormField {
    pub r#type: FieldType,
    pub format: Option<String>,
    pub default: Option<serde_json::Value>,
    pub description: Option<String>,
    pub title: Option<String>,
    pub r#enum: Option<Vec<String>>,
    #[serde(rename = "enumLabels")]
    pub enum_labels: Option<HashMap<String, String>>,
    pub nullable: Option<bool>,
    pub placeholder: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResumeFormRow {
    pub resume_form: Option<serde_json::Value>,
    pub hide_cancel: Option<bool>,
}

#[derive(Deserialize)]
pub struct QueryMessage {
    pub message: Option<String>,
}

#[derive(Deserialize)]
pub struct QueryFlowStepId {
    pub flow_step_id: String,
}

#[derive(Deserialize, Debug)]
pub struct QueryDefaultArgsJson {
    pub default_args_json: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct QueryDynamicEnumJson {
    pub dynamic_enums_json: Option<serde_json::Value>,
}
