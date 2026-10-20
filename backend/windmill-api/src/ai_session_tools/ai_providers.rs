//! The provider config of AI agent and AI decision steps, checked against the workspace's
//! AI provider resources before a flow is written.
//!
//! The catalog of those resources and the models each serves is the chat's own
//! (`chat/flow/aiAgentProviderCatalog.ts`): its prompt needs it anyway, and it lists each
//! resource's models through the provider. The chat sends it when a call needs it. An MCP
//! client gets one built here from the resources and the AI settings alone, without model
//! listings, so it can rule out a malformed or unknown provider but never a model id.

use serde::Deserialize;
use serde_json::{json, Value};

use super::chat::Effect;
use super::{array, truthy, Ctx, Surface};

#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Catalog {
    #[serde(default)]
    options: Vec<ProviderOption>,
    /// `options` holds every AI provider resource of the workspace: one missing from it is
    /// not a resource at all.
    #[serde(default)]
    resources_are_complete: bool,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProviderOption {
    kind: String,
    resource_ref: String,
    models: Models,
    /// The models came from the endpoint's own listing, not a fallback.
    models_are_live: bool,
    /// The resource points at a gateway that may serve ids its listing omits.
    custom_endpoint: bool,
}

#[derive(Deserialize, Clone, Default)]
struct Models {
    ids: Vec<String>,
    complete: bool,
}

/// The kinds that serve decision models only, which an AI agent step cannot run on.
const DECISION_ONLY_KINDS: &[&str] = &["typesafe", "cloudflare"];
/// The model of OpenAI's Decisions API, the one kind both step types run on.
const OPENAI_DECISION_MODEL: &str = "gpt-6-luna";
const DECISION_PROVIDER_KINDS: &[&str] = &["typesafe", "cloudflare", "openai"];
const MAX_PROMPTED_MODELS: usize = 25;
const PROVIDER_SHAPE: &str = r#"{ "type": "static", "value": { "kind": "<provider kind>", "resource": "$res:<resource path>", "model": "<model id>" } }"#;

/// Every AI agent and AI decision module of a module tree, agent tools included, in the
/// order the editor's `forEachAiAgentModule` visits them.
fn for_each_ai_module<'a>(modules: &'a Value, f: &mut impl FnMut(&'a Value, &'a Value)) {
    for module in array(Some(modules)) {
        let Some(value) = module.get("value") else {
            continue;
        };
        match value.get("type").and_then(Value::as_str) {
            Some("aiagent") => {
                f(module, value);
                if let Some(tools) = value.get("tools") {
                    for_each_ai_module(tools, f);
                }
            }
            Some("forloopflow" | "whileloopflow") => {
                if let Some(inner) = value.get("modules") {
                    for_each_ai_module(inner, f);
                }
            }
            Some("aidecision") => f(module, value),
            Some(kind @ ("branchone" | "branchall")) => {
                if kind == "branchone" {
                    if let Some(default) = value.get("default") {
                        for_each_ai_module(default, f);
                    }
                }
                for branch in array(value.get("branches")) {
                    if let Some(inner) = branch.get("modules") {
                        for_each_ai_module(inner, f);
                    }
                }
            }
            _ => {}
        }
    }
}

/// A step's own static provider: what the catalog checks.
fn static_provider(value: &Value) -> Option<&Value> {
    if truthy(value.get("agent")) {
        return None;
    }
    let transform = value.get("input_transforms")?.get("provider")?;
    (transform.get("type").and_then(Value::as_str) == Some("static")).then_some(transform)
}

/// The `$res:` references the steps' providers use, or `None` when no step states a provider
/// of its own and the catalog is not needed.
fn provider_refs(modules: &Value) -> Option<Vec<String>> {
    let mut needed = false;
    let mut refs: Vec<String> = Vec::new();
    for_each_ai_module(modules, &mut |_, value| {
        let Some(transform) = static_provider(value) else {
            return;
        };
        needed = true;
        if let Some(resource) = transform
            .get("value")
            .and_then(|v| v.get("resource"))
            .and_then(Value::as_str)
        {
            if !refs.iter().any(|r| r == resource) {
                refs.push(resource.to_string());
            }
        }
    });
    needed.then_some(refs)
}

/// What the catalog for `modules` is: none needed, the one to check against, or none yet
/// because the chat has to send it first.
pub(super) enum CatalogFor {
    NotNeeded,
    Ready(Catalog),
    Requested,
}

