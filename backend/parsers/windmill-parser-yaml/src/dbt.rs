//! The `ScriptLang::Dbt` script artifact: a YAML descriptor holding a dbt
//! project's run configuration. The project itself is the script's module
//! bundle, so the descriptor names no source for it.
//!
//! Field names track dbt's and astronomer-cosmos's vocabulary so the mental
//! model ports without translation (docs/dbt-runtime.md, decision 22).
//! `select` / `exclude` / `selector` are passed to dbt **verbatim**: the
//! selector grammar is dbt's, and reimplementing it is a standing source of
//! divergence.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use windmill_parser::{Arg, MainArgSignature, Typ};

/// Which dbt to run. The shipped default is `dbt-core-1x` because it runs
/// today's projects untouched; `fusion` is never bundled and is fetched from
/// dbt Labs at runtime (docs/dbt-runtime.md, decision 1).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DbtEngine {
    #[default]
    #[serde(rename = "dbt-core-1x")]
    DbtCore1x,
    #[serde(rename = "dbt-core-2x")]
    DbtCore2x,
    Fusion,
}

impl DbtEngine {
    pub fn as_str(&self) -> &'static str {
        match self {
            DbtEngine::DbtCore1x => "dbt-core-1x",
            DbtEngine::DbtCore2x => "dbt-core-2x",
            DbtEngine::Fusion => "fusion",
        }
    }

    /// The level the machine-readable file log is written at. Separate from the
    /// console log, which always stays human-readable at the default level and
    /// is what reaches the job log.
    pub fn progress_log_level(&self) -> &'static str {
        match self {
            DbtEngine::DbtCore1x => "info",
            DbtEngine::DbtCore2x | DbtEngine::Fusion => "debug",
        }
    }

    /// Whether the engine writes per-node events to its JSON **file** log — the
    /// only source of *live* per-model status.
    ///
    /// Not a claim that the others produce none: dbt-core 2.x and Fusion emit
    /// the same events, on the console, and ignore `--log-format-file json`
    /// though both accept it. Reading them would mean taking over the console
    /// and re-rendering the job log. Flip this the moment either honours the
    /// flag — nothing else has to change (docs/dbt-runtime.md, "Live per-model
    /// progress").
    pub fn emits_node_events(&self) -> bool {
        matches!(self, DbtEngine::DbtCore1x)
    }
}

