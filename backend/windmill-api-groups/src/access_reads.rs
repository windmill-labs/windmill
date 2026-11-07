/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! What an item reads when it runs, and who performs each read.
//!
//! Each read is credited to the identity the runtime performs it as (`docs/access-reads.md`):
//! a change to who a flow step, an app runnable or a task runs as must be mirrored in [`tag`].

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use axum::{
    extract::{Extension, Path},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use windmill_api_auth::{check_scopes, ApiAuthed};
use windmill_api_scripts::body_parse::{
    asset_parser::{AssetKind, ParseAssetsOutput},
    item_reference::{
        scan_input_transform, scan_sdk_calls, ItemKind, ItemReference, ReferenceAccess,
    },
    parse_app_file, parse_body,
};
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    scripts::{to_i64, ScriptLang},
};

/// How many steps, SDK runs and agents deep the scan follows. A resource link adds no level.
const MAX_DEPTH: usize = 4;
/// How many items the scan reads besides the scanned one.
const MAX_NODES: usize = 500;
/// Code past this size is scanned lexically: its AST would cost memory in proportion.
const MAX_PARSED: usize = 1 << 20;

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum RootKind {
    Script,
    Flow,
    App,
}

impl RootKind {
    fn as_str(self) -> &'static str {
        match self {
            RootKind::Script => "script",
            RootKind::Flow => "flow",
            RootKind::App => "app",
        }
    }
}

#[derive(Deserialize)]
pub struct ReadsRequest {
    kind: RootKind,
    /// The saved item to scan or, with `source`, the path that labels it.
    path: Option<String>,
    /// Unsaved editor content, scanned in place of the saved item.
    source: Option<ReadsSource>,
}

#[derive(Deserialize)]
pub struct ReadsSource {
    language: Option<String>,
    content: Option<String>,
    modules: Option<Value>,
    value: Option<Value>,
    schema: Option<Value>,
    on_behalf_of: Option<String>,
    policy: Option<Value>,
}

/// Who performs a read.
#[derive(Serialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Reader {
    /// Whoever opens the app: a full-code app's frontend calls the API with their session.
    Viewer,
    /// Whoever the scanned item runs as, which `runs_as` names.
    Root,
    /// The fixed identity of a runnable the item reaches.
    Account { principal: String },
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RunsAs {
    Caller,
    /// `principal` is missing for an app whose policy names no account.
    Account {
        #[serde(skip_serializing_if = "Option::is_none")]
        principal: Option<String>,
    },
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    Read,
    Write,
    /// Started through the run API, which takes read on the item.
    Run,
}

#[derive(Serialize, PartialEq, Debug)]
pub struct Reference {
    kind: ItemKind,
    path: String,
    access: Access,
    reader: Reader,
    sources: Vec<String>,
}

