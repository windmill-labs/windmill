//! Lookups that touch no draft: hub, docs, npm, resource types, workers, folders.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use futures::future::join_all;
use serde_json::{json, Map, Value};
use windmill_common::utils::HTTP_CLIENT;

use super::chat::Effect;
use super::workspace::api_message;
use super::{
    array, enc, is_not_found, js_head, js_len, opt_str_arg, pretty, str_arg, truthy, Ctx, ToolResult,
    SCHEMAS,
};

pub(super) const TOOLS: &[&str] = &[
    "search_hub_scripts",
    "get_hub_integration",
    "search_npm_packages",
    "search_docs",
    "read_docs_page",
    "search_resource_types",
    "create_resource_type",
    "list_workers",
    "create_folder",
    "get_trigger_schema",
    "get_schedule_schema",
];

pub(super) async fn call(ctx: &Ctx<'_>, name: &str, args: &Value) -> ToolResult {
    match name {
        "search_hub_scripts" => search_hub_scripts(ctx, args).await,
        "get_hub_integration" => get_hub_integration(ctx, args).await,
        "search_npm_packages" => search_npm_packages(ctx, args).await,
        "search_docs" => search_docs(ctx, args).await,
        "read_docs_page" => read_docs_page(ctx, args).await,
        "search_resource_types" => search_resource_types(ctx, args).await,
        "create_resource_type" => create_resource_type(ctx, args).await,
        "list_workers" => list_workers(ctx).await,
        "create_folder" => create_folder(ctx, args).await,
        "get_trigger_schema" => get_trigger_schema(args),
        "get_schedule_schema" => Ok(SCHEMAS.schedule.clone()),
        _ => unreachable!("{name} is not a misc tool"),
    }
}

fn get_trigger_schema(args: &Value) -> ToolResult {
    let kind = str_arg(args, "kind")?;
    SCHEMAS
        .trigger
        .get(kind)
        .cloned()
        .ok_or_else(|| format!("Unknown trigger kind `{kind}`"))
}

/// Copies `keys` from `from` into `to`, each only when truthy.
fn copy_truthy(to: &mut Map<String, Value>, from: &Value, keys: &[&str]) {
    for key in keys {
        if truthy(from.get(key)) {
            to.insert(key.to_string(), from[key].clone());
        }
    }
}

/// Copies `keys` from `from` into `to`, each only when present (`null` included).
fn copy_present(to: &mut Map<String, Value>, from: &Value, keys: &[&str]) {
    for key in keys {
        if let Some(value) = from.get(key) {
            to.insert(key.to_string(), value.clone());
        }
    }
}

fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
}

// ---------------------------------------------------------------------------
// Hub
// ---------------------------------------------------------------------------

const MAX_BROWSED_HUB_SCRIPTS: usize = 20;
const MAX_SUGGESTED_INTEGRATIONS: usize = 5;
const MAX_INTEGRATION_EXAMPLES: usize = 5;

/// An unreachable hub degrades to "no suggestions" rather than a tool error.
async fn hub_integrations(ctx: &Ctx<'_>) -> Vec<Value> {
    match ctx.get("/integrations/hub/list?kind=script").await {
        Ok(Value::Array(integrations)) => integrations,
        _ => Vec::new(),
    }
}

/// The frontend's `RESOURCE_TYPE_WORDS`: how a slug word reads where capitalizing gets it
/// wrong, some into several words (`gcloud` -> `Google Cloud`).
static DISPLAY_WORDS: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("gen/display_words.json"))
        .expect("gen/display_words.json is valid")
});

