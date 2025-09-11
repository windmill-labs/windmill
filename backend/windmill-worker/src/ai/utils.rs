use windmill_common::ai_providers::AIProvider;
use crate::ai::types::ProviderWithResource;

/// Check if the provider is Anthropic (either direct or through OpenRouter)
pub fn is_anthropic_provider(provider: &ProviderWithResource) -> bool {
    let provider_is_anthropic = provider.kind.is_anthropic();
    let is_openrouter_anthropic = provider.kind == AIProvider::OpenRouter
        && provider.model.starts_with("anthropic/");
    provider_is_anthropic || is_openrouter_anthropic
}

/// Check if the provider supports tools with the given output type
pub fn supports_tools_with_output_type(provider: &ProviderWithResource, output_type: &crate::ai::types::OutputType) -> bool {
    use crate::ai::types::OutputType;
    
    match output_type {
        OutputType::Text => true, // All providers support tools with text output
        OutputType::Image => {
            // Only OpenAI and OpenRouter support tools with image output
            matches!(provider.kind, AIProvider::OpenAI | AIProvider::OpenRouter)
        }
    }
}