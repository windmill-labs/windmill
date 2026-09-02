//! Shared AWS Bedrock SDK code for AI chat proxy and worker.
//!
//! This module provides:
//! - BedrockClient: SDK wrapper with bearer token and IAM credentials auth
//! - OIDC role assumption: exchange a Windmill OIDC token for temporary IAM keys
//! - Message/tool conversion: OpenAI format <-> Bedrock Converse API format
//! - Stream event parsing: Extract text/tool deltas from Bedrock stream events
//!
//! Used by both windmill-api (chat proxy) and windmill-worker (AI agent).

use aws_config::BehaviorVersion;
use aws_credential_types::provider::token::ProvideToken;
use aws_credential_types::provider::ProvideCredentials;
use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, ConverseStreamOutput, DocumentBlock, DocumentFormat,
    DocumentSource, ImageBlock, ImageFormat, ImageSource, InferenceConfiguration, Message,
    SystemContentBlock, Tool, ToolInputSchema, ToolSpecification,
};
use aws_sdk_bedrockruntime::Client as BedrockRuntimeClient;
use serde::{Deserialize, Serialize};

use windmill_common::error::Error;

use crate::ai_types::{
    ContentPart, OpenAIContent, OpenAIFunction, OpenAIMessage, OpenAIToolCall, ToolDef,
};

// ============================================================================
// Cached AWS SDK Config
// ============================================================================

/// Cached AWS SDK config loaded from environment
/// Avoids repeated I/O for environment variable lookups and file reads
static AWS_SDK_CONFIG: tokio::sync::OnceCell<aws_config::SdkConfig> =
    tokio::sync::OnceCell::const_new();

/// Get or initialize the cached AWS SDK config
async fn get_aws_sdk_config() -> &'static aws_config::SdkConfig {
    AWS_SDK_CONFIG
        .get_or_init(|| async { aws_config::load_defaults(BehaviorVersion::latest()).await })
        .await
}

// ============================================================================
// Bedrock Client
// ============================================================================

/// Result of checking AWS Bedrock credentials availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockCredentialsCheck {
    pub available: bool,
    pub access_key_id_prefix: Option<String>,
    pub region: Option<String>,
    pub error: Option<String>,
}

/// Check if AWS credentials are available from the environment
pub async fn check_env_credentials() -> BedrockCredentialsCheck {
    let config = get_aws_sdk_config().await;

    if let Some(creds_provider) = config.credentials_provider() {
        match creds_provider.provide_credentials().await {
            Ok(creds) => {
                let access_key_id = creds.access_key_id();
                let prefix = if access_key_id.len() >= 8 {
                    format!("{}...", &access_key_id[..8])
                } else {
                    access_key_id.to_string()
                };

                BedrockCredentialsCheck {
                    available: true,
                    access_key_id_prefix: Some(prefix),
                    region: config.region().map(|r| r.to_string()),
                    error: None,
                }
            }
            Err(e) => BedrockCredentialsCheck {
                available: false,
                access_key_id_prefix: None,
                region: None,
                error: Some(format!("Failed to retrieve credentials: {}", e)),
            },
        }
    } else {
        BedrockCredentialsCheck {
            available: false,
            access_key_id_prefix: None,
            region: None,
            error: Some("No credentials provider configured".to_string()),
        }
    }
}

/// Constants for commonly used strings to avoid allocations
pub const FUNCTION_TYPE: &str = "function";

// AWS documents Bedrock prompt-caching support as a model allowlist rather than a
// capability exposed by the model metadata APIs:
// https://docs.aws.amazon.com/bedrock/latest/userguide/prompt-caching.html
const BEDROCK_PROMPT_CACHING_SUPPORTED_MODEL_IDS: &[&str] = &[
    "anthropic.claude-opus-4-5-20251101-v1:0",
    "anthropic.claude-opus-4-1-20250805-v1:0",
    "anthropic.claude-opus-4-20250514-v1:0",
    "anthropic.claude-sonnet-4-5-20250929-v1:0",
    "anthropic.claude-haiku-4-5-20251001-v1:0",
    "anthropic.claude-sonnet-4-20250514-v1:0",
    "anthropic.claude-3-7-sonnet-20250219-v1:0",
    "anthropic.claude-3-5-haiku-20241022-v1:0",
    "anthropic.claude-3-5-sonnet-20241022-v2:0",
];

/// Claude 4.6 and later are published under several id spellings for the same
/// model (`anthropic.claude-sonnet-4-6`, `...-4-6-v1`, `...-4-6-v1:0`), so they
/// are matched by family prefix rather than by exact id.
const BEDROCK_PROMPT_CACHING_SUPPORTED_MODEL_PREFIXES: &[&str] = &[
    "anthropic.claude-fable-5",
    "anthropic.claude-opus-4-6",
    "anthropic.claude-opus-4-7",
    "anthropic.claude-opus-4-8",
    "anthropic.claude-opus-5",
    "anthropic.claude-sonnet-4-6",
    "anthropic.claude-sonnet-5",
];

fn build_default_cache_point() -> aws_sdk_bedrockruntime::types::CachePointBlock {
    aws_sdk_bedrockruntime::types::CachePointBlock::builder()
        .r#type(aws_sdk_bedrockruntime::types::CachePointType::Default)
        .build()
        .expect("cache point type is required")
}

fn normalize_bedrock_model_id(model: &str) -> String {
    let model = model
        .rsplit('/')
        .next()
        .unwrap_or(model)
        .to_ascii_lowercase();

    for prefix in ["global.", "us.", "eu.", "apac.", "au."] {
        if let Some(normalized_model) = model.strip_prefix(prefix) {
            return normalized_model.to_string();
        }
    }

    model
}

pub fn bedrock_model_supports_prompt_caching(model: &str) -> bool {
    let normalized_model = normalize_bedrock_model_id(model);
    BEDROCK_PROMPT_CACHING_SUPPORTED_MODEL_IDS.contains(&normalized_model.as_str())
        || BEDROCK_PROMPT_CACHING_SUPPORTED_MODEL_PREFIXES
            .iter()
            .any(|prefix| normalized_model.starts_with(prefix))
}

fn append_cache_point_to_system_prompts(system_prompts: &mut Vec<SystemContentBlock>) {
    if system_prompts.is_empty()
        || matches!(
            system_prompts.last(),
            Some(SystemContentBlock::CachePoint(_))
        )
    {
        return;
    }

    system_prompts.push(SystemContentBlock::CachePoint(build_default_cache_point()));
}