/// What the frontend's `integrationDisplayName` calls a slug the hub names nothing.
fn slug_display_name(slug: &str) -> String {
    slug.split(['_', '-'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            DISPLAY_WORDS
                .get(&w.to_ascii_lowercase())
                .cloned()
                .unwrap_or_else(|| w.to_string())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A token matches a whole word of the slug or display name, or extends one from four
/// characters on: a bare substring test makes `for` hit sales*for*ce.
fn token_matches_integration(token: &str, integration: &Value) -> bool {
    let name = integration.get("name").and_then(Value::as_str);
    let display_name = integration
        .get("display_name")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| name.map(slug_display_name));
    let words: Vec<String> = [name, display_name.as_deref()]
        .into_iter()
        .flatten()
        .flat_map(words)
        .collect();
    if words.iter().any(|w| w == token) {
        return true;
    }
    token.len() >= 4
        && words
            .iter()
            .any(|w| w.len() >= 4 && (w.starts_with(token) || token.starts_with(w.as_str())))
}

/// Ranked by how many query words each integration accounts for, so "google drive"
/// puts `gdrive` ahead of every other `g*` integration matching on "google" alone.
fn suggest_hub_integrations(integrations: &[Value], query: &str) -> Vec<String> {
    let tokens: Vec<String> = words(query).filter(|t| t.len() >= 2).collect();
    let mut ranked: Vec<(usize, &str)> = integrations
        .iter()
        .filter_map(|integration| {
            let matched = tokens
                .iter()
                .filter(|t| token_matches_integration(t, integration))
                .count();
            let name = integration.get("name")?.as_str()?;
            (matched > 0).then_some((matched, name))
        })
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0));
    ranked
        .into_iter()
        .take(MAX_SUGGESTED_INTEGRATIONS)
        .map(|(_, name)| name.to_string())
        .collect()
}

/// The hub resolves a script by its version id alone; the other segments are descriptive.
fn hub_script_path(script: &Value) -> String {
    let summary = script
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();
    format!(
        "hub/{}/{}/{}",
        script.get("version_id").unwrap_or(&Value::Null),
        script
            .get("app")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        summary.split_whitespace().collect::<Vec<_>>().join("_")
    )
}

async fn search_hub_scripts(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let query = opt_str_arg(args, "query").filter(|s| !s.is_empty());
    let app = opt_str_arg(args, "integration").filter(|s| !s.is_empty());
    if query.is_none() && app.is_none() {
        return Ok(
            "Pass query (what the script should do), integration (a slug to list), or both."
                .to_string(),
        );
    }
    let app_param = app
        .map(|app| format!("&app={}", enc(app)))
        .unwrap_or_default();
    // Listing an integration goes through the top-scripts endpoint: it applies no
    // similarity floor, so near-misses worth reading as examples survive.
    let scripts = match query {
        Some(query) => {
            ctx.get(&format!(
                "/embeddings/query_hub_scripts?text={}&kind=script{app_param}",
                enc(query)
            ))
            .await?
        }
        None => ctx
            .get(&format!(
                "/scripts/hub/top?limit={MAX_BROWSED_HUB_SCRIPTS}&kind=script{app_param}"
            ))
            .await?
            .get("asks")
            .cloned()
            .unwrap_or_default(),
    };
    let scripts = array(Some(&scripts));
    let integrations = hub_integrations(ctx).await;

    if scripts.is_empty() {
        let subject = [app, query]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        return Ok(json!({
            "results": [],
            "suggested_integrations": suggest_hub_integrations(&integrations, &subject)
        })
        .to_string());
    }

    // Lowercased because a hub slug is case-sensitive and both casings can exist.
    let documented: HashSet<String> = integrations
        .iter()
        .filter(|i| i.get("documented") == Some(&Value::Bool(true)))
        .filter_map(|i| Some(i.get("name")?.as_str()?.to_lowercase()))
        .collect();
    let results: Vec<Value> = scripts
        .iter()
        .map(|script| {
            let mut result = Map::new();
            result.insert("path".to_string(), hub_script_path(script).into());
            copy_present(&mut result, script, &["summary"]);
            let app = script.get("app").cloned().unwrap_or_default();
            let is_documented =
                documented.contains(&app.as_str().unwrap_or_default().to_lowercase());
            result.insert("integration".to_string(), app);
            if is_documented {
                result.insert("documented".to_string(), true.into());
            }
            copy_truthy(&mut result, script, &["description"]);
            Value::Object(result)
        })
        .collect();
    Ok(json!({ "results": results }).to_string())
}

