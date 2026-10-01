use uuid::Uuid;

use crate::{db::DB, error::Result, jobs::JobKind, scripts::ScriptHash};

/// Where a job's code comes from, as far as its chain of parents can prove it.
pub struct JobProvenance {
    /// True only when the job and every ancestor run code at a path taken from a
    /// deployed version, never from the request that created the job.
    pub deployed: bool,
    /// Deployed, and every script and flow in the chain runs the version its path
    /// currently resolves to. Any past version still counts as deployed, and a run started
    /// before a redeploy stops being latest once the redeploy lands. App runs never count:
    /// app scripts are not tied to an app version.
    pub latest: bool,
    /// The job's own script hash (hex) or flow version id; `None` for other kinds.
    pub version: Option<String>,
    /// The chain reached a parent it cannot read (other workspace, depth cap), so
    /// nothing above that point is known.
    pub unproven_ancestry: bool,
    /// The item paths the chain took from a request, top-most first: for the job to act as
    /// them, its identity must be one that could have deployed there. A job whose code
    /// and path both derive from a claimed parent adds no claim of its own.
    pub claimed_paths: Vec<ClaimedPath>,
    /// The identity the job runs as.
    pub permissioned_as: String,
    pub parent_path: Option<String>,
    pub root_id: Uuid,
    pub root_path: Option<String>,
    pub root_kind: JobKind,
    pub root_trigger_kind: Option<String>,
    pub root_version: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ClaimedPath {
    /// The deployable item the claim stands for: an app script's claim is its app.
    pub path: String,
    pub item: ClaimedItem,
    /// The identity of the job that made the claim: a deployed child can run as another
    /// identity (`on_behalf_of`) than the preview it runs under.
    pub permissioned_as: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ClaimedItem {
    Script,
    /// A preview without a parent at `<app>/<component>`: deploying the app runs its
    /// inline script at that path with the same identity, so writing the app suffices.
    ScriptOrAppComponent,
    Flow,
    App,
}

struct LineageJob {
    id: Uuid,
    parent_job: Option<Uuid>,
    kind: JobKind,
    runnable_path: Option<String>,
    runnable_id: Option<i64>,
    trigger_kind: Option<String>,
    permissioned_as: String,
    origin_verified: bool,
    current_version: bool,
    app_stamped: bool,
    args_modules: bool,
}

/// Reads any job of `w_id` regardless of the caller: authorize access to `job_id` first.
pub async fn job_provenance(db: &DB, job_id: &Uuid, w_id: &str) -> Result<Option<JobProvenance>> {
    let lineage = sqlx::query_as!(
        LineageJob,
        r#"WITH RECURSIVE lineage AS (
            SELECT id, parent_job, kind, runnable_path, runnable_id, trigger_kind, trigger,
                permissioned_as, args, 0 AS depth
            FROM v2_job WHERE id = $1 AND workspace_id = $2
          UNION ALL
            SELECT p.id, p.parent_job, p.kind, p.runnable_path, p.runnable_id, p.trigger_kind,
                p.trigger, p.permissioned_as, p.args, l.depth + 1
            FROM v2_job p JOIN lineage l ON p.id = l.parent_job
            WHERE p.workspace_id = $2 AND l.depth < 100
        )
        SELECT id AS "id!", parent_job, kind AS "kind!: JobKind", runnable_path, runnable_id,
            trigger_kind::text AS trigger_kind, permissioned_as AS "permissioned_as!",
            -- A restart takes its flow version from the request, so a trusted flow path
            -- can carry another flow's code: the version must belong to that path.
            CASE kind
                WHEN 'flow' THEN EXISTS (SELECT 1 FROM flow_version fv
                    WHERE fv.id = runnable_id AND fv.path = runnable_path AND fv.workspace_id = $2)
                WHEN 'script' THEN EXISTS (SELECT 1 FROM script s
                    WHERE s.hash = runnable_id AND s.path = runnable_path AND s.workspace_id = $2
                        AND NOT s.deleted)
                ELSE true
            END AS "origin_verified!",
            -- What `run/f` and `run/p` resolve the path to now; for scripts the predicate of
            -- `get_latest_deployed_script_hash`.
            COALESCE(CASE kind
                WHEN 'flow' THEN runnable_id = (SELECT f.versions[array_upper(f.versions, 1)]
                    FROM flow f WHERE f.path = runnable_path AND f.workspace_id = $2)
                WHEN 'script' THEN runnable_id = (SELECT s.hash FROM script s
                    WHERE s.path = runnable_path AND s.workspace_id = $2 AND NOT s.deleted
                        AND s.lock IS NOT NULL AND s.lock_error_logs IS NULL
                    ORDER BY s.created_at DESC LIMIT 1)
            END, false) AS "current_version!",
            -- Only deployed-app runs are stamped with their app; an app editor preview
            -- runs app code at an app path it does not have to own.
            COALESCE(trigger_kind = 'app' AND starts_with(runnable_path, trigger || '/'), false)
                AS "app_stamped!",
            -- A preview's modules come from its args, which its parent may have taken from
            -- the caller.
            COALESCE(jsonb_typeof(args->'_MODULES') = 'object', false) AS "args_modules!"
        FROM lineage ORDER BY depth"#,
        job_id,
        w_id
    )
    .fetch_all(db)
    .await?;

    let Some(root) = lineage.last() else {
        return Ok(None);
    };
    // An ancestor missing from v2_job (other workspace, depth cap) leaves the chain unproven.
    let unproven_ancestry = root.parent_job.is_some();

    // Walk from the root down: whether a job runs stored code can depend on its parent.
    let mut claimed_paths = vec![];
    let mut all_stored = true;
    let mut all_current = true;
    let mut parent: Option<(&LineageJob, bool)> = None;
    for job in lineage.iter().rev() {
        all_current &= runs_current_version(job);
        let stored = runs_stored_code(job, parent);
        if !stored {
            all_stored = false;
            if !derives_from_claimed_parent(job, parent) {
                if let Some(claim) = claimed_path(job, parent.is_none()) {
                    claimed_paths.push(claim);
                }
            }
        }
        parent = Some((job, stored));
    }

    let deployed = !unproven_ancestry && all_stored;
    Ok(Some(JobProvenance {
        deployed,
        latest: deployed && all_current,
        version: version(&lineage[0]),
        unproven_ancestry,
        claimed_paths,
        permissioned_as: lineage[0].permissioned_as.clone(),
        parent_path: lineage.get(1).and_then(|j| j.runnable_path.clone()),
        root_id: root.id,
        root_path: root.runnable_path.clone(),
        root_kind: root.kind,
        root_trigger_kind: root.trigger_kind.clone(),
        root_version: version(root),
    }))
}

fn version(job: &LineageJob) -> Option<String> {
    match job.kind {
        JobKind::Script => job.runnable_id.map(|h| ScriptHash(h).to_string()),
        JobKind::Flow => job.runnable_id.map(|v| v.to_string()),
        _ => None,
    }
}

/// Only meaningful under `deployed`, which already excludes every kind that runs
/// request-supplied code.
fn runs_current_version(job: &LineageJob) -> bool {
    match job.kind {
        JobKind::Script | JobKind::Flow => job.current_version,
        // An app script is keyed by its content, not by an app version, so a past
        // deployment's script cannot be told apart from the current one's.
        JobKind::AppScript => false,
        JobKind::Preview if job.app_stamped => false,
        // Their code comes from the flow above them, which is checked itself, or (hub)
        // the path names the version.
        JobKind::Script_Hub
        | JobKind::SingleStepFlow
        | JobKind::FlowScript
        | JobKind::FlowNode
        | JobKind::AIAgent
        | JobKind::Preview
        | JobKind::FlowPreview
        | JobKind::Dependencies
        | JobKind::FlowDependencies
        | JobKind::AppDependencies
        | JobKind::Identity
        | JobKind::Noop
        | JobKind::DeploymentCallback
        | JobKind::UnassignedScript
        | JobKind::UnassignedFlow
        | JobKind::UnassignedSinglestepFlow => true,
    }
}

/// Whether the job's code and its path are both fixed by what was deployed (or, for a
/// hub script, published), rather than supplied by the request that created the job.
/// `parent` is the job's parent and whether it runs stored code.
fn runs_stored_code(job: &LineageJob, parent: Option<(&LineageJob, bool)>) -> bool {
    match job.kind {
        JobKind::Script
        | JobKind::Script_Hub
        | JobKind::Flow
        | JobKind::SingleStepFlow
        | JobKind::FlowScript
        | JobKind::FlowNode
        | JobKind::AIAgent => job.origin_verified,
        JobKind::AppScript => job.app_stamped,
        // A deployed app without `app_script` entries runs its inline scripts as previews
        // of the content its policy pins.
        JobKind::Preview if job.app_stamped => true,
        // No API route takes a preview's code together with a parent (`restart_flow` drops
        // the parent of a restarted flow preview): a preview with a parent runs its parent's
        // own definition (a flow step, loop or branch body, agent tool, workflow-as-code
        // task or module). A flow deployed before flow nodes, or run with
        // `DISABLE_FLOW_SCRIPT`, runs its inline steps this way.
        JobKind::Preview | JobKind::FlowPreview => {
            !job.args_modules
                && parent.is_some_and(|(p, p_stored)| p_stored && path_within(job, p))
        }
        JobKind::Dependencies
        | JobKind::FlowDependencies
        | JobKind::AppDependencies
        | JobKind::Identity
        | JobKind::Noop
        | JobKind::DeploymentCallback
        | JobKind::UnassignedScript
        | JobKind::UnassignedFlow
        | JobKind::UnassignedSinglestepFlow => false,
    }
}

/// A job pushed by a worker running a request-supplied parent, at or under that
/// parent's path, claims nothing the parent did not already claim. The parent's claim
/// covers it only if the parent's path names an item, so that its claim is checked.
fn derives_from_claimed_parent(job: &LineageJob, parent: Option<(&LineageJob, bool)>) -> bool {
    parent.is_some_and(|(p, p_stored)| {
        !p_stored && p.runnable_path.as_deref().is_some_and(names_an_item) && path_within(job, p)
    })
}

/// Items only live under these namespaces (a preview without a path runs at `tmp/main`):
/// a path outside them cannot be an item's path in a `sub`, so claiming it claims nothing.
fn names_an_item(path: &str) -> bool {
    ["u/", "f/", "g/", "hub/"]
        .iter()
        .any(|ns| path.starts_with(ns))
}

fn path_within(job: &LineageJob, parent: &LineageJob) -> bool {
    match (
        job.runnable_path.as_deref(),
        parent.runnable_path.as_deref(),
    ) {
        (Some(path), Some(parent_path)) if !parent_path.is_empty() => {
            path == parent_path
                || path
                    .strip_prefix(parent_path)
                    .is_some_and(|rest| rest.starts_with('/'))
        }
        _ => false,
    }
}

fn claimed_path(job: &LineageJob, top_level: bool) -> Option<ClaimedPath> {
    let path = job.runnable_path.as_deref().filter(|p| names_an_item(p))?;
    let (path, item) = match job.kind {
        // The server derives an app script's path as `<app path>/<component>`, with the
        // component a single segment.
        JobKind::AppScript => (
            path.rsplit_once('/').map_or(path, |(app, _)| app),
            ClaimedItem::App,
        ),
        JobKind::AppDependencies => (path, ClaimedItem::App),
        JobKind::Preview if top_level => (path, ClaimedItem::ScriptOrAppComponent),
        JobKind::Flow
        | JobKind::FlowPreview
        | JobKind::FlowDependencies
        | JobKind::FlowNode
        | JobKind::FlowScript
        | JobKind::SingleStepFlow
        | JobKind::AIAgent
        | JobKind::UnassignedFlow
        | JobKind::UnassignedSinglestepFlow => (path, ClaimedItem::Flow),
        JobKind::Script
        | JobKind::Script_Hub
        | JobKind::Preview
        | JobKind::Dependencies
        | JobKind::Identity
        | JobKind::Noop
        | JobKind::DeploymentCallback
        | JobKind::UnassignedScript => (path, ClaimedItem::Script),
    };
    Some(ClaimedPath {
        path: path.to_string(),
        item,
        permissioned_as: job.permissioned_as.clone(),
    })
}