/// How the warehouse connection is supplied. Both paths are supported
/// (decision 8): render `profiles.yml` from a Windmill resource, or keep the
/// project's own file and inject Windmill secrets as env vars for
/// `{{ env_var() }}`.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct DbtProfile {
    /// A warehouse configured on the workspace, by name; omitted takes the
    /// default one. This is the ONLY way a project names a warehouse — there is
    /// no per-descriptor resource — so a dbt project carries no connection at
    /// all, the same bargain `s3://` and `ducklake://` make with workspace
    /// storage, and asset identity has exactly one spelling to key on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warehouse: Option<String>,

    /// dbt target name. It selects which output of the profile runs; the
    /// `<warehouse>` component of asset identity comes from `warehouse` above.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Path (relative to `project`) of the project's own `profiles.yml`, used
    /// instead of rendering one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profiles_yml: Option<String>,
    /// Target schema (BigQuery calls it the dataset). Required for adapters
    /// whose Windmill resource carries none — a BigQuery resource is a raw
    /// service-account JSON, which has no dataset in it. Overrides the
    /// resource's own value where there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// dbt adapter (`postgres` | `snowflake` | `bigquery` | `databricks`),
    /// spelled as dbt's own `type:`. Optional: the worker infers it from the
    /// resource's shape, and this pins it when the inference is wrong or the
    /// resource is a custom type.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum DbtTestBehavior {
    /// `dbt build` — models and their tests interleaved, a model's tests gating
    /// its children. dbt's own default and the only behavior that stops bad
    /// data propagating mid-run.
    #[default]
    Build,
    /// `dbt run` then `dbt test`.
    AfterAll,
    /// `dbt run` only.
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct DbtDescriptor {
    #[serde(default)]
    pub engine: Option<DbtEngine>,
    #[serde(default)]
    pub profile: DbtProfile,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub select: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
    /// A named selector from the project's `selectors.yml`, passed verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    #[serde(default)]
    pub test_behavior: DbtTestBehavior,
    /// `--vars`. dbt vars are typed — numbers, booleans, lists and objects are
    /// all normal — so values keep their YAML type; only string leaves carry
    /// `{{ arg }}` placeholders the worker substitutes from job args. Coercing
    /// everything to a string would make a `false` var truthy in Jinja.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vars: BTreeMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threads: Option<u32>,
    #[serde(default)]
    pub full_refresh: bool,
    /// Automatic in-job retry of the nodes a build failed on.
    ///
    /// dbt already confines a failure to its own subtree, and `dbt retry`
    /// rebuilds exactly the failed and skipped set, so a transient warehouse
    /// error costs those nodes rather than the whole project. Doing it inside
    /// the same job is what keeps the state question out of it: the previous
    /// attempt's `run_results.json` is still in the job directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_failed_nodes: Option<DbtNodeRetry>,
    /// Extra environment for the dbt process, for the project's own
    /// `{{ env_var() }}` lookups and for engine flags such as
    /// `DBT_ALLOW_EXPERIMENTAL_ADAPTERS`. A `$var:<path>` value is resolved to
    /// that Windmill variable's value by the worker, so a password never has to
    /// sit in the descriptor — which is versioned script content.
    ///
    /// This is the map to use for anything the GRAPH depends on — an
    /// `env_var()` driving a schema, alias or `enabled` — because it applies at
    /// deploy as well as at run. Script-level environment variables reach the
    /// run only, so a graph parsed without them would disagree with what the
    /// build writes.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}

/// How many times, and how far apart, to retry a build's failed nodes.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DbtNodeRetry {
    /// Extra `dbt retry` attempts for the job, spent across whichever phases
    /// fail — a model phase that uses them all leaves none for the tests.
    pub attempts: u32,
    /// Seconds between attempts. A transient warehouse error is usually a lock
    /// or a restart, so a pause is the point.
    #[serde(default)]
    pub delay_seconds: u64,
}

impl DbtNodeRetry {
    /// Bounded because each attempt is a real dbt invocation inside a job that
    /// already holds a worker slot, and the job's own deadline still applies.
    pub const MAX_ATTEMPTS: u32 = 10;

    pub fn attempts(&self) -> u32 {
        self.attempts.min(Self::MAX_ATTEMPTS)
    }
}

/// The dbt subcommands a run may ask for. Kept here so the signature and the
/// worker's validation cannot drift apart.
///
/// An allowlist rather than a passthrough: the value becomes the dbt subcommand,
/// and running a script needs weaker permission than editing it, so an unchecked
/// arg would let a runner invoke `clean` or `source freshness` against the
/// descriptor's warehouse.
///
/// `run` is absent because it covers models only: a project with seeds or
/// snapshots would build a subset of what its graph claims. Narrowing what a run
/// touches is `select`/`exclude`, which scope the graph too. Tests run as part
/// of `build`, or as the second phase of `test_behavior: after_all`.
///
/// `show` writes nothing — it SELECTs from a model and returns rows. That is
/// only admissible because a dbt run no longer dispatches: while it did, any
/// successful job fired the script's whole deploy-time write set, so a command
/// that built none of them woke every consumer for relations nothing touched.
///
/// `parse` reads no relation either: it resolves the project into a manifest and
/// stores the graph, which is what the editor's model panel is refreshed by. It
/// still needs a resolvable warehouse, because the profile is rendered before
/// any dbt invocation — a misconfigured project therefore fails a refresh the way
/// it would fail a run, which is the early feedback worth having.
pub const DBT_COMMANDS: &[&str] = &["build", "retry", "show", "parse"];

/// Rows a `show` returns unless the run asks for fewer. dbt enforces it, so the
/// bound is not us splicing a `LIMIT` into someone's SQL.
pub const DBT_SHOW_DEFAULT_LIMIT: u32 = 100;

/// Hard ceiling on that limit. The worker buffers `dbt show`'s whole stdout to
/// read the rows out of it, so the argument decides how much memory a caller can
/// make it hold — and running a script needs only run permission. A preview is
/// for looking at a few rows; anything larger is a query, which is what a SQL
/// script is for.
pub const DBT_SHOW_MAX_LIMIT: u32 = 1_000;

