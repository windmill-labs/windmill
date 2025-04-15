use base64::engine::{Engine, general_purpose::STANDARD as BASE64_STANDARD};
use serde::{Deserialize, Serialize};

use super::{
    AnnotateAble, Annotations, RawEmbeddedResource, RawImageContent,
    content::{EmbeddedResource, ImageContent},
    resource::ResourceContents,
};

/// A prompt that can be used to generate text from a model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    /// The name of the prompt
    pub name: String,
    /// Optional description of what the prompt does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional arguments that can be passed to customize the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

impl Prompt {
    /// Create a new prompt with the given name, description and arguments
    pub fn new<N, D>(
        name: N,
        description: Option<D>,
        arguments: Option<Vec<PromptArgument>>,
    ) -> Self
    where
        N: Into<String>,
        D: Into<String>,
    {
        Prompt {
            name: name.into(),
            description: description.map(Into::into),
            arguments,
        }
    }
}

/// Represents a prompt argument that can be passed to customize the prompt
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptArgument {
    /// The name of the argument
    pub name: String,
    /// A description of what the argument is used for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this argument is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// Represents the role of a message sender in a prompt conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PromptMessageRole {
    User,
    Assistant,
}

/// Content types that can be included in prompt messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PromptMessageContent {
    /// Plain text content
    Text { text: String },
    /// Image content with base64-encoded data
    Image {
        #[serde(flatten)]
        image: ImageContent,
    },
    /// Embedded server-side resource
    Resource { resource: EmbeddedResource },
}

impl PromptMessageContent {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }
}

/// A message in a prompt conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptMessage {
    /// The role of the message sender
    pub role: PromptMessageRole,
    /// The content of the message
    pub content: PromptMessageContent,
}

impl PromptMessage {
    /// Create a new text message with the given role and text content
    pub fn new_text<S: Into<String>>(role: PromptMessageRole, text: S) -> Self {
        Self {
            role,
            content: PromptMessageContent::Text { text: text.into() },
        }
    }
    #[cfg(feature = "base64")]
    pub fn new_image(
        role: PromptMessageRole,
        data: &[u8],
        mime_type: &str,
        annotations: Option<Annotations>,
    ) -> Self {
        let mime_type = mime_type.into();

        let base64 = BASE64_STANDARD.encode(data);

        Self {
            role,
            content: PromptMessageContent::Image {
                image: RawImageContent {
                    data: base64,
                    mime_type,
                }
                .optional_annotate(annotations),
            },
        }
    }

    /// Create a new resource message
    pub fn new_resource(
        role: PromptMessageRole,
        uri: String,
        mime_type: String,
        text: Option<String>,
        annotations: Option<Annotations>,
    ) -> Self {
        let resource_contents = ResourceContents::TextResourceContents {
            uri,
            mime_type: Some(mime_type),
            text: text.unwrap_or_default(),
        };

        Self {
            role,
            content: PromptMessageContent::Resource {
                resource: RawEmbeddedResource {
                    resource: resource_contents,
                }
                .optional_annotate(annotations),
            },
        }
    }
}

/// A template for a prompt
#[derive(Debug, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub template: String,
    pub arguments: Vec<PromptArgumentTemplate>,
}

/// A template for a prompt argument, this should be identical to PromptArgument
#[derive(Debug, Serialize, Deserialize)]
pub struct PromptArgumentTemplate {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}
