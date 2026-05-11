use crate::{
    ai_providers::AIProvider,
    ai_types::{ContentPart, OpenAIContent},
};

lazy_static::lazy_static! {
    /// Parse AI_HTTP_HEADERS environment variable into a vector of (header_name, header_value) tuples
    /// Format: "header1: value1, header2: value2"
    pub static ref AI_HTTP_HEADERS: Vec<(String, String)> = {
        std::env::var("AI_HTTP_HEADERS")
            .ok()
            .map(|headers_str| {
                headers_str
                    .split(',')
                    .filter_map(|header| {
                        let parts: Vec<&str> = header.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let name = parts[0].trim().to_string();
                            let value = parts[1].trim().to_string();
                            if !name.is_empty() && !value.is_empty() {
                                Some((name, value))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
}

/// AWS Bedrock do not handle structured output query param, so we use a tool for structured output. Same for every Claude models.
pub fn should_use_structured_output_tool(provider: &AIProvider, model: &str) -> bool {
    model.contains("claude") || provider == &AIProvider::AWSBedrock
}

/// Extract text content from OpenAIContent, joining parts with space if multiple
pub fn extract_text_content(content: &OpenAIContent) -> String {
    match content {
        OpenAIContent::Text(text) => text.clone(),
        OpenAIContent::Parts(parts) => parts
            .iter()
            .filter_map(|p| {
                if let ContentPart::Text { text } = p {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(""),
    }
}
