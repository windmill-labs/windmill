use std::{borrow::Cow, sync::Arc};
mod annotated;
mod capabilities;
mod content;
mod extension;
mod meta;
mod prompt;
mod resource;
mod serde_impl;
mod tool;
pub use annotated::*;
pub use capabilities::*;
pub use content::*;
pub use extension::*;
pub use meta::*;
pub use prompt::*;
pub use resource::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use tool::*;

/// You can use [`crate::object!`] or [`crate::model::object`] to create a json object quickly.
pub type JsonObject<F = Value> = serde_json::Map<String, F>;

/// unwrap the JsonObject under [`serde_json::Value`]
///
/// # Panic
/// This will panic when the value is not a object in debug mode.
pub fn object(value: serde_json::Value) -> JsonObject {
    debug_assert!(value.is_object());
    match value {
        serde_json::Value::Object(map) => map,
        _ => JsonObject::default(),
    }
}

/// Use this macro just like [`serde_json::json!`]
#[cfg(feature = "macros")]
#[macro_export]
macro_rules! object {
    ({$($tt:tt)*}) => {
        $crate::model::object(serde_json::json! {
            {$($tt)*}
        })
    };
}
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Copy, Eq)]
#[cfg_attr(feature = "server", derive(schemars::JsonSchema))]
pub struct EmptyObject {}

pub trait ConstString: Default {
    const VALUE: &str;
}
#[macro_export]
macro_rules! const_string {
    ($name:ident = $value:literal) => {
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
        pub struct $name;

        impl ConstString for $name {
            const VALUE: &str = $value;
        }

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                $value.serialize(serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<$name, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let s: String = serde::Deserialize::deserialize(deserializer)?;
                if s == $value {
                    Ok($name)
                } else {
                    Err(serde::de::Error::custom(format!(concat!(
                        "expect const string value \"",
                        $value,
                        "\""
                    ))))
                }
            }
        }
    };
}

const_string!(JsonRpcVersion2_0 = "2.0");

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd)]
pub struct ProtocolVersion(Cow<'static, str>);

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::LATEST
    }
}
impl ProtocolVersion {
    pub const V_2025_03_26: Self = Self(Cow::Borrowed("2025-03-26"));
    pub const V_2024_11_05: Self = Self(Cow::Borrowed("2024-11-05"));
    pub const LATEST: Self = Self::V_2025_03_26;
}

impl Serialize for ProtocolVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ProtocolVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        #[allow(clippy::single_match)]
        match s.as_str() {
            "2024-11-05" => return Ok(ProtocolVersion::V_2024_11_05),
            "2025-03-26" => return Ok(ProtocolVersion::V_2025_03_26),
            _ => {}
        }
        Ok(ProtocolVersion(Cow::Owned(s)))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum NumberOrString {
    Number(u32),
    String(Arc<str>),
}

impl NumberOrString {
    pub fn into_json_value(self) -> Value {
        match self {
            NumberOrString::Number(n) => Value::Number(serde_json::Number::from(n)),
            NumberOrString::String(s) => Value::String(s.to_string()),
        }
    }
}

impl std::fmt::Display for NumberOrString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumberOrString::Number(n) => n.fmt(f),
            NumberOrString::String(s) => s.fmt(f),
        }
    }
}

impl Serialize for NumberOrString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            NumberOrString::Number(n) => n.serialize(serializer),
            NumberOrString::String(s) => s.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for NumberOrString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;
        match value {
            Value::Number(n) => Ok(NumberOrString::Number(
                n.as_u64()
                    .ok_or(serde::de::Error::custom("Expect an integer"))? as u32,
            )),
            Value::String(s) => Ok(NumberOrString::String(s.into())),
            _ => Err(serde::de::Error::custom("Expect number or string")),
        }
    }
}

pub type RequestId = NumberOrString;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(transparent)]
pub struct ProgressToken(pub NumberOrString);
#[derive(Debug, Clone)]
pub struct Request<M = String, P = JsonObject> {
    pub method: M,
    // #[serde(skip_serializing_if = "Option::is_none")]
    pub params: P,
    /// extensions will carry anything possible in the context, including [`Meta`]
    ///
    /// this is similar with the Extensions in `http` crate
    pub extensions: Extensions,
}