/// Whether the command only reads. Such a run publishes no graph, records no
/// materializations and runs no test phase — there is nothing it could have
/// changed.
pub fn is_read_only_command(command: &str) -> bool {
    command == "show"
}

/// The command a run uses when it does not name one. Public because the worker
/// must choose exactly what the run form's default advertises.
pub fn default_command(d: &DbtDescriptor) -> &'static str {
    match d.test_behavior {
        DbtTestBehavior::Build => "build",
        // Also `build`, with the tests excluded rather than the command
        // narrowed to `run`: `run` covers models only, so a selection that
        // includes a seed or a snapshot would silently not build it and the
        // models reading it would fail — or worse, read a stale table.
        DbtTestBehavior::AfterAll | DbtTestBehavior::None => "build",
    }
}

impl DbtDescriptor {
    pub fn engine(&self) -> DbtEngine {
        self.engine.unwrap_or_default()
    }
}

/// Names a `{{ placeholder }}` may not take. `command` is the argument holding
/// the run's command block; the rest are the fields inside it, which the worker
/// spreads over the run's arguments to read them — a placeholder of the same
/// name would be shadowed there, and the script could never be run (`select` is
/// an array, and interpolating one into a string is not something any invocation
/// can satisfy).
pub const RESERVED_ARG_NAMES: &[&str] = &[
    "command",
    "select",
    "exclude",
    "vars",
    "full_refresh",
    "dbt_command",
    "dbt_retry_job",
    "model",
    "limit",
];