/// How the authored notes were checked is provenance for a human reviewing the hub;
/// the verdict and the fields it flags as shaky are what a caller can act on.
fn without_provenance(meta: &Value) -> Value {
    let Value::Object(meta) = meta else {
        return meta.clone();
    };
    let mut rest = meta.clone();
    let validation = rest.shift_remove("validation");
    let kept: Map<String, Value> = validation
        .as_ref()
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter(|(key, _)| *key == "status" || *key == "low_confidence_fields")
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    if !kept.is_empty() {
        rest.insert("validation".to_string(), Value::Object(kept));
    }
    Value::Object(rest)
}

async fn get_hub_integration(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let integration = str_arg(args, "integration")?;
    let doc = match ctx
        .get(&format!("/integrations/hub/{}/meta", enc(integration)))
        .await
    {
        Ok(doc) => doc,
        Err(e) => {
            // Only a 404 means the integration is absent. Reporting a timeout the same
            // way would teach the model that a real integration does not exist. No slug holds
            // a `/`, and the route refuses one as malformed rather than absent.
            let error = if is_not_found(&e) || integration.contains('/') {
                format!("No hub metadata for \"{integration}\".")
            } else {
                format!("Could not reach the hub for \"{integration}\"; it may still exist. Read its scripts instead, or try again.")
            };
            let integrations = hub_integrations(ctx).await;
            return Ok(json!({
                "error": error,
                "suggested_integrations": suggest_hub_integrations(&integrations, integration)
            })
            .to_string());
        }
    };

    // The hub owns this response's shape and can be older than this server, so every
    // section is optional: a hub that sends less returns less.
    let mut out = Map::new();
    if let Some(app) = doc.get("app") {
        out.insert("integration".to_string(), app.clone());
    }
    copy_present(&mut out, &doc, &["display_name"]);
    copy_truthy(&mut out, &doc, &["description", "docs_url"]);
    if truthy(doc.get("meta")) {
        out.insert(
            "verified_provider_notes".to_string(),
            without_provenance(&doc["meta"]),
        );
    }
    let derived = doc.get("derived").filter(|d| truthy(Some(d)));
    if let Some(derived) = derived {
        let mut observed = Map::new();
        let hosts: Vec<Value> = array(derived.get("api_hosts"))
            .iter()
            .filter_map(|h| h.get("host").cloned())
            .collect();
        observed.insert("api_hosts".to_string(), hosts.into());
        copy_present(&mut observed, derived, &["style"]);
        let languages: Vec<&String> = derived
            .get("languages")
            .and_then(Value::as_object)
            .map(|l| l.keys().collect())
            .unwrap_or_default();
        observed.insert("languages".to_string(), json!(languages));
        copy_present(&mut observed, derived, &["script_counts"]);
        out.insert("observed_from_scripts".to_string(), observed.into());
    }
    if doc.get("curated") == Some(&Value::Bool(true)) {
        out.insert(
            "scripts_note".to_string(),
            "These scripts were pruned to idiomatic actions rather than generated one per API endpoint, so they are worth following as style examples.".into(),
        );
    }
    let resource_types: Vec<Value> = array(doc.get("resource_types"))
        .iter()
        .map(|rt| {
            let mut entry = Map::new();
            copy_present(&mut entry, rt, &["name"]);
            copy_truthy(&mut entry, rt, &["description"]);
            // The hub keeps a schema in a text column and hands it back as a JSON string.
            match rt.get("schema") {
                Some(Value::String(schema)) => {
                    let parsed = serde_json::from_str(schema)
                        .unwrap_or_else(|_| Value::String(schema.clone()));
                    entry.insert("schema".to_string(), parsed);
                }
                Some(schema) => {
                    entry.insert("schema".to_string(), schema.clone());
                }
                None => {}
            }
            Value::Object(entry)
        })
        .collect();
    out.insert("resource_types".to_string(), resource_types.into());
    let examples: Vec<Value> = array(derived.and_then(|d| d.get("top_scripts")))
        .iter()
        .take(MAX_INTEGRATION_EXAMPLES)
        .map(|script| {
            let mut entry = Map::new();
            copy_present(&mut entry, script, &["path", "summary"]);
            copy_truthy(&mut entry, script, &["description", "language"]);
            Value::Object(entry)
        })
        .collect();
    out.insert("example_scripts".to_string(), examples.into());
    Ok(Value::Object(out).to_string())
}