impl<M, P> GetExtensions for Request<M, P> {
    fn extensions(&self) -> &Extensions {
        &self.extensions
    }
    fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }
}

#[derive(Debug, Clone)]
pub struct RequestOptionalParam<M = String, P = JsonObject> {
    pub method: M,
    // #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<P>,
    /// extensions will carry anything possible in the context, including [`Meta`]
    ///
    /// this is similar with the Extensions in `http` crate
    pub extensions: Extensions,
}

#[derive(Debug, Clone)]
pub struct RequestNoParam<M = String> {
    pub method: M,
    /// extensions will carry anything possible in the context, including [`Meta`]
    ///
    /// this is similar with the Extensions in `http` crate
    pub extensions: Extensions,
}

impl<M> GetExtensions for RequestNoParam<M> {
    fn extensions(&self) -> &Extensions {
        &self.extensions
    }
    fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }
}
#[derive(Debug, Clone)]
pub struct Notification<M = String, P = JsonObject> {
    pub method: M,
    pub params: P,
    /// extensions will carry anything possible in the context, including [`Meta`]
    ///
    /// this is similar with the Extensions in `http` crate
    pub extensions: Extensions,
}

#[derive(Debug, Clone)]
pub struct NotificationNoParam<M = String> {
    pub method: M,
    /// extensions will carry anything possible in the context, including [`Meta`]
    ///
    /// this is similar with the Extensions in `http` crate
    pub extensions: Extensions,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct JsonRpcRequest<R = Request> {
    pub jsonrpc: JsonRpcVersion2_0,
    pub id: RequestId,
    #[serde(flatten)]
    pub request: R,
}
type DefaultResponse = JsonObject;
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct JsonRpcResponse<R = JsonObject> {
    pub jsonrpc: JsonRpcVersion2_0,
    pub id: RequestId,
    pub result: R,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct JsonRpcError {
    pub jsonrpc: JsonRpcVersion2_0,
    pub id: RequestId,
    pub error: ErrorData,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct JsonRpcNotification<N = Notification> {
    pub jsonrpc: JsonRpcVersion2_0,
    #[serde(flatten)]
    pub notification: N,
}

// Standard JSON-RPC error codes
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct ErrorCode(pub i32);

impl ErrorCode {
    pub const RESOURCE_NOT_FOUND: Self = Self(-32002);
    pub const INVALID_REQUEST: Self = Self(-32600);
    pub const METHOD_NOT_FOUND: Self = Self(-32601);
    pub const INVALID_PARAMS: Self = Self(-32602);
    pub const INTERNAL_ERROR: Self = Self(-32603);
    pub const PARSE_ERROR: Self = Self(-32700);
}

/// Error information for JSON-RPC error responses.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ErrorData {
    /// The error type that occurred.
    pub code: ErrorCode,

    /// A short description of the error. The message SHOULD be limited to a concise single sentence.
    pub message: Cow<'static, str>,

    /// Additional information about the error. The value of this member is defined by the
    /// sender (e.g. detailed error information, nested errors etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ErrorData {
    pub fn new(
        code: ErrorCode,
        message: impl Into<Cow<'static, str>>,
        data: Option<Value>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            data,
        }
    }
    pub fn resource_not_found(message: impl Into<Cow<'static, str>>, data: Option<Value>) -> Self {
        Self::new(ErrorCode::RESOURCE_NOT_FOUND, message, data)
    }
    pub fn parse_error(message: impl Into<Cow<'static, str>>, data: Option<Value>) -> Self {
        Self::new(ErrorCode::PARSE_ERROR, message, data)
    }
    pub fn invalid_request(message: impl Into<Cow<'static, str>>, data: Option<Value>) -> Self {
        Self::new(ErrorCode::INVALID_REQUEST, message, data)
    }
    pub fn method_not_found<M: ConstString>() -> Self {
        Self::new(ErrorCode::METHOD_NOT_FOUND, M::VALUE, None)
    }
    pub fn invalid_params(message: impl Into<Cow<'static, str>>, data: Option<Value>) -> Self {
        Self::new(ErrorCode::INVALID_PARAMS, message, data)
    }
    pub fn internal_error(message: impl Into<Cow<'static, str>>, data: Option<Value>) -> Self {
        Self::new(ErrorCode::INTERNAL_ERROR, message, data)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum JsonRpcBatchRequestItem<Req, Not> {
    Request(JsonRpcRequest<Req>),
    Notification(JsonRpcNotification<Not>),
}

impl<Req, Not> JsonRpcBatchRequestItem<Req, Not> {
    pub fn into_non_batch_message<Resp>(self) -> JsonRpcMessage<Req, Resp, Not> {
        match self {
            JsonRpcBatchRequestItem::Request(r) => JsonRpcMessage::Request(r),
            JsonRpcBatchRequestItem::Notification(n) => JsonRpcMessage::Notification(n),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum JsonRpcBatchResponseItem<Resp> {
    Response(JsonRpcResponse<Resp>),
    Error(JsonRpcError),
}

impl<Resp> JsonRpcBatchResponseItem<Resp> {
    pub fn into_non_batch_message<Req, Not>(self) -> JsonRpcMessage<Req, Resp, Not> {
        match self {
            JsonRpcBatchResponseItem::Response(r) => JsonRpcMessage::Response(r),
            JsonRpcBatchResponseItem::Error(e) => JsonRpcMessage::Error(e),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum JsonRpcMessage<Req = Request, Resp = DefaultResponse, Noti = Notification> {
    Request(JsonRpcRequest<Req>),
    Response(JsonRpcResponse<Resp>),
    Notification(JsonRpcNotification<Noti>),
    BatchRequest(Vec<JsonRpcBatchRequestItem<Req, Noti>>),
    BatchResponse(Vec<JsonRpcBatchResponseItem<Resp>>),
    Error(JsonRpcError),
}

impl<Req, Resp, Not> JsonRpcMessage<Req, Resp, Not> {
    #[inline]
    pub const fn request(request: Req, id: RequestId) -> Self {
        JsonRpcMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion2_0,
            id,
            request,
        })
    }
    #[inline]
    pub const fn response(response: Resp, id: RequestId) -> Self {
        JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: JsonRpcVersion2_0,
            id,
            result: response,
        })
    }
    #[inline]
    pub const fn error(error: ErrorData, id: RequestId) -> Self {
        JsonRpcMessage::Error(JsonRpcError {
            jsonrpc: JsonRpcVersion2_0,
            id,
            error,
        })
    }
    #[inline]
    pub const fn notification(notification: Not) -> Self {
        JsonRpcMessage::Notification(JsonRpcNotification {
            jsonrpc: JsonRpcVersion2_0,
            notification,
        })
    }
    pub fn into_request(self) -> Option<(Req, RequestId)> {
        match self {
            JsonRpcMessage::Request(r) => Some((r.request, r.id)),
            _ => None,
        }
    }
    pub fn into_response(self) -> Option<(Resp, RequestId)> {
        match self {
            JsonRpcMessage::Response(r) => Some((r.result, r.id)),
            _ => None,
        }
    }
    pub fn into_notification(self) -> Option<Not> {
        match self {
            JsonRpcMessage::Notification(n) => Some(n.notification),
            _ => None,
        }
    }
    pub fn into_error(self) -> Option<(ErrorData, RequestId)> {
        match self {
            JsonRpcMessage::Error(e) => Some((e.error, e.id)),
            _ => None,
        }
    }
    pub fn into_result(self) -> Option<(Result<Resp, ErrorData>, RequestId)> {
        match self {
            JsonRpcMessage::Response(r) => Some((Ok(r.result), r.id)),
            JsonRpcMessage::Error(e) => Some((Err(e.error), e.id)),

            _ => None,
        }
    }
}

/// # Empty result
/// A response that indicates success but carries no data.
pub type EmptyResult = EmptyObject;

impl From<()> for EmptyResult {
    fn from(_value: ()) -> Self {
        EmptyResult {}
    }
}

impl From<EmptyResult> for () {
    fn from(_value: EmptyResult) {}
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CancelledNotificationParam {
    pub request_id: RequestId,
    pub reason: Option<String>,
}

const_string!(CancelledNotificationMethod = "notifications/cancelled");

/// # Cancellation
/// This notification can be sent by either side to indicate that it is cancelling a previously-issued request.
///
/// The request SHOULD still be in-flight, but due to communication latency, it is always possible that this notification MAY arrive after the request has already finished.
///
/// This notification indicates that the result will be unused, so any associated processing SHOULD cease.
///
/// A client MUST NOT attempt to cancel its `initialize` request.
pub type CancelledNotification =
    Notification<CancelledNotificationMethod, CancelledNotificationParam>;

const_string!(InitializeResultMethod = "initialize");
/// # Initialization
/// This request is sent from the client to the server when it first connects, asking it to begin initialization.
pub type InitializeRequest = Request<InitializeResultMethod, InitializeRequestParam>;

const_string!(InitializedNotificationMethod = "notifications/initialized");
/// This notification is sent from the client to the server after initialization has finished.
pub type InitializedNotification = NotificationNoParam<InitializedNotificationMethod>;
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequestParam {
    pub protocol_version: ProtocolVersion,
    pub capabilities: ClientCapabilities,
    pub client_info: Implementation,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: ProtocolVersion,
    pub capabilities: ServerCapabilities,
    pub server_info: Implementation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

pub type ServerInfo = InitializeResult;
pub type ClientInfo = InitializeRequestParam;

impl Default for ServerInfo {
    fn default() -> Self {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation::from_build_env(),
            instructions: None,
        }
    }
}

impl Default for ClientInfo {
    fn default() -> Self {
        ClientInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation::from_build_env(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Implementation {
    pub name: String,
    pub version: String,
}

impl Default for Implementation {
    fn default() -> Self {
        Self::from_build_env()
    }
}

impl Implementation {
    pub fn from_build_env() -> Self {
        Implementation {
            name: env!("CARGO_CRATE_NAME").to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedRequestParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}
const_string!(PingRequestMethod = "ping");
pub type PingRequest = RequestNoParam<PingRequestMethod>;

const_string!(ProgressNotificationMethod = "notifications/progress");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProgressNotificationParam {
    pub progress_token: ProgressToken,
    /// The progress thus far. This should increase every time progress is made, even if the total is unknown.
    pub progress: u32,
    /// Total number of items to process (or total progress required), if known
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
    /// An optional message describing the current progress.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub type ProgressNotification = Notification<ProgressNotificationMethod, ProgressNotificationParam>;

pub type Cursor = String;

macro_rules! paginated_result {
    ($t:ident {
        $i_item: ident: $t_item: ty
    }) => {
        #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
        #[serde(rename_all = "camelCase")]
        pub struct $t {
            #[serde(skip_serializing_if = "Option::is_none")]
            pub next_cursor: Option<Cursor>,
            pub $i_item: $t_item,
        }
    };
}

const_string!(ListResourcesRequestMethod = "resources/list");
pub type ListResourcesRequest =
    RequestOptionalParam<ListResourcesRequestMethod, PaginatedRequestParam>;
paginated_result!(ListResourcesResult {
    resources: Vec<Resource>
});

const_string!(ListResourceTemplatesRequestMethod = "resources/templates/list");
pub type ListResourceTemplatesRequest =
    RequestOptionalParam<ListResourceTemplatesRequestMethod, PaginatedRequestParam>;
paginated_result!(ListResourceTemplatesResult {
    resource_templates: Vec<ResourceTemplate>
});

const_string!(ReadResourceRequestMethod = "resources/read");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceRequestParam {
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ReadResourceResult {
    pub contents: Vec<ResourceContents>,
}

pub type ReadResourceRequest = Request<ReadResourceRequestMethod, ReadResourceRequestParam>;

const_string!(ResourceListChangedNotificationMethod = "notifications/resources/list_changed");
pub type ResourceListChangedNotification =
    NotificationNoParam<ResourceListChangedNotificationMethod>;

const_string!(SubscribeRequestMethod = "resources/subscribe");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeRequestParam {
    pub uri: String,
}
pub type SubscribeRequest = Request<SubscribeRequestMethod, SubscribeRequestParam>;

const_string!(UnsubscribeRequestMethod = "resources/unsubscribe");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribeRequestParam {
    pub uri: String,
}
pub type UnsubscribeRequest = Request<UnsubscribeRequestMethod, UnsubscribeRequestParam>;

const_string!(ResourceUpdatedNotificationMethod = "notifications/resources/updated");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUpdatedNotificationParam {
    pub uri: String,
}
pub type ResourceUpdatedNotification =
    Notification<ResourceUpdatedNotificationMethod, ResourceUpdatedNotificationParam>;

const_string!(ListPromptsRequestMethod = "prompts/list");
pub type ListPromptsRequest = RequestOptionalParam<ListPromptsRequestMethod, PaginatedRequestParam>;
paginated_result!(ListPromptsResult {
    prompts: Vec<Prompt>
});

const_string!(GetPromptRequestMethod = "prompts/get");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptRequestParam {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<JsonObject>,
}
pub type GetPromptRequest = Request<GetPromptRequestMethod, GetPromptRequestParam>;

const_string!(PromptListChangedNotificationMethod = "notifications/prompts/list_changed");
pub type PromptListChangedNotification = NotificationNoParam<PromptListChangedNotificationMethod>;

const_string!(ToolListChangedNotificationMethod = "notifications/tools/list_changed");
pub type ToolListChangedNotification = NotificationNoParam<ToolListChangedNotificationMethod>;
// 日志相关
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Copy)]
#[serde(rename_all = "lowercase")] //match spec
pub enum LoggingLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

const_string!(SetLevelRequestMethod = "logging/setLevel");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetLevelRequestParam {
    pub level: LoggingLevel,
}
pub type SetLevelRequest = Request<SetLevelRequestMethod, SetLevelRequestParam>;

const_string!(LoggingMessageNotificationMethod = "notifications/message");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoggingMessageNotificationParam {
    pub level: LoggingLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
    pub data: Value,
}
pub type LoggingMessageNotification =
    Notification<LoggingMessageNotificationMethod, LoggingMessageNotificationParam>;

const_string!(CreateMessageRequestMethod = "sampling/createMessage");
pub type CreateMessageRequest = Request<CreateMessageRequestMethod, CreateMessageRequestParam>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SamplingMessage {
    pub role: Role,
    pub content: Content,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ContextInclusion {
    #[serde(rename = "allServers")]
    AllServers,
    #[serde(rename = "none")]
    None,
    #[serde(rename = "thisServer")]
    ThisServer,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageRequestParam {
    pub messages: Vec<SamplingMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_context: Option<ContextInclusion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelPreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<ModelHint>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_priority: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_priority: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_priority: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ModelHint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompleteRequestParam {
    pub r#ref: Reference,
    pub argument: ArgumentInfo,
}

pub type CompleteRequest = Request<CompleteRequestMethod, CompleteRequestParam>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompletionInfo {
    pub values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompleteResult {
    pub completion: CompletionInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum Reference {
    #[serde(rename = "ref/resource")]
    Resource(ResourceReference),
    #[serde(rename = "ref/prompt")]
    Prompt(PromptReference),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ResourceReference {
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PromptReference {
    pub name: String,
}

const_string!(CompleteRequestMethod = "completion/complete");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArgumentInfo {
    pub name: String,
    pub value: String,
}

// 根目录相关
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Root {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

const_string!(ListRootsRequestMethod = "roots/list");
pub type ListRootsRequest = RequestNoParam<ListRootsRequestMethod>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListRootsResult {
    pub roots: Vec<Root>,
}

const_string!(RootsListChangedNotificationMethod = "notifications/roots/list_changed");
pub type RootsListChangedNotification = NotificationNoParam<RootsListChangedNotificationMethod>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    pub content: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl CallToolResult {
    pub fn success(content: Vec<Content>) -> Self {
        CallToolResult {
            content,
            is_error: Some(false),
        }
    }
    pub fn error(content: Vec<Content>) -> Self {
        CallToolResult {
            content,
            is_error: Some(true),
        }
    }
}

const_string!(ListToolsRequestMethod = "tools/list");
pub type ListToolsRequest = RequestOptionalParam<ListToolsRequestMethod, PaginatedRequestParam>;
paginated_result!(
    ListToolsResult {
        tools: Vec<Tool>
    }
);

const_string!(CallToolRequestMethod = "tools/call");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CallToolRequestParam {
    pub name: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<JsonObject>,
}

pub type CallToolRequest = Request<CallToolRequestMethod, CallToolRequestParam>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageResult {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(flatten)]
    pub message: SamplingMessage,
}

impl CreateMessageResult {
    pub const STOP_REASON_END_TURN: &str = "endTurn";
    pub const STOP_REASON_END_SEQUENCE: &str = "stopSequence";
    pub const STOP_REASON_END_MAX_TOKEN: &str = "maxTokens";
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub messages: Vec<PromptMessage>,
}

macro_rules! ts_union {
    (
        export type $U: ident =
            $(|)?$($V: ident)|*;
    ) => {
        #[derive(Debug, Serialize, Deserialize, Clone)]
        #[serde(untagged)]
        pub enum $U {
            $($V($V),)*
        }
    };
}

ts_union!(
    export type ClientRequest =
    | PingRequest
    | InitializeRequest
    | CompleteRequest
    | SetLevelRequest
    | GetPromptRequest
    | ListPromptsRequest
    | ListResourcesRequest
    | ListResourceTemplatesRequest
    | ReadResourceRequest
    | SubscribeRequest
    | UnsubscribeRequest
    | CallToolRequest
    | ListToolsRequest;
);

ts_union!(
    export type ClientNotification =
    | CancelledNotification
    | ProgressNotification
    | InitializedNotification
    | RootsListChangedNotification;
);

ts_union!(
    export type ClientResult = CreateMessageResult | ListRootsResult | EmptyResult;
);

impl ClientResult {
    pub fn empty(_: ()) -> ClientResult {
        ClientResult::EmptyResult(EmptyResult {})
    }
}

pub type ClientJsonRpcMessage = JsonRpcMessage<ClientRequest, ClientResult, ClientNotification>;

ts_union!(
    export type ServerRequest =
    | PingRequest
    | CreateMessageRequest
    | ListRootsRequest;
);

ts_union!(
    export type ServerNotification =
    | CancelledNotification
    | ProgressNotification
    | LoggingMessageNotification
    | ResourceUpdatedNotification
    | ResourceListChangedNotification
    | ToolListChangedNotification
    | PromptListChangedNotification;
);

ts_union!(
    export type ServerResult =
    | InitializeResult
    | CompleteResult
    | GetPromptResult
    | ListPromptsResult
    | ListResourcesResult
    | ListResourceTemplatesResult
    | ReadResourceResult
    | CallToolResult
    | ListToolsResult
    | EmptyResult
    ;
);

impl ServerResult {
    pub fn empty(_: ()) -> ServerResult {
        ServerResult::EmptyResult(EmptyResult {})
    }
}

pub type ServerJsonRpcMessage = JsonRpcMessage<ServerRequest, ServerResult, ServerNotification>;

impl TryInto<CancelledNotification> for ServerNotification {
    type Error = ServerNotification;
    fn try_into(self) -> Result<CancelledNotification, Self::Error> {
        if let ServerNotification::CancelledNotification(t) = self {
            Ok(t)
        } else {
            Err(self)
        }
    }
}

impl TryInto<CancelledNotification> for ClientNotification {
    type Error = ClientNotification;
    fn try_into(self) -> Result<CancelledNotification, Self::Error> {
        if let ClientNotification::CancelledNotification(t) = self {
            Ok(t)
        } else {
            Err(self)
        }
    }
}
impl From<CancelledNotification> for ServerNotification {
    fn from(value: CancelledNotification) -> Self {
        ServerNotification::CancelledNotification(value)
    }
}

impl From<CancelledNotification> for ClientNotification {
    fn from(value: CancelledNotification) -> Self {
        ClientNotification::CancelledNotification(value)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_notification_serde() {
        let raw = json!( {
            "jsonrpc": JsonRpcVersion2_0,
            "method": InitializedNotificationMethod,
        });
        let message: ClientJsonRpcMessage =
            serde_json::from_value(raw.clone()).expect("invalid notification");
        match &message {
            ClientJsonRpcMessage::Notification(JsonRpcNotification {
                notification: ClientNotification::InitializedNotification(_n),
                ..
            }) => {}
            _ => panic!("Expected Notification"),
        }
        let json = serde_json::to_value(message).expect("valid json");
        assert_eq!(json, raw);
    }

    #[test]
    fn test_request_conversion() {
        let raw = json!( {
            "jsonrpc": JsonRpcVersion2_0,
            "id": 1,
            "method": "request",
            "params": {"key": "value"},
        });
        let message: JsonRpcMessage = serde_json::from_value(raw.clone()).expect("invalid request");

        match &message {
            JsonRpcMessage::Request(r) => {
                assert_eq!(r.id, RequestId::Number(1));
                assert_eq!(r.request.method, "request");
                assert_eq!(
                    &r.request.params,
                    json!({"key": "value"})
                        .as_object()
                        .expect("should be an object")
                );
            }
            _ => panic!("Expected Request"),
        }
        let json = serde_json::to_value(&message).expect("valid json");
        assert_eq!(json, raw);
    }

    #[test]
    fn test_initial_request_response_serde() {
        let request = json!({
          "jsonrpc": "2.0",
          "id": 1,
          "method": "initialize",
          "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
              "roots": {
                "listChanged": true
              },
              "sampling": {}
            },
            "clientInfo": {
              "name": "ExampleClient",
              "version": "1.0.0"
            }
          }
        });
        let raw_response_json = json!({
          "jsonrpc": "2.0",
          "id": 1,
          "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
              "logging": {},
              "prompts": {
                "listChanged": true
              },
              "resources": {
                "subscribe": true,
                "listChanged": true
              },
              "tools": {
                "listChanged": true
              }
            },
            "serverInfo": {
              "name": "ExampleServer",
              "version": "1.0.0"
            }
          }
        });
        let request: ClientJsonRpcMessage =
            serde_json::from_value(request.clone()).expect("invalid request");
        let (request, id) = request.into_request().expect("should be a request");
        assert_eq!(id, RequestId::Number(1));
        match request {
            ClientRequest::InitializeRequest(Request {
                method: _,
                params:
                    InitializeRequestParam {
                        protocol_version: _,
                        capabilities,
                        client_info,
                    },
                ..
            }) => {
                assert_eq!(capabilities.roots.unwrap().list_changed, Some(true));
                assert_eq!(capabilities.sampling.unwrap().len(), 0);
                assert_eq!(client_info.name, "ExampleClient");
                assert_eq!(client_info.version, "1.0.0");
            }
            _ => panic!("Expected InitializeRequest"),
        }
        let server_response: ServerJsonRpcMessage =
            serde_json::from_value(raw_response_json.clone()).expect("invalid response");
        let (response, id) = server_response
            .clone()
            .into_response()
            .expect("expect response");
        assert_eq!(id, RequestId::Number(1));
        match response {
            ServerResult::InitializeResult(InitializeResult {
                protocol_version: _,
                capabilities,
                server_info,
                instructions,
            }) => {
                assert_eq!(capabilities.logging.unwrap().len(), 0);
                assert_eq!(capabilities.prompts.unwrap().list_changed, Some(true));
                assert_eq!(
                    capabilities.resources.as_ref().unwrap().subscribe,
                    Some(true)
                );
                assert_eq!(capabilities.resources.unwrap().list_changed, Some(true));
                assert_eq!(capabilities.tools.unwrap().list_changed, Some(true));
                assert_eq!(server_info.name, "ExampleServer");
                assert_eq!(server_info.version, "1.0.0");
                assert_eq!(instructions, None);
            }
            other => panic!("Expected InitializeResult, got {other:?}"),
        }

        let server_response_json: Value = serde_json::to_value(&server_response).expect("msg");

        assert_eq!(server_response_json, raw_response_json);
    }

    #[test]
    fn test_protocol_version_order() {
        let v1 = ProtocolVersion::V_2024_11_05;
        let v2 = ProtocolVersion::V_2025_03_26;
        assert!(v1 < v2);
    }
}
