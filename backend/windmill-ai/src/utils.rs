use crate::{
    ai_providers::AIProvider,
    ai_types::{ContentPart, OpenAIContent, OpenAIMessage},
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

/// Collect the system prompt for providers that take it in a dedicated top-level field
/// (Anthropic's `system`, OpenAI's `instructions`) instead of inline in the message list.
///
/// Every system message in `messages` is joined, since manual-memory conversations can carry
/// system messages of their own alongside the one the caller prepends from `system_prompt`.
/// `system_prompt` is a fallback used only when `messages` holds no system message, for callers
/// that pass it without prepending it. Only text content survives: a system message carrying
/// images or files keeps its text and drops the rest, matching Bedrock (see `ai_bedrock.rs`).
/// Such providers must in turn leave system messages out of the message list they send, or the
/// same prompt goes over the wire twice.
pub fn collect_system_prompt(
    messages: &[OpenAIMessage],
    system_prompt: Option<&str>,
) -> Option<String> {
    let from_messages = messages
        .iter()
        .filter(|message| message.role == "system")
        .filter_map(|message| message.content.as_ref().map(extract_text_content))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>();

    if from_messages.is_empty() {
        system_prompt
            .filter(|prompt| !prompt.is_empty())
            .map(str::to_string)
    } else {
        Some(from_messages.join("\n\n"))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn message(role: &str, text: &str) -> OpenAIMessage {
        OpenAIMessage {
            role: role.to_string(),
            content: Some(OpenAIContent::Text(text.to_string())),
            ..Default::default()
        }
    }

    #[test]
    fn joins_every_system_message() {
        let messages = vec![
            message("system", "be helpful"),
            message("user", "hi"),
            message("system", "be terse"),
        ];

        assert_eq!(
            collect_system_prompt(&messages, Some("be helpful")),
            Some("be helpful\n\nbe terse".to_string())
        );
    }

    /// A dedicated system field is text-only, so non-text parts cannot be carried over.
    #[test]
    fn keeps_only_text_parts_of_a_system_message() {
        let messages = vec![OpenAIMessage {
            role: "system".to_string(),
            content: Some(OpenAIContent::Parts(vec![
                ContentPart::Text { text: "be terse".to_string() },
                ContentPart::ImageUrl {
                    image_url: crate::ai_types::ImageUrlData {
                        url: "data:image/png;base64,x".to_string(),
                    },
                },
            ])),
            ..Default::default()
        }];

        assert_eq!(
            collect_system_prompt(&messages, None),
            Some("be terse".to_string())
        );
    }

    /// The argument is a fallback, not an extra source: system messages win outright.
    #[test]
    fn prefers_system_messages_over_the_argument() {
        let messages = vec![message("system", "be terse"), message("user", "hi")];

        assert_eq!(
            collect_system_prompt(&messages, Some("unused fallback")),
            Some("be terse".to_string())
        );
    }

    #[test]
    fn falls_back_to_the_system_prompt_argument() {
        let messages = vec![message("user", "hi")];

        assert_eq!(
            collect_system_prompt(&messages, Some("be helpful")),
            Some("be helpful".to_string())
        );
    }

    #[test]
    fn treats_empty_prompts_as_absent() {
        assert_eq!(
            collect_system_prompt(&[message("user", "hi")], Some("")),
            None
        );
        assert_eq!(collect_system_prompt(&[message("user", "hi")], None), None);
        assert_eq!(collect_system_prompt(&[message("system", "")], None), None);
    }
}