fn append_cache_point_to_last_message(messages: &mut [Message]) -> Result<(), Error> {
    let Some(last_message) = messages.last_mut() else {
        return Ok(());
    };

    if matches!(
        last_message.content().last(),
        Some(ContentBlock::CachePoint(_))
    ) {
        return Ok(());
    }

    let mut content = last_message.content().to_vec();
    content.push(ContentBlock::CachePoint(build_default_cache_point()));

    *last_message = Message::builder()
        .role(last_message.role().clone())
        .set_content(Some(content))
        .build()
        .map_err(|e| Error::internal_err(format!("Failed to append cache point: {}", e)))?;

    Ok(())
}

#[derive(Debug, Clone)]
pub struct BearerTokenProvider {
    token: String,
}

impl BearerTokenProvider {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

impl ProvideToken for BearerTokenProvider {
    fn provide_token<'a>(&'a self) -> aws_credential_types::provider::future::ProvideToken<'a>
    where
        Self: 'a,
    {
        aws_credential_types::provider::future::ProvideToken::ready(Ok(
            aws_credential_types::Token::new(self.token.clone(), None),
        ))
    }
}

pub struct BedrockClient {
    client: BedrockRuntimeClient,
}

impl BedrockClient {
    pub async fn from_bearer_token(bearer_token: String, region: &str) -> Result<Self, Error> {
        let config = aws_sdk_bedrockruntime::config::Builder::new()
            .region(aws_config::Region::new(region.to_string()))
            .behavior_version(BehaviorVersion::latest())
            .token_provider(BearerTokenProvider::new(bearer_token))
            .build();

        Ok(Self { client: BedrockRuntimeClient::from_conf(config) })
    }

    pub async fn from_credentials(
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
        region: &str,
    ) -> Result<Self, Error> {
        let credentials = aws_credential_types::Credentials::new(
            access_key_id,
            secret_access_key,
            session_token,
            None, // expiration
            "windmill",
        );

        let config = aws_sdk_bedrockruntime::config::Builder::new()
            .region(aws_config::Region::new(region.to_string()))
            .behavior_version(BehaviorVersion::latest())
            .credentials_provider(credentials)
            .build();

        Ok(Self { client: BedrockRuntimeClient::from_conf(config) })
    }

    pub async fn from_env(region: &str) -> Result<Self, Error> {
        let config = get_aws_sdk_config().await;

        // Verify that credentials are actually available
        if let Some(creds_provider) = config.credentials_provider() {
            match creds_provider.provide_credentials().await {
                Ok(creds) => {
                    tracing::debug!(
                        "Bedrock: using env credentials, access_key={}...",
                        &creds.access_key_id().get(..8).unwrap_or("N/A"),
                    );
                }
                Err(e) => {
                    return Err(Error::internal_err(format!(
                        "AWS credentials not available from environment: {}",
                        e
                    )));
                }
            }
        } else {
            return Err(Error::internal_err(
                "No AWS credentials provider configured in environment".to_string(),
            ));
        }

        // Build client, only override region if explicitly provided
        let mut builder = aws_sdk_bedrockruntime::config::Builder::from(config);
        if !region.is_empty() {
            builder = builder.region(aws_config::Region::new(region.to_string()));
        }
        let bedrock_config = builder.build();

        let client = aws_sdk_bedrockruntime::Client::from_conf(bedrock_config);
        Ok(Self { client })
    }

    pub fn client(&self) -> &BedrockRuntimeClient {
        &self.client
    }
}

// ============================================================================
// OIDC Role Assumption
// ============================================================================

pub use windmill_common::auth::aws::AWS_OIDC_AUDIENCE;

/// Bedrock credential precedence: a bearer API key wins, then a complete pair of
/// explicit IAM keys, then OIDC role assumption, then the ambient environment.
/// Returns the role to assume only when both higher-priority modes are unset, so
/// a resource can carry a role ARN without it ever overriding keys set on it.
pub fn bedrock_oidc_role_to_assume<'a>(
    api_key: Option<&str>,
    aws_access_key_id: Option<&str>,
    aws_secret_access_key: Option<&str>,
    oidc_role_arn: Option<&'a str>,
) -> Option<&'a str> {
    let has_bearer_token = api_key.is_some_and(|key| !key.is_empty());
    let has_iam_keys = aws_access_key_id.is_some_and(|key| !key.is_empty())
        && aws_secret_access_key.is_some_and(|key| !key.is_empty());

    if has_bearer_token || has_iam_keys {
        return None;
    }

    oidc_role_arn.filter(|arn| !arn.is_empty())
}

/// Re-assume the role this long before AWS expires the credentials, so a request
/// that starts on a reused set cannot finish holding dead ones.
pub const ASSUMED_ROLE_REFRESH_MARGIN: std::time::Duration = std::time::Duration::from_secs(120);

/// Temporary IAM credentials minted by STS `AssumeRoleWithWebIdentity`.
#[derive(Clone, Debug)]
pub struct AssumedRoleCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
    pub expires_at: std::time::SystemTime,
}

impl AssumedRoleCredentials {
    /// Whether these have enough life left to sign another request.
    pub fn is_fresh(&self) -> bool {
        self.expires_at > std::time::SystemTime::now() + ASSUMED_ROLE_REFRESH_MARGIN
    }
}

/// Assume the resource's OIDC role for a job, reusing `cached` while it is still
/// fresh, and hand back the temporary keys to sign Bedrock requests with.
///
/// The agent loop calls this once per iteration, so without the reuse a
/// tool-heavy run would mint an OIDC token and call STS on every model turn.
/// Returns `None` when a higher-priority credential is set, so the caller can
/// call it unconditionally.
pub async fn refresh_bedrock_oidc_credentials<'a>(
    credentials: &crate::credentials::ProviderCredentials,
    cached: &'a mut Option<AssumedRoleCredentials>,
    client: &windmill_common::client::AuthedClient,
    job_id: &uuid::Uuid,
) -> Result<Option<&'a AssumedRoleCredentials>, Error> {
    let Some(role_arn) = bedrock_oidc_role_to_assume(
        credentials.api_key.as_deref(),
        credentials.aws_access_key_id.as_deref(),
        credentials.aws_secret_access_key.as_deref(),
        credentials.oidc_role_arn.as_deref(),
    ) else {
        return Ok(None);
    };

    if !cached
        .as_ref()
        .is_some_and(AssumedRoleCredentials::is_fresh)
    {
        let region = bedrock_oidc_region(credentials.region.as_deref())?;
        // A worker holds no OIDC signing key, so it asks the API to mint the
        // token for this job; the session name carries the job into CloudTrail.
        let id_token = client
            .get_id_token(AWS_OIDC_AUDIENCE)
            .await
            .map_err(|e| Error::internal_err(format!("Failed to get OIDC token: {}", e)))?;
        // The whole UUID: "windmill-ai-job-" plus 32 hex is 48 characters, well
        // inside AWS's limit, and a truncated one would make two runs
        // indistinguishable in CloudTrail.
        let session_name = aws_role_session_name("windmill-ai-job", &job_id.simple().to_string());
        *cached =
            Some(assume_role_with_oidc_token(role_arn, region, id_token, &session_name).await?);
    }

    Ok(cached.as_ref())
}

