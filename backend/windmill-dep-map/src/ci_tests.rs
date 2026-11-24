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

/// `promote_from` on a deploy endpoint: the fork of the target workspace to copy the item from.
/// Under `RequirePassingCiTests` the server then reads the item from that fork itself and uses the
/// request only for what belongs to the target (lineage, the principal it runs as).
#[derive(serde::Deserialize, Default, Clone)]
pub struct PromoteQuery {
    pub promote_from: Option<String>,
}

/// A deploy whose content the server copied from `fork`, where it was last deployed at
/// `deployed_at`.
pub struct Promotion {
    pub fork: String,
    pub deployed_at: chrono::DateTime<chrono::Utc>,
}

pub async fn ci_tests_rule_active(
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
) -> error::Result<bool> {
    use windmill_common::workspaces::{get_protection_rules, ProtectionRules};
    Ok(get_protection_rules(w_id, db)
        .await?
        .iter()
        .any(|r| r.rules.contains(ProtectionRules::REQUIRE_PASSING_CI_TESTS)))
}

/// Whether the caller may write past `RequirePassingCiTests` in `w_id`.
pub async fn may_bypass_ci_tests_rule(
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
    username: &str,
    groups: &[String],
    is_admin: bool,
) -> error::Result<bool> {
    use windmill_common::workspaces::{
        check_user_against_rule, ProtectionRuleKind, RuleCheckResult,
    };
    Ok(matches!(
        check_user_against_rule(
            w_id,
            &ProtectionRuleKind::RequirePassingCiTests,
            username,
            groups,
            is_admin,
            db
        )
        .await?,
        RuleCheckResult::Allowed
    ))
}

/// Enforces `RequirePassingCiTests` on a write of `kind` at `path` in `w_id`. An item the
/// workspace's CI tests cover deploys only as a promotion from a fork whose latest runs of every
/// one of those tests passed on what it copied, with the same test code. A test is held to the
/// same, on its own runs there: whoever could rewrite one in place could switch the rule off. A
/// user who may bypass the rule writes anyway, and a write that would have been refused is
/// audited. A rename also answers to the tests of the path it leaves, since their references
/// follow it.
pub async fn check_ci_tests_rule(
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
    kind: &str,
    path: &str,
    renamed_from: Option<&str>,
    promotion: Option<&Promotion>,
    author: &impl windmill_audit::audit_oss::AuditAuthorable,
    groups: &[String],
    is_admin: bool,
) -> error::Result<()> {
    use windmill_common::workspaces::{
        check_user_against_rule, ProtectionRuleKind, RuleCheckResult,
    };
    if !ci_tests_rule_active(db, w_id).await? {
        return Ok(());
    }
    let renamed_from = renamed_from.filter(|p| *p != path);
    let Some(failure) = ci_tests_failure(db, w_id, kind, path, renamed_from, promotion).await?
    else {
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
                Some(&format!("{kind}/{path}")),
                Some([("reason", failure.as_str())].into()),
            )
            .await?;
            Ok(())
        }
    }
}