// ---------------------------------------------------------------------------
// npm
// ---------------------------------------------------------------------------

/// The registry's best match is kept, and the runner-up only when it scores this share of it:
/// the scores have no fixed scale (a descriptive query's right answer can score a few hundred,
/// a package's own name thousands), so a match is judged against the best one.
const NPM_RUNNER_UP_SHARE: f64 = 0.6;
/// A package's readme and types are each capped at 1% of the model's context window. The
/// chat sends its own model's cap; an MCP client's model is unknown, so this is 1% of the
/// window assumed for an unknown model (128000).
const NPM_TEXT_LIMIT: usize = 1280;

async fn fetch(url: &str) -> Result<reqwest::Response, reqwest::Error> {
    HTTP_CLIENT.get(url).send().await
}

/// Where a package's files come from: the public CDN, or the workspace's npm proxy, which
/// is the authoritative source once the instance names a registry of its own. Each falls
/// back to the other.
struct NpmSources<'a> {
    ctx: &'a Ctx<'a>,
    proxy_first: bool,
}

impl NpmSources<'_> {
    async fn new<'a>(ctx: &'a Ctx<'a>) -> NpmSources<'a> {
        let proxy_first = ctx
            .get(&ctx.ws("npm_proxy/config"))
            .await
            .is_ok_and(|config| truthy(config.get("registry_configured")));
        NpmSources { ctx, proxy_first }
    }

    async fn json(&self, cdn: String, proxy: String) -> Option<Value> {
        let via_cdn = || async {
            let response = fetch(&cdn).await.ok()?.error_for_status().ok()?;
            response.json::<Value>().await.ok()
        };
        let via_proxy = || async { self.ctx.get(&self.ctx.ws(&proxy)).await.ok() };
        if self.proxy_first {
            match via_proxy().await {
                Some(v) => Some(v),
                None => via_cdn().await,
            }
        } else {
            match via_cdn().await {
                Some(v) => Some(v),
                None => via_proxy().await,
            }
        }
    }

    async fn text(&self, cdn: String, proxy: String) -> Option<String> {
        let via_cdn = || async {
            let response = fetch(&cdn).await.ok()?.error_for_status().ok()?;
            response.text().await.ok()
        };
        let via_proxy = || async {
            self.ctx
                .request("GET", &self.ctx.ws(&proxy), None)
                .await
                .ok()
        };
        if self.proxy_first {
            match via_proxy().await {
                Some(v) => Some(v),
                None => via_cdn().await,
            }
        } else {
            match via_cdn().await {
                Some(v) => Some(v),
                None => via_proxy().await,
            }
        }
    }

    /// `tag` as an exact version; one with two dots already is one.
    async fn version(&self, module: &str, tag: &str) -> Option<String> {
        if tag.split('.').count() >= 3 {
            return Some(tag.to_string());
        }
        let resolved = self
            .json(
                format!("https://data.jsdelivr.com/v1/package/resolve/npm/{module}@{tag}"),
                format!("npm_proxy/resolve/{}?tag={}", enc(module), enc(tag)),
            )
            .await?;
        resolved.get("version")?.as_str().map(String::from)
    }

    /// The `.d.ts` files of `module` at `version`, in the order its file tree lists them.
    async fn dts_files(&self, module: &str, version: &str) -> Option<Vec<String>> {
        let tree = self
            .json(
                format!("https://data.jsdelivr.com/v1/package/npm/{module}@{version}/flat"),
                format!("npm_proxy/filetree/{}/{}", enc(module), enc(version)),
            )
            .await?;
        Some(
            array(tree.get("files"))
                .iter()
                .filter_map(|f| f.get("name")?.as_str())
                .filter(|name| name.ends_with(".d.ts"))
                .map(String::from)
                .collect(),
        )
    }
}

