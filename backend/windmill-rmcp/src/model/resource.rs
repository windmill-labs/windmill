use serde::{Deserialize, Serialize};

use super::Annotated;

/// Represents a resource in the extension with metadata
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RawResource {
    /// URI representing the resource location (e.g., "file:///path/to/file" or "str:///content")
    pub uri: String,
    /// Name of the resource
    pub name: String,
    /// Optional description of the resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type of the resource content ("text" or "blob")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// The size of the raw resource content, in bytes (i.e., before base64 encoding or any tokenization), if known.
    ///
    /// This can be used by Hosts to display file sizes and estimate context window us
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
}

pub type Resource = Annotated<RawResource>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RawResourceTemplate {
    pub uri_template: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

pub type ResourceTemplate = Annotated<RawResourceTemplate>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase", untagged)]
pub enum ResourceContents {
    TextResourceContents {
        uri: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        text: String,
    },
    BlobResourceContents {
        uri: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        blob: String,
    },
}

impl ResourceContents {
    pub fn text(text: impl Into<String>, uri: impl Into<String>) -> Self {
        Self::TextResourceContents {
            uri: uri.into(),
            mime_type: Some("text".into()),
            text: text.into(),
        }
    }
}

impl RawResource {
    /// Creates a new Resource from a URI with explicit mime type
    pub fn new(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            description: None,
            mime_type: None,
            size: None,
        }
    }
}
