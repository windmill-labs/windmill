#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::ci_tests_ee::*;

use std::collections::BTreeSet;
use std::time::Duration;

use uuid::Uuid;
use windmill_common::error;

use crate::scoped_dependency_map::ScopedDependencyMap;

/// `skip_ci_tests` on a deploy endpoint. A caller deploying several items in a row sets it
/// and queues the CI tests once the whole batch has landed (`trigger_ci_tests_for_batch`): a
/// test run between two of its deploys resolves imports against a half-deployed workspace.
#[derive(serde::Deserialize, Default, Clone, Copy)]
pub struct SkipCiTestsQuery {
    #[serde(default)]
    pub skip_ci_tests: bool,
}

/// How long after a deploy that brings its own lock the scripts importing it are relocked:
/// that deploy queues no dependency job, so until then nothing shows the relock is coming.
pub const LOCKED_DEPLOY_RELOCK_DELAY: Duration = Duration::from_secs(10);

#[derive(serde::Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CiTestBatchItem {
    pub kind: String,
    pub path: String,
}

#[derive(serde::Serialize, Default)]
pub struct CiTestBatchOutcome {
    /// Paths with a dependency job still queued or running. When non-empty and the caller
    /// did not ask to ignore it, nothing was queued.
    pub pending: Vec<String>,
    pub job_ids: Vec<Uuid>,
}

/// Queue the CI tests of a batch deployed with `skip_ci_tests`, once nothing those tests read
/// is still being locked. The relocks a deployed script causes in the scripts importing it
/// carry the flag too, so those importers' tests are part of the batch.
///
/// Checks no permission and queues the tests as `email`/`username`: the caller must have
/// verified that this identity may deploy each of `items`, as a deploy would.
pub async fn trigger_ci_tests_for_batch(
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
    items: Vec<CiTestBatchItem>,
    ignore_pending: bool,
    email: &str,
    username: &str,
) -> error::Result<CiTestBatchOutcome> {
    let mut items: BTreeSet<CiTestBatchItem> = items
        .into_iter()
        .filter(|i| matches!(i.kind.as_str(), "script" | "flow" | "resource"))
        .collect();

    let mut frontier: Vec<String> = items
        .iter()
        .filter(|i| i.kind == "script")
        .map(|i| i.path.clone())
        .collect();
    let mut relocks: Vec<(String, String)> = vec![];
    while let Some(path) = frontier.pop() {
        for d in ScopedDependencyMap::get_dependents(&path, w_id, db).await? {
            if d.importer_kind != "script" {
                continue;
            }
            relocks.push((path.clone(), d.importer_path.clone()));
            if items.insert(CiTestBatchItem {
                kind: "script".to_string(),
                path: d.importer_path.clone(),
            }) {
                frontier.push(d.importer_path);
            }
        }
    }

    let refs = sqlx::query!(
        "SELECT test_script_path, tested_item_path, tested_item_kind, has_wildcard AS \"has_wildcard!\" \
         FROM ci_test_reference WHERE workspace_id = $1",
        w_id
    )
    .fetch_all(db)
    .await?;
    // What a test run reads: the tested item, the test script and everything it imports.
    // Any of them still being locked would have the test run on its previous lock.
    let mut watched: BTreeSet<String> = BTreeSet::new();
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut to_visit: Vec<String> = vec![];
    for r in refs {
        let mut tested = items
            .iter()
            .filter(|i| {
                i.kind == r.tested_item_kind
                    && if r.has_wildcard {
                        windmill_common::schema::ci_test_path_matches(&i.path, &r.tested_item_path)
                    } else {
                        i.path == r.tested_item_path
                    }
            })
            .peekable();
        if tested.peek().is_none() {
            continue;
        }
        watched.extend(tested.map(|i| i.path.clone()));
        if visited.insert(r.test_script_path.clone()) {
            to_visit.push(r.test_script_path);
        }
    }
    // Waiting on the batch's locks is only worth it when there is a test to run after them.
    if to_visit.is_empty() {
        return Ok(CiTestBatchOutcome::default());
    }
    while let Some(path) = to_visit.pop() {
        let imported = sqlx::query_scalar!(
            "SELECT DISTINCT imported_path FROM dependency_map \
             WHERE workspace_id = $1 AND importer_path = $2 AND importer_kind = 'script'",
            w_id,
            &path
        )
        .fetch_all(db)
        .await?;
        for p in imported {
            if visited.insert(p.clone()) {
                to_visit.push(p);
            }
        }
    }
    watched.extend(visited);

    let watched: Vec<String> = watched.into_iter().collect();
    let mut pending = sqlx::query_scalar!(
        "SELECT DISTINCT j.runnable_path AS \"path!\" FROM v2_job_queue q
         JOIN v2_job j ON j.id = q.id
         WHERE q.workspace_id = $1
           AND j.kind IN ('dependencies', 'flowdependencies')
           AND j.runnable_path = ANY($2)",
        w_id,
        &watched
    )
    .fetch_all(db)
    .await?;
    // A relock still waiting out its delay is in no queue yet: count its source as pending.
    let relocking: Vec<String> = relocks
        .into_iter()
        .filter(|(_, importer)| watched.contains(importer))
        .map(|(source, _)| source)
        .collect();
    let recently_deployed = sqlx::query_scalar!(
        "SELECT DISTINCT path FROM script WHERE workspace_id = $1 AND path = ANY($2) \
         AND created_at > now() - make_interval(secs => $3)",
        w_id,
        &relocking,
        (LOCKED_DEPLOY_RELOCK_DELAY + Duration::from_secs(5)).as_secs_f64()
    )
    .fetch_all(db)
    .await?;
    for path in recently_deployed {
        if !pending.contains(&path) {
            pending.push(path);
        }
    }
    if !pending.is_empty() && !ignore_pending {
        return Ok(CiTestBatchOutcome { pending, job_ids: vec![] });
    }

    let mut job_ids = vec![];
    for item in items {
        job_ids.extend(
            trigger_ci_tests_for_item(db, w_id, &item.path, &item.kind, email, username).await?,
        );
    }
    Ok(CiTestBatchOutcome { pending, job_ids })
}