/// Why this deploy has not passed `w_id`'s CI tests, or `None` when it has, or when no test
/// covers it.
async fn ci_tests_failure(
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
    kind: &str,
    path: &str,
    renamed_from: Option<&str>,
    promotion: Option<&Promotion>,
) -> error::Result<Option<String>> {
    let refs = sqlx::query!(
        "SELECT test_script_path, tested_item_path, has_wildcard AS \"has_wildcard!\" \
         FROM ci_test_reference \
         WHERE workspace_id = $1 AND tested_item_kind = $2 \
           AND (has_wildcard OR tested_item_path = $3 OR tested_item_path = $4)",
        w_id,
        kind,
        path,
        renamed_from
    )
    .fetch_all(db)
    .await?;
    let tests: BTreeSet<String> =
        refs.into_iter()
            .filter(|r| {
                !r.has_wildcard
                    || [Some(path), renamed_from].into_iter().flatten().any(|p| {
                        windmill_common::schema::ci_test_path_matches(p, &r.tested_item_path)
                    })
            })
            .map(|r| r.test_script_path)
            .collect();
    let is_test = kind == "script"
        && sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM ci_test_reference \
             WHERE workspace_id = $1 AND (test_script_path = $2 OR test_script_path = $3))",
            w_id,
            path,
            renamed_from
        )
        .fetch_one(db)
        .await?
        .unwrap_or(false);
    if tests.is_empty() && !is_test {
        return Ok(None);
    }
    let Some(p) = promotion else {
        return Ok(Some(format!(
            "{kind} {path} has CI tests or is one, so it deploys here only when promoted from a \
             fork where they passed"
        )));
    };
    if !tests.is_empty() {
        if let Some(f) =
            fork_tests_failure(db, w_id, &p.fork, kind, path, &tests, p.deployed_at).await?
        {
            return Ok(Some(f));
        }
    }
    if is_test {
        return own_runs_failure(db, &p.fork, path, p.deployed_at).await;
    }
    Ok(None)
}

/// Why the runs of the test at `path` in `fork` since it was deployed there at `deployed_at` do
/// not vouch for it, or `None` when the latest one for each item it tests passed.
async fn own_runs_failure(
    db: &sqlx::Pool<sqlx::Postgres>,
    fork: &str,
    path: &str,
    deployed_at: chrono::DateTime<chrono::Utc>,
) -> error::Result<Option<String>> {
    let runs = sqlx::query!(
        "SELECT DISTINCT ON (j.trigger) j.trigger, jc.status::text AS status \
         FROM v2_job j LEFT JOIN v2_job_completed jc ON jc.id = j.id \
         WHERE j.workspace_id = $1 AND j.runnable_path = $2 AND j.trigger_kind = 'ci_test' \
           AND j.created_at >= $3 \
         ORDER BY j.trigger, j.created_at DESC",
        fork,
        path,
        deployed_at
    )
    .fetch_all(db)
    .await?;
    if runs.is_empty() {
        return Ok(Some(format!(
            "CI test {path} has not run since its last deploy in fork {fork}"
        )));
    }
    Ok(runs.into_iter().find_map(|r| {
        let failure = match r.status.as_deref() {
            Some("success") => return None,
            None => "is still running",
            Some(_) => "failed",
        };
        Some(format!(
            "CI test {path} {failure} on {} in fork {fork}",
            r.trigger.unwrap_or_default()
        ))
    }))
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
                    s.content AS \"ran_content?\", s.language::text AS ran_language, \
                    s.modules AS ran_modules, s.codebase AS ran_codebase, \
                    t.content AS \"own_content?\", t.language::text AS own_language, \
                    t.modules AS own_modules, t.codebase AS own_codebase \
             FROM v2_job j \
             LEFT JOIN v2_job_completed jc ON jc.id = j.id \
             LEFT JOIN script s ON s.workspace_id = j.workspace_id AND s.hash = j.runnable_id \
             LEFT JOIN LATERAL ( \
                 SELECT content, language, modules, codebase FROM script \
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
            Some(r)
                if r.ran_content.is_none()
                    || r.ran_content != r.own_content
                    || r.ran_language != r.own_language
                    || r.ran_codebase != r.own_codebase
                    || without_locks(&r.ran_modules) != without_locks(&r.own_modules) =>
            {
                "ran with test code that differs from this workspace's"
            }
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

/// A script's modules with their locks, which each workspace resolves on its own.
fn without_locks(modules: &Option<serde_json::Value>) -> Option<serde_json::Value> {
    let mut modules = modules.clone()?;
    if let Some(m) = modules.as_object_mut() {
        m.values_mut()
            .filter_map(|v| v.as_object_mut())
            .for_each(|v| {
                v.remove("lock");
            });
    }
    Some(modules)
}