pub fn parse_dbt_descriptor(inner_content: &str) -> anyhow::Result<DbtDescriptor> {
    let d = serde_yml::from_str::<DbtDescriptor>(inner_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse dbt descriptor: {e}"))?;
    if let Some(name) = placeholders(&d)
        .into_iter()
        .find(|n| RESERVED_ARG_NAMES.contains(&n.as_str()))
    {
        return Err(anyhow::anyhow!(
            "`{{{{ {name} }}}}` collides with the run argument `{name}` this runtime already \
             defines ({}); rename the placeholder",
            RESERVED_ARG_NAMES.join(", ")
        ));
    }
    Ok(d)
}

/// The workspace warehouse a descriptor gets when it names none — spelled like
/// the default lake (`ducklake://main.orders`), so one workspace concept reads
/// the same across kinds.
pub const DBT_DEFAULT_WAREHOUSE: &str = "main";

/// The single argument holding the command and the overrides it takes. Its
/// variant IS the command, so a run cannot carry an override the command
/// ignores: `retry` rebuilds the failed run's nodes with the arguments that run
/// had, and `full_refresh` means nothing to a `show`.
pub const DBT_COMMAND_ARG: &str = "command";

/// The variant discriminator, which is the key Windmill's run form tags a
/// `oneOf` value with.
pub const DBT_COMMAND_LABEL: &str = "label";

fn list_arg(name: &str, default: &[String]) -> Arg {
    Arg {
        name: name.to_string(),
        otyp: None,
        typ: Typ::List(Box::new(Typ::Str(None))),
        has_default: true,
        default: Some(serde_json::json!(default)),
        oidx: None,
        otyp_inferred: false,
    }
}

/// An OVERRIDE map, so its default is empty rather than a copy of the
/// descriptor's vars. Seeding it with the descriptor would make the run form
/// post the raw `{{ placeholder }}` text back and clobber the value the worker
/// just interpolated for it.
fn vars_arg() -> Arg {
    Arg {
        name: "vars".to_string(),
        otyp: None,
        typ: Typ::Object(windmill_parser::ObjectType::new(None, Some(vec![]))),
        has_default: true,
        default: Some(serde_json::json!({})),
        oidx: None,
        otyp_inferred: false,
    }
}

/// What each command takes, in form order.
///
/// `show` and `parse` are absent on purpose, for the same reason: each is a thing
/// to do to the project you are looking at rather than a job to fill a form in
/// for. `show` previews ONE model's rows, offered by the run page's graph and the
/// assets list where the tables are; `parse` refreshes the model graph, offered by
/// the dbt editor over the buffer being edited. The worker still accepts
/// `{label: show, model, limit}` and `{label: parse, vars}` from a flow, the CLI or
/// the API (`DBT_COMMANDS`, docs/dbt-runtime.md).
fn command_variants(d: &DbtDescriptor) -> Vec<(&'static str, Vec<Arg>)> {
    let selection = || {
        vec![
            list_arg("select", &d.select),
            list_arg("exclude", &d.exclude),
            vars_arg(),
        ]
    };
    vec![
        (
            "build",
            selection()
                .into_iter()
                .chain([Arg {
                    name: "full_refresh".to_string(),
                    otyp: None,
                    typ: Typ::Bool,
                    has_default: true,
                    default: Some(serde_json::json!(d.full_refresh)),
                    oidx: None,
                    otyp_inferred: false,
                }])
                .collect(),
        ),
        (
            "retry",
            // The run it resumes, named rather than implied: only the latest
            // failure of this script is kept, so "resume the last one" would
            // silently aim somewhere else the moment another run failed. No
            // default, so this variant requires it.
            vec![Arg {
                name: "dbt_retry_job".to_string(),
                otyp: None,
                typ: Typ::Str(None),
                has_default: false,
                default: None,
                oidx: None,
                otyp_inferred: false,
            }],
        ),
    ]
}

/// The command block a run gets when it overrides nothing: the descriptor's own
/// selection under the descriptor's default command, so an untouched run
/// reproduces it exactly.
fn default_command_value(d: &DbtDescriptor) -> serde_json::Value {
    let label = default_command(d);
    let mut obj = serde_json::Map::new();
    obj.insert(DBT_COMMAND_LABEL.to_string(), serde_json::json!(label));
    for arg in command_variants(d)
        .into_iter()
        .find(|(l, _)| *l == label)
        .map(|(_, args)| args)
        .unwrap_or_default()
    {
        if let Some(default) = arg.default {
            obj.insert(arg.name, default);
        }
    }
    serde_json::Value::Object(obj)
}

/// Run-time arguments of a dbt script: the command block above, plus one
/// argument per `{{ placeholder }}` the descriptor interpolates. Placeholders
/// stay top-level — they are the project's own inputs, not a command's.
pub fn parse_dbt_sig(inner_content: &str) -> anyhow::Result<MainArgSignature> {
    let d = parse_dbt_descriptor(inner_content)?;
    let mut args = vec![Arg {
        name: DBT_COMMAND_ARG.to_string(),
        otyp: None,
        typ: Typ::Object(windmill_parser::ObjectType::new(None, Some(vec![]))),
        has_default: true,
        default: Some(default_command_value(&d)),
        oidx: None,
        otyp_inferred: false,
    }];

    for name in placeholders(&d) {
        if args.iter().any(|a| a.name == name) {
            continue;
        }
        args.push(Arg {
            name,
            otyp: None,
            // Untyped, not `Str`: a placeholder standing alone in a var takes
            // the argument's own JSON type, and declaring it a string makes the
            // run form post `"false"` — truthy in Jinja — for a boolean.
            typ: Typ::Unknown,
            has_default: false,
            default: None,
            oidx: None,
            otyp_inferred: false,
        });
    }

    Ok(MainArgSignature {
        star_args: false,
        star_kwargs: false,
        args,
        auto_kind: None,
        has_preprocessor: None,
        ..Default::default()
    })
}

/// The run form's JSON schema for a dbt descriptor.
///
/// Derived here rather than in the browser or the CLI: both infer a script's
/// schema client-side through `windmill-parser-wasm`, whose published package
/// has no dbt arm, so they leave the schema untouched. Without this a dbt
/// script deploys with an empty schema and its run form offers none of the
/// overrides — and an edited descriptor keeps the previous one's arguments.
/// The command is a `oneOf` rather than an enum beside the overrides it selects:
/// a variant carries exactly the arguments its command takes, so `dbt_retry_job`
/// is required where it means something and absent everywhere else, and no run
/// can submit a `full_refresh` to a command that ignores it. The run form renders
/// it as a toggle over the variants and tags the value with `label`.
pub fn dbt_arg_schema(inner_content: &str) -> anyhow::Result<serde_json::Value> {
    let d = parse_dbt_descriptor(inner_content)?;
    let sig = parse_dbt_sig(inner_content)?;
    let mut properties = serde_json::Map::new();
    let mut required: Vec<serde_json::Value> = vec![];
    let mut order: Vec<serde_json::Value> = vec![];

    let variants: Vec<serde_json::Value> = command_variants(&d)
        .into_iter()
        .map(|(label, args)| {
            let mut props = serde_json::Map::new();
            let mut var_required: Vec<serde_json::Value> = vec![];
            let mut var_order: Vec<serde_json::Value> = vec![serde_json::json!(DBT_COMMAND_LABEL)];
            // The discriminator. Single-valued, so the form's toggle is what sets
            // it and a submitted block cannot claim one command while carrying
            // another's fields.
            props.insert(
                DBT_COMMAND_LABEL.to_string(),
                serde_json::json!({"type": "string", "enum": [label]}),
            );
            for arg in args {
                if !arg.has_default {
                    var_required.push(serde_json::json!(arg.name));
                }
                var_order.push(serde_json::json!(arg.name));
                props.insert(arg.name.clone(), property_of(&arg));
            }
            serde_json::json!({
                "title": label,
                "type": "object",
                "properties": props,
                "order": var_order,
                "required": var_required,
            })
        })
        .collect();

    for arg in &sig.args {
        let mut prop = property_of(arg);
        if arg.name == DBT_COMMAND_ARG {
            if let Some(obj) = prop.as_object_mut() {
                obj.insert(
                    "oneOf".to_string(),
                    serde_json::Value::Array(variants.clone()),
                );
                obj.insert(
                    "description".to_string(),
                    serde_json::json!(
                        "`build` runs the project. `retry` resumes a failed run, rebuilding only \
                         its failed and skipped nodes with the arguments it ran with."
                    ),
                );
            }
        }
        if !arg.has_default {
            required.push(serde_json::json!(arg.name));
        }
        order.push(serde_json::json!(arg.name));
        properties.insert(arg.name.clone(), prop);
    }
    Ok(serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": properties,
        "required": required,
        "order": order,
    }))
}