#[cfg(not(feature = "private"))]
pub async fn trigger_ci_tests_for_item(
    _db: &sqlx::Pool<sqlx::Postgres>,
    _w_id: &str,
    _item_path: &str,
    _item_kind: &str,
    _email: &str,
    _username: &str,
) -> error::Result<Vec<Uuid>> {
    Ok(vec![])
}

#[cfg(not(feature = "private"))]
pub async fn trigger_all_ci_tests(
    _db: &sqlx::Pool<sqlx::Postgres>,
    _w_id: &str,
    _email: &str,
    _username: &str,
) -> error::Result<Vec<Uuid>> {
    Ok(vec![])
}

/// What a deploy writes at its path, as `RequirePassingCiTests` compares it with a fork's.
pub enum DeployedContent<'a> {
    Script {
        content: &'a str,
        modules: Option<serde_json::Value>,
    },
    Flow(&'a serde_json::value::RawValue),
    /// `None` when the write clears the value.
    Resource(Option<&'a serde_json::value::RawValue>),
}

impl DeployedContent<'_> {
    fn kind(&self) -> &'static str {
        match self {
            DeployedContent::Script { .. } => "script",
            DeployedContent::Flow(_) => "flow",
            DeployedContent::Resource(_) => "resource",
        }
    }
}

/// Enforces `RequirePassingCiTests` on a deploy of `path` in `w_id`. The workspace's own CI tests
/// of the item must all have passed, in a fork of it holding the same content, on that content
/// and with the same test code. A user who may bypass the rule deploys anyway, and a deploy that
/// would have been refused is audited.
pub async fn check_ci_tests_rule(
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
    path: &str,
    content: DeployedContent<'_>,
    author: &impl windmill_audit::audit_oss::AuditAuthorable,
    groups: &[String],
    is_admin: bool,
) -> error::Result<()> {
    use windmill_common::workspaces::{
        check_user_against_rule, get_protection_rules, ProtectionRuleKind, ProtectionRules,
        RuleCheckResult,
    };
    let rulesets = get_protection_rules(w_id, db).await?;
    if !rulesets
        .iter()
        .any(|r| r.rules.contains(ProtectionRules::REQUIRE_PASSING_CI_TESTS))
    {
        return Ok(());
    }
    let Some(failure) = ci_tests_failure(db, w_id, path, &content).await? else {
        return Ok(());
    };
    match check_user_against_rule(
        w_id,
        &ProtectionRuleKind::RequirePassingCiTests,
        author.username(),
        groups,
        is_admin,
        db,
    )
    .await?
    {
        RuleCheckResult::Blocked(msg) => {
            Err(error::Error::PermissionDenied(format!("{msg}. {failure}")))
        }
        RuleCheckResult::Allowed => {
            windmill_audit::audit_oss::audit_log(
                db,
                author,
                "ci_tests.bypass",
                windmill_audit::ActionKind::Update,
                w_id,
                Some(&format!("{}/{path}", content.kind())),
                Some([("reason", failure.as_str())].into()),
            )
            .await?;
            Ok(())
        }
    }
}