#[derive(Serialize, PartialEq, Debug)]
pub struct OtherAsset {
    kind: AssetKind,
    path: String,
    reader: Reader,
    sources: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct ReadsReport {
    runs_as: RunsAs,
    references: Vec<Reference>,
    others: Vec<OtherAsset>,
    errors: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum NodeKey {
    /// The scanned item.
    Root,
    /// A script called by path runs its latest deployed version, unless the step pins a hash.
    Script {
        path: String,
        hash: Option<i64>,
    },
    Flow(String),
    Resource(String),
    /// An `ai_agent` resource, whose tools run like the steps of a flow.
    Agent(String),
}

impl NodeKey {
    fn path(&self) -> &str {
        match self {
            NodeKey::Root => "",
            NodeKey::Script { path, .. }
            | NodeKey::Flow(path)
            | NodeKey::Resource(path)
            | NodeKey::Agent(path) => path,
        }
    }
}

#[derive(Debug)]
enum Fetched {
    Node(Node),
    /// Missing, hidden by RLS, outside the token's scopes, or a pinned hash of another path.
    Unreadable,
    TooDeep,
    OverBudget,
}

#[derive(Default, Debug)]
struct Node {
    /// The identity the item runs as, whoever starts it.
    on_behalf_of: Option<String>,
    refs: Vec<LocalRef>,
    edges: Vec<Edge>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Who {
    /// Whoever runs this node.
    Node,
    /// Whoever opens the app, in every execution mode.
    Viewer,
    /// Whoever runs the target of `edges[i]`: a path step's static inputs are resolved by the
    /// step's own job.
    Edge(usize),
}

#[derive(Debug)]
enum Target {
    Item { kind: ItemKind, path: String, access: Access },
    Asset { kind: AssetKind, path: String },
}

#[derive(Debug)]
struct LocalRef {
    target: Target,
    who: Who,
    source: String,
}

#[derive(Debug)]
struct Edge {
    target: NodeKey,
    /// A flow step, an AI tool, an SDK run and a task run as their target's own `on_behalf_of`
    /// when it has one. An app runnable runs as the app whatever its target says.
    keep_obo: bool,
    who: Who,
}

impl Node {
    fn add(&mut self, target: Target, who: Who, source: &str) {
        self.refs
            .push(LocalRef { target, who, source: source.to_string() });
    }

    fn reference(&mut self, reference: ItemReference, who: Who, source: &str) {
        let ItemReference { kind, path, access } = reference;
        let access = match access {
            ReferenceAccess::Read => {
                // Its `$res:` and `$var:` links are resolved as whoever reads it.
                if kind == ItemKind::Resource {
                    self.edges.push(Edge {
                        target: NodeKey::Resource(path.clone()),
                        keep_obo: false,
                        who,
                    });
                }
                Access::Read
            }
            ReferenceAccess::Write => Access::Write,
            ReferenceAccess::Run | ReferenceAccess::Task => {
                let target = match kind {
                    ItemKind::Script => NodeKey::Script { path: path.clone(), hash: None },
                    ItemKind::Flow => NodeKey::Flow(path.clone()),
                    ItemKind::Resource | ItemKind::Variable => return,
                };
                self.edges.push(Edge { target, keep_obo: true, who });
                // A task is dispatched by the job itself and takes no read on its target.
                if access == ReferenceAccess::Task {
                    return;
                }
                Access::Run
            }
        };
        self.add(Target::Item { kind, path, access }, who, source);
    }

    fn agent(&mut self, reference: &str, who: Who, source: &str) {
        let Some(agent) = ItemReference::of(ItemKind::Resource, ReferenceAccess::Read, reference)
        else {
            return;
        };
        self.edges
            .push(Edge { target: NodeKey::Agent(agent.path.clone()), keep_obo: false, who });
        self.add(
            Target::Item { kind: ItemKind::Resource, path: agent.path, access: Access::Read },
            who,
            source,
        );
    }

    fn code(&mut self, language: Option<ScriptLang>, content: &str, who: Who, source: &str) {
        let output = match language {
            Some(language) if content.len() <= MAX_PARSED => parse_body(&language, content).output,
            _ => ParseAssetsOutput { references: scan_sdk_calls(content), ..Default::default() },
        };
        self.parsed(output, who, source);
    }

    fn parsed(&mut self, output: ParseAssetsOutput, who: Who, source: &str) {
        for reference in output.references {
            self.reference(reference, who, source);
        }
        for asset in output.assets {
            // A resource asset's access type is what the code does to the data behind it.
            // Reaching that data takes read on the resource either way.
            let resource = (asset.kind == AssetKind::Resource)
                .then(|| ItemReference::of(ItemKind::Resource, ReferenceAccess::Read, &asset.path))
                .flatten();
            match resource {
                Some(reference) => self.reference(reference, who, source),
                None => self.add(
                    Target::Asset { kind: asset.kind, path: asset.path },
                    who,
                    source,
                ),
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Ctx {
    /// A flow's value or an agent's: steps, AI tools and input transforms.
    Flow,
    /// A low-code or full-code app's value: runnables.
    App,
    /// A resource's value or a schema, where only links are read.
    Data,
}

fn walk(node: &mut Node, item: &str, value: &Value, ctx: Ctx, who: Who, label: &str) {
    match value {
        Value::String(s) => {
            if let Some(reference) = json_link(s) {
                node.reference(reference, who, label);
            }
        }
        Value::Array(values) => {
            for v in values {
                walk(node, item, v, ctx, who, label);
            }
        }
        Value::Object(o) if ctx == Ctx::Data => {
            for v in o.values() {
                walk(node, item, v, ctx, who, label);
            }
        }
        Value::Object(o) => walk_object(node, item, o, ctx, who, label),
        _ => {}
    }
}

/// Read through serde, which names every language: `ScriptLang`'s `FromStr` has no `duckdb`.
fn script_lang(name: &str) -> Option<ScriptLang> {
    serde_json::from_value(Value::String(name.to_string())).ok()
}

/// What `transform_json_value` resolves in a job's arguments. `res://` and `var://` are SDK
/// syntax, which no JSON value is resolved from.
fn json_link(s: &str) -> Option<ItemReference> {
    ["$res:", "$var:", "$jsonvar:"]
        .iter()
        .any(|prefix| s.starts_with(prefix))
        .then(|| ItemReference::of_prefixed(s))
        .flatten()
}

fn walk_object(
    node: &mut Node,
    item: &str,
    o: &Map<String, Value>,
    ctx: Ctx,
    who: Who,
    label: &str,
) {
    let str_of = |key: &str| o.get(key).and_then(Value::as_str);
    let named = ["summary", "name", "id"]
        .iter()
        .find_map(|key| str_of(key).filter(|name| !name.is_empty()))
        .map(|name| format!("{item} › {name}"));
    let label = named.as_deref().unwrap_or(label);

    // A `frontend` script runs in the viewer's browser and reaches no workspace item.
    let code = match (str_of("content"), str_of("language")) {
        (Some(content), Some(language)) if language != "frontend" => {
            node.code(script_lang(language), content, who, label);
            true
        }
        _ => false,
    };
    let mut static_who = who;
    let mut agent = false;
    match ctx {
        Ctx::Flow => {
            match (str_of("type"), str_of("path")) {
                (Some(kind @ ("script" | "flow")), Some(path)) if !path.starts_with("hub/") => {
                    let target = if kind == "script" {
                        let hash = str_of("hash").and_then(|hash| to_i64(hash).ok());
                        NodeKey::Script { path: path.to_string(), hash }
                    } else {
                        NodeKey::Flow(path.to_string())
                    };
                    static_who = Who::Edge(node.edges.len());
                    node.edges.push(Edge { target, keep_obo: true, who });
                }
                (Some("aiagent"), _) => {
                    if let Some(reference) = str_of("agent") {
                        node.agent(reference, who, label);
                        agent = true;
                    }
                }
                _ => {}
            }
            if str_of("tool_type") == Some("mcp") {
                if let Some(reference) = str_of("resource_path").and_then(|path| {
                    ItemReference::of(ItemKind::Resource, ReferenceAccess::Read, path)
                }) {
                    node.reference(reference, who, label);
                }
            }
        }
        Ctx::App => {
            let target = match (str_of("runType"), str_of("path")) {
                (Some("script"), Some(path)) => {
                    Some(NodeKey::Script { path: path.to_string(), hash: None })
                }
                (Some("flow"), Some(path)) => Some(NodeKey::Flow(path.to_string())),
                (Some("agent"), Some(path)) => {
                    node.agent(path, who, label);
                    None
                }
                _ => None,
            };
            if let Some(target) = target {
                node.edges.push(Edge { target, keep_obo: false, who });
            }
        }
        Ctx::Data => {}
    }

    for (key, v) in o {
        match (key.as_str(), v) {
            ("content", _) if code => {}
            // Followed as a plain resource link, the agent's tools would be walked as data,
            // crediting a tool's inputs to the flow's runner rather than to the tool's.
            ("agent", _) if agent => {}
            ("expr", Value::String(expr)) if ctx == Ctx::Flow => {
                for reference in scan_input_transform(expr) {
                    node.reference(reference, who, label);
                }
            }
            ("input_transforms", Value::Object(inputs)) if ctx == Ctx::Flow => {
                for transform in inputs.values() {
                    match (
                        transform.get("type").and_then(Value::as_str),
                        transform.get("value"),
                    ) {
                        (Some("static"), Some(v)) => {
                            walk(node, item, v, Ctx::Data, static_who, label)
                        }
                        _ => walk(node, item, transform, ctx, who, label),
                    }
                }
            }
            _ => walk(node, item, v, ctx, who, label),
        }
    }
}

fn script_node(
    item: &str,
    language: Option<ScriptLang>,
    content: &str,
    modules: Option<&Value>,
    schema: Option<&Value>,
    on_behalf_of: Option<String>,
) -> Node {
    let mut node = Node { on_behalf_of, ..Default::default() };
    node.code(language, content, Who::Node, item);
    // The script's other files, which its job runs too.
    for (name, module) in modules.and_then(Value::as_object).into_iter().flatten() {
        if let Some(content) = module.get("content").and_then(Value::as_str) {
            let language = module
                .get("language")
                .and_then(Value::as_str)
                .and_then(script_lang);
            node.code(language, content, Who::Node, &format!("{item} › {name}"));
        }
    }
    if let Some(schema) = schema {
        walk(&mut node, item, schema, Ctx::Data, Who::Node, item);
    }
    node
}

fn flow_node(
    item: &str,
    value: Option<&Value>,
    schema: Option<&Value>,
    on_behalf_of: Option<String>,
) -> Node {
    let mut node = Node { on_behalf_of, ..Default::default() };
    if let Some(value) = value {
        walk(&mut node, item, value, Ctx::Flow, Who::Node, item);
    }
    if let Some(schema) = schema {
        walk(&mut node, item, schema, Ctx::Data, Who::Node, item);
    }
    node
}

fn app_node(item: &str, value: &Value) -> Node {
    let mut node = Node::default();
    let Value::Object(o) = value else {
        return node;
    };
    for (key, v) in o {
        match (key.as_str(), v) {
            // A full-code app's frontend, which calls the API with the viewer's session.
            ("files", Value::Object(files)) => {
                for (name, content) in files {
                    let Some(output) = content.as_str().and_then(|c| parse_app_file(name, c))
                    else {
                        continue;
                    };
                    node.parsed(output, Who::Viewer, &format!("{item} › {name}"));
                }
            }
            _ => walk(&mut node, item, v, Ctx::App, Who::Node, item),
        }
    }
    node
}

fn resource_node(item: &str, value: Option<&Value>, ctx: Ctx) -> Node {
    let mut node = Node::default();
    if let Some(value) = value {
        walk(&mut node, item, value, ctx, Who::Node, item);
    }
    node
}

fn item_runs_as(on_behalf_of: Option<String>) -> RunsAs {
    match on_behalf_of {
        Some(principal) => RunsAs::Account { principal: Some(principal) },
        None => RunsAs::Caller,
    }
}

/// Every runnable of an app runs as its policy's `on_behalf_of`, except in `viewer` mode.
fn app_runs_as(policy: Option<&Value>) -> RunsAs {
    let field = |key: &str| policy.and_then(|p| p.get(key)).and_then(Value::as_str);
    if field("execution_mode") == Some("viewer") {
        RunsAs::Caller
    } else {
        RunsAs::Account { principal: field("on_behalf_of").map(str::to_string) }
    }
}

fn root_from_source(kind: RootKind, item: &str, source: ReadsSource) -> Result<(Node, RunsAs)> {
    let missing =
        |field: &str| Error::BadRequest(format!("the source of a {} needs {field}", kind.as_str()));
    Ok(match kind {
        RootKind::Script => {
            let content = source.content.ok_or_else(|| missing("content"))?;
            let language = source.language.as_deref().and_then(script_lang);
            let node = script_node(
                item,
                language,
                &content,
                source.modules.as_ref(),
                source.schema.as_ref(),
                source.on_behalf_of.clone(),
            );
            (node, item_runs_as(source.on_behalf_of))
        }
        RootKind::Flow => {
            let value = source.value.ok_or_else(|| missing("value"))?;
            let node = flow_node(
                item,
                Some(&value),
                source.schema.as_ref(),
                source.on_behalf_of.clone(),
            );
            (node, item_runs_as(source.on_behalf_of))
        }
        RootKind::App => {
            let value = source.value.ok_or_else(|| missing("value"))?;
            (app_node(item, &value), app_runs_as(source.policy.as_ref()))
        }
    })
}

struct ScriptRow {
    path: String,
    hash: i64,
    language: ScriptLang,
    content: String,
    schema: Option<Value>,
    modules: Option<Value>,
    on_behalf_of: Option<String>,
}

struct FlowRow {
    path: String,
    on_behalf_of: Option<String>,
    value: Option<Value>,
    schema: Option<Value>,
}

struct ResourceRow {
    path: String,
    value: Option<Value>,
}

struct AppRow {
    policy: Value,
    value: Value,
}

fn join_error(e: tokio::task::JoinError) -> Error {
    Error::internal_err(format!("reading what an item reads failed: {e}"))
}

/// Fetches `keys` under the caller's RLS and scopes, in one transaction, and parses them.
async fn fetch_round(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    keys: Vec<NodeKey>,
) -> Result<HashMap<NodeKey, Fetched>> {
    let mut fetched = HashMap::new();
    let (mut paths, mut hashes, mut flows, mut resources) = (vec![], vec![], vec![], vec![]);
    let mut wanted = vec![];
    for key in keys {
        let scope = match &key {
            NodeKey::Root => continue,
            NodeKey::Script { path, .. } => format!("scripts:read:{path}"),
            NodeKey::Flow(path) => format!("flows:read:{path}"),
            NodeKey::Resource(path) | NodeKey::Agent(path) => format!("resources:read:{path}"),
        };
        if check_scopes(authed, || scope).is_err() {
            fetched.insert(key, Fetched::Unreadable);
            continue;
        }
        match &key {
            NodeKey::Script { path, hash: None } => paths.push(path.clone()),
            NodeKey::Script { hash: Some(hash), .. } => hashes.push(*hash),
            NodeKey::Flow(path) => flows.push(path.clone()),
            NodeKey::Resource(path) | NodeKey::Agent(path) => resources.push(path.clone()),
            NodeKey::Root => {}
        }
        wanted.push(key);
    }
    if wanted.is_empty() {
        return Ok(fetched);
    }

    let mut tx = user_db.clone().begin(authed).await?;
    let latest_rows = if paths.is_empty() {
        vec![]
    } else {
        // The version a run by path starts: the newest with a lock, as
        // `get_latest_deployed_script_hash` picks it, else the newest.
        sqlx::query_as!(
            ScriptRow,
            r#"SELECT DISTINCT ON (path) path AS "path!", hash AS "hash!",
                language AS "language!: ScriptLang", content AS "content!",
                schema AS "schema?", modules AS "modules?", on_behalf_of AS "on_behalf_of?"
            FROM script
            WHERE workspace_id = $1 AND path = ANY($2) AND deleted = false
            ORDER BY path, (lock IS NOT NULL AND lock_error_logs IS NULL) DESC, created_at DESC"#,
            w_id,
            &paths[..],
        )
        .fetch_all(&mut *tx)
        .await?
    };
    let pinned_rows = if hashes.is_empty() {
        vec![]
    } else {
        sqlx::query_as!(
            ScriptRow,
            r#"SELECT path AS "path!", hash AS "hash!", language AS "language!: ScriptLang",
                content AS "content!", schema AS "schema?", modules AS "modules?",
                on_behalf_of AS "on_behalf_of?"
            FROM script
            WHERE workspace_id = $1 AND hash = ANY($2) AND deleted = false"#,
            w_id,
            &hashes[..],
        )
        .fetch_all(&mut *tx)
        .await?
    };
    let flow_rows = if flows.is_empty() {
        vec![]
    } else {
        sqlx::query_as!(
            FlowRow,
            r#"SELECT flow.path AS "path!", flow.on_behalf_of AS "on_behalf_of?",
                flow_version.value AS "value?", flow_version.schema AS "schema?"
            FROM flow
            LEFT JOIN flow_version
                ON flow_version.id = flow.versions[array_upper(flow.versions, 1)]
            WHERE flow.workspace_id = $1 AND flow.path = ANY($2)"#,
            w_id,
            &flows[..],
        )
        .fetch_all(&mut *tx)
        .await?
    };
    let resource_rows = if resources.is_empty() {
        vec![]
    } else {
        sqlx::query_as!(
            ResourceRow,
            r#"SELECT path AS "path!", value AS "value?"
            FROM resource
            WHERE workspace_id = $1 AND path = ANY($2)"#,
            w_id,
            &resources[..],
        )
        .fetch_all(&mut *tx)
        .await?
    };
    tx.commit().await?;

    let built = tokio::task::spawn_blocking(move || {
        build_round(wanted, latest_rows, pinned_rows, flow_rows, resource_rows)
    })
    .await
    .map_err(join_error)?;
    fetched.extend(built);
    Ok(fetched)
}

fn build_round(
    wanted: Vec<NodeKey>,
    latest_rows: Vec<ScriptRow>,
    pinned_rows: Vec<ScriptRow>,
    flow_rows: Vec<FlowRow>,
    resource_rows: Vec<ResourceRow>,
) -> Vec<(NodeKey, Fetched)> {
    let latest: HashMap<&str, &ScriptRow> =
        latest_rows.iter().map(|s| (s.path.as_str(), s)).collect();
    let pinned: HashMap<i64, &ScriptRow> = pinned_rows.iter().map(|s| (s.hash, s)).collect();
    let flows: HashMap<&str, &FlowRow> = flow_rows.iter().map(|f| (f.path.as_str(), f)).collect();
    let resources: HashMap<&str, &ResourceRow> =
        resource_rows.iter().map(|r| (r.path.as_str(), r)).collect();
    let script = |s: &ScriptRow| {
        script_node(
            &s.path,
            Some(s.language),
            &s.content,
            s.modules.as_ref(),
            s.schema.as_ref(),
            s.on_behalf_of.clone(),
        )
    };
    wanted
        .into_iter()
        .map(|key| {
            let node = match &key {
                NodeKey::Script { path, hash: None } => {
                    latest.get(path.as_str()).map(|s| script(s))
                }
                // A hash of another path is no version of this one.
                NodeKey::Script { path, hash: Some(hash) } => pinned
                    .get(hash)
                    .filter(|s| &s.path == path)
                    .map(|s| script(s)),
                NodeKey::Flow(path) => flows.get(path.as_str()).map(|f| {
                    flow_node(
                        &f.path,
                        f.value.as_ref(),
                        f.schema.as_ref(),
                        f.on_behalf_of.clone(),
                    )
                }),
                NodeKey::Resource(path) => resources
                    .get(path.as_str())
                    .map(|r| resource_node(&r.path, r.value.as_ref(), Ctx::Data)),
                NodeKey::Agent(path) => resources
                    .get(path.as_str())
                    .map(|r| resource_node(&r.path, r.value.as_ref(), Ctx::Flow)),
                NodeKey::Root => None,
            };
            (key, node.map_or(Fetched::Unreadable, Fetched::Node))
        })
        .collect()
}

async fn fetch_root(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    kind: RootKind,
    path: &str,
) -> Result<(Node, RunsAs, Vec<String>)> {
    check_scopes(authed, || format!("{}s:read:{path}", kind.as_str()))?;
    let not_found = || Error::NotFound(format!("{} {path} not found", kind.as_str()));
    let key = match kind {
        RootKind::Script => NodeKey::Script { path: path.to_string(), hash: None },
        RootKind::Flow => NodeKey::Flow(path.to_string()),
        RootKind::App => {
            let mut tx = user_db.clone().begin(authed).await?;
            let app = sqlx::query_as!(
                AppRow,
                r#"SELECT app.policy AS "policy!", app_version.value AS "value!"
                FROM app
                JOIN app_version ON app_version.id = app.versions[array_upper(app.versions, 1)]
                WHERE app.workspace_id = $1 AND app.path = $2"#,
                w_id,
                path,
            )
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(not_found)?;
            tx.commit().await?;
            let runs_as = app_runs_as(Some(&app.policy));
            let mut errors = vec![];
            if runs_as == (RunsAs::Account { principal: None }) {
                errors.push(format!(
                    "{path} has no account to run its runnables as, so they fail until it is redeployed"
                ));
            }
            let item = path.to_string();
            let node = tokio::task::spawn_blocking(move || app_node(&item, &app.value))
                .await
                .map_err(join_error)?;
            return Ok((node, runs_as, errors));
        }
    };
    match fetch_round(authed, user_db, w_id, vec![key.clone()])
        .await?
        .remove(&key)
    {
        Some(Fetched::Node(node)) => {
            let runs_as = item_runs_as(node.on_behalf_of.clone());
            Ok((node, runs_as, vec![]))
        }
        _ => Err(not_found()),
    }
}

/// Queues what `node` reaches. A reference back to the scanned item's own path is the root,
/// which may be unsaved content.
fn queue_reached(
    node: &mut Node,
    depth: usize,
    root_path: Option<&NodeKey>,
    pending: &mut Vec<(NodeKey, usize)>,
) {
    for edge in &mut node.edges {
        if root_path == Some(&edge.target) {
            edge.target = NodeKey::Root;
        }
        let depth = match edge.target {
            NodeKey::Resource(_) => depth,
            _ => depth + 1,
        };
        pending.push((edge.target.clone(), depth));
    }
}

/// Reads everything the root reaches, one level per transaction.
async fn fetch_graph(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    mut root: Node,
    root_path: Option<NodeKey>,
) -> Result<HashMap<NodeKey, Fetched>> {
    let mut graph = HashMap::new();
    let mut pending = vec![];
    queue_reached(&mut root, 0, root_path.as_ref(), &mut pending);
    graph.insert(NodeKey::Root, Fetched::Node(root));
    let mut read = 0;
    while !pending.is_empty() {
        let mut batch: BTreeMap<NodeKey, usize> = BTreeMap::new();
        for (key, depth) in pending.drain(..) {
            if !graph.contains_key(&key) {
                let least = batch.entry(key).or_insert(depth);
                *least = (*least).min(depth);
            }
        }
        let mut wanted = vec![];
        for (key, depth) in batch {
            if depth > MAX_DEPTH {
                graph.insert(key, Fetched::TooDeep);
            } else if read >= MAX_NODES {
                graph.insert(key, Fetched::OverBudget);
            } else {
                read += 1;
                wanted.push((key, depth));
            }
        }
        if wanted.is_empty() {
            break;
        }
        let keys = wanted.iter().map(|(key, _)| key.clone()).collect();
        let mut fetched = fetch_round(authed, user_db, w_id, keys).await?;
        for (key, depth) in wanted {
            let mut node = fetched.remove(&key).unwrap_or(Fetched::Unreadable);
            if let Fetched::Node(node) = &mut node {
                queue_reached(node, depth, root_path.as_ref(), &mut pending);
            }
            graph.insert(key, node);
        }
    }
    Ok(graph)
}

struct Tagged {
    references: Vec<Reference>,
    others: Vec<OtherAsset>,
    errors: Vec<String>,
}

fn performer(who: Who, reader: &Reader, runners: &[Reader]) -> Reader {
    match who {
        Who::Node => reader.clone(),
        Who::Viewer => Reader::Viewer,
        Who::Edge(i) => runners.get(i).cloned().unwrap_or_else(|| reader.clone()),
    }
}

/// Credits every read in `graph` to whoever performs it, starting from the root's runner.
fn tag(graph: &HashMap<NodeKey, Fetched>) -> Tagged {
    let mut references: BTreeMap<(ItemKind, String, Reader), (Access, BTreeSet<String>)> =
        BTreeMap::new();
    let mut others: BTreeMap<(AssetKind, String, Reader), BTreeSet<String>> = BTreeMap::new();
    let (mut unreadable, mut too_deep, mut over_budget) =
        (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
    let mut seen = HashSet::new();
    let mut stack = vec![(&NodeKey::Root, Reader::Root)];
    while let Some((key, reader)) = stack.pop() {
        if !seen.insert((key, reader.clone())) {
            continue;
        }
        let node = match graph.get(key) {
            Some(Fetched::Node(node)) => node,
            Some(Fetched::TooDeep) => {
                too_deep.insert(key.path());
                continue;
            }
            Some(Fetched::OverBudget) => {
                over_budget.insert(key.path());
                continue;
            }
            Some(Fetched::Unreadable) | None => {
                unreadable.insert(key.path());
                continue;
            }
        };
        // Who runs each edge's target: its own account when the edge keeps it, else whoever
        // performs the edge.
        let mut runners: Vec<Reader> = Vec::with_capacity(node.edges.len());
        for edge in &node.edges {
            let by = performer(edge.who, &reader, &runners);
            let runner = match graph.get(&edge.target) {
                Some(Fetched::Node(Node { on_behalf_of: Some(principal), .. }))
                    if edge.keep_obo =>
                {
                    Reader::Account { principal: principal.clone() }
                }
                _ => by,
            };
            stack.push((&edge.target, runner.clone()));
            runners.push(runner);
        }
        for local in &node.refs {
            let by = performer(local.who, &reader, &runners);
            match &local.target {
                Target::Item { kind, path, access } => {
                    let (merged, sources) = references
                        .entry((*kind, path.clone(), by))
                        .or_insert((*access, BTreeSet::new()));
                    if *access == Access::Write {
                        *merged = Access::Write;
                    }
                    sources.insert(local.source.clone());
                }
                Target::Asset { kind, path } => {
                    others
                        .entry((*kind, path.clone(), by))
                        .or_default()
                        .insert(local.source.clone());
                }
            }
        }
    }

    let mut errors: Vec<String> = unreadable
        .iter()
        .map(|path| format!("{path}: not found or not readable"))
        .collect();
    if !too_deep.is_empty() {
        let paths: Vec<&str> = too_deep.into_iter().collect();
        errors.push(format!(
            "Not followed past {MAX_DEPTH} levels: {}",
            paths.join(", ")
        ));
    }
    if !over_budget.is_empty() {
        errors.push(format!(
            "Stopped after reading {MAX_NODES} items: what {} more read is not listed",
            over_budget.len()
        ));
    }
    Tagged {
        references: references
            .into_iter()
            .map(|((kind, path, reader), (access, sources))| Reference {
                kind,
                path,
                access,
                reader,
                sources: sources.into_iter().collect(),
            })
            .collect(),
        others: others
            .into_iter()
            .map(|((kind, path, reader), sources)| OtherAsset {
                kind,
                path,
                reader,
                sources: sources.into_iter().collect(),
            })
            .collect(),
        errors,
    }
}

pub async fn get_reads(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(request): Json<ReadsRequest>,
) -> JsonResult<ReadsReport> {
    let ReadsRequest { kind, path, source } = request;
    let (root, runs_as, mut errors) = match (source, path.as_deref()) {
        (Some(source), _) => {
            let item = path.clone().unwrap_or_else(|| kind.as_str().to_string());
            let (node, runs_as) =
                tokio::task::spawn_blocking(move || root_from_source(kind, &item, source))
                    .await
                    .map_err(join_error)??;
            (node, runs_as, vec![])
        }
        (None, Some(path)) => fetch_root(&authed, &user_db, &w_id, kind, path).await?,
        (None, None) => return Err(Error::BadRequest("path or source is required".to_string())),
    };
    let root_path = path.and_then(|path| match kind {
        RootKind::Script => Some(NodeKey::Script { path, hash: None }),
        RootKind::Flow => Some(NodeKey::Flow(path)),
        RootKind::App => None,
    });
    let graph = fetch_graph(&authed, &user_db, &w_id, root, root_path).await?;
    let tagged = tag(&graph);
    errors.extend(tagged.errors);
    Ok(Json(ReadsReport {
        runs_as,
        references: tagged.references,
        others: tagged.others,
        errors,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: ItemKind, path: &str, access: Access, who: Who) -> LocalRef {
        LocalRef {
            target: Target::Item { kind, path: path.to_string(), access },
            who,
            source: path.to_string(),
        }
    }

    fn node(on_behalf_of: Option<&str>, refs: Vec<LocalRef>, edges: Vec<Edge>) -> Fetched {
        Fetched::Node(Node { on_behalf_of: on_behalf_of.map(str::to_string), refs, edges })
    }

    fn script(path: &str) -> NodeKey {
        NodeKey::Script { path: path.to_string(), hash: None }
    }

    fn edge(target: NodeKey, keep_obo: bool) -> Edge {
        Edge { target, keep_obo, who: Who::Node }
    }

    fn account(principal: &str) -> Reader {
        Reader::Account { principal: principal.to_string() }
    }

    #[test]
    fn tag_credits_each_read_to_whoever_performs_it() {
        use Access::*;
        use ItemKind::*;
        let graph = HashMap::from([
            (
                NodeKey::Root,
                node(
                    None,
                    vec![
                        item(Variable, "f/x/front", Read, Who::Viewer),
                        // A static input of the step at `edges[0]`.
                        item(Resource, "f/x/input", Read, Who::Edge(0)),
                        item(Script, "f/x/runner", Run, Who::Node),
                    ],
                    vec![
                        // A flow step keeps the account its target runs as, an app runnable
                        // drops it.
                        edge(script("f/x/bot"), true),
                        edge(script("f/x/bot"), false),
                        edge(script("f/x/runner"), true),
                        edge(script("f/x/hidden"), true),
                    ],
                ),
            ),
            (
                script("f/x/bot"),
                node(
                    Some("u/bot"),
                    vec![
                        item(Variable, "f/x/token", Read, Who::Node),
                        item(Resource, "f/x/state", Read, Who::Node),
                        item(Resource, "f/x/state", Write, Who::Node),
                    ],
                    vec![],
                ),
            ),
            (
                script("f/x/runner"),
                node(
                    Some("u/runner"),
                    vec![item(Variable, "f/x/runner_var", Read, Who::Node)],
                    vec![],
                ),
            ),
            (script("f/x/hidden"), Fetched::Unreadable),
        ]);
        let tagged = tag(&graph);
        let got: Vec<_> = tagged
            .references
            .iter()
            .map(|r| (r.kind, r.path.as_str(), r.access, r.reader.clone()))
            .collect();
        assert_eq!(
            got,
            vec![
                (Resource, "f/x/input", Read, account("u/bot")),
                (Resource, "f/x/state", Write, Reader::Root),
                (Resource, "f/x/state", Write, account("u/bot")),
                (Variable, "f/x/front", Read, Reader::Viewer),
                (Variable, "f/x/runner_var", Read, account("u/runner")),
                (Variable, "f/x/token", Read, Reader::Root),
                (Variable, "f/x/token", Read, account("u/bot")),
                (Script, "f/x/runner", Run, Reader::Root),
            ]
        );
        assert_eq!(tagged.errors, vec!["f/x/hidden: not found or not readable"]);
    }
}