/// One argument as a JSON-schema property, with the description the run form
/// shows under its label.
fn property_of(arg: &Arg) -> serde_json::Value {
    let mut prop = match &arg.typ {
        Typ::Bool => serde_json::json!({"type": "boolean"}),
        // `limit` is an integer to the worker, which clamps it; without this the
        // generated clients and the run form offer no numeric control.
        Typ::Int => serde_json::json!({"type": "integer"}),
        Typ::List(_) => serde_json::json!({"type": "array", "items": {"type": "string"}}),
        Typ::Object(_) => serde_json::json!({"type": "object"}),
        Typ::Str(Some(variants)) => serde_json::json!({"type": "string", "enum": variants}),
        Typ::Str(None) => serde_json::json!({"type": "string"}),
        // A placeholder takes the JSON type of whatever is passed, so it is
        // deliberately left untyped rather than guessed at.
        _ => serde_json::json!({}),
    };
    if let Some(obj) = prop.as_object_mut() {
        if let Some(default) = arg.default.as_ref() {
            obj.insert("default".to_string(), default.clone());
        }
        let description = match arg.name.as_str() {
            "dbt_retry_job" => Some(
                "The failed run to resume, by run id. Its failed and skipped nodes are rebuilt \
                 with the arguments it ran with. Resuming from that run's page fills this in.",
            ),
            "select" => Some(
                "dbt selection syntax, e.g. `tag:nightly`, `stg_orders+`, \
                 `config.materialized:incremental`. Empty runs the descriptor's own selection.",
            ),
            "exclude" => Some("Nodes to leave out of the selection above, same syntax."),
            "vars" => Some(
                "dbt `--vars`, merged over the descriptor's. A var that changes which models \
                 exist makes this run store its own graph rather than the deployed one.",
            ),
            "full_refresh" => Some("Rebuild incremental models from scratch instead of appending."),
            "model" => Some(
                "The model to preview, by name — `stg_orders`, or `my_package.stg_orders` when \
                 two packages share a name. Any dbt selector resolving to ONE node works.",
            ),
            "limit" => Some("Rows the preview returns."),
            // A placeholder the descriptor interpolates: its meaning is the
            // project's, so there is nothing generic to say about it.
            _ => None,
        };
        if let Some(d) = description {
            obj.insert("description".to_string(), serde_json::json!(d));
        }
    }
    prop
}