/// `@bla/foo` → `bla__foo`, the DefinitelyTyped name of a scoped package.
fn dt_name(name: &str) -> String {
    match name.strip_prefix('@') {
        Some(scoped) if scoped.contains('/') => scoped.replacen('/', "__", 1),
        _ => name.to_string(),
    }
}

/// Every declaration file of the package, or of its `@types` package when it ships none,
/// each headed by the path it would have under `node_modules`.
/// Files fetched at once, as the browser's type acquisition did.
const NPM_TYPES_CONCURRENCY: usize = 6;

/// The package's declarations joined in order, fetched only until they reach `limit`
/// characters: only that head is returned, and `@types/lodash` alone has hundreds of files.
async fn npm_package_types(
    sources: &NpmSources<'_>,
    name: &str,
    version: &str,
    limit: usize,
) -> String {
    let mut module = name.to_string();
    let mut prefix = format!("/node_modules/{name}");
    let mut version = match sources.version(name, version).await {
        Some(version) => version,
        None => return String::new(),
    };
    let mut files = sources.dts_files(name, &version).await.unwrap_or_default();
    if files.is_empty() {
        module = format!("@types/{}", dt_name(name));
        prefix = format!("/node_modules/@types/{}", dt_name(name));
        let Some(dt_version) = sources.version(&module, "latest").await else {
            return String::new();
        };
        version = dt_version;
        files = sources
            .dts_files(&module, &version)
            .await
            .unwrap_or_default();
    }
    let mut parts: Vec<String> = Vec::new();
    for batch in files.chunks(NPM_TYPES_CONCURRENCY) {
        let contents = join_all(batch.iter().map(|file| {
            sources.text(
                format!("https://cdn.jsdelivr.net/npm/{module}@{version}{file}"),
                format!("npm_proxy/file/{}/{}{file}", enc(&module), enc(&version)),
            )
        }))
        .await;
        parts.extend(
            batch
                .iter()
                .zip(contents)
                .filter_map(|(file, code)| Some(format!("// {prefix}{file}\n{}", code?))),
        );
        if js_len(&parts.join("\n\n")) >= limit {
            break;
        }
    }
    parts.join("\n\n")
}

fn confident_npm_matches(objects: &[Value]) -> Vec<&Value> {
    let score = |o: &Value| o.get("searchScore").and_then(Value::as_f64).unwrap_or(0.0);
    let best = objects.iter().map(score).fold(0.0, f64::max);
    objects
        .iter()
        .filter(|o| score(o) >= best * NPM_RUNNER_UP_SHARE)
        .collect()
}

async fn npm_search(ctx: &Ctx<'_>, query: &str, limit: usize) -> Result<Vec<Value>, String> {
    let data: Value = fetch(&format!(
        "https://registry.npmjs.org/-/v1/search?text={}&size=2",
        enc(query)
    ))
    .await
    .map_err(|e| e.to_string())?
    .json()
    .await
    .map_err(|e| e.to_string())?;
    let objects = data
        .get("objects")
        .and_then(Value::as_array)
        .ok_or("no `objects` in the registry answer")?;
    let packages = confident_npm_matches(objects)
        .into_iter()
        .filter_map(|o| {
            let package = o.get("package")?;
            Some((
                package.get("name")?.as_str()?,
                package.get("version").and_then(Value::as_str),
            ))
        });
    let sources = &NpmSources::new(ctx).await;
    Ok(join_all(packages.map(|(name, version)| async move {
        let documentation = match fetch(&format!("https://unpkg.com/{name}/readme.md")).await {
            Ok(response) => response.text().await.unwrap_or_default(),
            Err(_) => String::new(),
        };
        let types = npm_package_types(sources, name, version.unwrap_or("latest"), limit).await;
        json!({
            "package": name,
            "documentation": js_head(&documentation, limit),
            "types": js_head(&types, limit),
        })
    }))
    .await)
}