/// Why `content` at `path` has not passed `w_id`'s CI tests in any fork, or `None` when it has,
/// or when no test covers it.
async fn ci_tests_failure(
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
    path: &str,
    content: &DeployedContent<'_>,
) -> error::Result<Option<String>> {
    let kind = content.kind();
    let refs = sqlx::query!(
        "SELECT test_script_path, tested_item_path, has_wildcard AS \"has_wildcard!\" \
         FROM ci_test_reference \
         WHERE workspace_id = $1 AND tested_item_kind = $2 \
           AND (has_wildcard OR tested_item_path = $3)",
        w_id,
        kind,
        path
    )
    .fetch_all(db)
    .await?;
    let tests: BTreeSet<String> = refs
        .into_iter()
        .filter(|r| {
            !r.has_wildcard
                || windmill_common::schema::ci_test_path_matches(path, &r.tested_item_path)
        })
        .map(|r| r.test_script_path)
        .collect();
    if tests.is_empty() {
        return Ok(None);
    }

    let forks = sqlx::query_scalar!(
        "SELECT id FROM workspace WHERE parent_workspace_id = $1 AND deleted = false ORDER BY id",
        w_id
    )
    .fetch_all(db)
    .await?;
    let raw = match content {
        DeployedContent::Script { .. } => None,
        DeployedContent::Flow(v) => Some(Some(*v)),
        DeployedContent::Resource(v) => Some(*v),
    };
    let value = raw
        .map(|v| {
            v.map_or(Ok(serde_json::Value::Null), |v| {
                serde_json::from_str(v.get())
            })
            .map_err(|e| error::Error::BadRequest(format!("invalid {kind} value: {e}")))
        })
        .transpose()?;
    let mut failure = format!("No fork of {w_id} holds this version of {kind} {path}");
    for fork in forks {
        let Some(deployed_at) = fork_deployed_at(db, &fork, path, content, value.as_ref()).await?
        else {
            continue;
        };
        match fork_tests_failure(db, w_id, &fork, kind, path, &tests, deployed_at).await? {
            None => return Ok(None),
            Some(f) => failure = f,
        }
    }
    Ok(Some(failure))
}

/// When `fork` last deployed `path`, if what it holds there is `content` (`value` being its JSON,
/// parsed, for a flow or a resource).
async fn fork_deployed_at(
    db: &sqlx::Pool<sqlx::Postgres>,
    fork: &str,
    path: &str,
    content: &DeployedContent<'_>,
    value: Option<&serde_json::Value>,
) -> error::Result<Option<chrono::DateTime<chrono::Utc>>> {
    Ok(match (content, value) {
        (DeployedContent::Script { content: c, modules }, _) => sqlx::query!(
            "SELECT content, modules, created_at FROM script \
             WHERE workspace_id = $1 AND path = $2 AND deleted = false AND archived = false \
             ORDER BY created_at DESC LIMIT 1",
            fork,
            path
        )
        .fetch_optional(db)
        .await?
        .filter(|s| s.content == *c && s.modules == *modules)
        .map(|s| s.created_at),
        (DeployedContent::Flow(_), Some(v)) => sqlx::query!(
            "SELECT f.value AS \"value: sqlx::types::Json<serde_json::Value>\", \
                    (SELECT max(created_at) FROM flow_version \
                     WHERE workspace_id = $1 AND path = $2) AS deployed_at \
             FROM flow f WHERE f.workspace_id = $1 AND f.path = $2 AND f.archived = false",
            fork,
            path
        )
        .fetch_optional(db)
        .await?
        .filter(|f| same_flow(&f.value.0, v))
        .and_then(|f| f.deployed_at),
        (DeployedContent::Resource(_), Some(v)) => sqlx::query!(
            "SELECT value AS \"value: sqlx::types::Json<serde_json::Value>\", resource_type, \
                    edited_at \
             FROM resource WHERE workspace_id = $1 AND path = $2",
            fork,
            path
        )
        .fetch_optional(db)
        .await?
        .filter(|r| {
            let fork_value = r.value.as_ref().map_or(&serde_json::Value::Null, |v| &v.0);
            same_resource(&r.resource_type, fork_value, v)
        })
        .and_then(|r| r.edited_at),
        _ => None,
    })
}

