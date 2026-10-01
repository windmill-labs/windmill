pub mod anthropic;
#[cfg(feature = "bedrock")]
pub mod bedrock;
pub mod google_ai;
pub mod openai;
pub mod openrouter;
pub mod other;

use std::time::{Duration, Instant};

/// The effort token the chat and agent surfaces send to turn reasoning off.
/// It is not a provider-native level — each provider translates it to its own
/// disable (Anthropic and Bedrock to `thinking: {type: "disabled"}`, DeepSeek to
/// its `thinking` param, Gemini to a zero budget or the model's floor).
pub(crate) const REASONING_OFF_SENTINEL: &str = "none";

/// The effort to send for a model, dropping the off sentinel on a model that rejects every
/// disable: the model then reasons at its default instead of failing the request. The UI
/// never offers off on these, so this guards an agent step saved before it stopped, or an
/// effort passed in as a flow input.
pub fn effective_reasoning_effort<'a>(model: &str, effort: Option<&'a str>) -> Option<&'a str> {
    match effort {
        Some(effort)
            if effort == REASONING_OFF_SENTINEL
                && reasoning_rule(model).is_some_and(|rule| !rule.can_disable) =>
        {
            None
        }
        effort => effort,
    }
}

/// Whether a request with function tools must send the off sentinel on Chat Completions:
/// the model refuses tools there while it reasons, and does accept being turned off.
pub(crate) fn completions_tools_need_reasoning_off(model: &str) -> bool {
    reasoning_rule(model).is_some_and(|rule| rule.completions_tools_need_off && rule.can_disable)
}

/// What the backend needs to know about a model's reasoning: the rows of `REASONING_RULES`
/// in the frontend's `reasoningRegistry.ts`, cut down to the two facts the wire needs.
/// `reasoningParity.json` next to that file is checked by both sides' tests.
struct ReasoningRule {
    matches: fn(&str) -> bool,
    /// False when the provider rejects every disable, so the off sentinel must not be sent.
    can_disable: bool,
    /// Chat Completions refuses function tools while the model reasons, even with the
    /// effort omitted (live-verified); the Responses API has no such limit.
    completions_tools_need_off: bool,
}

/// Matched in order against the lowercased model id, first match wins. A model no row
/// matches keeps whatever effort it was given.
const REASONING_RULES: &[ReasoningRule] = &[
    // Live-verified: Fable, Mythos and the 5.x point releases reject `thinking: disabled`.
    ReasoningRule {
        matches: |m| m.contains("claude-fable") || m.contains("claude-mythos"),
        can_disable: false,
        completions_tools_need_off: false,
    },
    ReasoningRule {
        matches: is_claude_5_point_release,
        can_disable: false,
        completions_tools_need_off: false,
    },
    // Live-verified: astra takes low..max only, where sol and luna also take `none`.
    ReasoningRule {
        matches: |m| base_id(m).starts_with("gpt-6-astra"),
        can_disable: false,
        completions_tools_need_off: true,
    },
    ReasoningRule {
        matches: |m| gpt_version(m).is_some_and(|(major, _)| major >= 6),
        can_disable: true,
        completions_tools_need_off: true,
    },
    ReasoningRule {
        matches: |m| matches!(gpt_version(m), Some((5, Some(minor))) if minor >= 5),
        can_disable: true,
        completions_tools_need_off: true,
    },
    ReasoningRule {
        matches: |m| matches!(gpt_version(m), Some((5, Some(_)))),
        can_disable: true,
        completions_tools_need_off: false,
    },
    // gpt-5 and the o-series reject `none`.
    ReasoningRule {
        matches: |m| matches!(gpt_version(m), Some((5, None))),
        can_disable: false,
        completions_tools_need_off: false,
    },
    ReasoningRule {
        matches: |m| {
            let base = base_id(m);
            base.starts_with('o') && base[1..].starts_with(|c: char| c.is_ascii_digit())
        },
        can_disable: false,
        completions_tools_need_off: false,
    },
];

fn reasoning_rule(model: &str) -> Option<&'static ReasoningRule> {
    let model = model.to_lowercase();
    REASONING_RULES.iter().find(|rule| (rule.matches)(&model))
}