async fn search_npm_packages(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let query = str_arg(args, "query")?;
    let limit = ctx
        .chat
        .and_then(|chat| chat.npm_text_limit)
        .unwrap_or(NPM_TEXT_LIMIT);
    Ok(match npm_search(ctx, query, limit).await {
        Ok(results) => Value::Array(results).to_string(),
        Err(e) => {
            tracing::warn!("npm package search failed: {e}");
            "Error searching external integration resources".to_string()
        }
    })
}

// ---------------------------------------------------------------------------
// Docs
// ---------------------------------------------------------------------------

/// `text` of a `/docs/*` answer, or the session's wording for a failed lookup.
fn docs_text(result: Result<Value, String>, action: &str) -> String {
    match result {
        Ok(response) => response
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        Err(e) => format!(
            "Failed to {action}: {}, pursuing with the user request...",
            api_message(&e)
        ),
    }
}

async fn search_docs(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let query = opt_str_arg(args, "query").unwrap_or_default().trim();
    if query.is_empty() {
        return Ok(
            "No search query was provided. Provide a `query` of one or more keywords.".to_string(),
        );
    }
    let result = ctx.get(&format!("/docs/search?query={}", enc(query))).await;
    if let Ok(response) = &result {
        let mut facts = Map::new();
        facts.insert(
            "result_count".into(),
            array(response.get("results")).len().into(),
        );
        ctx.effect(Effect::Facts(facts));
    }
    Ok(docs_text(result, "search documentation"))
}

async fn read_docs_page(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let url = opt_str_arg(args, "url").unwrap_or_default();
    if url.trim().is_empty() {
        return Ok("No documentation page URL was provided. Provide a `url` — e.g. a `Source` URL returned by search_docs.".to_string());
    }
    let section = opt_str_arg(args, "section")
        .filter(|s| !s.trim().is_empty())
        .map(|s| format!("&section={}", enc(s)))
        .unwrap_or_default();
    let result = ctx
        .get(&format!("/docs/page?url={}{section}", enc(url)))
        .await;
    Ok(docs_text(result, "read documentation page"))
}

// ---------------------------------------------------------------------------
// Resource types
// ---------------------------------------------------------------------------

/// The hub publishes hundreds of resource types and an instance seeds only a few of its
/// own, so fewer than this means the instance never synced with the hub.
const UNSYNCED_RESOURCE_TYPE_COUNT: usize = 20;

const HUB_SYNC_INSTRUCTIONS: &str = "Tell the user: no resource of a missing type can be created until a superadmin syncs resource types from the hub, with \"Sync resource types with Hub\" in the Add resource dialog of the Resources page.";

async fn search_resource_types(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let query = str_arg(args, "query")?;
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(5);
    let search_route = ctx.ws(&format!(
        "embeddings/query_resource_types?text={}&limit={limit}",
        enc(query)
    ));
    let names_route = ctx.ws("resources/type/listnames");
    let (results, names) =
        futures::future::join(ctx.get(&search_route), ctx.get(&names_route)).await;
    let results = results?;
    let results = array(Some(&results));
    let count = names
        .map(|names| {
            array(Some(&names))
                .iter()
                .filter_map(Value::as_str)
                .collect::<HashSet<_>>()
                .len()
        })
        .unwrap_or(0);
    let listing: Vec<Value> = results
        .iter()
        .map(|rt| {
            let mut entry = Map::new();
            copy_present(&mut entry, rt, &["name", "schema"]);
            Value::Object(entry)
        })
        .collect();
    let listing = pretty(&Value::Array(listing));
    // The similarity search still returns the nearest of the few types there are, so the
    // note cannot wait for an empty result; it replaces the "no dedicated type" one, which
    // would have the model create a duplicate of a type a sync brings.
    if count > 0 && count < UNSYNCED_RESOURCE_TYPE_COUNT {
        return Ok(format!("{listing}\n\nOnly {count} resource types exist on this instance, so it has most likely never synced with the Windmill Hub, which has types for most services. {HUB_SYNC_INSTRUCTIONS}"));
    }
    // The search is semantic and always returns its closest types, so a miss reads
    // like a hit unless the absence is spelled out.
    let words: Vec<String> = words(query).filter(|w| w.len() >= 3).collect();
    let named = results.iter().any(|rt| {
        let name = rt
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_lowercase();
        words.iter().any(|w| name.contains(w.as_str()))
    });
    Ok(if named || words.is_empty() {
        listing
    } else {
        format!("{listing}\n\nNo resource type name contains \"{query}\": these are only the nearest matches, so no dedicated type exists. Searching again with synonyms will not find one — use a fitting generic type above, or create a custom c_<name> type.")
    })
}

