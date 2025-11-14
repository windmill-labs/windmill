// Copyright Windmill Labs. All rights reserved.

//! Bearer token authentication provider for AWS Bedrock
//!
//! This module provides a simple token provider that uses bearer tokens
//! (API keys) for AWS Bedrock authentication instead of AWS access keys.

use aws_credential_types::provider::token::ProvideToken;

/// Bearer token provider for AWS Bedrock authentication
///
/// This provider uses simple API keys from Windmill resources as bearer tokens
/// for authenticating with AWS Bedrock, which is simpler than managing
/// AWS access keys and secret keys.
#[derive(Debug, Clone)]
pub struct BearerTokenProvider {
    token: String,
}

impl BearerTokenProvider {
    /// Create a new bearer token provider
    ///
    /// # Arguments
    /// * `token` - The bearer token (API key) to use for authentication
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

impl ProvideToken for BearerTokenProvider {
    fn provide_token<'a>(
        &'a self,
    ) -> aws_credential_types::provider::future::ProvideToken<'a>
    where
        Self: 'a,
    {
        aws_credential_types::provider::future::ProvideToken::ready(Ok(
            aws_credential_types::Token::new(self.token.clone(), None),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bearer_token_provider_creation() {
        let token = "test_token_123".to_string();
        let provider = BearerTokenProvider::new(token.clone());

        // Just verify we can create the provider
        assert_eq!(provider.token, token);
    }
}