/// The id after a gateway's `vendor/` and before a `:variant`.
fn base_id(model: &str) -> &str {
    let last = model.rsplit('/').next().unwrap_or(model);
    last.split(':').next().unwrap_or(last)
}

/// `(major, minor)` of a `gpt-` id. The major is one digit then a separator or the end,
/// since Azure names gpt-3.5 `gpt-35-turbo`.
fn gpt_version(model: &str) -> Option<(u32, Option<u32>)> {
    let rest = base_id(model).strip_prefix("gpt-")?;
    let mut chars = rest.chars();
    let major = chars.next()?.to_digit(10)?;
    match chars.next() {
        None | Some('-') => Some((major, None)),
        Some('.') => {
            let minor: String = chars.take_while(char::is_ascii_digit).collect();
            Some((major, minor.parse().ok()))
        }
        _ => None,
    }
}

/// Sonnet or Opus 5.x with x >= 1. The version match stops at one digit so a dated id
/// (`claude-sonnet-5-20260101`) stays Sonnet 5.
fn is_claude_5_point_release(model: &str) -> bool {
    let model = model.replace('.', "-");
    ["claude-opus-5-", "claude-sonnet-5-"].iter().any(|prefix| {
        model.split(prefix).skip(1).any(|rest| {
            let mut chars = rest.chars();
            matches!(chars.next(), Some('1'..='9')) && !matches!(chars.next(), Some('0'..='9'))
        })
    })
}

/// Whether a Claude model removed the sampling params (`temperature`, `top_p`,
/// `top_k`). On these, any value is a hard 400 — `temperature is deprecated for
/// this model` — whatever the thinking mode, so the param has to be dropped on
/// the reasoning-off and no-reasoning paths too, not only under adaptive
/// thinking.
///
/// Probed against the Messages API: `claude-opus-5`, `claude-sonnet-5` and
/// `claude-opus-4-8` reject them; `claude-sonnet-4-6` still accepts them. Opus
/// 4.7, Fable and Mythos are included from Anthropic's migration guide, which
/// documents the same removal, rather than from a probe.
///
/// Matching is on the model name, so a Bedrock *application* inference profile —
/// whose id is opaque (`k1c3lwu20lem`) rather than derived from the model —
/// cannot be classified and keeps its sampling params. Resolving the backing
/// model would need a per-request AWS lookup; `bedrock_model_supports_prompt_caching`
/// degrades on the same ids for the same reason.
pub(crate) fn anthropic_model_rejects_sampling_params(model: &str) -> bool {
    let model = model.to_lowercase().replace('.', "-");
    model.contains("claude-opus-4-7")
        || model.contains("claude-opus-4-8")
        || model.contains("claude-opus-5")
        || model.contains("claude-sonnet-5")
        || model.contains("claude-fable")
        || model.contains("claude-mythos")
}

use windmill_common::cache::Cache;

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
        // Azure OpenAI serves the same Responses API as OpenAI under `/openai/v1`, and
        // newer deployments reject a reasoning effort combined with function tools on
        // `/chat/completions`. This cannot be decided per model: on Azure the model name
        // is a user-chosen deployment name, which says nothing about the model behind it.
        AIProvider::OpenAI | AIProvider::AzureOpenAI => {
            Box::new(OpenAIQueryBuilder::new(credentials.provider.clone()))
        }
        AIProvider::Anthropic => Box::new(AnthropicQueryBuilder::new(
            credentials.provider.clone(),
            credentials.platform.clone(),
        )),
        AIProvider::OpenRouter => Box::new(OpenRouterQueryBuilder::new()),
        AIProvider::AzureFoundry if AIProvider::is_anthropic_model(model) => Box::new(
            AnthropicQueryBuilder::new(AIProvider::AzureFoundry, AIPlatform::Standard),
        ),
        // Only for forwarding through the API proxy, which is bearer auth on `<base>/<path>` as
        // TypeSafe expects. TypeSafe serves no chat route, and `run_agent` refuses it before a
        // chat request could be built.
        AIProvider::TypeSafe => Box::new(OtherQueryBuilder::new(AIProvider::TypeSafe)),
        _ => Box::new(OtherQueryBuilder::new(credentials.provider.clone())),
    }
}