/// Distinguishing digest appended to a session name that had to be truncated.
const SESSION_NAME_DIGEST_LEN: usize = 8;

/// Build a role session name AWS accepts: it constrains them to
/// `[\w+=,.@-]{2,64}`, and rejects the whole request otherwise. Disallowed
/// characters are replaced rather than dropped.
///
/// The name is the only thing carrying the caller into the assumed-role ARN and
/// CloudTrail, so an identity too long to fit keeps a prefix plus a digest of the
/// whole thing: two identities sharing a prefix would otherwise be
/// indistinguishable in exactly the audit trail this feature exists to produce.
pub fn aws_role_session_name(prefix: &str, identity: &str) -> String {
    let sanitize = |s: &str| -> String {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | '=' | ',' | '.' | '@' | '-')
                {
                    c
                } else {
                    '-'
                }
            })
            .collect()
    };

    let name = sanitize(&format!("{}-{}", prefix, identity));
    if name.len() <= 64 {
        return name;
    }

    let digest = &windmill_common::utils::calculate_hash(identity)[..SESSION_NAME_DIGEST_LEN];
    let kept = 64 - SESSION_NAME_DIGEST_LEN - 1;
    format!("{}-{}", &name[..kept], digest)
}

/// The region an OIDC role assumption runs in.
///
/// The assumption resolves to explicit IAM keys, and the SDK client built from
/// those has no ambient-region fallback the way [`BedrockClient::from_env`] does,
/// so a resource that assumes a role has to name its region. Callers check this
/// before minting the OIDC token, so a misconfigured resource never issues an
/// identity token it is only going to throw away.
pub fn bedrock_oidc_region(region: Option<&str>) -> Result<&str, Error> {
    region.filter(|r| !r.is_empty()).ok_or_else(|| {
        Error::BadRequest(
            "AWS Bedrock resources that assume a role through OIDC must set a region".to_string(),
        )
    })
}

/// Exchange a Windmill OIDC token for temporary IAM credentials on `role_arn`.
pub async fn assume_role_with_oidc_token(
    role_arn: &str,
    region: &str,
    id_token: String,
    session_name: &str,
) -> Result<AssumedRoleCredentials, Error> {
    use windmill_common::auth::aws::{
        get_assume_role_with_web_identity_fluent_builder, GetAuthenticationOutput, OidcAuth,
    };

    let oidc_auth = OidcAuth { region: Some(region.to_string()), role_arn: role_arn.to_string() };

    let output =
        get_assume_role_with_web_identity_fluent_builder(&oidc_auth, id_token, Some(session_name))
            .await?
            .send()
            .await
            .map_err(|e| {
                Error::internal_err(format!(
                    "Failed to assume AWS role {} with Windmill's OIDC token: {}",
                    role_arn,
                    format_bedrock_error(&e)
                ))
            })?;

    let credentials = output.get_credentials()?;

    Ok(AssumedRoleCredentials {
        access_key_id: credentials.access_key_id.clone(),
        secret_access_key: credentials.secret_access_key.clone(),
        session_token: credentials.session_token.clone(),
        expires_at: std::time::UNIX_EPOCH
            + std::time::Duration::from_secs(credentials.expiration.secs().max(0) as u64),
    })
}

// ============================================================================
// Error Formatting
// ============================================================================

/// Format AWS SDK errors with detailed information
pub fn format_bedrock_error<E, R>(error: &aws_sdk_bedrockruntime::error::SdkError<E, R>) -> String
where
    E: std::fmt::Debug + std::fmt::Display,
    R: std::fmt::Debug,
{
    use aws_sdk_bedrockruntime::error::SdkError;

    match error {
        SdkError::ServiceError(err) => {
            format!("Service error: {} (details: {:?})", err.err(), err)
        }
        SdkError::ConstructionFailure(err) => {
            format!("Request construction failed: {:?}", err)
        }
        SdkError::DispatchFailure(err) => {
            format!("Request dispatch failed: {:?}", err)
        }
        SdkError::ResponseError(err) => {
            format!("Response error: {:?}", err)
        }
        SdkError::TimeoutError(err) => {
            format!("Request timeout: {:?}", err)
        }
        _ => format!("{:?}", error),
    }
}

// ============================================================================
// Type Conversion Utilities
// ============================================================================

/// Convert serde_json::Value to AWS Smithy Document
pub fn json_to_document(value: serde_json::Value) -> aws_smithy_types::Document {
    use aws_smithy_types::Document;
    use serde_json::Value;

    match value {
        Value::Object(map) => {
            let mut doc_map = std::collections::HashMap::new();
            for (k, v) in map {
                doc_map.insert(k, json_to_document(v));
            }
            Document::Object(doc_map)
        }
        Value::Array(arr) => Document::Array(arr.into_iter().map(json_to_document).collect()),
        Value::Number(num) => {
            if let Some(u) = num.as_u64() {
                Document::Number(aws_smithy_types::Number::PosInt(u))
            } else if let Some(i) = num.as_i64() {
                Document::Number(aws_smithy_types::Number::NegInt(i))
            } else if let Some(f) = num.as_f64() {
                Document::Number(aws_smithy_types::Number::Float(f))
            } else {
                Document::Number(aws_smithy_types::Number::PosInt(0))
            }
        }
        Value::String(s) => Document::String(s),
        Value::Bool(b) => Document::Bool(b),
        Value::Null => Document::Null,
    }
}

// ============================================================================
// Message Conversion (OpenAI -> Bedrock)
// ============================================================================