/// `{{ name }}` placeholders in the interpolated descriptor fields, in a stable
/// order. Must stay in sync with the fields the worker actually interpolates.
fn placeholders(d: &DbtDescriptor) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    let mut push_from = |s: &str| {
        for caps in PLACEHOLDER_RE.captures_iter(s) {
            let name = caps[1].to_string();
            if !out.contains(&name) {
                out.push(name);
            }
        }
    };
    for v in d.vars.values() {
        for leaf in string_leaves(v) {
            push_from(leaf);
        }
    }
    out
}

/// Every string inside a var's value, at any depth — the only places a
/// `{{ arg }}` placeholder can appear.
pub fn string_leaves(v: &serde_json::Value) -> Vec<&str> {
    match v {
        serde_json::Value::String(s) => vec![s.as_str()],
        serde_json::Value::Array(a) => a.iter().flat_map(string_leaves).collect(),
        serde_json::Value::Object(o) => o.values().flat_map(string_leaves).collect(),
        _ => vec![],
    }
}

lazy_static::lazy_static! {
    /// Same spelling as the Ansible executor's `interpolate_template`, which is
    /// what actually performs the substitution at run time.
    static ref PLACEHOLDER_RE: regex::Regex =
        regex::Regex::new(r"\{\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*\}\}").unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESCRIPTOR: &str = r#"
engine: dbt-core-2x
profile:
  warehouse: main
  target: prod
select: ["tag:nightly+"]
test_behavior: after_all
vars:
  run_date: "{{ day }}"
  strict: false
threads: 8
full_refresh: true
"#;

    // A descriptor drives warehouse writes, so a field it does not recognise is
    // an error rather than a default. `selcet:` would otherwise leave `select`
    // empty and build the entire project; a misspelled `target` would silently
    // fall back to the profile's own default.
    #[test]
    fn an_unknown_descriptor_field_is_refused() {
        let err = parse_dbt_descriptor("profile:\n  warehouse: main\nselcet: [a]\n")
            .unwrap_err()
            .to_string();
        assert!(err.contains("selcet"), "{err}");
        let err = parse_dbt_descriptor("profile:\n  warehouse: main\n  targt: prod\n")
            .unwrap_err()
            .to_string();
        assert!(err.contains("targt"), "{err}");
    }

    // A placeholder that takes a run argument's name would be shadowed by it,
    // leaving a descriptor no invocation can satisfy: `select` is an array and
    // the interpolation needs a scalar. Refused at parse, so the deploy says so
    // rather than the script becoming unrunnable after it.
    #[test]
    fn a_placeholder_may_not_take_a_run_argument_name() {
        for name in RESERVED_ARG_NAMES {
            let d = format!("profile:\n  warehouse: main\nvars:\n  v: \"{{{{ {name} }}}}\"\n");
            let err = parse_dbt_descriptor(&d).unwrap_err().to_string();
            assert!(err.contains(name), "{name}: {err}");
        }
        // A name of its own is fine.
        assert!(
            parse_dbt_descriptor("profile:\n  warehouse: main\nvars:\n  v: \"{{ day }}\"\n")
                .is_ok()
        );
    }

    // The run form is built from this schema, and nothing else can build it:
    // the browser and the CLI both infer through a wasm parser that has no dbt
    // arm, so a missing property here is an override the user cannot reach.
    // The variants are what make the form show a command's own fields and only
    // those, so each is asserted by the arguments it carries.
    #[test]
    fn the_schema_carries_every_run_override_and_placeholder() {
        let schema = dbt_arg_schema(DESCRIPTOR).unwrap();
        let props = schema["properties"].as_object().unwrap();
        let variants = props[DBT_COMMAND_ARG]["oneOf"].as_array().unwrap();
        let of = |label: &str| {
            let v = variants
                .iter()
                .find(|v| v["title"] == label)
                .unwrap_or_else(|| panic!("no `{label}` variant: {schema}"));
            let mut names: Vec<String> = v["properties"]
                .as_object()
                .unwrap()
                .keys()
                .filter(|k| k.as_str() != DBT_COMMAND_LABEL)
                .cloned()
                .collect();
            names.sort();
            (v.clone(), names)
        };

        let (build, build_args) = of("build");
        assert_eq!(build_args, ["exclude", "full_refresh", "select", "vars"]);
        assert_eq!(build["properties"]["full_refresh"]["type"], "boolean");
        // Defaults come from the descriptor, so an untouched run reproduces it.
        assert_eq!(
            build["properties"]["select"]["default"],
            serde_json::json!(["tag:nightly+"])
        );
        // The discriminator takes one value per variant: the toggle sets it, and
        // a block cannot claim one command while carrying another's fields.
        assert_eq!(
            build["properties"][DBT_COMMAND_LABEL]["enum"],
            serde_json::json!(["build"])
        );

        let (retry, retry_args) = of("retry");
        assert_eq!(retry_args, ["dbt_retry_job"]);
        // Required where it means something, rather than required everywhere and
        // hidden: a retry names the run it resumes.
        assert_eq!(retry["required"], serde_json::json!(["dbt_retry_job"]));

        // Neither `show` nor `parse` is a form variant: previewing one model's
        // rows and refreshing the model graph are both things you do to the
        // project in front of you, and the graph, the assets list and the dbt
        // editor are where those live. The worker still accepts both
        // programmatically, which is what makes them scriptable.
        for hidden in ["show", "parse"] {
            assert!(
                !variants.iter().any(|v| v["title"] == hidden),
                "{hidden} must not be offered in the run form: {schema}"
            );
            assert!(
                DBT_COMMANDS.contains(&hidden),
                "but the worker still takes {hidden}"
            );
        }

        // Every `{{ placeholder }}` the descriptor interpolates is an argument a
        // run must supply. The command block is not one: it defaults to the
        // descriptor's own selection under the descriptor's command.
        assert_eq!(schema["required"], serde_json::json!(["day"]));
        assert_eq!(schema["order"], serde_json::json!([DBT_COMMAND_ARG, "day"]));
    }

    #[test]
    fn parses_descriptor() {
        let d = parse_dbt_descriptor(DESCRIPTOR).unwrap();
        assert_eq!(d.engine(), DbtEngine::DbtCore2x);
        assert_eq!(d.profile.target.as_deref(), Some("prod"));
        assert_eq!(d.select, vec!["tag:nightly+"]);
        assert_eq!(d.threads, Some(8));
        assert!(d.full_refresh);
        // dbt vars keep their YAML type: a `false` coerced to "false" is truthy
        // in Jinja and would silently invert the condition it gates.
        assert_eq!(d.vars["strict"], serde_json::json!(false));
        assert_eq!(d.vars["run_date"], serde_json::json!("{{ day }}"));
    }

    #[test]
    fn an_empty_descriptor_defaults_to_the_bundled_engine() {
        let d = parse_dbt_descriptor("").unwrap();
        assert_eq!(d.engine(), DbtEngine::DbtCore1x);
    }

    // The ORDER is asserted, not just the set: the schema's `order` is built from
    // this vec and the run form follows it, so the command block leading is what
    // puts the choice of what the run does above the project's own inputs.
    #[test]
    fn signature_exposes_the_command_block_and_placeholders() {
        let sig = parse_dbt_sig(DESCRIPTOR).unwrap();
        let names: Vec<&str> = sig.args.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec![DBT_COMMAND_ARG, "day"]);
        // An untouched run reproduces the descriptor: its default is the whole
        // block — the descriptor's command, carrying the descriptor's selection.
        let cmd = sig.args.iter().find(|a| a.name == DBT_COMMAND_ARG).unwrap();
        let default = cmd.default.clone().unwrap();
        // `build`, not `run`: `run` covers models only, so a selection naming a
        // seed or a snapshot would silently not build it.
        assert_eq!(default[DBT_COMMAND_LABEL], serde_json::json!("build"));
        assert_eq!(default["select"], serde_json::json!(["tag:nightly+"]));
        assert_eq!(default["full_refresh"], serde_json::json!(true));
        // `vars` is the exception: it overrides, so its default must be empty.
        // Seeded with the descriptor, the run form would post `{{ day }}` back
        // and overwrite the value the worker interpolated for it.
        assert_eq!(default["vars"], serde_json::json!({}));
        // Placeholders are required (the descriptor names no value for them)
        // and untyped, so a `{{ }}` var can carry a boolean or a number rather
        // than the string "false", which Jinja treats as truthy.
        let day = sig.args.iter().find(|a| a.name == "day").unwrap();
        assert!(!day.has_default);
        assert_eq!(day.typ, Typ::Unknown);
    }
}
