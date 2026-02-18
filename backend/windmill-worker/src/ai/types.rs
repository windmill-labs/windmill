use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::HashMap;
use uuid::Uuid;
#[cfg(feature = "mcp")]
pub use windmill_mcp::McpToolSource;

/// Stub type when mcp feature is not enabled
#[cfg(not(feature = "mcp"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpToolSource {
    pub name: String,
    pub tool_name: String,
    pub resource_path: String,
}
use windmill_common::{
    ai_providers::{empty_string_as_none, AIProvider},
    db::DB,
    error::Error,
    flow_status::AgentAction,
    flows::FlowModule,
};
use windmill_types::s3::S3Object;
use windmill_parser::Typ;

// Re-export shared types from windmill_common::ai_types
pub use windmill_common::ai_types::{
    ContentPart, ImageUrlData, OpenAIContent, OpenAIMessage, ToolDef, ToolDefFunction, UrlCitation,
};

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

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Memory {
    Off,
    Auto {
        #[serde(default)]
        context_length: usize,
        #[serde(default)]
        memory_id: Option<Uuid>,
    },
    Manual {
        messages: Vec<OpenAIMessage>,
    },
}

#[derive(Deserialize)]
struct AIAgentArgsRaw {
    provider: ProviderWithResource,
    system_prompt: Option<String>,
    user_message: Option<String>,
    temperature: Option<f32>,
    max_completion_tokens: Option<u32>,
    output_schema: Option<OpenAPISchema>,
    output_type: Option<OutputType>,
    user_images: Option<Vec<S3Object>>,
    streaming: Option<bool>,
    max_iterations: Option<usize>,
    memory: Option<Memory>,
    // Legacy field for backward compatibility
    messages_context_length: Option<usize>,
    #[serde(default)]
    credentials_check: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(from = "AIAgentArgsRaw")]
pub struct AIAgentArgs {
    pub provider: ProviderWithResource,
    pub system_prompt: Option<String>,
    pub user_message: Option<String>,
    pub temperature: Option<f32>,
    pub max_completion_tokens: Option<u32>,
    pub output_schema: Option<OpenAPISchema>,
    pub output_type: Option<OutputType>,
    pub user_images: Option<Vec<S3Object>>,
    pub streaming: Option<bool>,
    pub max_iterations: Option<usize>,
    pub memory: Option<Memory>,
    pub credentials_check: bool,
}

impl From<AIAgentArgsRaw> for AIAgentArgs {
    fn from(raw: AIAgentArgsRaw) -> Self {
        // Backward compatibility: if messages_context_length is set, use auto mode
        let memory = raw.memory.or_else(|| {
            raw.messages_context_length
                .map(|context_length| Memory::Auto { context_length, memory_id: None })
        });

        // Backward compatibility: if context_length is 0, use off mode
        let memory = memory.map(|memory| {
            if let Memory::Auto { context_length: 0, .. } = memory {
                Memory::Off
            } else {
                memory
            }
        });

        AIAgentArgs {
            provider: raw.provider,
            system_prompt: raw.system_prompt,
            user_message: raw.user_message,
            temperature: raw.temperature,
            max_completion_tokens: raw.max_completion_tokens,
            output_schema: raw.output_schema,
            output_type: raw.output_type,
            user_images: raw.user_images,
            streaming: raw.streaming,
            max_iterations: raw.max_iterations,
            memory,
            credentials_check: raw.credentials_check.unwrap_or(false),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AnthropicPlatform {
    #[default]
    Standard,
    GoogleVertexAi,
}

#[derive(Deserialize, Debug)]
pub struct ProviderResource {
    #[serde(alias = "apiKey", default, deserialize_with = "empty_string_as_none")]
    pub api_key: Option<String>,
    #[serde(alias = "baseUrl", default, deserialize_with = "empty_string_as_none")]
    pub base_url: Option<String>,
    #[allow(dead_code)]
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub region: Option<String>,
    #[allow(dead_code)]
    #[serde(
        alias = "awsAccessKeyId",
        default,
        deserialize_with = "empty_string_as_none"
    )]
    pub aws_access_key_id: Option<String>,
    #[allow(dead_code)]
    #[serde(
        alias = "awsSecretAccessKey",
        default,
        deserialize_with = "empty_string_as_none"
    )]
    pub aws_secret_access_key: Option<String>,
    #[allow(dead_code)]
    #[serde(
        alias = "awsSessionToken",
        default,
        deserialize_with = "empty_string_as_none"
    )]
    pub aws_session_token: Option<String>,
    /// Platform for Anthropic API (standard or google_vertex_ai)
    #[serde(default)]
    pub platform: AnthropicPlatform,
    /// Enable 1M context window for Anthropic
    #[serde(alias = "enable_1M_context", default)]
    pub enable_1m_context: bool,
}

#[derive(Deserialize, Debug)]
pub struct ProviderWithResource {
    pub kind: AIProvider,
    pub resource: ProviderResource,
    pub model: String,
}

impl ProviderWithResource {
    pub fn get_api_key(&self) -> Option<&str> {
        self.resource.api_key.as_deref()
    }

    pub fn get_model(&self) -> &str {
        &self.model
    }

