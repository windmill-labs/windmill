pub mod anthropic;
#[cfg(feature = "bedrock")]
pub mod bedrock;
pub mod google_ai;
pub mod openai;
pub mod openrouter;
pub mod other;

use crate::{ai_providers::AIProvider, query_builder::QueryBuilder, types::ProviderWithResource};

use self::{
    anthropic::AnthropicQueryBuilder, google_ai::GoogleAIQueryBuilder, openai::OpenAIQueryBuilder,
    openrouter::OpenRouterQueryBuilder, other::OtherQueryBuilder,
};

/// Factory function to create the appropriate query builder for a provider.
pub fn create_query_builder(provider: &ProviderWithResource) -> Box<dyn QueryBuilder> {
    match provider.kind {
        AIProvider::GoogleAI => {
            Box::new(GoogleAIQueryBuilder::new(provider.get_platform().clone()))
        }
        AIProvider::OpenAI => Box::new(OpenAIQueryBuilder::new(provider.kind.clone())),
        AIProvider::Anthropic => Box::new(AnthropicQueryBuilder::new(
            provider.kind.clone(),
            provider.get_platform().clone(),
            provider.get_enable_1m_context(),
        )),
        AIProvider::OpenRouter => Box::new(OpenRouterQueryBuilder::new()),
        _ => Box::new(OtherQueryBuilder::new(provider.kind.clone())),
    }
}
