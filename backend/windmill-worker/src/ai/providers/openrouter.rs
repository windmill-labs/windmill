use async_trait::async_trait;
use windmill_common::{
    client::AuthedClient,
    error::Error,
};

use crate::ai::{
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder},
    types::*,
    providers::openai::OpenAIQueryBuilder,
};

pub struct OpenRouterQueryBuilder {
    // OpenRouter uses OpenAI-compatible API, so we delegate most work to OpenAI builder
    openai_builder: OpenAIQueryBuilder,
}

impl OpenRouterQueryBuilder {
    pub fn new() -> Self {
        Self {
            openai_builder: OpenAIQueryBuilder::new(),
        }
    }
}

#[async_trait]
impl QueryBuilder for OpenRouterQueryBuilder {
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool {
        // OpenRouter supports tools for both text and image output (via OpenAI-compatible API)
        true
    }

    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        // OpenRouter uses OpenAI-compatible API for both text and image generation
        self.openai_builder.build_request(args, client, workspace_id).await
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        // OpenRouter uses OpenAI-compatible format, so delegate to OpenAI parser
        self.openai_builder.parse_response(response).await
    }

    fn get_endpoint(&self, base_url: &str, output_type: &OutputType) -> String {
        // OpenRouter uses the same endpoint for both text and image generation
        format!("{}/chat/completions", base_url)
    }
}