/// The builder for the same resource on the OpenAI-compatible `/chat/completions`
/// surface, for a provider whose preferred surface it turned out not to serve
/// (`QueryBuilder::supports_chat_completions_fallback`).
pub fn create_chat_completions_query_builder(
    credentials: &ProviderCredentials,
) -> Box<dyn QueryBuilder> {
    Box::new(OtherQueryBuilder::new(credentials.provider.clone()))
}

lazy_static::lazy_static! {
    /// Deployments that turned out not to serve the endpoint their provider prefers,
    /// so the steps after the one that found out start on `/chat/completions` rather
    /// than paying a rejected call each to learn the same thing. Keyed by endpoint and
    /// deployment name, because Azure's Responses API support varies by both.
    ///
    /// Only rejections are remembered: a surface that answers costs nothing to keep
    /// using. The entry expires because a deployment can gain Responses support
    /// (Azure rolls it out per model and region) without anything here changing.
    static ref CHAT_COMPLETIONS_ONLY: Cache<(String, String), Instant> = Cache::new(500);
}

const CHAT_COMPLETIONS_ONLY_TTL: Duration = Duration::from_secs(3600);

/// Whether this deployment is known not to serve the endpoint its provider prefers.
pub fn is_chat_completions_only(base_url: &str, model: &str) -> bool {
    CHAT_COMPLETIONS_ONLY
        .get(&(base_url.to_string(), model.to_string()))
        .is_some_and(|learned_at| learned_at.elapsed() < CHAT_COMPLETIONS_ONLY_TTL)
}

/// Record that this deployment rejected the endpoint its provider prefers.
pub fn remember_chat_completions_only(base_url: &str, model: &str) {
    CHAT_COMPLETIONS_ONLY.insert((base_url.to_string(), model.to_string()), Instant::now());
}

#[cfg(test)]
mod reasoning_rule_tests {
    use super::*;

    /// The frontend registry's test reads the same file. These rules see the model id
    /// alone, so it holds only rows whose answer doesn't depend on the provider.
    #[test]
    fn agrees_with_the_frontend_registry() {
        let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../frontend/src/lib/components/copilot/reasoningParity.json"
        ))
        .unwrap();
        for row in rows {
            let model = row["model"].as_str().unwrap();
            let can_disable = row["canDisable"].as_bool().unwrap();
            let tools_need_off = row["completionsToolsNeedOff"].as_bool().unwrap();
            let sent = effective_reasoning_effort(model, Some(REASONING_OFF_SENTINEL));
            assert_eq!(sent.is_some(), can_disable, "{model}");
            assert_eq!(
                effective_reasoning_effort(model, Some("low")),
                Some("low"),
                "{model}"
            );
            assert_eq!(
                completions_tools_need_reasoning_off(model),
                tools_need_off && can_disable,
                "{model}"
            );
        }
    }

    #[test]
    fn reads_ids_the_frontend_resolves_first() {
        // Azure's gpt-3.5 is not major 35, and a gateway prefix is not part of the id.
        assert_eq!(gpt_version("gpt-35-turbo"), None);
        assert!(completions_tools_need_reasoning_off("openai/gpt-6-sol"));
        assert_eq!(effective_reasoning_effort("openai/o3", Some("none")), None);
    }
}

#[cfg(test)]
mod chat_completions_only_tests {
    use super::*;

    /// Azure's Responses API support varies by deployment, so what one deployment of a
    /// resource rejected says nothing about the next one.
    #[test]
    fn a_rejection_is_remembered_per_deployment() {
        let base_url = "https://rejection-per-deployment.openai.azure.com/openai";

        remember_chat_completions_only(base_url, "legacy-deployment");

        assert!(is_chat_completions_only(base_url, "legacy-deployment"));
        assert!(!is_chat_completions_only(base_url, "gpt-5-deployment"));
        assert!(!is_chat_completions_only(
            "https://other.openai.azure.com/openai",
            "legacy-deployment"
        ));
    }
}