/// Why the latest run of one of `tests` for `kind/path` in `fork` does not vouch for what it
/// deployed at `deployed_at`, or `None` when every one does.
async fn fork_tests_failure(
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
    fork: &str,
    kind: &str,
    path: &str,
    tests: &BTreeSet<String>,
    deployed_at: chrono::DateTime<chrono::Utc>,
) -> error::Result<Option<String>> {
    let trigger = format!("{kind}/{path}");
    for test in tests {
        let run = sqlx::query!(
            "SELECT j.created_at, jc.status::text AS status, \
                    (s.content IS NOT DISTINCT FROM t.content \
                     AND s.modules IS NOT DISTINCT FROM t.modules) AS \"same_test!\" \
             FROM v2_job j \
             LEFT JOIN v2_job_completed jc ON jc.id = j.id \
             LEFT JOIN script s ON s.workspace_id = j.workspace_id AND s.hash = j.runnable_id \
             LEFT JOIN LATERAL ( \
                 SELECT content, modules FROM script \
                 WHERE workspace_id = $4 AND path = $2 AND deleted = false AND archived = false \
                 ORDER BY created_at DESC LIMIT 1 \
             ) t ON true \
             WHERE j.workspace_id = $1 AND j.runnable_path = $2 \
               AND j.trigger_kind = 'ci_test' AND j.trigger = $3 \
             ORDER BY j.created_at DESC LIMIT 1",
            fork,
            test,
            &trigger,
            w_id
        )
        .fetch_optional(db)
        .await?;
        let failure = match run {
            None => "has not run",
            Some(r) if r.created_at < deployed_at => "has not run since its last deploy",
            Some(r) if !r.same_test => "ran with test code that differs from this workspace's",
            Some(r) => match r.status.as_deref() {
                None => "is still running",
                Some("success") => continue,
                Some(_) => "failed",
            },
        };
        return Ok(Some(format!(
            "CI test {test} of {kind} {path} {failure} in fork {fork}"
        )));
    }
    Ok(None)
}

/// A flow carries over to its parent without the hashes of its script steps, and its locks
/// are recomputed per workspace: neither tells two versions of its logic apart. Step arguments
/// are user data, compared as they are even where they look like a step.
fn same_flow(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    fn strip(v: &serde_json::Value) -> serde_json::Value {
        match v {
            serde_json::Value::Object(m) => {
                let step = matches!(
                    m.get("type").and_then(|t| t.as_str()),
                    Some("rawscript" | "script" | "flow")
                );
                serde_json::Value::Object(
                    m.iter()
                        .filter(|(k, _)| !(step && matches!(k.as_str(), "hash" | "lock")))
                        .map(|(k, v)| {
                            let v = if k == "input_transforms" {
                                v.clone()
                            } else {
                                strip(v)
                            };
                            (k.clone(), v)
                        })
                        .collect(),
                )
            }
            serde_json::Value::Array(a) => serde_json::Value::Array(a.iter().map(strip).collect()),
            v => v.clone(),
        }
    }
    strip(a) == strip(b)
}

/// An agent carries over to its parent with the principal it runs as replaced.
fn same_resource(fork_type: &str, a: &serde_json::Value, b: &serde_json::Value) -> bool {
    if fork_type != "ai_agent" {
        return a == b;
    }
    let strip = |v: &serde_json::Value| {
        let mut v = v.clone();
        if let Some(m) = v.as_object_mut() {
            m.remove("on_behalf_of");
        }
        v
    };
    strip(a) == strip(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn same_flow_ignores_step_hashes_and_locks_only() {
        let fork = json!({"modules": [{"id": "a", "value": {"type": "rawscript", "content": "x",
            "lock": "l1", "input_transforms": {"lock": {"type": "static", "value": 1}}}},
            {"id": "b", "value": {"type": "script", "path": "f/s", "hash": "h1"}}]});
        let merged = json!({"modules": [{"id": "a", "value": {"type": "rawscript", "content": "x",
            "input_transforms": {"lock": {"type": "static", "value": 1}}}},
            {"id": "b", "value": {"type": "script", "path": "f/s"}}]});
        assert!(same_flow(&fork, &merged));
        let mut changed_arg = merged.clone();
        changed_arg["modules"][0]["value"]["input_transforms"]["lock"]["value"] = json!(2);
        assert!(!same_flow(&fork, &changed_arg));
        let mut changed_code = merged.clone();
        changed_code["modules"][0]["value"]["content"] = json!("y");
        assert!(!same_flow(&fork, &changed_code));
        let (mut fork_arg, mut merged_arg) = (fork.clone(), merged);
        fork_arg["modules"][0]["value"]["input_transforms"]["lock"]["value"] =
            json!({"type": "script", "hash": "v1"});
        merged_arg["modules"][0]["value"]["input_transforms"]["lock"]["value"] =
            json!({"type": "script", "hash": "v2"});
        assert!(!same_flow(&fork_arg, &merged_arg));
    }

    #[test]
    fn same_resource_ignores_agent_principal() {
        let fork = json!({"model": "m", "on_behalf_of": "u/a"});
        let other_principal = json!({"model": "m", "on_behalf_of": "u/b"});
        assert!(same_resource("ai_agent", &fork, &other_principal));
        assert!(!same_resource("custom", &fork, &other_principal));
        assert!(!same_resource("ai_agent", &fork, &json!({"model": "n"})));
    }
}