pub(super) async fn catalog_for(ctx: &Ctx<'_>, modules: &Value) -> CatalogFor {
    let Some(resource_refs) = provider_refs(modules) else {
        return CatalogFor::NotNeeded;
    };
    match ctx.surface {
        Surface::Chat => match ctx.chat.and_then(|chat| chat.ai_provider_catalog.clone()) {
            Some(catalog) => CatalogFor::Ready(catalog),
            None => {
                ctx.effect(Effect::NeedsAiProviderCatalog { resource_refs });
                CatalogFor::Requested
            }
        },
        Surface::Mcp => CatalogFor::Ready(listed_catalog(ctx).await),
    }
}

/// The resource types an AI agent or AI decision step's provider can be, as the editor lists
/// them (`AI_PROVIDERS` and `DECISION_AI_PROVIDERS` in `copilot/lib.ts`).
const AI_RESOURCE_TYPES: &[&str] = &[
    "openai",
    "anthropic",
    "googleai",
    "azure_openai",
    "azure_foundry",
    "mistral",
    "deepseek",
    "groq",
    "openrouter",
    "togetherai",
    "aws_bedrock",
    "customai",
    "typesafe",
    "cloudflare",
];
const MAX_PROVIDER_RESOURCES: usize = 8;

/// The workspace's AI provider resources, without model listings: each one's models are the
/// ones its AI settings configure, which can never rule an id out.
async fn listed_catalog(ctx: &Ctx<'_>) -> Catalog {
    let route = ctx.ws(&format!(
        "resources/list?resource_type={}",
        AI_RESOURCE_TYPES.join(",")
    ));
    let (resources, listed_all) = match ctx.get(&route).await {
        Ok(resources) => (resources, true),
        Err(_) => (json!([]), false),
    };
    let config = ctx
        .get(&ctx.ws("workspaces/get_copilot_info"))
        .await
        .unwrap_or(Value::Null);
    let providers = config.get("providers").and_then(Value::as_object);
    let configured = |path: &str| {
        providers
            .into_iter()
            .flatten()
            .find(|(_, p)| p.get("resource_path").and_then(Value::as_str) == Some(path))
            .map(|(kind, p)| (kind.clone(), p))
    };
    let default_kind = config
        .get("default_model")
        .and_then(|m| m.get("provider"))
        .and_then(Value::as_str);
    let mut candidates: Vec<(String, String)> = array(Some(&resources))
        .iter()
        .filter_map(|r| {
            let kind = r.get("resource_type")?.as_str()?;
            let path = r.get("path")?.as_str()?;
            AI_RESOURCE_TYPES
                .contains(&kind)
                .then(|| (kind.to_string(), path.to_string()))
        })
        .collect();
    let rank = |(kind, path): &(String, String)| match configured(path) {
        None => 2,
        Some(_) if Some(kind.as_str()) == default_kind => 0,
        Some(_) => 1,
    };
    candidates.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.1.cmp(&b.1)));
    let complete = listed_all && candidates.len() <= MAX_PROVIDER_RESOURCES;
    let options = candidates
        .into_iter()
        .take(MAX_PROVIDER_RESOURCES)
        .map(|(kind, path)| {
            let ids = configured(&path)
                .map(|(_, p)| {
                    array(p.get("models"))
                        .iter()
                        .filter_map(|m| m.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            ProviderOption {
                kind,
                resource_ref: format!("$res:{path}"),
                models: Models { ids, complete: false },
                models_are_live: false,
                custom_endpoint: true,
            }
        })
        .collect();
    Catalog { options, resources_are_complete: complete }
}

struct Issue {
    message: String,
    blocking: bool,
    /// The catalog established it rather than failing to rule it out.
    settled: bool,
}

fn blocking(message: String) -> Option<Issue> {
    Some(Issue { message, blocking: true, settled: false })
}

fn rules_out_other_models(option: &ProviderOption) -> bool {
    option.models_are_live && option.models.complete && !option.custom_endpoint
}

fn check_provider(value: Option<&Value>, catalog: &Catalog, step_type: &str) -> Option<Issue> {
    let provider = match value {
        Some(Value::String(reference)) => {
            return blocking(format!(
                "provider must be an object {PROVIDER_SHAPE}, not the resource reference string {}",
                json!(reference)
            ))
        }
        Some(Value::Object(provider)) => provider,
        _ => return blocking(format!("provider must be an object {PROVIDER_SHAPE}")),
    };
    let text = |key: &str| {
        provider
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
    };
    let (kind, resource, model) = (text("kind"), text("resource"), text("model"));
    if kind.is_empty() {
        return blocking(format!(
            "provider.kind is missing. Expected {PROVIDER_SHAPE}"
        ));
    }
    if step_type == "aidecision" && !DECISION_PROVIDER_KINDS.contains(&kind) {
        return blocking(format!("an AI decision step runs on a decision model: provider.kind must be \"typesafe\", \"cloudflare\" or \"openai\", not \"{kind}\""));
    }
    if step_type != "aidecision" && DECISION_ONLY_KINDS.contains(&kind) {
        return blocking(format!("\"{kind}\" serves decision models, which answer typed questions rather than messages: use an \"aidecision\" step rather than an AI agent"));
    }
    if !resource.starts_with("$res:") {
        return blocking(format!(
            "provider.resource must be a \"$res:<resource path>\" reference. Expected {PROVIDER_SHAPE}"
        ));
    }
    if model.is_empty() {
        return blocking(format!(
            "provider.model is missing. Expected {PROVIDER_SHAPE}"
        ));
    }
    if catalog.options.is_empty() {
        return catalog.resources_are_complete.then(|| Issue {
            message: format!("this workspace has no AI provider resources, so \"{resource}\" cannot resolve and the step cannot run. Tell the user to create an AI provider resource (Anthropic, OpenAI, ...) and rewrite the step against it."),
            blocking: false,
            settled: true,
        });
    }
    let Some(found) = catalog.options.iter().find(|o| o.resource_ref == resource) else {
        return Some(Issue {
            message: format!(
                "provider.resource \"{resource}\" is not one of this workspace's AI provider resources"
            ),
            blocking: catalog.resources_are_complete,
            settled: false,
        });
    };
    if found.kind != kind {
        return blocking(format!(
            "provider.kind \"{kind}\" does not match \"{resource}\", which is a `{}` resource",
            found.kind
        ));
    }
    // An OpenAI resource lists its chat models, which says nothing about its decision model.
    if step_type == "aidecision" && kind == "openai" {
        return (model != OPENAI_DECISION_MODEL).then(|| Issue {
            message: format!("model \"{model}\" is not the model OpenAI's Decisions API is known to serve, \"{OPENAI_DECISION_MODEL}\""),
            blocking: false,
            settled: false,
        });
    }
    if found.models_are_live && !found.models.ids.iter().any(|id| id == model) {
        return Some(Issue {
            message: format!("model \"{model}\" is not in the model listing of \"{resource}\""),
            blocking: rules_out_other_models(found),
            settled: false,
        });
    }
    None
}

fn describe_option(option: &ProviderOption) -> String {
    let shown = &option.models.ids[..option.models.ids.len().min(MAX_PROMPTED_MODELS)];
    let more = if option.models.ids.len() > shown.len() {
        format!(", ... ({} more)", option.models.ids.len() - shown.len())
    } else {
        String::new()
    };
    let caveat = if !option.models_are_live {
        " (the endpoint would not list its models, so these are a guess — confirm the id with the user)"
    } else if !rules_out_other_models(option) {
        " (the endpoint may also serve model names this list does not show)"
    } else {
        ""
    };
    let models = if shown.is_empty() {
        "none listed".to_string()
    } else {
        format!("{}{more}{caveat}", shown.join(", "))
    };
    format!(
        "- `{}` (kind `{}`) — models: {models}",
        option.resource_ref, option.kind
    )
}

/// Refuse a provider config that would fail at run time, and report in `warnings` what the
/// catalog could not establish, so an incomplete catalog never rejects a provider that works.
pub(super) fn validate(
    modules: &Value,
    catalog: Option<&Catalog>,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    let empty = Catalog::default();
    let catalog = catalog.unwrap_or(&empty);
    let mut errors = Vec::new();
    for_each_ai_module(modules, &mut |module, value| {
        let Some(transform) = static_provider(value) else {
            return;
        };
        let step_type = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(issue) = check_provider(transform.get("value"), catalog, step_type) else {
            return;
        };
        let id = module.get("id").and_then(Value::as_str).unwrap_or_default();
        if issue.blocking {
            errors.push(format!("Step \"{id}\": {}", issue.message));
        } else if issue.settled {
            warnings.push(format!("Step \"{id}\": {}", issue.message));
        } else {
            warnings.push(format!("Step \"{id}\": {}, which could not be ruled out. Confirm it with the user if the step fails to run.", issue.message));
        }
    });
    if errors.is_empty() {
        return Ok(());
    }
    let hint = if catalog.options.is_empty() {
        String::new()
    } else {
        format!(
            "\nAI provider resources in this workspace:\n{}",
            catalog
                .options
                .iter()
                .map(describe_option)
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    Err(format!(
        "Invalid AI agent provider configuration:\n{}{hint}",
        errors.join("\n")
    ))
}

/// The tool-result suffix for provider findings that did not block the write.
pub(super) fn format_warnings(warnings: &[String]) -> String {
    if warnings.is_empty() {
        return String::new();
    }
    format!("\n\nAI agent provider warnings:\n{}", warnings.join("\n"))
}