/// The proxy (workspace/instance AI settings) and the query builder (AI agent step)
/// each derive the endpoint and the credential header for the same resource. They
/// must agree, or a resource authenticates in one and 401s in the other.
#[cfg(test)]
mod parity_tests {
    use super::*;
    use crate::proxy::{
        credential_header, proxy_execution_mode, retain_effective_credentials, ProxyBuildArgs,
        ProxyExecutionMode, CREDENTIAL_HEADERS,
    };
    use crate::types::OutputType;
    use http::{HeaderMap, Method};
    use std::collections::HashMap;

    struct ParityCase {
        provider: AIProvider,
        platform: AIPlatform,
        base_url: &'static str,
        model: &'static str,
        /// The path the client SDK sends for the endpoint the query builder targets.
        proxy_path: &'static str,
    }

    fn case(
        provider: AIProvider,
        base_url: &'static str,
        model: &'static str,
        proxy_path: &'static str,
    ) -> ParityCase {
        ParityCase { provider, platform: AIPlatform::Standard, base_url, model, proxy_path }
    }

    fn credentials(kase: &ParityCase) -> ProviderCredentials {
        ProviderCredentials {
            provider: kase.provider.clone(),
            base_url: kase.base_url.to_string(),
            api_key: Some("secret".to_string()),
            access_token: None,
            organization_id: None,
            user: None,
            region: None,
            aws_access_key_id: None,
            aws_secret_access_key: None,
            aws_session_token: None,
            oidc_role_arn: None,
            platform: kase.platform.clone(),
            custom_headers: HashMap::new(),
        }
    }

    fn only_credentials<I: IntoIterator<Item = (String, String)>>(
        headers: I,
    ) -> Vec<(String, String)> {
        let mut found = headers
            .into_iter()
            .filter(|(name, _)| {
                CREDENTIAL_HEADERS
                    .iter()
                    .any(|credential| name.eq_ignore_ascii_case(credential))
            })
            .map(|(name, value)| (name.to_ascii_lowercase(), value))
            .collect::<Vec<_>>();
        found.sort();
        found
    }

    fn cases() -> Vec<ParityCase> {
        vec![
            case(
                AIProvider::Anthropic,
                "https://api.anthropic.com/v1",
                "claude-sonnet-5",
                "v1/messages",
            ),
            // A gateway base URL without the `/v1` must resolve the same way in both.
            case(
                AIProvider::Anthropic,
                "https://gateway.example/proxy/anthropic",
                "claude-sonnet-5",
                "v1/messages",
            ),
            ParityCase {
                provider: AIProvider::Anthropic,
                platform: AIPlatform::GoogleVertexAi,
                base_url: "https://europe-west1-aiplatform.googleapis.com/v1/projects/p/locations/europe-west1/publishers/anthropic/models",
                model: "claude-sonnet-5",
                proxy_path: "v1/messages",
            },
            case(
                AIProvider::AzureFoundry,
                "https://example.services.ai.azure.com",
                "claude-sonnet-5",
                "v1/messages",
            ),
            case(
                AIProvider::AzureOpenAI,
                "https://example.openai.azure.com/openai",
                "gpt-4o",
                "responses",
            ),
            case(
                AIProvider::OpenAI,
                "https://api.openai.com/v1",
                "gpt-5.6-terra",
                "responses",
            ),
            // An OpenAI-compatible gateway keeps bearer auth and the plain path.
            case(
                AIProvider::OpenAI,
                "https://gateway.example/proxy/openai",
                "gpt-5.6-terra",
                "responses",
            ),
            // An OpenAI resource pointed at Azure through `openai_azure_base_path`.
            case(
                AIProvider::OpenAI,
                "https://example.openai.azure.com/openai/deployments/gpt-4o",
                "gpt-4o",
                "responses",
            ),
            case(
                AIProvider::CustomAI,
                "https://litellm.example/v1",
                "gpt-4o",
                "chat/completions",
            ),
            // A base URL stored with a trailing slash must not double it.
            case(
                AIProvider::CustomAI,
                "https://litellm.example/v1/",
                "gpt-4o",
                "chat/completions",
            ),
            case(
                AIProvider::OpenRouter,
                "https://openrouter.ai/api/v1",
                "anthropic/claude-sonnet-5",
                "chat/completions",
            ),
        ]
    }

