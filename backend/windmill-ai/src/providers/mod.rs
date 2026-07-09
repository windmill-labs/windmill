pub mod anthropic;
#[cfg(feature = "bedrock")]
pub mod bedrock;
pub mod google_ai;
pub mod openai;
pub mod openrouter;
pub mod other;

use crate::{
    ai_providers::{AIPlatform, AIProvider},
    credentials::ProviderCredentials,
    query_builder::QueryBuilder,
};

use self::{
    anthropic::AnthropicQueryBuilder, google_ai::GoogleAIQueryBuilder, openai::OpenAIQueryBuilder,
    openrouter::OpenRouterQueryBuilder, other::OtherQueryBuilder,
};

/// Factory function to create the appropriate query builder from resolved credentials.
///
/// `model` is the deployment/model name of the request. It matters only for Azure AI
/// Foundry, which fronts multiple model families under one resource: Claude
/// deployments speak the Anthropic Messages API while everything else is
/// OpenAI-compatible, so the builder is chosen per model rather than per provider.
pub fn create_query_builder(
    credentials: &ProviderCredentials,
    model: &str,
) -> Box<dyn QueryBuilder> {
    match credentials.provider {
        AIProvider::GoogleAI => Box::new(GoogleAIQueryBuilder::new(credentials.platform.clone())),
        AIProvider::OpenAI => Box::new(OpenAIQueryBuilder::new(credentials.provider.clone())),
        AIProvider::Anthropic => Box::new(AnthropicQueryBuilder::new(
            credentials.provider.clone(),
            credentials.platform.clone(),
        )),
        AIProvider::OpenRouter => Box::new(OpenRouterQueryBuilder::new()),
        AIProvider::AzureFoundry if AIProvider::is_anthropic_model(model) => Box::new(
            AnthropicQueryBuilder::new(AIProvider::AzureFoundry, AIPlatform::Standard),
        ),
        _ => Box::new(OtherQueryBuilder::new(credentials.provider.clone())),
    }
}