    pub async fn get_base_url(&self, db: &DB) -> Result<String, Error> {
        self.kind
            .get_base_url(self.resource.base_url.clone(), db)
            .await
    }

    #[cfg(feature = "bedrock")]
    pub fn get_region(&self) -> Option<&str> {
        self.resource.region.as_deref()
    }

    #[cfg(feature = "bedrock")]
    pub fn get_aws_access_key_id(&self) -> Option<&str> {
        self.resource.aws_access_key_id.as_deref()
    }

    #[cfg(feature = "bedrock")]
    pub fn get_aws_secret_access_key(&self) -> Option<&str> {
        self.resource.aws_secret_access_key.as_deref()
    }

    #[cfg(feature = "bedrock")]
    pub fn get_aws_session_token(&self) -> Option<&str> {
        self.resource.aws_session_token.as_deref()
    }

    pub fn get_platform(&self) -> &AnthropicPlatform {
        &self.resource.platform
    }

    pub fn get_enable_1m_context(&self) -> bool {
        self.resource.enable_1m_context
    }
}

/// Token usage information from the AI provider
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TokenUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_input_tokens: Option<i32>,
}

impl TokenUsage {
    /// Create a new TokenUsage with basic token counts
    pub fn new(input: Option<i32>, output: Option<i32>, total: Option<i32>) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            total_tokens: total,
            cache_read_input_tokens: None,
            cache_write_input_tokens: None,
        }
    }

    /// Create a new TokenUsage with input/output tokens and compute total
    pub fn from_input_output(input: Option<i32>, output: Option<i32>) -> Self {
        let total = match (input, output) {
            (Some(i), Some(o)) => Some(i.saturating_add(o)),
            _ => None,
        };
        Self::new(input, output, total)
    }

    /// Add cache token information
    pub fn with_cache(mut self, read: Option<i32>, write: Option<i32>) -> Self {
        self.cache_read_input_tokens = read;
        self.cache_write_input_tokens = write;
        self
    }

    pub fn is_empty(&self) -> bool {
        self.input_tokens.is_none()
            && self.output_tokens.is_none()
            && self.total_tokens.is_none()
            && self.cache_read_input_tokens.is_none()
            && self.cache_write_input_tokens.is_none()
    }

    /// Accumulate another TokenUsage into this one (all fields including cache tokens)
    /// Uses saturating addition to prevent overflow in long-running agents
    pub fn accumulate(&mut self, other: &TokenUsage) {
        fn add_option(a: Option<i32>, b: Option<i32>) -> Option<i32> {
            match (a, b) {
                (Some(x), Some(y)) => Some(x.saturating_add(y)),
                (Some(x), None) | (None, Some(x)) => Some(x),
                (None, None) => None,
            }
        }
        self.input_tokens = add_option(self.input_tokens, other.input_tokens);
        self.output_tokens = add_option(self.output_tokens, other.output_tokens);
        self.total_tokens = add_option(self.total_tokens, other.total_tokens);
        self.cache_read_input_tokens =
            add_option(self.cache_read_input_tokens, other.cache_read_input_tokens);
        self.cache_write_input_tokens = add_option(
            self.cache_write_input_tokens,
            other.cache_write_input_tokens,
        );
    }
}