    /// The agent step assembles credential headers itself (`ai_executor`), so the
    /// rule it applies must match `credential_header`: the built-in credential is
    /// dropped when the resource carries its own and when there is no key at all,
    /// and an empty credential never goes out.
    #[test]
    fn both_paths_agree_on_when_the_built_in_credential_is_dropped() {
        let mut kase = case(
            AIProvider::CustomAI,
            "https://gateway.example/v1",
            "gpt-4o",
            "chat/completions",
        );
        kase.platform = AIPlatform::Standard;

        for (api_key, resource_header) in [
            // Keyless endpoint: no credential from either path.
            (None, None),
            // Resource carries its own: only that one travels.
            (None, Some(("x-api-key", "resource-key"))),
            (Some("secret"), Some(("Authorization", "Bearer resource"))),
            // Nothing to defer to: the built-in credential travels.
            (Some("secret"), None),
        ] {
            let mut credentials = credentials(&kase);
            credentials.api_key = api_key.map(str::to_string);
            if let Some((name, value)) = resource_header {
                credentials.custom_headers = HashMap::from([(name.to_string(), value.to_string())]);
            }
            let label = format!("api_key={api_key:?} resource_header={resource_header:?}");

            let from_proxy = credential_header(&credentials, "authorization");
            let from_agent_step = agent_step_credential(&credentials, &kase);

            assert_eq!(
                from_proxy, from_agent_step,
                "{label}: proxy and agent step disagree on the built-in credential"
            );
            assert!(
                from_agent_step
                    .as_ref()
                    .is_none_or(|(_, value)| value != "Bearer "),
                "{label}: an empty credential must not be sent"
            );
        }
    }

    /// The credential the agent step is left with, through the same helper it uses.
    fn agent_step_credential(
        credentials: &ProviderCredentials,
        kase: &ParityCase,
    ) -> Option<(String, String)> {
        let built = create_query_builder(credentials, kase.model).get_auth_headers(
            credentials.api_key.as_deref().unwrap_or(""),
            &credentials.base_url,
            &OutputType::Text,
        );
        only_credentials(
            retain_effective_credentials(credentials, built)
                .into_iter()
                .map(|(name, value)| (name.to_string(), value)),
        )
        .pop()
    }

    #[test]
    fn both_paths_agree_on_endpoint_and_credential() {
        for kase in cases() {
            let credentials = credentials(&kase);
            let builder = create_query_builder(&credentials, kase.model);
            let label = format!("{:?} {:?} {}", kase.provider, kase.platform, kase.base_url);

            assert_eq!(
                proxy_execution_mode(&kase.provider),
                ProxyExecutionMode::HttpForward,
                "{label}: only forwarded providers have both paths to compare"
            );

            let headers = HeaderMap::new();
            // Vertex reads the model from the body to build its URL, so the body must
            // name the same model the query builder is given.
            let body = format!(r#"{{"model":"{}","messages":[]}}"#, kase.model);
            let args = ProxyBuildArgs {
                method: &Method::POST,
                path: kase.proxy_path,
                headers: &headers,
                body: body.as_bytes(),
                credentials: &credentials,
            };
            let proxied = builder
                .build_proxy_request(&args)
                .unwrap_or_else(|e| panic!("{label}: proxy request failed: {e}"));

            assert_eq!(
                builder.get_endpoint(&credentials.base_url, kase.model, &OutputType::Text),
                proxied.url,
                "{label}: agent step and proxy disagree on the endpoint"
            );

            let from_builder = builder
                .get_auth_headers("secret", &credentials.base_url, &OutputType::Text)
                .into_iter()
                .map(|(name, value)| (name.to_string(), value));
            assert_eq!(
                only_credentials(from_builder),
                only_credentials(proxied.headers),
                "{label}: agent step and proxy disagree on the credential header"
            );
        }
    }
}