/// The prefix the Add resource type form adds: a hub type of the same name, synced later,
/// would otherwise clash with this one.
fn custom_resource_type_name(name: &str) -> String {
    if name.starts_with("c_") {
        name.to_string()
    } else {
        format!("c_{name}")
    }
}

async fn create_resource_type(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let name = custom_resource_type_name(str_arg(args, "name")?);
    let description = str_arg(args, "description")?;
    let schema = args
        .get("schema")
        .filter(|s| s.is_object())
        .ok_or("Missing required object argument `schema`")?;
    ctx.post(
        &ctx.ws("resources/type/create"),
        json!({ "name": name, "description": description, "schema": schema }),
    )
    .await?;
    Ok(json!({
        "success": true,
        "message": format!("Created resource type {name}. Refer to it by this name.")
    })
    .to_string())
}

// ---------------------------------------------------------------------------
// Workers
// ---------------------------------------------------------------------------

const WORKER_PAGE_SIZE: usize = 100;
// `GET /workers/list` hides workers from a caller without the devops role by returning
// an empty list, not an error, when HIDE_WORKERS_FOR_NON_ADMINS is set. The flag is not
// visible here, so absence is only provable for a devops or superadmin caller.
const NO_WORKERS_VISIBLE_MESSAGE: &str = "No workers came back. This does NOT establish that no workers are running: an instance can hide workers from callers without the devops role, and it does so by returning an empty list rather than an error. Tell the user you cannot see any workers and that worker visibility may be restricted for your account, and suggest they check the Workers page themselves if they can open it, or ask a workspace admin otherwise. Never state that no workers are online or that the instance has none.";
const NO_WORKERS_CONNECTED_MESSAGE: &str = "No workers are connected to this instance (none pinged in the last 5 minutes). Queued runs will stay queued until a worker starts.";

async fn list_workers(ctx: &Ctx<'_>) -> ToolResult {
    let pings = ctx
        .get(&format!("/workers/list?per_page={WORKER_PAGE_SIZE}"))
        .await?;
    let pings = array(Some(&pings));
    if pings.is_empty() {
        let me = ctx.get("/users/whoami").await.unwrap_or_default();
        let sees_all = truthy(me.get("super_admin")) || truthy(me.get("devops"));
        return Ok(if sees_all {
            NO_WORKERS_CONNECTED_MESSAGE
        } else {
            NO_WORKERS_VISIBLE_MESSAGE
        }
        .to_string());
    }
    let workers: Vec<Value> = pings
        .iter()
        .map(|ping| {
            let mut worker = Map::new();
            copy_present(
                &mut worker,
                ping,
                &[
                    "worker",
                    "worker_group",
                    "custom_tags",
                    "last_ping",
                    "jobs_executed",
                ],
            );
            Value::Object(worker)
        })
        .collect();
    let mut out = Map::new();
    let listed = workers.len();
    out.insert("workers".to_string(), workers.into());
    if listed == WORKER_PAGE_SIZE {
        out.insert(
            "note".to_string(),
            format!("Only the first {WORKER_PAGE_SIZE} workers are listed; more may be connected.")
                .into(),
        );
    }
    Ok(pretty(&Value::Object(out)))
}