/// Convert OpenAI-style messages to Bedrock format
///
/// Separates system messages from conversation messages as required by Bedrock API.
///
/// Important: Bedrock requires messages to alternate between user and assistant roles.
/// When an assistant message has tool_use blocks, the next user message must contain
/// ALL corresponding tool_result blocks. This function groups consecutive tool messages
/// into a single user message.
///
/// # Returns
/// Tuple of (conversation_messages, system_prompts)
pub fn openai_messages_to_bedrock(
    messages: &[OpenAIMessage],
    enable_prompt_caching: bool,
) -> Result<(Vec<Message>, Vec<SystemContentBlock>), Error> {
    let mut bedrock_messages = Vec::new();
    let mut system_prompts = Vec::new();
    let mut pending_tool_results: Vec<ContentBlock> = Vec::new();

    for msg in messages {
        match msg.role.as_str() {
            "system" => {
                // Extract system messages separately
                if let Some(content) = &msg.content {
                    let text = content_to_text(content);
                    if !text.is_empty() {
                        system_prompts.push(SystemContentBlock::Text(text));
                    }
                }
            }
            "user" | "assistant" => {
                // Before adding a user/assistant message, flush any pending tool results
                if !pending_tool_results.is_empty() {
                    let tool_result_message = Message::builder()
                        .role(ConversationRole::User)
                        .set_content(Some(pending_tool_results.drain(..).collect()))
                        .build()
                        .map_err(|e| {
                            Error::internal_err(format!(
                                "Failed to build tool results message: {}",
                                e
                            ))
                        })?;
                    bedrock_messages.push(tool_result_message);
                }
                bedrock_messages.push(convert_message(msg)?);
            }
            "tool" => {
                // Accumulate tool results - they will be flushed as a single message
                // when we encounter a non-tool message or at the end
                let tool_result = convert_tool_result_content(msg)?;
                pending_tool_results.push(tool_result);
            }
            _ => {
                return Err(Error::BadRequest(format!("Unsupported role: {}", msg.role)));
            }
        }
    }

    // Flush any remaining tool results at the end
    if !pending_tool_results.is_empty() {
        let tool_result_message = Message::builder()
            .role(ConversationRole::User)
            .set_content(Some(pending_tool_results))
            .build()
            .map_err(|e| {
                Error::internal_err(format!("Failed to build tool results message: {}", e))
            })?;
        bedrock_messages.push(tool_result_message);
    }

    if enable_prompt_caching {
        append_cache_point_to_system_prompts(&mut system_prompts);
        append_cache_point_to_last_message(&mut bedrock_messages)?;
    }

    Ok((bedrock_messages, system_prompts))
}

/// Helper to extract text from OpenAIContent (ignoring images)
///
/// This is public so it can be reused by the worker module.
pub fn content_to_text(content: &OpenAIContent) -> String {
    match content {
        OpenAIContent::Text(text) => text.to_string(),
        OpenAIContent::Parts(parts) => {
            // Extract only text parts and join them
            let text_parts: Vec<&str> = parts
                .iter()
                .filter_map(|part| match part {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            text_parts.join(" ")
        }
    }
}

/// Parse a data URL and extract MIME type and decoded bytes.
fn parse_data_url_bytes(url: &str) -> Result<(String, Vec<u8>), Error> {
    if !url.starts_with("data:") {
        return Err(Error::internal_err("URL must be a data URL"));
    }

    let base64_start = url
        .find("base64,")
        .ok_or_else(|| Error::internal_err("Invalid data URL format"))?;

    let base64_data = &url[base64_start + 7..];
    let mime_type = url
        .split(';')
        .next()
        .and_then(|s| s.strip_prefix("data:"))
        .unwrap_or("application/octet-stream");

    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_data)
        .map_err(|e| Error::internal_err(format!("Failed to decode base64 data: {}", e)))?;

    Ok((mime_type.to_string(), bytes))
}

/// Parse an image data URL and extract ImageFormat and decoded bytes.
fn parse_image_data_url(url: &str) -> Result<(ImageFormat, Vec<u8>), Error> {
    let (mime_type, bytes) = parse_data_url_bytes(url)?;

    let format_str = mime_type
        .rsplit_once('/')
        .map(|(_, format)| format)
        .unwrap_or("png");

    let format = match format_str {
        "png" => ImageFormat::Png,
        "jpeg" | "jpg" => ImageFormat::Jpeg,
        "gif" => ImageFormat::Gif,
        "webp" => ImageFormat::Webp,
        _ => ImageFormat::Png,
    };

    Ok((format, bytes))
}

/// Map a MIME type to a Bedrock DocumentFormat.
fn mime_to_document_format(mime_type: &str) -> DocumentFormat {
    match mime_type {
        "application/pdf" => DocumentFormat::Pdf,
        "text/csv" => DocumentFormat::Csv,
        "text/html" => DocumentFormat::Html,
        "text/plain" => DocumentFormat::Txt,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
            DocumentFormat::Docx
        }
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => DocumentFormat::Xlsx,
        _ => DocumentFormat::Pdf,
    }
}

/// Convert a ContentPart to Bedrock ContentBlock
fn content_part_to_block(part: &ContentPart) -> Result<Option<ContentBlock>, Error> {
    match part {
        ContentPart::Text { text } => {
            if text.is_empty() {
                Ok(None)
            } else {
                Ok(Some(ContentBlock::Text(text.clone())))
            }
        }
        ContentPart::ImageUrl { image_url } => {
            let (format, bytes) = parse_image_data_url(&image_url.url)?;

            let image_source = ImageSource::Bytes(bytes.into());
            let image_block = ImageBlock::builder()
                .format(format)
                .source(image_source)
                .build()
                .map_err(|e| Error::internal_err(format!("Failed to build image block: {}", e)))?;

            Ok(Some(ContentBlock::Image(image_block)))
        }
        ContentPart::File { file } => {
            let (mime_type, bytes) = parse_data_url_bytes(&file.file_data)?;
            let doc_source = DocumentSource::Bytes(bytes.into());
            let doc_block = DocumentBlock::builder()
                .format(mime_to_document_format(&mime_type))
                .name(file.filename.replace('.', "_"))
                .source(doc_source)
                .build()
                .map_err(|e| {
                    Error::internal_err(format!("Failed to build document block: {}", e))
                })?;
            Ok(Some(ContentBlock::Document(doc_block)))
        }
        ContentPart::S3Object { .. } => {
            // S3Objects should be converted before calling this function
            Ok(None)
        }
    }
}

