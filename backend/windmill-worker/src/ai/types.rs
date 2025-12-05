use crate::ai::providers::openai::OpenAIToolCall;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::HashMap;
use windmill_common::mcp_client::McpToolSource;
use windmill_common::{
    ai_providers::AIProvider, db::DB, error::Error, flow_status::AgentAction, flows::FlowModule,
    s3_helpers::S3Object,
};
use windmill_parser::Typ;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text {
        text: String,
    },
    #[serde(rename = "image_url")]
    ImageUrl {
        image_url: ImageUrlData,
    },
    #[serde(rename = "s3_object")]
    S3Object {
        s3_object: S3Object,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImageUrlData {
    pub url: String, // data:image/png;base64,... or https://...
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum OpenAIContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct OpenAIMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<OpenAIContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing)]
    pub agent_action: Option<AgentAction>,
}

/// same as OpenAIMessage but with agent_action field included in the serialization
#[derive(Serialize)]
pub struct Message<'a> {
    #[serde(flatten)]
    pub message: &'a OpenAIMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_action: Option<&'a AgentAction>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ResponseFormat {
    pub r#type: String,
    pub json_schema: JsonSchemaFormat,
}

#[derive(Serialize, Clone, Debug)]
pub struct JsonSchemaFormat {
    pub name: String,
    pub schema: OpenAPISchema,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ToolDefFunction {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Box<RawValue>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ToolDef {
    pub r#type: String,
    pub function: ToolDefFunction,
}

#[derive(Serialize, Clone, Debug)]
pub struct Tool {
    pub module: Option<FlowModule>,
    pub def: ToolDef,
    pub mcp_source: Option<McpToolSource>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OutputType {
    Text,
    Image,
}

impl Default for OutputType {
    fn default() -> Self {
        OutputType::Text
    }
}

#[derive(Deserialize, Debug)]
pub struct AIAgentArgs {
    pub provider: ProviderWithResource,
    pub system_prompt: Option<String>,
    pub user_message: String,
    pub temperature: Option<f32>,
    pub max_completion_tokens: Option<u32>,
    pub output_schema: Option<OpenAPISchema>,
    pub output_type: Option<OutputType>,
    pub user_images: Option<Vec<S3Object>>,
    pub streaming: Option<bool>,
    pub messages_context_length: Option<usize>,
    pub max_iterations: Option<usize>,
}

#[derive(Deserialize, Debug)]
pub struct ProviderResource {
    #[serde(alias = "apiKey")]
    pub api_key: String,
    #[serde(alias = "baseUrl")]
    pub base_url: Option<String>,
    pub region: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ProviderWithResource {
    pub kind: AIProvider,
    pub resource: ProviderResource,
    pub model: String,
}

impl ProviderWithResource {
    pub fn get_api_key(&self) -> &str {
        &self.resource.api_key
    }

    pub fn get_model(&self) -> &str {
        &self.model
    }

    pub async fn get_base_url(&self, db: &DB) -> Result<String, Error> {
        self.kind
            .get_base_url(
                self.resource.base_url.clone(),
                self.resource.region.clone(),
                db,
            )
            .await
    }

    pub fn get_region(&self) -> Option<&str> {
        self.resource.region.as_deref()
    }
}

#[derive(Serialize)]
pub struct AIAgentResult<'a> {
    pub output: Box<RawValue>,
    pub messages: Vec<Message<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wm_stream: Option<String>,
}

/// Events for streaming AI responses
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamingEvent {
    /// Individual token from the AI response
    TokenDelta { content: String },
    /// Tool call has started
    ToolCall { call_id: String, function_name: String },
    /// Tool call arguments are complete
    ToolCallArguments { call_id: String, function_name: String, arguments: String },
    /// Tool execution has started
    ToolExecution { call_id: String, function_name: String },
    /// Tool execution result
    ToolResult { call_id: String, function_name: String, result: String, success: bool },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum SchemaType {
    Single(String),
    Multiple(Vec<String>),
}

impl Default for SchemaType {
    fn default() -> Self {
        SchemaType::Single("object".to_string())
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct OpenAPISchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<SchemaType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<OpenAPISchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Box<OpenAPISchema>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "oneOf")]
    pub one_of: Option<Vec<Box<OpenAPISchema>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#enum: Option<Vec<String>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "additionalProperties"
    )]
    pub additional_properties: Option<bool>,
}

impl OpenAPISchema {
    pub fn from_str(typ: &str) -> Self {
        OpenAPISchema { r#type: Some(SchemaType::Single(typ.to_string())), ..Default::default() }
    }

    pub fn from_str_with_enum(typ: &str, enu: &Option<Vec<String>>) -> Self {
        OpenAPISchema {
            r#type: Some(SchemaType::Single(typ.to_string())),
            r#enum: enu.clone(),
            ..Default::default()
        }
    }

    pub fn datetime() -> Self {
        Self {
            r#type: Some(SchemaType::Single("string".to_string())),
            format: Some("date-time".to_string()),
            ..Default::default()
        }
    }

    pub fn from_typ(typ: &Typ) -> Self {
        match typ {
            Typ::Str(enu) => Self::from_str_with_enum("string", enu),
            Typ::Int => Self::from_str("integer"),
            Typ::Float => Self::from_str("number"),
            Typ::Bool => Self::from_str("boolean"),
            Typ::Bytes => Self::from_str("string"),
            Typ::Datetime => Self::datetime(),
            Typ::Resource(_) => Self::from_str("string"),
            Typ::Email => Self::from_str("string"),
            Typ::Sql => Self::from_str("string"),
            Typ::DynSelect(_) => Self::from_str("string"),
            Typ::DynMultiselect(_) => Self::from_str("string"),
            Typ::List(typ) => OpenAPISchema {
                r#type: Some(SchemaType::Single("array".to_string())),
                items: Some(Box::new(Self::from_typ(typ))),
                ..Default::default()
            },
            Typ::Object(typ) => OpenAPISchema {
                r#type: Some(SchemaType::Single("object".to_string())),
                items: None,
                properties: typ.props.as_ref().map(|props| {
                    props
                        .iter()
                        .map(|prop| (prop.key.clone(), Box::new(Self::from_typ(&prop.typ))))
                        .collect()
                }),
                required: typ
                    .props
                    .as_ref()
                    .map(|props| props.iter().map(|prop| prop.key.clone()).collect()),
                ..Default::default()
            },
            Typ::OneOf(variants) => OpenAPISchema {
                r#type: Some(SchemaType::Single("object".to_string())),
                one_of: Some(
                    variants
                        .iter()
                        .map(|variant| {
                            let schema = OpenAPISchema {
                                r#type: Some(SchemaType::Single("object".to_string())),
                                properties: Some(
                                    variant
                                        .properties
                                        .iter()
                                        .map(|prop| {
                                            (
                                                prop.key.clone(),
                                                Box::new(
                                                    if prop.key == "label" || prop.key == "kind" {
                                                        Self::from_str_with_enum(
                                                            "string",
                                                            &Some(vec![variant.label.clone()]),
                                                        )
                                                    } else {
                                                        Self::from_typ(&prop.typ)
                                                    },
                                                ),
                                            )
                                        })
                                        .collect(),
                                ),
                                required: Some(
                                    variant
                                        .properties
                                        .iter()
                                        .map(|prop| prop.key.clone())
                                        .collect(),
                                ),
                                ..Default::default()
                            };
                            Box::new(schema)
                        })
                        .collect(),
                ),
                ..Default::default()
            },
            Typ::Unknown => Self::from_str("object"),
        }
    }

    /// Makes this schema compatible with OpenAI's strict mode by:
    /// - Adding additionalProperties: false to all object types
    /// - Making non-required properties nullable
    /// - Ensuring all properties are in the required array
    pub fn make_strict(mut self) -> Self {
        // Handle this schema if it's an object type
        if let Some(SchemaType::Single(ref type_str)) = self.r#type {
            if type_str == "object" {
                // Set additionalProperties to false
                self.additional_properties = Some(false);

                if let Some(properties) = self.properties.as_mut() {
                    // Get original required fields
                    let original_required = self.required.as_ref();

                    if let Some(required) = original_required {
                        // Update properties to make non-required fields nullable
                        for (key, prop) in properties.iter_mut() {
                            let mut new_prop = (**prop).clone();
                            // Make non-required fields nullable
                            if !required.contains(key) {
                                new_prop = new_prop.make_nullable();
                            }
                            // Recursively make nested schemas strict
                            new_prop = new_prop.make_strict();
                            *prop = Box::new(new_prop);
                        }
                    }

                    // All properties must be in required array for strict mode
                    self.required = Some(properties.keys().cloned().collect());
                }
            }
        }

        // Recursively process nested schemas
        if let Some(ref mut items) = self.items {
            **items = items.as_ref().clone().make_strict();
        }

        if let Some(ref mut one_of) = self.one_of {
            *one_of = one_of
                .iter()
                .map(|schema| Box::new(schema.as_ref().clone().make_strict()))
                .collect();
        }

        self
    }

    /// Makes this property nullable by converting its type to a union with null
    pub fn make_nullable(mut self) -> Self {
        match self.r#type.take() {
            Some(SchemaType::Single(type_str)) => {
                if type_str != "null" {
                    self.r#type = Some(SchemaType::Multiple(vec![type_str, "null".into()]));
                } else {
                    self.r#type = Some(SchemaType::Single("null".into()));
                }
            }
            Some(SchemaType::Multiple(mut types)) => {
                if !types.iter().any(|t| t == "null") {
                    types.push("null".into());
                }
                self.r#type = Some(SchemaType::Multiple(types));
            }
            None => {
                self.r#type = Some(SchemaType::Single("null".into()));
            }
        }
        self
    }
}

/// Wrapper for S3Object with type discriminator for conversation storage
#[derive(Serialize)]
pub struct S3ObjectWithType {
    #[serde(flatten)]
    pub s3_object: S3Object,
    pub r#type: String,
}
