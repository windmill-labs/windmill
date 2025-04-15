//! Content sent around agents, extensions, and LLMs
//! The various content types can be display to humans but also understood by models
//! They include optional annotations used to help inform agent usage
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{AnnotateAble, Annotated, resource::ResourceContents};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTextContent {
    pub text: String,
}
pub type TextContent = Annotated<RawTextContent>;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawImageContent {
    /// The base64-encoded image
    pub data: String,
    pub mime_type: String,
}

pub type ImageContent = Annotated<RawImageContent>;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawEmbeddedResource {
    pub resource: ResourceContents,
}
pub type EmbeddedResource = Annotated<RawEmbeddedResource>;

impl EmbeddedResource {
    pub fn get_text(&self) -> String {
        match &self.resource {
            ResourceContents::TextResourceContents { text, .. } => text.clone(),
            _ => String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawAudioContent {
    pub data: String,
    pub mime_type: String,
}

pub type AudioContent = Annotated<RawAudioContent>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RawContent {
    Text(RawTextContent),
    Image(RawImageContent),
    Resource(RawEmbeddedResource),
    Audio(AudioContent),
}

pub type Content = Annotated<RawContent>;

impl RawContent {
    pub fn json<S: Serialize>(json: S) -> Result<Self, crate::Error> {
        let json = serde_json::to_string(&json).map_err(|e| {
            crate::Error::internal_error(
                "fail to serialize response to json",
                Some(json!(
                    {"reason": e.to_string()}
                )),
            )
        })?;
        Ok(RawContent::text(json))
    }

    pub fn text<S: Into<String>>(text: S) -> Self {
        RawContent::Text(RawTextContent { text: text.into() })
    }

    pub fn image<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        RawContent::Image(RawImageContent {
            data: data.into(),
            mime_type: mime_type.into(),
        })
    }

    pub fn resource(resource: ResourceContents) -> Self {
        RawContent::Resource(RawEmbeddedResource { resource })
    }

    pub fn embedded_text<S: Into<String>, T: Into<String>>(uri: S, content: T) -> Self {
        RawContent::Resource(RawEmbeddedResource {
            resource: ResourceContents::TextResourceContents {
                uri: uri.into(),
                mime_type: Some("text".to_string()),
                text: content.into(),
            },
        })
    }

    /// Get the text content if this is a TextContent variant
    pub fn as_text(&self) -> Option<&RawTextContent> {
        match self {
            RawContent::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Get the image content if this is an ImageContent variant
    pub fn as_image(&self) -> Option<&RawImageContent> {
        match self {
            RawContent::Image(image) => Some(image),
            _ => None,
        }
    }

    /// Get the resource content if this is an ImageContent variant
    pub fn as_resource(&self) -> Option<&RawEmbeddedResource> {
        match self {
            RawContent::Resource(resource) => Some(resource),
            _ => None,
        }
    }
}

impl Content {
    pub fn text<S: Into<String>>(text: S) -> Self {
        RawContent::text(text).no_annotation()
    }

    pub fn image<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        RawContent::image(data, mime_type).no_annotation()
    }

    pub fn resource(resource: ResourceContents) -> Self {
        RawContent::resource(resource).no_annotation()
    }

    pub fn embedded_text<S: Into<String>, T: Into<String>>(uri: S, content: T) -> Self {
        RawContent::embedded_text(uri, content).no_annotation()
    }

    pub fn json<S: Serialize>(json: S) -> Result<Self, crate::Error> {
        RawContent::json(json).map(|c| c.no_annotation())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonContent<S: Serialize>(S);
/// Types that can be converted into a list of contents
pub trait IntoContents {
    fn into_contents(self) -> Vec<Content>;
}

impl IntoContents for Content {
    fn into_contents(self) -> Vec<Content> {
        vec![self]
    }
}

impl IntoContents for String {
    fn into_contents(self) -> Vec<Content> {
        vec![Content::text(self)]
    }
}