/// Convert a single OpenAI message to Bedrock Message
fn convert_message(msg: &OpenAIMessage) -> Result<Message, Error> {
    let role = match msg.role.as_str() {
        "user" => ConversationRole::User,
        "assistant" => ConversationRole::Assistant,
        _ => {
            return Err(Error::internal_err(format!(
                "Unsupported role: {}",
                msg.role
            )));
        }
    };

    let mut content_blocks = Vec::new();

    // Replay the Claude reasoning block first: when thinking is enabled,
    // Anthropic requires the reasoning block (with its unmodified signature) to
    // precede toolUse in the assistant turn it was emitted in. The proxy
    // round-trips it on the tool call's extra_content (see BedrockExtraContent).
    if role == ConversationRole::Assistant {
        if let Some(reasoning) = msg
            .tool_calls
            .as_ref()
            .and_then(|tcs| {
                tcs.iter()
                    .find_map(|tc| tc.extra_content.as_ref().and_then(|ec| ec.bedrock.as_ref()))
            })
            .map(bedrock_reasoning_block_from_extra)
            .transpose()?
            .flatten()
        {
            content_blocks.push(reasoning);
        }
    }

    // Handle content (text and/or images)
    if let Some(content) = &msg.content {
        match content {
            OpenAIContent::Text(text) => {
                if !text.is_empty() {
                    content_blocks.push(ContentBlock::Text(text.clone()));
                }
            }
            OpenAIContent::Parts(parts) => {
                for part in parts {
                    if let Some(block) = content_part_to_block(part)? {
                        content_blocks.push(block);
                    }
                }
            }
        }
    }

    // Handle tool calls (for assistant messages)
    if let Some(tool_calls) = &msg.tool_calls {
        for tc in tool_calls {
            content_blocks.push(convert_tool_call_to_content(tc)?);
        }
    }

    // Bedrock requires at least one content block
    if content_blocks.is_empty() {
        content_blocks.push(ContentBlock::Text(String::new()));
    }

    Message::builder()
        .role(role)
        .set_content(Some(content_blocks))
        .build()
        .map_err(|e| Error::internal_err(format!("Failed to build message: {}", e)))
}

/// Rebuild a Bedrock reasoning content block from the round-tripped
/// [`BedrockExtraContent`](crate::ai_types::BedrockExtraContent).
fn bedrock_reasoning_block_from_extra(
    extra: &crate::ai_types::BedrockExtraContent,
) -> Result<Option<ContentBlock>, Error> {
    if let Some(redacted) = extra.redacted_content.as_deref() {
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, redacted)
            .map_err(|e| {
                Error::internal_err(format!("Failed to decode redacted reasoning: {}", e))
            })?;
        return Ok(Some(ContentBlock::ReasoningContent(
            aws_sdk_bedrockruntime::types::ReasoningContentBlock::RedactedContent(bytes.into()),
        )));
    }

    let Some(text) = extra.reasoning_text.as_deref() else {
        return Ok(None);
    };
    let mut builder = aws_sdk_bedrockruntime::types::ReasoningTextBlock::builder().text(text);
    if let Some(signature) = extra.signature.as_deref() {
        builder = builder.signature(signature);
    }
    Ok(Some(ContentBlock::ReasoningContent(
        aws_sdk_bedrockruntime::types::ReasoningContentBlock::ReasoningText(
            builder.build().map_err(|e| {
                Error::internal_err(format!("Failed to build reasoning block: {}", e))
            })?,
        ),
    )))
}

/// Convert OpenAI tool call to Bedrock ToolUse content block
fn convert_tool_call_to_content(tool_call: &OpenAIToolCall) -> Result<ContentBlock, Error> {
    let input = json_to_document(
        serde_json::from_str(&tool_call.function.arguments)
            .unwrap_or_else(|_| serde_json::json!({})),
    );
    Ok(ContentBlock::ToolUse(
        aws_sdk_bedrockruntime::types::ToolUseBlock::builder()
            .tool_use_id(&tool_call.id)
            .name(&tool_call.function.name)
            .input(input)
            .build()
            .map_err(|e| Error::internal_err(format!("Failed to build tool use: {}", e)))?,
    ))
}

/// Convert tool result message to Bedrock ToolResult ContentBlock
///
/// Returns just the ContentBlock (not a full Message) so multiple tool results
/// can be combined into a single user message.
fn convert_tool_result_content(msg: &OpenAIMessage) -> Result<ContentBlock, Error> {
    let tool_call_id = msg
        .tool_call_id
        .as_ref()
        .ok_or_else(|| Error::internal_err("Tool message missing tool_call_id"))?;

    let content_str = msg
        .content
        .as_ref()
        .map(|c| content_to_text(c))
        .unwrap_or_default();

    // Try to parse as JSON, otherwise use text
    let tool_result_content =
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content_str) {
            if json_val.is_object() {
                vec![aws_sdk_bedrockruntime::types::ToolResultContentBlock::Json(
                    json_to_document(json_val),
                )]
            } else {
                // Wrap primitives and arrays in an object
                vec![aws_sdk_bedrockruntime::types::ToolResultContentBlock::Json(
                    json_to_document(serde_json::json!({"result": json_val})),
                )]
            }
        } else {
            vec![aws_sdk_bedrockruntime::types::ToolResultContentBlock::Text(
                content_str.to_string(),
            )]
        };

    Ok(ContentBlock::ToolResult(
        aws_sdk_bedrockruntime::types::ToolResultBlock::builder()
            .tool_use_id(tool_call_id)
            .set_content(Some(tool_result_content))
            .build()
            .map_err(|e| Error::internal_err(format!("Failed to build tool result: {}", e)))?,
    ))
}

// ============================================================================
// Tool Conversion (OpenAI -> Bedrock)
// ============================================================================

/// Convert OpenAI tool definitions to Bedrock format
pub fn openai_tools_to_bedrock(tools: &[ToolDef]) -> Result<Vec<Tool>, Error> {
    tools
        .iter()
        .map(|tool_def| {
            let spec = &tool_def.function;

            // Convert parameters (RawValue) to Document via serde_json::Value
            let param_value: serde_json::Value = serde_json::from_str(spec.parameters.get())
                .map_err(|e| Error::internal_err(format!("Invalid tool schema: {}", e)))?;
            let input_schema = ToolInputSchema::Json(json_to_document(param_value));

            let tool_spec = ToolSpecification::builder()
                .name(&spec.name)
                .set_description(spec.description.clone())
                .input_schema(input_schema)
                .build()
                .map_err(|e| Error::internal_err(format!("Failed to build tool spec: {}", e)))?;

            Ok(Tool::ToolSpec(tool_spec))
        })
        .collect()
}

// ============================================================================
// Inference Configuration
// ============================================================================

/// Create inference configuration from parameters
pub fn create_inference_config(
    temperature: Option<f32>,
    max_tokens: Option<i32>,
) -> Option<InferenceConfiguration> {
    if temperature.is_none() && max_tokens.is_none() {
        return None;
    }

    let mut builder = InferenceConfiguration::builder();

    if let Some(temp) = temperature {
        builder = builder.temperature(temp);
    }

    if let Some(max_tok) = max_tokens {
        builder = builder.max_tokens(max_tok);
    }

    Some(builder.build())
}

// ============================================================================
// Stream Event Parsing
// ============================================================================

/// Extract text delta from Bedrock stream event
pub fn bedrock_stream_event_to_text(event: &ConverseStreamOutput) -> Option<String> {
    match event {
        ConverseStreamOutput::ContentBlockDelta(delta) => delta
            .delta()
            .and_then(|d| d.as_text().ok())
            .map(|s| s.to_string()),
        _ => None,
    }
}

