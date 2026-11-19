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
use crate::jobs::{require_job_update_read_access, run_preview_flow_job, PreviewFlow, RunJobQuery};

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
            | ScriptLang::Nativets
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
                script_lang AS \"script_lang: ScriptLang\"
         FROM v2_job WHERE id = $1 AND workspace_id = $2",
        job_id,
        w_id
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Job {job_id} not found")))?;

    if let (Some(content), Some(language)) = (job.raw_code, job.script_lang) {
        return Ok(ReplacementCode {
            content,
            language,
            lock: job.raw_lock,
            path: job.runnable_path,
        });
    }
    match (job.kind.as_str(), job.runnable_id) {
        ("script", Some(hash)) => {
            let s = sqlx::query!(
                "SELECT content, lock, language AS \"language: ScriptLang\", path
                 FROM script WHERE hash = $1 AND workspace_id = $2",
                hash,
                w_id
            )
            .fetch_optional(db)
            .await?
            .ok_or_else(|| Error::NotFound(format!("script of job {job_id} not found")))?;
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

/// Every object holding a string `id` and an object `value` with a `type`, at any depth: steps
/// inside loops, branches, failure and preprocessor modules, and AI agent tools alike.
fn for_each_module(v: &mut Value, cb: &mut dyn FnMut(&str, &mut Map<String, Value>)) {
    match v {
        Value::Object(m) => {
            let id = match (m.get("id"), m.get("value")) {
                (Some(Value::String(id)), Some(Value::Object(inner)))
                    if inner.get("type").is_some_and(Value::is_string) =>
                {
                    Some(id.clone())
                }
                _ => None,
            };
            if let Some(id) = id {
                cb(&id, m);
            }
            for child in m.values_mut() {
                for_each_module(child, cb);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|c| for_each_module(c, cb)),
        _ => {}
    }
}

/// Applies `overrides` to a flow value. Every step id must exist in the flow, so a renamed step
/// fails the test rather than silently running for real.
pub(crate) fn apply_step_overrides(
    value: &mut Value,
    overrides: &HashMap<String, StepOverride>,
) -> Result<()> {
    let mut seen = HashSet::new();
    for_each_module(value, &mut |id, module| {
        let Some(o) = overrides.get(id) else {
            return;
        };
        seen.insert(id.to_string());
        match o {
            StepOverride::Mock(rv) => {
                module.insert(
                    "mock".to_string(),
                    json!({ "enabled": true, "return_value": rv }),
                );
            }
            StepOverride::Function { entrypoint, code } => {
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
                module.insert("value".to_string(), raw);
                module.remove("mock");
            }
        }
    });
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

pub(crate) async fn run_flow_test(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    Json(req): Json<FlowTestRequest>,
) -> Result<(StatusCode, String)> {
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
    let userdb_authed = UserDbWithAuthed { db: user_db.clone(), authed: &authed.to_authed_ref() };
    let info =
        get_latest_flow_version_info_for_path(Some(userdb_authed), &db, &w_id, &flow_path, false)
            .await?;
    let stored = sqlx::query_scalar!(
        "SELECT value AS \"value: sqlx::types::Json<Value>\" FROM flow_version
         WHERE id = $1 AND workspace_id = $2",
        info.version,
        w_id
    )
    .fetch_one(&db)
    .await?;
    let mut value = stored.0;
    apply_step_overrides(&mut value, &overrides)?;
    let value: FlowValue = serde_json::from_value(value)
        .map_err(|e| Error::internal_err(format!("overridden flow value: {e}")))?;

    run_preview_flow_job(
        authed,
        Extension(db),
        Extension(user_db),
        Path(w_id),
        Query(run_query),
        Json(PreviewFlow {
            value,
            path: Some(flow_path),
            args: Some(req.args),
            tag: info.tag,
            restarted_from: None,
            temp_script_refs: None,
            on_behalf_of: None,
        }),
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
    fn unknown_step_is_an_error() {
        let mut v = json!({ "modules": [ { "id": "a", "value": { "type": "identity" } } ] });
        let overrides = HashMap::from([("nope".to_string(), StepOverride::Mock(json!(1)))]);
        assert!(apply_step_overrides(&mut v, &overrides).is_err());
    }
}
