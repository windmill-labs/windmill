//! `POST /w/{workspace}/jobs/run/flow_test/{*path}`: run the deployed flow at `path` with some
//! of its steps overridden, for tests. An override either mocks the step's result or replaces
//! its logic with a function of the test's own code. The flow is run as a preview of the edited
//! value, so the SDKs only send step ids, values and function names; everything else is here.

use std::collections::{HashMap, HashSet};

use axum::{
    extract::{Path, Query},
    Extension, Json,
};
use hyper::StatusCode;
use serde::Deserialize;
use serde_json::{json, value::RawValue, Map, Value};
use uuid::Uuid;
use windmill_common::{
    db::{UserDB, UserDbWithAuthed},
    error::{Error, Result},
    get_latest_flow_version_info_for_path,
    scripts::ScriptLang,
    utils::StripPath,
};
use windmill_types::{
    flows::FlowValue,
    jobs::{is_valid_entrypoint_name, ENTRYPOINT_OVERRIDE},
};

use crate::db::{ApiAuthed, DB};
use crate::jobs::{
    operator_preview_refusal, push_preview_flow_job, require_job_update_read_access, role_can_code,
    PreviewFlow, RunJobQuery,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FlowTestRequest {
    #[serde(default)]
    args: HashMap<String, Box<RawValue>>,
    /// Step id -> `{"return_value": <any>}` or `{"function": "<name>"}`.
    #[serde(default)]
    steps: HashMap<String, Map<String, Value>>,
    /// The job whose code holds the replacement functions, usually the calling test itself.
    caller_job_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReplacementCode {
    pub content: String,
    pub language: ScriptLang,
    pub lock: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum StepOverride {
    Mock(Value),
    Function { entrypoint: String, code: ReplacementCode },
}

/// An entrypoint override is only honored by these executors; elsewhere `main` would run.
fn supports_entrypoint_override(language: &ScriptLang) -> bool {
    matches!(
        language,
        ScriptLang::Bun
            | ScriptLang::Bunnative
            | ScriptLang::Deno
            | ScriptLang::Python3
            | ScriptLang::Php
    )
}

enum ParsedOverride {
    Mock(Value),
    Function(String),
}

fn parse_override(step_id: &str, o: Map<String, Value>) -> Result<ParsedOverride> {
    let bad = |msg: &str| {
        Error::BadRequest(format!(
            "step `{step_id}`: {msg}; expected {{\"return_value\": ...}} or {{\"function\": \"<name>\"}}"
        ))
    };
    if o.len() != 1 {
        return Err(bad("an override takes exactly one key"));
    }
    let (k, v) = o.into_iter().next().unwrap();
    match k.as_str() {
        "return_value" => Ok(ParsedOverride::Mock(v)),
        "function" => {
            let Value::String(name) = v else {
                return Err(bad("`function` must be a function name"));
            };
            if !is_valid_entrypoint_name(&name) {
                return Err(bad(&format!("invalid function name {name:?}")));
            }
            Ok(ParsedOverride::Function(name))
        }
        _ => Err(bad(&format!("unknown key `{k}`"))),
    }
}

/// The code a job ran: inline for a preview, the deployed script's for a script run.
async fn caller_job_code(
    db: &DB,
    user_db: &UserDB,
    authed: &ApiAuthed,
    w_id: &str,
    job_id: Uuid,
) -> Result<ReplacementCode> {
    require_job_update_read_access(db, user_db, authed, w_id, &job_id, None).await?;
    let job = sqlx::query!(
        "SELECT kind::text AS \"kind!\", runnable_id, runnable_path, raw_code, raw_lock,
                script_lang AS \"script_lang: ScriptLang\",
                COALESCE(args ? '_MODULES', false) AS \"has_modules!\"
         FROM v2_job WHERE id = $1 AND workspace_id = $2",
        job_id,
        w_id
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Job {job_id} not found")))?;

    // A flow step holds a single file, so a replacement importing a companion file would fail.
    let multi_file = || {
        Error::BadRequest(format!(
            "cannot replace a step with a function of job {job_id}: its script has companion \
             files, which a flow step cannot carry"
        ))
    };
    if let (Some(content), Some(language)) = (job.raw_code, job.script_lang) {
        if job.has_modules {
            return Err(multi_file());
        }
        return Ok(ReplacementCode {
            content,
            language,
            lock: job.raw_lock,
            path: job.runnable_path,
        });
    }
    match (job.kind.as_str(), job.runnable_id) {
        ("script", Some(hash)) => {
            // As the caller: reading a run of a script does not grant reading its source.
            let mut tx = user_db.clone().begin(authed).await?;
            let s = sqlx::query!(
                "SELECT content, lock, language AS \"language: ScriptLang\", path,
                        COALESCE(modules IS NOT NULL AND modules <> '{}'::jsonb, false)
                            AS \"has_modules!\"
                 FROM script WHERE hash = $1 AND workspace_id = $2",
                hash,
                w_id
            )
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| Error::NotFound(format!("script of job {job_id} not found")))?;
            tx.commit().await?;
            if s.has_modules {
                return Err(multi_file());
            }
            Ok(ReplacementCode {
                content: s.content,
                language: s.language,
                lock: s.lock,
                path: Some(s.path),
            })
        }
        (kind, _) => Err(Error::BadRequest(format!(
            "cannot replace a step with a function of job {job_id}: its code cannot be read \
             back from a `{kind}` job, only from a script run or a script preview"
        ))),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ModuleKind {
    Step,
    Preprocessor,
    /// An AI agent tool: it has no `mock`, and the agent reads its name and schema from the
    /// tool's own definition, so neither kind of override would take effect as written.
    Tool,
}

fn walk_list(
    list: Option<&mut Value>,
    kind: ModuleKind,
    cb: &mut dyn FnMut(ModuleKind, &mut Map<String, Value>),
) {
    if let Some(Value::Array(a)) = list {
        for m in a {
            walk_module(m, kind, cb);
        }
    }
}

/// Visits only where the flow schema puts modules: input values and mocks hold arbitrary data,
/// and a module-shaped value there must not be taken for a step.
fn walk_module(
    m: &mut Value,
    kind: ModuleKind,
    cb: &mut dyn FnMut(ModuleKind, &mut Map<String, Value>),
) {
    let Value::Object(m) = m else {
        return;
    };
    cb(kind, m);
    let Some(Value::Object(v)) = m.get_mut("value") else {
        return;
    };
    walk_list(v.get_mut("modules"), ModuleKind::Step, cb);
    walk_list(v.get_mut("default"), ModuleKind::Step, cb);
    if let Some(Value::Array(branches)) = v.get_mut("branches") {
        for b in branches {
            walk_list(b.get_mut("modules"), ModuleKind::Step, cb);
        }
    }
    walk_list(v.get_mut("tools"), ModuleKind::Tool, cb);
}

fn replacement_value(
    module: &Map<String, Value>,
    entrypoint: &str,
    code: &ReplacementCode,
) -> Value {
    let mut input_transforms = module
        .get("value")
        .and_then(|v| v.get("input_transforms"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    input_transforms.insert(
        ENTRYPOINT_OVERRIDE.to_string(),
        json!({ "type": "static", "value": entrypoint }),
    );
    let mut raw = json!({
        "type": "rawscript",
        "content": code.content,
        "language": code.language,
        "input_transforms": input_transforms,
    });
    if let Some(lock) = &code.lock {
        raw["lock"] = json!(lock);
    }
    if let Some(path) = &code.path {
        raw["path"] = json!(path);
    }
    raw
}

/// Applies `overrides` to a flow value. Every step id must exist in the flow, so a renamed step
/// fails the test rather than silently running for real.
pub(crate) fn apply_step_overrides(
    value: &mut Value,
    overrides: &HashMap<String, StepOverride>,
) -> Result<()> {
    let mut seen = HashSet::new();
    let mut refused = vec![];
    let mut cb = |kind: ModuleKind, module: &mut Map<String, Value>| {
        let Some(id) = module.get("id").and_then(Value::as_str).map(str::to_string) else {
            return;
        };
        let Some(o) = overrides.get(&id) else {
            // An editor pin kept for test runs: a step the test did not override runs for real.
            if module
                .get("mock")
                .and_then(|m| m.get("preview_only"))
                .and_then(Value::as_bool)
                == Some(true)
            {
                module.remove("mock");
            }
            return;
        };
        seen.insert(id.clone());
        match o {
            // A preview never runs the preprocessor: `args` are what the flow receives after it.
            _ if kind == ModuleKind::Preprocessor => refused.push(format!(
                "`{id}` is the flow's preprocessor, which does not run in a flow test: pass the \
                 args it would produce instead"
            )),
            _ if kind == ModuleKind::Tool => refused.push(format!(
                "`{id}` is an AI agent tool, which cannot be overridden: override the agent \
                 step instead"
            )),
            StepOverride::Mock(rv) => {
                module.insert(
                    "mock".to_string(),
                    json!({ "enabled": true, "return_value": rv }),
                );
            }
            // Called with the flow input in place of its input transforms, so the replacement's
            // entrypoint would be ignored and the test's `main` would run.
            StepOverride::Function { .. }
                if module.get("pass_flow_input_directly") == Some(&Value::Bool(true)) =>
            {
                refused.push(format!(
                    "`{id}` receives the flow input directly and cannot be replaced with a \
                     function: mock its result instead"
                ))
            }
            StepOverride::Function { entrypoint, code } => {
                let raw = replacement_value(module, entrypoint, code);
                module.insert("value".to_string(), raw);
                module.remove("mock");
            }
        }
    };
    walk_list(value.get_mut("modules"), ModuleKind::Step, &mut cb);
    if let Some(m) = value.get_mut("failure_module") {
        walk_module(m, ModuleKind::Step, &mut cb);
    }
    if let Some(m) = value.get_mut("preprocessor_module") {
        walk_module(m, ModuleKind::Preprocessor, &mut cb);
    }
    if !refused.is_empty() {
        refused.sort();
        return Err(Error::BadRequest(refused.join("; ")));
    }
    let mut missing: Vec<&str> = overrides
        .keys()
        .filter(|k| !seen.contains(*k))
        .map(String::as_str)
        .collect();
    if !missing.is_empty() {
        missing.sort();
        return Err(Error::BadRequest(format!(
            "no step with id {} in the flow",
            missing.join(", ")
        )));
    }
    Ok(())
}

/// A cached result is keyed by the flow's own value and args, which leave out the overrides of
/// its subflows, so a test could read the result of a run with other overrides.
fn disable_result_cache(value: &mut Value) {
    if let Some(v) = value.as_object_mut() {
        v.remove("cache_ttl");
    }
    let mut cb = |_: ModuleKind, module: &mut Map<String, Value>| {
        module.remove("cache_ttl");
    };
    walk_list(value.get_mut("modules"), ModuleKind::Step, &mut cb);
    if let Some(m) = value.get_mut("failure_module") {
        walk_module(m, ModuleKind::Step, &mut cb);
    }
}

/// Nesting limit of `a:b:c` step keys, so a subflow calling itself cannot recurse forever.
const MAX_SUBFLOW_DEPTH: usize = 10;

/// The deployed path of the subflow step `id` of a flow value.
fn subflow_step_path(value: &mut Value, id: &str) -> Result<String> {
    let mut found: Option<std::result::Result<String, String>> = None;
    let mut cb = |_: ModuleKind, module: &mut Map<String, Value>| {
        if module.get("id").and_then(Value::as_str) != Some(id) {
            return;
        }
        let v = module.get("value");
        found = Some(
            match (
                v.and_then(|v| v.get("type")).and_then(Value::as_str),
                v.and_then(|v| v.get("path")).and_then(Value::as_str),
            ) {
                (Some("flow"), Some(path)) => Ok(path.to_string()),
                _ => Err(format!("`{id}` is not a subflow step")),
            },
        );
    };
    walk_list(value.get_mut("modules"), ModuleKind::Step, &mut cb);
    if let Some(m) = value.get_mut("failure_module") {
        walk_module(m, ModuleKind::Step, &mut cb);
    }
    match found {
        Some(Ok(path)) => Ok(path),
        Some(Err(e)) => Err(Error::BadRequest(e)),
        None => Err(Error::BadRequest(format!(
            "no step with id {id} in the flow"
        ))),
    }
}

/// The latest deployed value and tag of a flow, read as the caller.
async fn deployed_flow_value(
    db: &DB,
    user_db: &UserDB,
    authed: &ApiAuthed,
    w_id: &str,
    path: &str,
) -> Result<(Value, Option<String>)> {
    let userdb_authed = UserDbWithAuthed { db: user_db.clone(), authed: &authed.to_authed_ref() };
    let info =
        get_latest_flow_version_info_for_path(Some(userdb_authed), db, w_id, path, false).await?;
    let stored = sqlx::query_scalar!(
        "SELECT value AS \"value: sqlx::types::Json<Value>\" FROM flow_version
         WHERE id = $1 AND workspace_id = $2",
        info.version,
        w_id
    )
    .fetch_one(db)
    .await?;
    Ok((stored.0, info.tag))
}

/// Applies `overrides` to `value`. A key `sub:inner` overrides step `inner` of the subflow that
/// step `sub` calls: that subflow's deployed value, overridden in turn, goes to `subflows` under
/// its path, which the run uses for every call of that subflow.
fn resolve_overrides<'a>(
    value: &'a mut Value,
    overrides: HashMap<String, StepOverride>,
    subflows: &'a mut HashMap<String, Value>,
    depth: usize,
    ctx: (&'a DB, &'a UserDB, &'a ApiAuthed, &'a str),
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        let mut local = HashMap::new();
        let mut nested: HashMap<String, HashMap<String, StepOverride>> = HashMap::new();
        for (key, o) in overrides {
            match key.split_once(':') {
                Some((step, rest)) => {
                    nested
                        .entry(step.to_string())
                        .or_default()
                        .insert(rest.to_string(), o);
                }
                None => {
                    local.insert(key, o);
                }
            }
        }
        if let Some(step) = nested.keys().find(|k| local.contains_key(*k)) {
            return Err(Error::BadRequest(format!(
                "`{step}` is overridden both as a whole and inside: override one or the other"
            )));
        }
        apply_step_overrides(value, &local)?;
        let mut nested = nested.into_iter().collect::<Vec<_>>();
        nested.sort_by(|a, b| a.0.cmp(&b.0));
        for (step, inner) in nested {
            if depth >= MAX_SUBFLOW_DEPTH {
                return Err(Error::BadRequest(format!(
                    "step keys nest at most {MAX_SUBFLOW_DEPTH} subflows deep"
                )));
            }
            let path = subflow_step_path(value, &step)?;
            let (db, user_db, authed, w_id) = ctx;
            let (mut sub, _) = deployed_flow_value(db, user_db, authed, w_id, &path).await?;
            resolve_overrides(&mut sub, inner, subflows, depth + 1, ctx)
                .await
                .map_err(|e| match e {
                    Error::BadRequest(m) => Error::BadRequest(format!("in subflow `{step}`: {m}")),
                    e => e,
                })?;
            serde_json::from_value::<FlowValue>(sub.clone())
                .map_err(|e| Error::internal_err(format!("overridden subflow {path}: {e}")))?;
            match subflows.get(&path) {
                Some(existing) if existing != &sub => {
                    return Err(Error::BadRequest(format!(
                        "subflow {path} is overridden differently by two steps calling it: \
                         a subflow's overrides apply to every call of it in the run"
                    )))
                }
                _ => {
                    subflows.insert(path, sub);
                }
            }
        }
        Ok(())
    })
}

pub(crate) async fn run_flow_test(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    Json(req): Json<FlowTestRequest>,
) -> Result<(StatusCode, String)> {
    // Even a mock-only test runs an edited flow value; the preview route's builder right is
    // not enough for it.
    if !role_can_code(&db, &w_id, &authed).await? {
        return Err(operator_preview_refusal(None));
    }
    let flow_path = flow_path.to_path().to_string();

    let mut parsed = HashMap::new();
    let mut needs_code = false;
    for (id, o) in req.steps {
        let p = parse_override(&id, o)?;
        needs_code |= matches!(p, ParsedOverride::Function(_));
        parsed.insert(id, p);
    }
    let code = if needs_code {
        let job_id = req.caller_job_id.ok_or_else(|| {
            Error::BadRequest(
                "replacing a step with a function needs `caller_job_id`, the job whose code \
                 defines it"
                    .to_string(),
            )
        })?;
        let code = caller_job_code(&db, &user_db, &authed, &w_id, job_id).await?;
        if !supports_entrypoint_override(&code.language) {
            return Err(Error::BadRequest(format!(
                "replacing a step with a function is not supported for {} code",
                code.language.as_str()
            )));
        }
        Some(code)
    } else {
        None
    };
    let overrides = parsed
        .into_iter()
        .map(|(id, p)| {
            let o = match (p, &code) {
                (ParsedOverride::Mock(v), _) => StepOverride::Mock(v),
                (ParsedOverride::Function(entrypoint), Some(code)) => {
                    StepOverride::Function { entrypoint, code: code.clone() }
                }
                (ParsedOverride::Function(_), None) => {
                    return Err(Error::internal_err("function override without code"))
                }
            };
            Ok((id, o))
        })
        .collect::<Result<HashMap<_, _>>>()?;

    // Read as the caller, so a flow they cannot see cannot be run through this route.
    let (mut value, tag) = deployed_flow_value(&db, &user_db, &authed, &w_id, &flow_path).await?;
    let mut subflows = HashMap::new();
    resolve_overrides(
        &mut value,
        overrides,
        &mut subflows,
        0,
        (&db, &user_db, &authed, w_id.as_str()),
    )
    .await?;
    if !subflows.is_empty() {
        windmill_common::min_version::MIN_VERSION_SUPPORTS_FLOW_TEST_SUBFLOWS
            .assert()
            .await?;
    }
    disable_result_cache(&mut value);
    for sub in subflows.values_mut() {
        disable_result_cache(sub);
    }
    let value: FlowValue = serde_json::from_value(value)
        .map_err(|e| Error::internal_err(format!("overridden flow value: {e}")))?;

    push_preview_flow_job(
        authed,
        db,
        user_db,
        w_id,
        run_query,
        PreviewFlow {
            value,
            path: Some(flow_path),
            args: Some(req.args),
            tag,
            restarted_from: None,
            temp_script_refs: None,
            on_behalf_of: None,
        },
        Some(subflows),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code() -> ReplacementCode {
        ReplacementCode {
            content: "export function fake(x) { return x }".into(),
            language: ScriptLang::Bun,
            lock: None,
            path: Some("f/tests/t".into()),
        }
    }

    #[test]
    fn overrides_nested_steps_and_keeps_input_transforms() {
        let mut v = json!({ "modules": [
            { "id": "a", "value": { "type": "rawscript", "language": "python3", "content": "",
                "input_transforms": { "x": { "type": "javascript", "expr": "flow_input.x" } } } },
            { "id": "loop", "value": { "type": "forloopflow", "iterator": { "type": "static", "value": [] },
                "modules": [ { "id": "b", "value": { "type": "identity" },
                               "mock": { "enabled": true, "return_value": 0 } } ] } }
        ]});
        let overrides = HashMap::from([
            ("a".to_string(), StepOverride::Mock(json!({ "k": 1 }))),
            (
                "b".to_string(),
                StepOverride::Function { entrypoint: "fake".into(), code: code() },
            ),
        ]);
        apply_step_overrides(&mut v, &overrides).unwrap();

        let a = &v["modules"][0];
        assert_eq!(
            a["mock"],
            json!({ "enabled": true, "return_value": { "k": 1 } })
        );
        assert_eq!(a["value"]["language"], "python3");

        let b = &v["modules"][1]["value"]["modules"][0];
        assert!(
            b.get("mock").is_none(),
            "a step's own mock would skip the replacement"
        );
        assert_eq!(b["value"]["type"], "rawscript");
        assert_eq!(b["value"]["language"], "bun");
        assert_eq!(
            b["value"]["input_transforms"][ENTRYPOINT_OVERRIDE],
            json!({ "type": "static", "value": "fake" })
        );
        serde_json::from_value::<FlowValue>(v).unwrap();
    }

    #[test]
    fn agent_tool_cannot_be_overridden() {
        let v = json!({ "modules": [ { "id": "agent", "value": { "type": "aiagent",
            "input_transforms": {}, "tools": [ { "id": "send", "summary": "send",
            "value": { "tool_type": "websearch" } } ] } } ] });
        for o in [
            StepOverride::Mock(json!(1)),
            StepOverride::Function { entrypoint: "fake".into(), code: code() },
        ] {
            let overrides = HashMap::from([("send".to_string(), o)]);
            assert!(apply_step_overrides(&mut v.clone(), &overrides).is_err());
        }
    }

    #[test]
    fn module_shaped_data_is_not_a_step() {
        let mut v = json!({ "modules": [ { "id": "a", "value": { "type": "identity" } } ] });
        let fake = json!({ "id": "a", "value": { "type": "identity" } });
        let overrides = HashMap::from([("a".to_string(), StepOverride::Mock(fake.clone()))]);
        apply_step_overrides(&mut v, &overrides).unwrap();
        assert_eq!(v["modules"][0]["mock"]["return_value"], fake);
    }

    #[test]
    fn preprocessor_cannot_be_overridden() {
        let mut v = json!({ "modules": [],
            "preprocessor_module": { "id": "preprocessor", "value": { "type": "identity" } } });
        let overrides = HashMap::from([("preprocessor".to_string(), StepOverride::Mock(json!(1)))]);
        assert!(apply_step_overrides(&mut v, &overrides).is_err());
    }

    #[test]
    fn direct_input_step_cannot_be_replaced() {
        let mut v = json!({ "modules": [ { "id": "a", "pass_flow_input_directly": true,
            "value": { "type": "identity" } } ] });
        let overrides = HashMap::from([(
            "a".to_string(),
            StepOverride::Function { entrypoint: "fake".into(), code: code() },
        )]);
        assert!(apply_step_overrides(&mut v, &overrides).is_err());
    }

    #[test]
    fn test_run_pins_are_dropped() {
        let mut v = json!({ "modules": [
            { "id": "a", "value": { "type": "identity" },
              "mock": { "enabled": true, "return_value": 1, "preview_only": true } },
            { "id": "b", "value": { "type": "identity" },
              "mock": { "enabled": true, "return_value": 2 } } ] });
        apply_step_overrides(&mut v, &HashMap::new()).unwrap();
        assert!(v["modules"][0].get("mock").is_none());
        assert_eq!(v["modules"][1]["mock"]["return_value"], 2);
    }

    #[test]
    fn subflow_step_is_found_at_any_depth() {
        let mut v = json!({ "modules": [ { "id": "loop", "value": { "type": "forloopflow",
            "iterator": { "type": "static", "value": [] }, "modules": [
              { "id": "sub", "value": { "type": "flow", "path": "f/x/sub" } },
              { "id": "s", "value": { "type": "identity" } } ] } } ] });
        assert_eq!(subflow_step_path(&mut v, "sub").unwrap(), "f/x/sub");
        assert!(subflow_step_path(&mut v, "s").is_err());
        assert!(subflow_step_path(&mut v, "nope").is_err());
    }

    #[test]
    fn result_cache_is_disabled_at_every_level() {
        let mut v = json!({ "cache_ttl": 60, "modules": [ { "id": "loop", "cache_ttl": 60,
            "value": { "type": "forloopflow", "iterator": { "type": "static", "value": [] },
            "modules": [ { "id": "s", "cache_ttl": 60, "value": { "type": "identity" } } ] } } ] });
        disable_result_cache(&mut v);
        assert!(v.get("cache_ttl").is_none());
        assert!(v["modules"][0].get("cache_ttl").is_none());
        assert!(v["modules"][0]["value"]["modules"][0].get("cache_ttl").is_none());
    }

    #[test]
    fn unknown_step_is_an_error() {
        let mut v = json!({ "modules": [ { "id": "a", "value": { "type": "identity" } } ] });
        let overrides = HashMap::from([("nope".to_string(), StepOverride::Mock(json!(1)))]);
        assert!(apply_step_overrides(&mut v, &overrides).is_err());
    }
}