#[derive(Serialize)]
pub struct AIAgentResult<'a> {
    pub output: Box<RawValue>,
    pub messages: Vec<Message<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wm_stream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum AdditionalProperties {
    Bool(bool),
    Schema(Box<OpenAPISchema>),
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct OpenAPISchema {
    // Core type fields
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
    pub additional_properties: Option<AdditionalProperties>,

    // Schema metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "$schema")]
    pub schema_url: Option<String>,

    // References and definitions
    #[serde(skip_serializing_if = "Option::is_none", rename = "$ref")]
    pub ref_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "$defs")]
    pub defs: Option<HashMap<String, Box<OpenAPISchema>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definitions: Option<HashMap<String, Box<OpenAPISchema>>>,

    // Schema composition
    #[serde(skip_serializing_if = "Option::is_none", rename = "allOf")]
    pub all_of: Option<Vec<Box<OpenAPISchema>>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "anyOf")]
    pub any_of: Option<Vec<Box<OpenAPISchema>>>,

    // Value constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#const: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,

    // String constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "minLength")]
    pub min_length: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxLength")]
    pub max_length: Option<u64>,

    // Number constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "exclusiveMinimum")]
    pub exclusive_minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "exclusiveMaximum")]
    pub exclusive_maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "multipleOf")]
    pub multiple_of: Option<f64>,

    // Array constraints
    #[serde(skip_serializing_if = "Option::is_none", rename = "minItems")]
    pub min_items: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxItems")]
    pub max_items: Option<u64>,
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
            Typ::Date => Self {
                r#type: Some(SchemaType::Single("string".to_string())),
                format: Some("date".to_string()),
                ..Default::default()
            },
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
    /// - Flattening allOf schemas (not supported by OpenAI strict mode)
    /// - Adding additionalProperties: false to all object types (if not already set)
    /// - Making non-required properties nullable
    /// - Ensuring all properties are in the required array
    pub fn make_strict(&mut self) {
        // First, flatten any allOf schemas since OpenAI strict mode doesn't support them
        self.flatten_all_of();

        // Handle this schema if it's an object type
        if let Some(SchemaType::Single(ref type_str)) = self.r#type {
            if type_str == "object" {
                // Only set additionalProperties to false if not already set
                // If user provided a value (bool or schema), preserve it and let OpenAI handle it
                if self.additional_properties.is_none() {
                    self.additional_properties = Some(AdditionalProperties::Bool(false));
                }

                if let Some(properties) = self.properties.as_mut() {
                    // Get original required fields
                    let original_required = self.required.clone();

                    // Always iterate over properties to recursively process nested schemas
                    for (key, prop) in properties.iter_mut() {
                        // Make non-required fields nullable (only if there were required fields specified)
                        if let Some(ref required) = original_required {
                            if !required.contains(key) {
                                prop.make_nullable();
                            }
                        }
                        // Recursively make nested schemas strict
                        prop.make_strict();
                    }

                    // All properties must be in required array for strict mode
                    self.required = Some(properties.keys().cloned().collect());
                }
            }
        }

        // Recursively process nested schemas
        if let Some(ref mut items) = self.items {
            items.make_strict();
        }

        if let Some(ref mut one_of) = self.one_of {
            for schema in one_of.iter_mut() {
                schema.make_strict();
            }
        }

        // Process anyOf schemas (supported by OpenAI)
        if let Some(ref mut any_of) = self.any_of {
            for schema in any_of.iter_mut() {
                schema.make_strict();
            }
        }

        // Process $defs schemas (definitions are used via $ref)
        if let Some(ref mut defs) = self.defs {
            for (_, schema) in defs.iter_mut() {
                schema.make_strict();
            }
        }

        // Process definitions schemas
        if let Some(ref mut definitions) = self.definitions {
            for (_, schema) in definitions.iter_mut() {
                schema.make_strict();
            }
        }
    }

    /// Makes this property nullable by converting its type to a union with null
    pub fn make_nullable(&mut self) {
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
    }

    /// Flattens all schemas in `allOf` into this schema, then removes `allOf`.
    /// This is needed because OpenAI strict mode doesn't support allOf.
    /// The allOf schemas are recursively flattened first, then merged.
    pub fn flatten_all_of(&mut self) {
        if let Some(all_of) = self.all_of.take() {
            for mut schema in all_of {
                // Recursively flatten any nested allOf in the schema being merged
                schema.flatten_all_of();
                self.merge_from(&schema);
            }
        }
    }

    /// Merges fields from another schema into this one.
    /// Properties from `other` are added if they don't already exist in `self`.
    /// Required fields from `other` are appended to `self`'s required array.
    fn merge_from(&mut self, other: &OpenAPISchema) {
        // Merge type (take other's if self has none)
        if self.r#type.is_none() {
            self.r#type = other.r#type.clone();
        }

        // Merge properties (other's properties are added if key doesn't exist)
        if let Some(ref other_props) = other.properties {
            let props = self.properties.get_or_insert_with(HashMap::new);
            for (key, value) in other_props {
                props.entry(key.clone()).or_insert_with(|| value.clone());
            }
        }

        // Merge required arrays (deduplicated)
        if let Some(ref other_required) = other.required {
            let required = self.required.get_or_insert_with(Vec::new);
            for item in other_required {
                if !required.contains(item) {
                    required.push(item.clone());
                }
            }
        }

        // Merge items (for arrays) - take other's if self has none
        if self.items.is_none() && other.items.is_some() {
            self.items = other.items.clone();
        }

        // Merge additionalProperties - take other's if self has none
        if self.additional_properties.is_none() && other.additional_properties.is_some() {
            self.additional_properties = other.additional_properties.clone();
        }

        // Merge format - take other's if self has none
        if self.format.is_none() && other.format.is_some() {
            self.format = other.format.clone();
        }

        // Merge enum values - combine them if both have enums
        if let Some(ref other_enum) = other.r#enum {
            let enums = self.r#enum.get_or_insert_with(Vec::new);
            for item in other_enum {
                if !enums.contains(item) {
                    enums.push(item.clone());
                }
            }
        }

        // Merge title/description - take other's if self has none
        if self.title.is_none() && other.title.is_some() {
            self.title = other.title.clone();
        }
        if self.description.is_none() && other.description.is_some() {
            self.description = other.description.clone();
        }

        // Merge $defs and definitions
        if let Some(ref other_defs) = other.defs {
            let defs = self.defs.get_or_insert_with(HashMap::new);
            for (key, value) in other_defs {
                defs.entry(key.clone()).or_insert_with(|| value.clone());
            }
        }
        if let Some(ref other_definitions) = other.definitions {
            let definitions = self.definitions.get_or_insert_with(HashMap::new);
            for (key, value) in other_definitions {
                definitions
                    .entry(key.clone())
                    .or_insert_with(|| value.clone());
            }
        }

        // Merge numeric constraints - use the more restrictive values
        if other.minimum.is_some() {
            self.minimum = match (self.minimum, other.minimum) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (None, Some(b)) => Some(b),
                (a, None) => a,
            };
        }
        if other.maximum.is_some() {
            self.maximum = match (self.maximum, other.maximum) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (None, Some(b)) => Some(b),
                (a, None) => a,
            };
        }

        // Merge string constraints
        if other.min_length.is_some() {
            self.min_length = match (self.min_length, other.min_length) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (None, Some(b)) => Some(b),
                (a, None) => a,
            };
        }
        if other.max_length.is_some() {
            self.max_length = match (self.max_length, other.max_length) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (None, Some(b)) => Some(b),
                (a, None) => a,
            };
        }

        // Merge array constraints
        if other.min_items.is_some() {
            self.min_items = match (self.min_items, other.min_items) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (None, Some(b)) => Some(b),
                (a, None) => a,
            };
        }
        if other.max_items.is_some() {
            self.max_items = match (self.max_items, other.max_items) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (None, Some(b)) => Some(b),
                (a, None) => a,
            };
        }

        // Merge pattern - take other's if self has none (can't really combine patterns)
        if self.pattern.is_none() && other.pattern.is_some() {
            self.pattern = other.pattern.clone();
        }
    }

    /// Sanitizes this schema for Google AI's API by removing unsupported fields.
    /// See https://github.com/windmill-labs/windmill/issues/7759
    pub fn sanitize_for_google(&mut self) {
        self.schema_url = None;
        self.default = None;
        self.exclusive_minimum = None;
        self.exclusive_maximum = None;
        self.r#const = None;
        self.multiple_of = None;

        // Recursively sanitize nested schemas
        if let Some(ref mut items) = self.items {
            items.sanitize_for_google();
        }

        if let Some(ref mut properties) = self.properties {
            for prop in properties.values_mut() {
                prop.sanitize_for_google();
            }
        }

        if let Some(ref mut one_of) = self.one_of {
            for schema in one_of.iter_mut() {
                schema.sanitize_for_google();
            }
        }

        if let Some(ref mut any_of) = self.any_of {
            for schema in any_of.iter_mut() {
                schema.sanitize_for_google();
            }
        }

        if let Some(ref mut all_of) = self.all_of {
            for schema in all_of.iter_mut() {
                schema.sanitize_for_google();
            }
        }

        if let Some(ref mut defs) = self.defs {
            for schema in defs.values_mut() {
                schema.sanitize_for_google();
            }
        }

        if let Some(ref mut definitions) = self.definitions {
            for schema in definitions.values_mut() {
                schema.sanitize_for_google();
            }
        }
    }
}