// ---------------------------------------------------------------------------
// Folders
// ---------------------------------------------------------------------------

async fn create_folder(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let name = str_arg(args, "name")?;
    // Same rule as the API's own check, so an invalid name gets this wording.
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Ok(json!({
            "success": false,
            "error": "Folder name can only contain alphanumeric characters, underscores, and hyphens."
        })
        .to_string());
    }
    let mut body = Map::new();
    body.insert("name".to_string(), name.into());
    copy_present(&mut body, args, &["summary"]);
    Ok(
        match ctx.post(&ctx.ws("folders/create"), body.into()).await {
            Ok(_) => json!({
                "success": true,
                "message": format!("Created folder `f/{name}`. You can now write items to `f/{name}/<name>`.")
            }),
            Err(error) => json!({ "success": false, "error": api_message(&error) }),
        }
        .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npm_matches_are_judged_against_the_best_one() {
        let names = |scores: &[f64]| {
            let objects: Vec<Value> = scores
                .iter()
                .enumerate()
                .map(|(i, s)| json!({ "searchScore": s, "package": { "name": i.to_string() } }))
                .collect();
            confident_npm_matches(&objects)
                .iter()
                .map(|o| o["package"]["name"].as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        };
        // A descriptive query's right answer scores a few hundred; a weak runner-up is dropped.
        assert_eq!(names(&[815.0, 454.0]), ["0"]);
        assert_eq!(names(&[419.0, 384.0]), ["0", "1"]);
        assert!(names(&[]).is_empty());
    }

    #[test]
    fn suggestions_rank_by_matched_words() {
        let integrations = vec![
            json!({ "name": "gcal", "display_name": "Google Calendar" }),
            json!({ "name": "gdrive", "display_name": "Google Drive" }),
            json!({ "name": "salesforce", "display_name": null }),
            json!({ "name": "s3", "display_name": "S3" }),
        ];
        assert_eq!(
            suggest_hub_integrations(&integrations, "google drive"),
            ["gdrive", "gcal"]
        );
        // A short token must equal a word; a long one may extend it.
        assert!(suggest_hub_integrations(&integrations, "for").is_empty());
        assert_eq!(
            suggest_hub_integrations(&integrations, "sales"),
            ["salesforce"]
        );
        assert_eq!(suggest_hub_integrations(&integrations, "s3 bucket"), ["s3"]);
        // With no display name from the hub, the slug reads as the frontend labels it.
        let unnamed = vec![json!({ "name": "gcloud", "display_name": null })];
        assert_eq!(
            suggest_hub_integrations(&unnamed, "deploy to google cloud"),
            ["gcloud"]
        );
    }

    #[test]
    fn provenance_keeps_only_the_verdict() {
        let meta = json!({
            "pagination": "cursor",
            "validation": { "status": "verified", "method": "live", "low_confidence_fields": ["a"] }
        });
        assert_eq!(
            without_provenance(&meta),
            json!({
                "pagination": "cursor",
                "validation": { "status": "verified", "low_confidence_fields": ["a"] }
            })
        );
        assert_eq!(
            without_provenance(&json!({ "a": 1, "validation": { "method": "live" } })),
            json!({ "a": 1 })
        );
    }

    #[test]
    fn custom_resource_types_carry_the_c_prefix_once() {
        assert_eq!(custom_resource_type_name("acme"), "c_acme");
        assert_eq!(custom_resource_type_name("c_acme"), "c_acme");
    }

    #[test]
    fn hub_path_from_summary() {
        let script = json!({ "version_id": 28479, "app": "stripe", "summary": "Cancel Or  Reverse a Payout" });
        assert_eq!(
            hub_script_path(&script),
            "hub/28479/stripe/cancel_or_reverse_a_payout"
        );
    }
}
