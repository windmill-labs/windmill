//! The `ScriptLang::Dbt` script artifact: a YAML descriptor pointing at a dbt
//! project in an external git repo, plus the run configuration.
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

    /// Whether the engine writes per-node events to its JSON file log — the
    /// only source of *live* per-model status. dbt-core 2.0.0-alpha.5 accepts
    /// `--log-format-file json` but still writes a text log, so its runs settle
    /// their models from `run_results.json` when the invocation ends instead.
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
    /// `$res:<path>` of the `git_repository` resource holding the project.
    pub repo: String,
    /// Subdirectory containing `dbt_project.yml`; empty means the repo root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Tag / branch / commit, or the literal `latest` to resolve HEAD at run
    /// time. Pinned by default (decision 5); `{{ arg }}` placeholders are
    /// substituted from job args.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
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
    /// Windmill variables holding private SSH keys for cloning the repo, the
    /// same shape as Ansible's `git_ssh_identity`. Token auth lives in the
    /// `git_repository` resource's URL instead. GitHub App resources are NOT
    /// supported: minting their installation token needs an authorization path
    /// that does not exist for arbitrary runnables, so they are rejected with
    /// that reason (docs/dbt-runtime.md, decision 10).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub git_ssh_identity: Vec<String>,
}

/// Sentinel `ref` meaning "resolve HEAD at run time" rather than at deploy.
pub const REF_LATEST: &str = "latest";

/// The dbt subcommands a run may ask for. Kept here so the signature and the
/// worker's validation cannot drift apart.
///
/// `test` is deliberately absent. A test-only run writes nothing, but the asset
/// dispatcher fires a script's deploy-time writes on any successful job, so it
/// would notify every downstream consumer that tables it never touched had
/// changed. Tests run as part of `build`, or as the second phase of
/// `test_behavior: after_all`.
pub const DBT_COMMANDS: &[&str] = &["build", "run", "retry"];

fn default_command(d: &DbtDescriptor) -> &'static str {
    match d.test_behavior {
        DbtTestBehavior::Build => "build",
        DbtTestBehavior::AfterAll | DbtTestBehavior::None => "run",
    }
}

impl DbtDescriptor {
    pub fn engine(&self) -> DbtEngine {
        self.engine.unwrap_or_default()
    }

    /// Only the explicit `latest` floats. An omitted `ref` still pins: the
    /// deploy resolves the resource's default branch to a commit and locks it,
    /// which is what "pinned by default" means (decision 5).
    pub fn is_latest_ref(&self) -> bool {
        self.r#ref
            .as_deref()
            .is_some_and(|r| r.trim().eq_ignore_ascii_case(REF_LATEST))
    }
}

pub fn parse_dbt_descriptor(inner_content: &str) -> anyhow::Result<DbtDescriptor> {
    serde_yml::from_str::<DbtDescriptor>(inner_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse dbt descriptor: {e}"))
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
            typ: Typ::Str(Some(
                DBT_COMMANDS.iter().map(|c| c.to_string()).collect(),
            )),
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
    if let Some(r) = d.r#ref.as_deref() {
        push_from(r);
    }
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
repo: $res:u/rf/analytics_repo
project: transform
ref: "{{ commit }}"
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

    #[test]
    fn parses_descriptor() {
        let d = parse_dbt_descriptor(DESCRIPTOR).unwrap();
        assert_eq!(d.repo, "$res:u/rf/analytics_repo");
        assert_eq!(d.project.as_deref(), Some("transform"));
        assert_eq!(d.engine(), DbtEngine::DbtCore2x);
        assert_eq!(d.profile.target.as_deref(), Some("prod"));
        assert_eq!(d.select, vec!["tag:nightly+"]);
        assert_eq!(d.threads, Some(8));
        assert!(d.full_refresh);
        // dbt vars keep their YAML type: a `false` coerced to "false" is truthy
        // in Jinja and would silently invert the condition it gates.
        assert_eq!(d.vars["strict"], serde_json::json!(false));
        assert_eq!(d.vars["run_date"], serde_json::json!("{{ day }}"));
        assert!(!d.is_latest_ref());
    }

    #[test]
    fn minimal_descriptor_defaults_to_the_bundled_engine_and_a_pinned_ref() {
        let d = parse_dbt_descriptor("repo: $res:u/rf/repo\n").unwrap();
        assert_eq!(d.engine(), DbtEngine::DbtCore1x);
        // Pinned by default: an omitted ref is locked at deploy, and only the
        // explicit `latest` resolves HEAD per run.
        assert!(!d.is_latest_ref());
        assert!(parse_dbt_descriptor("repo: r\nref: latest\n")
            .unwrap()
            .is_latest_ref());
        assert!(parse_dbt_descriptor("repo: r\nref: main\n")
            .unwrap()
            .is_latest_ref()
            == false);
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
                "commit",
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
        assert_eq!(cmd.default, Some(serde_json::json!("run")));
        // Defaults come from the descriptor so an untouched run reproduces it.
        let select = sig.args.iter().find(|a| a.name == "select").unwrap();
        assert_eq!(select.default, Some(serde_json::json!(["tag:nightly+"])));
        // `vars` is the exception: it overrides, so its default must be empty.
        // Seeded with the descriptor, the run form would post `{{ day }}` back
        // and overwrite the value the worker interpolated for it.
        let vars = sig.args.iter().find(|a| a.name == "vars").unwrap();
        assert_eq!(vars.default, Some(serde_json::json!({})));
        // Placeholders are required (no sane default for a commit) and untyped,
        // so a `{{ }}` var can carry a boolean or a number rather than the
        // string "false", which Jinja treats as truthy.
        let commit = sig.args.iter().find(|a| a.name == "commit").unwrap();
        assert!(!commit.has_default);
        assert_eq!(commit.typ, Typ::Unknown);
    }
}