/// Wrapper for S3Object with type discriminator for conversation storage
#[derive(Serialize)]
pub struct S3ObjectWithType {
    #[serde(flatten)]
    pub s3_object: S3Object,
    pub r#type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Helper to create a simple string type schema
    fn string_schema() -> OpenAPISchema {
        OpenAPISchema {
            r#type: Some(SchemaType::Single("string".to_string())),
            ..Default::default()
        }
    }

    /// Helper to create a simple integer type schema
    fn integer_schema() -> OpenAPISchema {
        OpenAPISchema {
            r#type: Some(SchemaType::Single("integer".to_string())),
            ..Default::default()
        }
    }

    /// Helper to create an object schema with given properties
    fn object_schema(properties: Vec<(&str, OpenAPISchema)>) -> OpenAPISchema {
        OpenAPISchema {
            r#type: Some(SchemaType::Single("object".to_string())),
            properties: Some(
                properties
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), Box::new(v)))
                    .collect(),
            ),
            ..Default::default()
        }
    }

    #[test]
    fn test_make_strict_adds_additional_properties_false() {
        let mut schema = object_schema(vec![("name", string_schema())]);

        schema.make_strict();

        assert!(
            matches!(
                schema.additional_properties,
                Some(AdditionalProperties::Bool(false))
            ),
            "Expected additionalProperties to be false"
        );
    }

    #[test]
    fn test_make_strict_preserves_existing_additional_properties() {
        let mut schema = object_schema(vec![("name", string_schema())]);
        schema.additional_properties = Some(AdditionalProperties::Bool(true));

        schema.make_strict();

        assert!(
            matches!(
                schema.additional_properties,
                Some(AdditionalProperties::Bool(true))
            ),
            "Expected additionalProperties to remain true (user-specified)"
        );
    }

    #[test]
    fn test_make_strict_all_properties_required() {
        let mut schema = object_schema(vec![("name", string_schema()), ("age", integer_schema())]);
        schema.required = Some(vec!["name".to_string()]); // Only name is required initially

        schema.make_strict();

        let required = schema.required.as_ref().expect("required should be set");
        assert!(
            required.contains(&"name".to_string()),
            "name should be required"
        );
        assert!(
            required.contains(&"age".to_string()),
            "age should be required"
        );
        assert_eq!(required.len(), 2, "Should have exactly 2 required fields");
    }

    #[test]
    fn test_make_strict_non_required_becomes_nullable() {
        let mut schema = object_schema(vec![("name", string_schema()), ("age", integer_schema())]);
        schema.required = Some(vec!["name".to_string()]); // Only name is required

        schema.make_strict();

        // age field should now be nullable (type becomes ["integer", "null"])
        let age_prop = schema
            .properties
            .as_ref()
            .unwrap()
            .get("age")
            .expect("age property should exist");

        match &age_prop.r#type {
            Some(SchemaType::Multiple(types)) => {
                assert!(
                    types.contains(&"integer".to_string()),
                    "Should contain integer"
                );
                assert!(types.contains(&"null".to_string()), "Should contain null");
            }
            _ => panic!("Expected age to have multiple types including null"),
        }

        // name field should NOT be nullable (it was already required)
        let name_prop = schema
            .properties
            .as_ref()
            .unwrap()
            .get("name")
            .expect("name property should exist");

        match &name_prop.r#type {
            Some(SchemaType::Single(t)) => {
                assert_eq!(t, "string", "name should remain a simple string type");
            }
            _ => panic!("Expected name to remain a single type"),
        }
    }

    #[test]
    fn test_make_strict_recursive_nested_objects() {
        let nested = object_schema(vec![("field", string_schema())]);
        let mut schema = object_schema(vec![("nested", nested)]);

        schema.make_strict();

        // Nested object should also have additionalProperties: false
        let nested_prop = schema
            .properties
            .as_ref()
            .unwrap()
            .get("nested")
            .expect("nested property should exist");

        assert!(
            matches!(
                nested_prop.additional_properties,
                Some(AdditionalProperties::Bool(false))
            ),
            "Nested object should have additionalProperties: false"
        );

        // Nested object should have all properties required
        let nested_required = nested_prop
            .required
            .as_ref()
            .expect("nested required should be set");
        assert!(nested_required.contains(&"field".to_string()));
    }

    #[test]
    fn test_make_strict_array_items() {
        let item_schema = object_schema(vec![("id", string_schema())]);
        let mut schema = OpenAPISchema {
            r#type: Some(SchemaType::Single("array".to_string())),
            items: Some(Box::new(item_schema)),
            ..Default::default()
        };

        schema.make_strict();

        let items = schema.items.as_ref().expect("items should exist");
        assert!(
            matches!(
                items.additional_properties,
                Some(AdditionalProperties::Bool(false))
            ),
            "Array items should have additionalProperties: false"
        );
    }

    #[test]
    fn test_make_strict_one_of() {
        let variant1 = object_schema(vec![("type", string_schema())]);
        let variant2 = object_schema(vec![("value", integer_schema())]);

        let mut schema = OpenAPISchema {
            one_of: Some(vec![Box::new(variant1), Box::new(variant2)]),
            ..Default::default()
        };

        schema.make_strict();

        for (i, variant) in schema.one_of.as_ref().unwrap().iter().enumerate() {
            assert!(
                matches!(
                    variant.additional_properties,
                    Some(AdditionalProperties::Bool(false))
                ),
                "oneOf variant {} should have additionalProperties: false",
                i
            );
        }
    }

    #[test]
    fn test_make_strict_any_of() {
        let variant1 = object_schema(vec![("a", string_schema())]);
        let variant2 = object_schema(vec![("b", integer_schema())]);

        let mut schema = OpenAPISchema {
            any_of: Some(vec![Box::new(variant1), Box::new(variant2)]),
            ..Default::default()
        };

        schema.make_strict();

        for (i, variant) in schema.any_of.as_ref().unwrap().iter().enumerate() {
            assert!(
                matches!(
                    variant.additional_properties,
                    Some(AdditionalProperties::Bool(false))
                ),
                "anyOf variant {} should have additionalProperties: false",
                i
            );
        }
    }

    #[test]
    fn test_make_strict_defs() {
        let def_schema = object_schema(vec![("prop", string_schema())]);
        let mut defs = HashMap::new();
        defs.insert("MyType".to_string(), Box::new(def_schema));

        let mut schema = OpenAPISchema { defs: Some(defs), ..Default::default() };

        schema.make_strict();

        let my_type = schema
            .defs
            .as_ref()
            .unwrap()
            .get("MyType")
            .expect("MyType def should exist");

        assert!(
            matches!(
                my_type.additional_properties,
                Some(AdditionalProperties::Bool(false))
            ),
            "$defs schema should have additionalProperties: false"
        );
    }

    #[test]
    fn test_make_strict_definitions() {
        let def_schema = object_schema(vec![("prop", string_schema())]);
        let mut definitions = HashMap::new();
        definitions.insert("MyType".to_string(), Box::new(def_schema));

        let mut schema = OpenAPISchema { definitions: Some(definitions), ..Default::default() };

        schema.make_strict();

        let my_type = schema
            .definitions
            .as_ref()
            .unwrap()
            .get("MyType")
            .expect("MyType definition should exist");

        assert!(
            matches!(
                my_type.additional_properties,
                Some(AdditionalProperties::Bool(false))
            ),
            "definitions schema should have additionalProperties: false"
        );
    }

    #[test]
    fn test_make_nullable_single_type() {
        let mut schema = string_schema();

        schema.make_nullable();

        match &schema.r#type {
            Some(SchemaType::Multiple(types)) => {
                assert!(types.contains(&"string".to_string()));
                assert!(types.contains(&"null".to_string()));
                assert_eq!(types.len(), 2);
            }
            _ => panic!("Expected multiple types after make_nullable"),
        }
    }

    #[test]
    fn test_make_nullable_already_null() {
        let mut schema = OpenAPISchema {
            r#type: Some(SchemaType::Single("null".to_string())),
            ..Default::default()
        };

        schema.make_nullable();

        // Should remain just "null"
        match &schema.r#type {
            Some(SchemaType::Single(t)) => {
                assert_eq!(t, "null");
            }
            _ => panic!("Expected single null type"),
        }
    }

    #[test]
    fn test_make_nullable_multiple_types() {
        let mut schema = OpenAPISchema {
            r#type: Some(SchemaType::Multiple(vec![
                "string".to_string(),
                "integer".to_string(),
            ])),
            ..Default::default()
        };

        schema.make_nullable();

        match &schema.r#type {
            Some(SchemaType::Multiple(types)) => {
                assert!(types.contains(&"string".to_string()));
                assert!(types.contains(&"integer".to_string()));
                assert!(types.contains(&"null".to_string()));
                assert_eq!(types.len(), 3);
            }
            _ => panic!("Expected multiple types after make_nullable"),
        }
    }

    #[test]
    fn test_make_nullable_already_has_null() {
        let mut schema = OpenAPISchema {
            r#type: Some(SchemaType::Multiple(vec![
                "string".to_string(),
                "null".to_string(),
            ])),
            ..Default::default()
        };

        schema.make_nullable();

        match &schema.r#type {
            Some(SchemaType::Multiple(types)) => {
                // Should not duplicate null
                assert_eq!(types.iter().filter(|t| *t == "null").count(), 1);
                assert_eq!(types.len(), 2);
            }
            _ => panic!("Expected multiple types"),
        }
    }

    // ========== allOf flattening tests ==========

    #[test]
    fn test_make_strict_flattens_all_of_properties() {
        let schema1 = object_schema(vec![("name", string_schema())]);
        let schema2 = object_schema(vec![("age", integer_schema())]);

        let mut schema = OpenAPISchema {
            all_of: Some(vec![Box::new(schema1), Box::new(schema2)]),
            ..Default::default()
        };

        schema.make_strict();

        // allOf should be removed
        assert!(
            schema.all_of.is_none(),
            "allOf should be removed after flattening"
        );

        // Properties should be merged
        let props = schema.properties.as_ref().expect("properties should exist");
        assert!(props.contains_key("name"), "Should have 'name' property");
        assert!(props.contains_key("age"), "Should have 'age' property");

        // Type should be set to object
        match &schema.r#type {
            Some(SchemaType::Single(t)) => assert_eq!(t, "object"),
            _ => panic!("Expected single 'object' type"),
        }

        // Should have additionalProperties: false (from make_strict)
        assert!(
            matches!(
                schema.additional_properties,
                Some(AdditionalProperties::Bool(false))
            ),
            "Should have additionalProperties: false"
        );
    }

    #[test]
    fn test_make_strict_merges_all_of_required() {
        let mut schema1 = object_schema(vec![("name", string_schema())]);
        schema1.required = Some(vec!["name".to_string()]);

        let mut schema2 = object_schema(vec![("age", integer_schema())]);
        schema2.required = Some(vec!["age".to_string()]);

        let mut schema = OpenAPISchema {
            all_of: Some(vec![Box::new(schema1), Box::new(schema2)]),
            ..Default::default()
        };

        schema.make_strict();

        // Both name and age should be in required (from merge + make_strict makes all required)
        let required = schema.required.as_ref().expect("required should be set");
        assert!(
            required.contains(&"name".to_string()),
            "name should be required"
        );
        assert!(
            required.contains(&"age".to_string()),
            "age should be required"
        );
    }

    #[test]
    fn test_make_strict_nested_all_of() {
        // Create a nested allOf structure:
        // allOf: [
        //   { allOf: [{ properties: { a } }, { properties: { b } }] },
        //   { properties: { c } }
        // ]
        let inner_schema1 = object_schema(vec![("a", string_schema())]);
        let inner_schema2 = object_schema(vec![("b", string_schema())]);
        let inner_all_of = OpenAPISchema {
            all_of: Some(vec![Box::new(inner_schema1), Box::new(inner_schema2)]),
            ..Default::default()
        };

        let outer_schema = object_schema(vec![("c", string_schema())]);

        let mut schema = OpenAPISchema {
            all_of: Some(vec![Box::new(inner_all_of), Box::new(outer_schema)]),
            ..Default::default()
        };

        schema.make_strict();

        // All three properties should be present after recursive flattening
        let props = schema.properties.as_ref().expect("properties should exist");
        assert!(props.contains_key("a"), "Should have 'a' property");
        assert!(props.contains_key("b"), "Should have 'b' property");
        assert!(props.contains_key("c"), "Should have 'c' property");
    }

    #[test]
    fn test_make_strict_all_of_with_base_properties() {
        // Schema has both its own properties AND allOf
        let all_of_schema = object_schema(vec![("extra", string_schema())]);

        let mut schema = object_schema(vec![("base", string_schema())]);
        schema.all_of = Some(vec![Box::new(all_of_schema)]);

        schema.make_strict();

        // Both base and extra properties should exist
        let props = schema.properties.as_ref().expect("properties should exist");
        assert!(props.contains_key("base"), "Should have 'base' property");
        assert!(props.contains_key("extra"), "Should have 'extra' property");
    }

    #[test]
    fn test_make_strict_all_of_merges_constraints() {
        let schema1 = OpenAPISchema {
            r#type: Some(SchemaType::Single("integer".to_string())),
            minimum: Some(0.0),
            ..Default::default()
        };

        let schema2 = OpenAPISchema {
            r#type: Some(SchemaType::Single("integer".to_string())),
            minimum: Some(5.0), // More restrictive
            maximum: Some(100.0),
            ..Default::default()
        };

        let mut schema = OpenAPISchema {
            all_of: Some(vec![Box::new(schema1), Box::new(schema2)]),
            ..Default::default()
        };

        schema.flatten_all_of();

        // Should take the more restrictive minimum (5.0)
        assert_eq!(
            schema.minimum,
            Some(5.0),
            "Should have more restrictive minimum"
        );
        assert_eq!(
            schema.maximum,
            Some(100.0),
            "Should have maximum from schema2"
        );
    }

    #[test]
    fn test_flatten_all_of_preserves_defs() {
        let def_schema = object_schema(vec![("field", string_schema())]);
        let mut defs = HashMap::new();
        defs.insert("MyType".to_string(), Box::new(def_schema));

        let schema_with_defs = OpenAPISchema { defs: Some(defs), ..Default::default() };

        let mut schema =
            OpenAPISchema { all_of: Some(vec![Box::new(schema_with_defs)]), ..Default::default() };

        schema.flatten_all_of();

        // $defs should be merged
        let defs = schema.defs.as_ref().expect("defs should exist");
        assert!(defs.contains_key("MyType"), "Should have 'MyType' def");
    }

    // ========== Google AI sanitization tests ==========

    #[test]
    fn test_sanitize_for_google_removes_schema_url() {
        let mut schema = OpenAPISchema {
            r#type: Some(SchemaType::Single("object".to_string())),
            schema_url: Some("http://json-schema.org/draft-07/schema#".to_string()),
            ..Default::default()
        };

        schema.sanitize_for_google();

        assert!(schema.schema_url.is_none(), "$schema should be removed");
        // Type should be preserved
        assert!(matches!(&schema.r#type, Some(SchemaType::Single(t)) if t == "object"));
    }

    #[test]
    fn test_sanitize_for_google_recursive_properties() {
        let nested = OpenAPISchema {
            r#type: Some(SchemaType::Single("string".to_string())),
            schema_url: Some("http://json-schema.org/draft-07/schema#".to_string()),
            ..Default::default()
        };

        let mut schema = OpenAPISchema {
            r#type: Some(SchemaType::Single("object".to_string())),
            schema_url: Some("http://json-schema.org/draft-07/schema#".to_string()),
            properties: Some(
                vec![("field".to_string(), Box::new(nested))]
                    .into_iter()
                    .collect(),
            ),
            ..Default::default()
        };

        schema.sanitize_for_google();

        assert!(
            schema.schema_url.is_none(),
            "Root $schema should be removed"
        );
        let field = schema.properties.as_ref().unwrap().get("field").unwrap();
        assert!(
            field.schema_url.is_none(),
            "Nested $schema should be removed"
        );
    }

    #[test]
    fn test_sanitize_for_google_recursive_items() {
        let item_schema = OpenAPISchema {
            r#type: Some(SchemaType::Single("string".to_string())),
            schema_url: Some("http://json-schema.org/draft-07/schema#".to_string()),
            ..Default::default()
        };

        let mut schema = OpenAPISchema {
            r#type: Some(SchemaType::Single("array".to_string())),
            items: Some(Box::new(item_schema)),
            ..Default::default()
        };

        schema.sanitize_for_google();

        let items = schema.items.as_ref().unwrap();
        assert!(
            items.schema_url.is_none(),
            "Array items $schema should be removed"
        );
    }

    #[test]
    fn test_sanitize_for_google_recursive_one_of() {
        let variant = OpenAPISchema {
            r#type: Some(SchemaType::Single("string".to_string())),
            schema_url: Some("http://json-schema.org/draft-07/schema#".to_string()),
            ..Default::default()
        };

        let mut schema =
            OpenAPISchema { one_of: Some(vec![Box::new(variant)]), ..Default::default() };

        schema.sanitize_for_google();

        let variant = &schema.one_of.as_ref().unwrap()[0];
        assert!(
            variant.schema_url.is_none(),
            "oneOf variant $schema should be removed"
        );
    }

    #[test]
    fn test_sanitize_for_google_recursive_defs() {
        let def_schema = OpenAPISchema {
            r#type: Some(SchemaType::Single("object".to_string())),
            schema_url: Some("http://json-schema.org/draft-07/schema#".to_string()),
            ..Default::default()
        };
        let mut defs = HashMap::new();
        defs.insert("MyType".to_string(), Box::new(def_schema));

        let mut schema = OpenAPISchema {
            defs: Some(defs),
            schema_url: Some("http://json-schema.org/draft-07/schema#".to_string()),
            ..Default::default()
        };

        schema.sanitize_for_google();

        assert!(
            schema.schema_url.is_none(),
            "Root $schema should be removed"
        );
        let my_type = schema.defs.as_ref().unwrap().get("MyType").unwrap();
        assert!(
            my_type.schema_url.is_none(),
            "$defs schema $schema should be removed"
        );
    }

    #[test]
    fn test_sanitize_for_google_preserves_other_fields() {
        let mut schema = OpenAPISchema {
            r#type: Some(SchemaType::Single("object".to_string())),
            schema_url: Some("http://json-schema.org/draft-07/schema#".to_string()),
            title: Some("Test Schema".to_string()),
            description: Some("A test schema".to_string()),
            minimum: Some(1.0),
            maximum: Some(100.0),
            properties: Some(
                vec![("name".to_string(), Box::new(string_schema()))]
                    .into_iter()
                    .collect(),
            ),
            required: Some(vec!["name".to_string()]),
            ..Default::default()
        };

        schema.sanitize_for_google();

        // $schema should be removed
        assert!(schema.schema_url.is_none());
        // Other fields should be preserved
        assert_eq!(schema.title, Some("Test Schema".to_string()));
        assert_eq!(schema.description, Some("A test schema".to_string()));
        assert!(schema.properties.is_some());
        assert!(schema.required.is_some());
        assert!(matches!(&schema.r#type, Some(SchemaType::Single(t)) if t == "object"));
        assert_eq!(schema.minimum, Some(1.0));
        assert_eq!(schema.maximum, Some(100.0));
    }

    #[test]
    fn test_sanitize_for_google_removes_default() {
        let mut schema = OpenAPISchema {
            r#type: Some(SchemaType::Single("string".to_string())),
            default: Some(serde_json::Value::String("hello".to_string())),
            ..Default::default()
        };

        schema.sanitize_for_google();

        assert!(schema.default.is_none(), "default should be removed");
        assert!(matches!(&schema.r#type, Some(SchemaType::Single(t)) if t == "string"));
    }

    #[test]
    fn test_sanitize_for_google_removes_exclusive_minimum() {
        let mut schema = OpenAPISchema {
            r#type: Some(SchemaType::Single("number".to_string())),
            exclusive_minimum: Some(0.0),
            ..Default::default()
        };

        schema.sanitize_for_google();

        assert!(
            schema.exclusive_minimum.is_none(),
            "exclusiveMinimum should be removed"
        );
    }

    #[test]
    fn test_sanitize_for_google_removes_exclusive_maximum() {
        let mut schema = OpenAPISchema {
            r#type: Some(SchemaType::Single("number".to_string())),
            exclusive_maximum: Some(100.0),
            ..Default::default()
        };

        schema.sanitize_for_google();

        assert!(
            schema.exclusive_maximum.is_none(),
            "exclusiveMaximum should be removed"
        );
    }

    #[test]
    fn test_sanitize_for_google_removes_const() {
        let mut schema = OpenAPISchema {
            r#type: Some(SchemaType::Single("string".to_string())),
            r#const: Some(serde_json::Value::String("fixed".to_string())),
            ..Default::default()
        };

        schema.sanitize_for_google();

        assert!(schema.r#const.is_none(), "const should be removed");
    }

    #[test]
    fn test_sanitize_for_google_removes_multiple_of() {
        let mut schema = OpenAPISchema {
            r#type: Some(SchemaType::Single("integer".to_string())),
            multiple_of: Some(5.0),
            ..Default::default()
        };

        schema.sanitize_for_google();

        assert!(
            schema.multiple_of.is_none(),
            "multipleOf should be removed"
        );
    }

    #[test]
    fn test_sanitize_for_google_removes_unsupported_fields_recursively() {
        let nested = OpenAPISchema {
            r#type: Some(SchemaType::Single("number".to_string())),
            default: Some(serde_json::json!(42)),
            exclusive_minimum: Some(0.0),
            exclusive_maximum: Some(100.0),
            multiple_of: Some(2.0),
            r#const: Some(serde_json::json!(10)),
            ..Default::default()
        };

        let mut schema = OpenAPISchema {
            r#type: Some(SchemaType::Single("object".to_string())),
            schema_url: Some("http://json-schema.org/draft-07/schema#".to_string()),
            default: Some(serde_json::json!({})),
            properties: Some(
                vec![("value".to_string(), Box::new(nested))]
                    .into_iter()
                    .collect(),
            ),
            ..Default::default()
        };

        schema.sanitize_for_google();

        assert!(schema.schema_url.is_none());
        assert!(schema.default.is_none());

        let value_prop = schema.properties.as_ref().unwrap().get("value").unwrap();
        assert!(value_prop.default.is_none(), "nested default should be removed");
        assert!(
            value_prop.exclusive_minimum.is_none(),
            "nested exclusiveMinimum should be removed"
        );
        assert!(
            value_prop.exclusive_maximum.is_none(),
            "nested exclusiveMaximum should be removed"
        );
        assert!(
            value_prop.multiple_of.is_none(),
            "nested multipleOf should be removed"
        );
        assert!(value_prop.r#const.is_none(), "nested const should be removed");

        assert!(schema.properties.is_some());
        assert!(matches!(&schema.r#type, Some(SchemaType::Single(t)) if t == "object"));
    }
}
