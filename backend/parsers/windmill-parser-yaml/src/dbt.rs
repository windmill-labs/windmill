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
pub struct DbtProfile {
    /// `$res:<path>` of the warehouse resource to render into `profiles.yml`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    /// dbt target name. Also the `<resource_path>` component's companion when
    /// resolving asset identity.
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
pub struct DbtNodeRetry {
    /// Extra `dbt retry` attempts after the first failed build.
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
/// Only commands whose writes match the graph the deploy registered. The asset
/// dispatcher fires a script's deploy-time writes on any successful job, so a
/// command that builds a SUBSET of them notifies consumers of relations this
/// invocation left stale.
///
/// That rules out `test`, which writes nothing, and `run`, which covers models
/// only: a project with seeds or snapshots registers those as writes too.
/// Narrowing what a run touches is `select`/`exclude`, which scope the graph as
/// well, resolved by asking dbt. Tests run as part of `build`, or as the second
/// phase of `test_behavior: after_all`.
pub const DBT_COMMANDS: &[&str] = &["build", "retry"];

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

/// Run arguments the descriptor's own fields already claim. A `{{ placeholder }}`
/// may not take one of these names: the built-in argument would shadow it, and
/// the script could never be run — `select` is an array, and interpolating one
/// into a string is not something any invocation can satisfy.
pub const RESERVED_ARG_NAMES: &[&str] =
    &["select", "exclude", "vars", "full_refresh", "dbt_command"];

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

/// Run-time arguments of a dbt script: the descriptor fields that can be
/// overridden per run (decision 7), plus one argument per `{{ placeholder }}`
/// the descriptor interpolates. Defaults come from the descriptor, so an
/// untouched run reproduces it exactly.
pub fn parse_dbt_sig(inner_content: &str) -> anyhow::Result<MainArgSignature> {
    let d = parse_dbt_descriptor(inner_content)?;
    let mut args = vec![
        Arg {
            name: "select".to_string(),
            otyp: None,
            typ: Typ::List(Box::new(Typ::Str(None))),
            has_default: true,
            default: Some(serde_json::json!(d.select)),
            oidx: None,
            otyp_inferred: false,
        },
        Arg {
            name: "exclude".to_string(),
            otyp: None,
            typ: Typ::List(Box::new(Typ::Str(None))),
            has_default: true,
            default: Some(serde_json::json!(d.exclude)),
            oidx: None,
            otyp_inferred: false,
        },
        // An OVERRIDE map, so its default is empty rather than a copy of the
        // descriptor's vars. Seeding it with the descriptor would make the run
        // form post the raw `{{ placeholder }}` text back and clobber the value
        // the worker just interpolated for it.
        Arg {
            name: "vars".to_string(),
            otyp: None,
            typ: Typ::Object(windmill_parser::ObjectType::new(None, Some(vec![]))),
            has_default: true,
            default: Some(serde_json::json!({})),
            oidx: None,
            otyp_inferred: false,
        },
        Arg {
            name: "full_refresh".to_string(),
            otyp: None,
            typ: Typ::Bool,
            has_default: true,
            default: Some(serde_json::json!(d.full_refresh)),
            oidx: None,
            otyp_inferred: false,
        },
        // `retry` resumes from the previous run's failure point rather than
        // rebuilding. Enumerated so the run form offers it and so the value
        // cannot reach the engine as an arbitrary subcommand.
        Arg {
            name: "dbt_command".to_string(),
            otyp: None,
            typ: Typ::Str(Some(DBT_COMMANDS.iter().map(|c| c.to_string()).collect())),
            has_default: true,
            default: Some(serde_json::json!(default_command(&d))),
            oidx: None,
            otyp_inferred: false,
        },
    ];

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
/// Built from `parse_dbt_sig` so the argument list has exactly one definition.
pub fn dbt_arg_schema(inner_content: &str) -> anyhow::Result<serde_json::Value> {
    let sig = parse_dbt_sig(inner_content)?;
    let mut properties = serde_json::Map::new();
    let mut required: Vec<serde_json::Value> = vec![];
    let mut order: Vec<serde_json::Value> = vec![];
    for arg in &sig.args {
        let mut prop = match &arg.typ {
            Typ::Bool => serde_json::json!({"type": "boolean"}),
            Typ::List(_) => serde_json::json!({"type": "array", "items": {"type": "string"}}),
            Typ::Object(_) => serde_json::json!({"type": "object"}),
            Typ::Str(Some(variants)) => serde_json::json!({"type": "string", "enum": variants}),
            Typ::Str(None) => serde_json::json!({"type": "string"}),
            // A placeholder takes the JSON type of whatever is passed, so it is
            // deliberately left untyped rather than guessed at.
            _ => serde_json::json!({}),
        };
        if let (Some(obj), Some(default)) = (prop.as_object_mut(), arg.default.as_ref()) {
            obj.insert("default".to_string(), default.clone());
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
  resource: $res:f/prod/snowflake
  target: prod
select: ["tag:nightly+"]
test_behavior: after_all
vars:
  run_date: "{{ day }}"
  strict: false
threads: 8
full_refresh: true
"#;

    // A placeholder that takes a run argument's name would be shadowed by it,
    // leaving a descriptor no invocation can satisfy: `select` is an array and
    // the interpolation needs a scalar. Refused at parse, so the deploy says so
    // rather than the script becoming unrunnable after it.
    #[test]
    fn a_placeholder_may_not_take_a_run_argument_name() {
        for name in RESERVED_ARG_NAMES {
            let d = format!(
                "repo: $res:u/rf/r\nprofile:\n  resource: $res:u/rf/wh\nvars:\n  v: \"{{{{ {name} }}}}\"\n"
            );
            let err = parse_dbt_descriptor(&d).unwrap_err().to_string();
            assert!(err.contains(name), "{name}: {err}");
        }
        // A name of its own is fine.
        assert!(parse_dbt_descriptor(
            "profile:\n  resource: $res:u/rf/wh\nvars:\n  v: \"{{ day }}\"\n"
        )
        .is_ok());
    }

    // The run form is built from this schema, and nothing else can build it:
    // the browser and the CLI both infer through a wasm parser that has no dbt
    // arm, so a missing property here is an override the user cannot reach.
    #[test]
    fn the_schema_carries_every_run_override_and_placeholder() {
        let schema = dbt_arg_schema(DESCRIPTOR).unwrap();
        let props = schema["properties"].as_object().unwrap();
        for name in ["select", "exclude", "vars", "full_refresh", "dbt_command"] {
            assert!(props.contains_key(name), "missing {name}: {schema}");
        }
        assert_eq!(
            props["dbt_command"]["enum"],
            serde_json::json!(DBT_COMMANDS)
        );
        assert_eq!(props["full_refresh"]["type"], "boolean");
        // Every `{{ placeholder }}` the descriptor interpolates is an argument
        // a run must supply — the overrides above all default to the
        // descriptor's own values instead.
        assert_eq!(schema["required"], serde_json::json!(["day"]));
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

    #[test]
    fn signature_exposes_overridable_fields_and_placeholders() {
        let sig = parse_dbt_sig(DESCRIPTOR).unwrap();
        let names: Vec<&str> = sig.args.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "select",
                "exclude",
                "vars",
                "full_refresh",
                "dbt_command",
                "day"
            ]
        );
        // Enumerated, so the run form offers `retry` and an arbitrary value
        // can't reach the engine as a subcommand.
        let cmd = sig.args.iter().find(|a| a.name == "dbt_command").unwrap();
        assert_eq!(
            cmd.typ,
            Typ::Str(Some(DBT_COMMANDS.iter().map(|c| c.to_string()).collect()))
        );
        // `build`, not `run`: `run` covers models only, so a selection naming a
        // seed or a snapshot would silently not build it.
        assert_eq!(cmd.default, Some(serde_json::json!("build")));
        // Defaults come from the descriptor so an untouched run reproduces it.
        let select = sig.args.iter().find(|a| a.name == "select").unwrap();
        assert_eq!(select.default, Some(serde_json::json!(["tag:nightly+"])));
        // `vars` is the exception: it overrides, so its default must be empty.
        // Seeded with the descriptor, the run form would post `{{ day }}` back
        // and overwrite the value the worker interpolated for it.
        let vars = sig.args.iter().find(|a| a.name == "vars").unwrap();
        assert_eq!(vars.default, Some(serde_json::json!({})));
        // Placeholders are required (the descriptor names no value for them)
        // and untyped, so a `{{ }}` var can carry a boolean or a number rather
        // than the string "false", which Jinja treats as truthy.
        let day = sig.args.iter().find(|a| a.name == "day").unwrap();
        assert!(!day.has_default);
        assert_eq!(day.typ, Typ::Unknown);
    }
}
