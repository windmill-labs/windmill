use bytes::Bytes;
use serde::Deserialize;
use serde_json::json;
use windmill_common::error::{Error, Result};

use crate::ai_providers::AIProvider;

#[derive(Debug, Eq, PartialEq)]
pub struct FimProxyTransform {
    pub body: Bytes,
    pub path: String,
}

#[derive(Deserialize)]
struct FimRequest {
    model: String,
    prompt: String,
    suffix: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    stop: Option<Vec<String>>,
}

pub fn supports_native_fim(provider: &AIProvider) -> bool {
    matches!(provider, AIProvider::Mistral)
}

pub fn maybe_transform_fim_request(
    provider: &AIProvider,
    path: &str,
    body: &[u8],
) -> Result<Option<FimProxyTransform>> {
    if path.contains("fim/completions") && !supports_native_fim(provider) {
        transform_fim_to_chat_completions(body).map(Some)
    } else {
        Ok(None)
    }
}

fn transform_fim_to_chat_completions(body: &[u8]) -> Result<FimProxyTransform> {
    let fim_req: FimRequest = serde_json::from_slice(body)
        .map_err(|e| Error::BadRequest(format!("Failed to parse FIM request: {}", e)))?;

    let suffix = fim_req.suffix.unwrap_or_default();

    let system_prompt = "You are a code completion assistant. Complete the code at the <CURSOR/> position between the given prefix and suffix. Output ONLY the code that goes at the cursor - no explanations, no markdown, no repeating the prefix or suffix.";

    let user_content = format!(
        "<PREFIX>\n{}\n<CURSOR/>\n<SUFFIX>\n{}",
        fim_req.prompt, suffix
    );

    let chat_req = json!({
        "model": fim_req.model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_content}
        ],
        "temperature": fim_req.temperature.unwrap_or(0.0),
        "max_tokens": fim_req.max_tokens.unwrap_or(256),
        "stop": fim_req.stop
    });

    let body = serde_json::to_vec(&chat_req)
        .map_err(|e| Error::internal_err(format!("Failed to serialize chat request: {}", e)))?;

    Ok(FimProxyTransform { body: Bytes::from(body), path: "chat/completions".to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mistral_keeps_native_fim_request() {
        let transformed =
            maybe_transform_fim_request(&AIProvider::Mistral, "fim/completions", br#"{}"#).unwrap();

        assert!(transformed.is_none());
        assert!(supports_native_fim(&AIProvider::Mistral));
    }

    #[test]
    fn openai_fim_request_is_transformed_to_chat_completion() {
        let transformed = maybe_transform_fim_request(
            &AIProvider::OpenAI,
            "fim/completions",
            br#"{
                "model": "gpt-4.1",
                "prompt": "fn main() {",
                "suffix": "}",
                "stop": ["\n\n"]
            }"#,
        )
        .unwrap()
        .expect("OpenAI FIM should be transformed");

        assert_eq!(transformed.path, "chat/completions");

        let body: serde_json::Value = serde_json::from_slice(&transformed.body).unwrap();
        assert_eq!(body["model"], "gpt-4.1");
        assert_eq!(body["temperature"], 0.0);
        assert_eq!(body["max_tokens"], 256);
        assert_eq!(body["stop"], serde_json::json!(["\n\n"]));
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(
            body["messages"][1]["content"],
            "<PREFIX>\nfn main() {\n<CURSOR/>\n<SUFFIX>\n}"
        );
    }

    #[test]
    fn invalid_fim_body_is_bad_request() {
        let err =
            maybe_transform_fim_request(&AIProvider::OpenAI, "fim/completions", br#"{"model": 1}"#)
                .unwrap_err();

        assert!(matches!(err, Error::BadRequest(_)));
    }
}