/// Represents a streaming tool call being accumulated
#[derive(Debug, Clone)]
pub struct StreamingToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Extract tool use start event from stream
pub fn bedrock_stream_event_to_tool_start(
    event: &ConverseStreamOutput,
) -> Option<StreamingToolCall> {
    match event {
        ConverseStreamOutput::ContentBlockStart(start) => {
            if let Some(tool_use) = start.start().and_then(|s| s.as_tool_use().ok()) {
                Some(StreamingToolCall {
                    id: tool_use.tool_use_id().to_string(),
                    name: tool_use.name().to_string(),
                    arguments: String::new(),
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn bedrock_stream_event_to_tool_start_with_block_index(
    event: &ConverseStreamOutput,
) -> Option<(usize, StreamingToolCall)> {
    match event {
        ConverseStreamOutput::ContentBlockStart(start) => {
            let block_index = usize::try_from(start.content_block_index()).ok()?;
            let tool_use = start.start().and_then(|s| s.as_tool_use().ok())?;
            Some((
                block_index,
                StreamingToolCall {
                    id: tool_use.tool_use_id().to_string(),
                    name: tool_use.name().to_string(),
                    arguments: String::new(),
                },
            ))
        }
        _ => None,
    }
}

/// Extract tool use input delta from stream
pub fn bedrock_stream_event_to_tool_delta(event: &ConverseStreamOutput) -> Option<String> {
    match event {
        ConverseStreamOutput::ContentBlockDelta(delta) => delta
            .delta()
            .and_then(|d| d.as_tool_use().ok())
            .map(|tool_use| tool_use.input().to_string()),
        _ => None,
    }
}

pub fn bedrock_stream_event_to_tool_delta_with_block_index(
    event: &ConverseStreamOutput,
) -> Option<(usize, String)> {
    match event {
        ConverseStreamOutput::ContentBlockDelta(delta) => {
            let block_index = usize::try_from(delta.content_block_index()).ok()?;
            let input = delta
                .delta()
                .and_then(|d| d.as_tool_use().ok())
                .map(|tool_use| tool_use.input().to_string())?;
            Some((block_index, input))
        }
        _ => None,
    }
}

/// Check if stream event indicates content block stop
pub fn bedrock_stream_event_is_block_stop(event: &ConverseStreamOutput) -> bool {
    matches!(event, ConverseStreamOutput::ContentBlockStop(_))
}

/// Convert accumulated streaming tool calls to OpenAI format.
///
/// When thinking is enabled, `reasoning` carries the turn's Claude reasoning
/// block; it is attached to the first tool call so the next request replays it
/// before `toolUse` (required by Claude when thinking is enabled — see
/// [`convert_message`]). When `None` (thinking off), tool calls carry no extra
/// content, byte-identical to the pre-feature output.
pub fn streaming_tool_calls_to_openai(
    tool_calls: Vec<StreamingToolCall>,
    reasoning: Option<crate::ai_types::BedrockExtraContent>,
) -> Vec<OpenAIToolCall> {
    let mut reasoning = reasoning;
    tool_calls
        .into_iter()
        .map(|tc| {
            // `take` so only the first tool call carries the reasoning block.
            let extra_content = reasoning.take().map(|block| crate::ai_types::ExtraContent {
                bedrock: Some(block),
                ..Default::default()
            });
            OpenAIToolCall {
                id: tc.id,
                function: OpenAIFunction { name: tc.name, arguments: tc.arguments },
                r#type: FUNCTION_TYPE.to_string(),
                extra_content,
            }
        })
        .collect()
}

/// Fold a Bedrock `ReasoningContent` stream delta into an accumulating reasoning
/// block (text + signature, or redacted bytes). Returns the readable text delta
/// when the event carried one, so the caller can stream it as reasoning content.
/// Mirrors the proxy path's accumulation in `providers/bedrock.rs`.
pub fn bedrock_stream_event_to_reasoning_delta(
    event: &ConverseStreamOutput,
    reasoning: &mut Option<crate::ai_types::BedrockExtraContent>,
) -> Option<String> {
    let ConverseStreamOutput::ContentBlockDelta(delta_event) = event else {
        return None;
    };
    let aws_sdk_bedrockruntime::types::ContentBlockDelta::ReasoningContent(rc) =
        delta_event.delta()?
    else {
        return None;
    };

    let entry = reasoning.get_or_insert_with(Default::default);
    match rc {
        aws_sdk_bedrockruntime::types::ReasoningContentBlockDelta::Text(text) => {
            entry
                .reasoning_text
                .get_or_insert_with(String::new)
                .push_str(text);
            Some(text.clone())
        }
        aws_sdk_bedrockruntime::types::ReasoningContentBlockDelta::Signature(signature) => {
            entry
                .signature
                .get_or_insert_with(String::new)
                .push_str(signature);
            None
        }
        aws_sdk_bedrockruntime::types::ReasoningContentBlockDelta::RedactedContent(blob) => {
            // Base64 of concatenated fragments != concatenated base64 fragments,
            // so accumulate raw bytes and re-encode.
            let mut bytes = entry
                .redacted_content
                .as_deref()
                .and_then(|existing| {
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, existing)
                        .ok()
                })
                .unwrap_or_default();
            bytes.extend_from_slice(blob.as_ref());
            entry.redacted_content = Some(base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                bytes,
            ));
            None
        }
        _ => None,
    }
}

// ============================================================================
// Tool Configuration Builder
// ============================================================================

/// Build tool configuration with optional ToolChoice for structured output
pub fn build_tool_config(
    tools: Option<&[ToolDef]>,
    force_tool_use: bool,
    enable_prompt_caching: bool,
) -> Result<Option<aws_sdk_bedrockruntime::types::ToolConfiguration>, Error> {
    if let Some(tools) = tools {
        let mut bedrock_tools = openai_tools_to_bedrock(tools)?;

        if enable_prompt_caching && !matches!(bedrock_tools.last(), Some(Tool::CachePoint(_))) {
            bedrock_tools.push(Tool::CachePoint(build_default_cache_point()));
        }

        let mut tool_config_builder = aws_sdk_bedrockruntime::types::ToolConfiguration::builder()
            .set_tools(Some(bedrock_tools));

        // For structured output, force the model to use the tool
        if force_tool_use {
            tool_config_builder =
                tool_config_builder.tool_choice(aws_sdk_bedrockruntime::types::ToolChoice::Any(
                    aws_sdk_bedrockruntime::types::AnyToolChoice::builder().build(),
                ));
        }

        Ok(Some(tool_config_builder.build().map_err(|e| {
            Error::internal_err(format!("Failed to build tool configuration: {}", e))
        })?))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_bedrockruntime::types::ReasoningContentBlock;
    use serde_json::value::RawValue;

    #[test]
    fn oidc_role_yields_only_to_bearer_and_complete_iam_keys() {
        let arn = Some("arn:aws:iam::1:role/bedrock");

        assert_eq!(
            bedrock_oidc_role_to_assume(None, None, None, arn),
            arn,
            "nothing else set: assume the role rather than fall through to the environment"
        );
        assert_eq!(
            bedrock_oidc_role_to_assume(Some("key"), None, None, arn),
            None
        );
        assert_eq!(
            bedrock_oidc_role_to_assume(None, Some("id"), Some("secret"), arn),
            None
        );
        assert_eq!(
            bedrock_oidc_role_to_assume(None, Some("id"), None, arn),
            arn,
            "half a key pair is not usable IAM credentials, so the role still wins"
        );
        // Cleared resource fields arrive as empty strings, not as absent ones.
        assert_eq!(
            bedrock_oidc_role_to_assume(Some(""), Some(""), Some(""), arn),
            arn
        );
        assert_eq!(
            bedrock_oidc_role_to_assume(None, None, None, Some("")),
            None
        );
    }

    #[test]
    fn credentials_stop_being_fresh_before_aws_expires_them() {
        let at = |d: std::time::Duration| AssumedRoleCredentials {
            access_key_id: String::new(),
            secret_access_key: String::new(),
            session_token: String::new(),
            expires_at: std::time::SystemTime::now() + d,
        };
        // Reused only while there is still margin left to finish a request.
        assert!(at(ASSUMED_ROLE_REFRESH_MARGIN * 2).is_fresh());
        assert!(!at(ASSUMED_ROLE_REFRESH_MARGIN / 2).is_fresh());
    }

    #[test]
    fn role_session_name_stays_within_what_aws_accepts() {
        // A whole job UUID fits, so CloudTrail names the exact run.
        assert_eq!(
            aws_role_session_name("windmill-ai-job", "0a1b2c3d4e5f60718293a4b5c6d7e8f9"),
            "windmill-ai-job-0a1b2c3d4e5f60718293a4b5c6d7e8f9"
        );
        // Usernames and emails routinely carry characters AWS rejects.
        assert_eq!(
            aws_role_session_name("windmill-ai-copilot", "ada/lovelace (admin)"),
            "windmill-ai-copilot-ada-lovelace--admin-"
        );

        // Identities too long to fit keep a digest, so a shared prefix does not
        // collapse two callers into one CloudTrail identity.
        let a = aws_role_session_name("windmill-ai-copilot", &format!("{}a", "x".repeat(60)));
        let b = aws_role_session_name("windmill-ai-copilot", &format!("{}b", "x".repeat(60)));
        assert_ne!(a, b);
        for name in [&a, &b] {
            assert_eq!(name.len(), 64);
            assert!(name.chars().all(|c| c.is_ascii_alphanumeric()
                || matches!(c, '_' | '+' | '=' | ',' | '.' | '@' | '-')));
        }

        // Sanitising before truncating keeps every char one byte, so the cap is
        // never applied mid-character.
        let unicode = aws_role_session_name("windmill-ai-copilot", &"é".repeat(100));
        assert_eq!(unicode.len(), 64);
    }

    fn text_message(role: &str, content: &str) -> OpenAIMessage {
        OpenAIMessage {
            role: role.to_string(),
            content: Some(OpenAIContent::Text(content.to_string())),
            ..Default::default()
        }
    }

    fn test_tool() -> ToolDef {
        ToolDef {
            r#type: FUNCTION_TYPE.to_string(),
            function: crate::ai_types::ToolDefFunction {
                name: "test_tool".to_string(),
                description: Some("A test tool".to_string()),
                parameters: RawValue::from_string(
                    r#"{"type":"object","properties":{},"additionalProperties":false}"#.to_string(),
                )
                .expect("valid raw json"),
            },
        }
    }

    #[test]
    fn json_to_document_preserves_negative_integers() {
        let value = serde_json::json!(-1);
        let doc = json_to_document(value);
        assert!(matches!(
            doc,
            aws_smithy_types::Document::Number(aws_smithy_types::Number::NegInt(-1))
        ));
    }

    #[test]
    fn json_to_document_handles_large_u64_above_i64_max() {
        let value = serde_json::json!(u64::MAX);
        let doc = json_to_document(value);
        assert!(matches!(
            doc,
            aws_smithy_types::Document::Number(aws_smithy_types::Number::PosInt(u)) if u == u64::MAX
        ));
    }

    #[test]
    fn json_to_document_handles_positive_integers() {
        let value = serde_json::json!(42);
        let doc = json_to_document(value);
        assert!(matches!(
            doc,
            aws_smithy_types::Document::Number(aws_smithy_types::Number::PosInt(42))
        ));
    }

    #[test]
    fn openai_messages_to_bedrock_adds_cache_points_when_enabled() {
        let messages = vec![
            text_message("system", "Reply concisely"),
            text_message("user", "Tell me a joke"),
        ];

        let (bedrock_messages, system_prompts) =
            openai_messages_to_bedrock(&messages, true).expect("bedrock conversion succeeds");

        assert!(matches!(
            system_prompts.last(),
            Some(SystemContentBlock::CachePoint(_))
        ));
        assert!(matches!(
            bedrock_messages
                .last()
                .and_then(|message| message.content().last()),
            Some(ContentBlock::CachePoint(_))
        ));
    }

    #[test]
    fn openai_messages_to_bedrock_replays_reasoning_before_tool_use() {
        let tool_call = OpenAIToolCall {
            id: "call_1".to_string(),
            function: OpenAIFunction { name: "lookup".to_string(), arguments: "{}".to_string() },
            r#type: FUNCTION_TYPE.to_string(),
            extra_content: Some(crate::ai_types::ExtraContent {
                bedrock: Some(crate::ai_types::BedrockExtraContent {
                    reasoning_text: Some("let me think".to_string()),
                    signature: Some("sig-abc".to_string()),
                    redacted_content: None,
                }),
                ..Default::default()
            }),
        };
        let assistant = OpenAIMessage {
            role: "assistant".to_string(),
            tool_calls: Some(vec![tool_call]),
            ..Default::default()
        };
        let messages = vec![text_message("user", "hi"), assistant];

        let (bedrock_messages, _) =
            openai_messages_to_bedrock(&messages, false).expect("bedrock conversion succeeds");

        let content = bedrock_messages
            .last()
            .expect("assistant message")
            .content();
        match &content[0] {
            ContentBlock::ReasoningContent(ReasoningContentBlock::ReasoningText(rt)) => {
                assert_eq!(rt.text(), "let me think");
                assert_eq!(rt.signature(), Some("sig-abc"));
            }
            other => panic!("expected reasoning block first, got {:?}", other),
        }
        assert!(matches!(&content[1], ContentBlock::ToolUse(_)));
    }

    #[test]
    fn streaming_tool_calls_to_openai_attaches_reasoning_to_first_call_only() {
        let calls = vec![
            StreamingToolCall {
                id: "call_1".to_string(),
                name: "a".to_string(),
                arguments: "{}".to_string(),
            },
            StreamingToolCall {
                id: "call_2".to_string(),
                name: "b".to_string(),
                arguments: "{}".to_string(),
            },
        ];
        let reasoning = crate::ai_types::BedrockExtraContent {
            reasoning_text: Some("thinking".to_string()),
            signature: Some("sig".to_string()),
            redacted_content: None,
        };

        let openai = streaming_tool_calls_to_openai(calls, Some(reasoning));

        let with_reasoning = openai
            .iter()
            .filter(|tc| {
                tc.extra_content
                    .as_ref()
                    .and_then(|e| e.bedrock.as_ref())
                    .is_some()
            })
            .count();
        assert_eq!(
            with_reasoning, 1,
            "exactly one tool call carries the reasoning block"
        );
        let block = openai[0]
            .extra_content
            .as_ref()
            .and_then(|e| e.bedrock.as_ref())
            .expect("first tool call carries reasoning");
        assert_eq!(block.reasoning_text.as_deref(), Some("thinking"));
        assert_eq!(block.signature.as_deref(), Some("sig"));
    }

    #[test]
    fn streaming_tool_calls_to_openai_without_reasoning_has_no_extra_content() {
        let calls = vec![StreamingToolCall {
            id: "call_1".to_string(),
            name: "a".to_string(),
            arguments: "{}".to_string(),
        }];

        let openai = streaming_tool_calls_to_openai(calls, None);

        assert!(openai[0].extra_content.is_none());
    }

    #[test]
    fn bedrock_reasoning_delta_accumulates_text_and_signature() {
        use aws_sdk_bedrockruntime::types::{
            ContentBlockDelta, ContentBlockDeltaEvent, ConverseStreamOutput,
            ReasoningContentBlockDelta,
        };

        let mut reasoning = None;
        let text_event = ConverseStreamOutput::ContentBlockDelta(
            ContentBlockDeltaEvent::builder()
                .content_block_index(0)
                .delta(ContentBlockDelta::ReasoningContent(
                    ReasoningContentBlockDelta::Text("let me ".to_string()),
                ))
                .build()
                .unwrap(),
        );
        assert_eq!(
            bedrock_stream_event_to_reasoning_delta(&text_event, &mut reasoning).as_deref(),
            Some("let me ")
        );

        let sig_event = ConverseStreamOutput::ContentBlockDelta(
            ContentBlockDeltaEvent::builder()
                .content_block_index(0)
                .delta(ContentBlockDelta::ReasoningContent(
                    ReasoningContentBlockDelta::Signature("sig".to_string()),
                ))
                .build()
                .unwrap(),
        );
        assert!(bedrock_stream_event_to_reasoning_delta(&sig_event, &mut reasoning).is_none());

        let block = reasoning.expect("reasoning accumulated");
        assert_eq!(block.reasoning_text.as_deref(), Some("let me "));
        assert_eq!(block.signature.as_deref(), Some("sig"));
    }

    #[test]
    fn openai_messages_to_bedrock_skips_cache_points_when_disabled() {
        let messages = vec![
            text_message("system", "Reply concisely"),
            text_message("user", "Tell me a joke"),
        ];

        let (bedrock_messages, system_prompts) =
            openai_messages_to_bedrock(&messages, false).expect("bedrock conversion succeeds");

        assert!(!matches!(
            system_prompts.last(),
            Some(SystemContentBlock::CachePoint(_))
        ));
        assert!(!matches!(
            bedrock_messages
                .last()
                .and_then(|message| message.content().last()),
            Some(ContentBlock::CachePoint(_))
        ));
    }

    #[test]
    fn build_tool_config_adds_cache_point_when_enabled() {
        let tools = vec![test_tool()];
        let tool_config =
            build_tool_config(Some(&tools), false, true).expect("tool config succeeds");

        assert!(matches!(
            tool_config
                .as_ref()
                .and_then(|config| config.tools().last()),
            Some(Tool::CachePoint(_))
        ));
    }

    #[test]
    fn bedrock_prompt_caching_supports_documented_claude_model_ids() {
        assert!(bedrock_model_supports_prompt_caching(
            "anthropic.claude-haiku-4-5-20251001-v1:0"
        ));
        assert!(bedrock_model_supports_prompt_caching(
            "global.anthropic.claude-haiku-4-5-20251001-v1:0"
        ));
        assert!(bedrock_model_supports_prompt_caching(
            "arn:aws:bedrock:us-east-1::inference-profile/us.anthropic.claude-3-7-sonnet-20250219-v1:0"
        ));
    }

    /// Claude 4.6+ ships under bare, `-v1` and `-v1:0` spellings of the same id,
    /// so every one of them has to reach the prefix match.
    #[test]
    fn bedrock_prompt_caching_supports_claude_4_6_and_later_id_spellings() {
        for model in [
            "anthropic.claude-sonnet-4-6",
            "anthropic.claude-sonnet-4-6-v1:0",
            "us.anthropic.claude-opus-4-6-v1",
            "global.anthropic.claude-opus-4-8",
            "anthropic.claude-opus-5",
            "eu.anthropic.claude-sonnet-5-v1:0",
            "au.anthropic.claude-sonnet-5",
            "anthropic.claude-fable-5",
        ] {
            assert!(
                bedrock_model_supports_prompt_caching(model),
                "{model} must support prompt caching"
            );
        }
    }

    #[test]
    fn bedrock_prompt_caching_rejects_unsupported_or_opaque_model_ids() {
        assert!(!bedrock_model_supports_prompt_caching(
            "anthropic.claude-3-haiku-20240307-v1:0"
        ));
        assert!(!bedrock_model_supports_prompt_caching(
            "arn:aws:bedrock:us-east-1:123456789012:application-inference-profile/my-profile"
        ));
        // Opus 4.5 is dated-id only — the 4.6+ prefixes must not swallow it.
        assert!(!bedrock_model_supports_prompt_caching(
            "anthropic.claude-opus-4-5-20251101-v2:0"
        ));
    }
}
