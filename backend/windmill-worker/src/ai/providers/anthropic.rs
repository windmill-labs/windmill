use async_trait::async_trait;
use serde_json;
use windmill_common::{
    client::AuthedClient,
    error::Error,
};

use crate::ai::{
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder},
    types::*,
};

pub struct AnthropicQueryBuilder;

impl AnthropicQueryBuilder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl QueryBuilder for AnthropicQueryBuilder {
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool {
        // Anthropic only supports tools for text output
        matches!(output_type, OutputType::Text)
    }

    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        match args.output_type {
            OutputType::Text => {
                // For text output, use OpenAI-compatible format
                // This allows Anthropic to work with the standard OpenAI API format
                let openai_builder = super::openai::OpenAIQueryBuilder::new();
                openai_builder.build_request(args, client, workspace_id).await
            }
            OutputType::Image => {
                Err(Error::internal_err(
                    "Anthropic does not support image generation".to_string(),
                ))
            }
        }
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        // For text responses, parse as OpenAI format
        let openai_builder = super::openai::OpenAIQueryBuilder::new();
        openai_builder.parse_response(response).await
    }

    fn get_endpoint(&self, base_url: &str, output_type: &OutputType) -> String {
        match output_type {
            OutputType::Text => format!("{}/chat/completions", base_url), // Use OpenAI-compatible endpoint
            OutputType::Image => format!("{}/messages", base_url), // Not used, but keep for consistency
        }
    }
}