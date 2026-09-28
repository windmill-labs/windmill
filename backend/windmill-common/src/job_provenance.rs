use uuid::Uuid;

use crate::{db::DB, error::Result, jobs::JobKind};

/// Where a job's code comes from, as far as its chain of parents can prove it.
pub struct JobProvenance {
    /// True only when the job and every ancestor run code at a path taken from a
    /// deployed version, never from the request that created the job.
    pub deployed: bool,
    /// The chain reached a parent it cannot read (other workspace, depth cap), so
    /// nothing above that point is known.
    pub unproven_ancestry: bool,
    /// The paths the chain took from a request, top-most first: for the job to act as
    /// them, its identity must be one that could have deployed there. A job whose code
    /// and path both derive from a claimed parent adds no claim of its own.
    pub claimed_paths: Vec<ClaimedPath>,
    pub parent_path: Option<String>,
    pub root_id: Uuid,
    pub root_path: Option<String>,
    pub root_kind: JobKind,
    pub root_trigger_kind: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ClaimedPath {
    /// The deployable item the claim stands for: an app script's claim is its app.
    pub path: String,
    pub item: ClaimedItem,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ClaimedItem {
    Script,
    Flow,
    App,
}

struct LineageJob {
    id: Uuid,
    parent_job: Option<Uuid>,
    kind: JobKind,
    runnable_path: Option<String>,
    trigger_kind: Option<String>,
    origin_verified: bool,
    app_stamped: bool,
}

/// Reads any job of `w_id` regardless of the caller: authorize access to `job_id` first.
pub async fn job_provenance(db: &DB, job_id: &Uuid, w_id: &str) -> Result<Option<JobProvenance>> {
    let lineage = sqlx::query_as!(
        LineageJob,
        r#"WITH RECURSIVE lineage AS (
            SELECT id, parent_job, kind, runnable_path, runnable_id, trigger_kind, trigger,
                0 AS depth
            FROM v2_job WHERE id = $1 AND workspace_id = $2
          UNION ALL
            SELECT p.id, p.parent_job, p.kind, p.runnable_path, p.runnable_id, p.trigger_kind,
                p.trigger, l.depth + 1
            FROM v2_job p JOIN lineage l ON p.id = l.parent_job
            WHERE p.workspace_id = $2 AND l.depth < 100
        )
        SELECT id AS "id!", parent_job, kind AS "kind!: JobKind", runnable_path,
            trigger_kind::text AS trigger_kind,
            -- A restart takes its flow version from the request, so a trusted flow path
            -- can carry another flow's code: the version must belong to that path.
            CASE kind
                WHEN 'flow' THEN EXISTS (SELECT 1 FROM flow_version fv
                    WHERE fv.id = runnable_id AND fv.path = runnable_path AND fv.workspace_id = $2)
                WHEN 'script' THEN EXISTS (SELECT 1 FROM script s
                    WHERE s.hash = runnable_id AND s.path = runnable_path AND s.workspace_id = $2)
                ELSE true
            END AS "origin_verified!",
            -- Only deployed-app runs are stamped with their app; an app editor preview
            -- runs app code at an app path it does not have to own.
            COALESCE(trigger_kind = 'app' AND starts_with(runnable_path, trigger || '/'), false)
                AS "app_stamped!"
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
    let mut parent: Option<(&LineageJob, bool)> = None;
    for job in lineage.iter().rev() {
        let stored = runs_stored_code(job, parent);
        if !stored {
            all_stored = false;
            if !derives_from_claimed_parent(job, parent) {
                if let Some(claim) = claimed_path(job) {
                    claimed_paths.push(claim);
                }
            }
        }
        parent = Some((job, stored));
    }

    Ok(Some(JobProvenance {
        deployed: !unproven_ancestry && all_stored,
        unproven_ancestry,
        claimed_paths,
        parent_path: lineage.get(1).and_then(|j| j.runnable_path.clone()),
        root_id: root.id,
        root_path: root.runnable_path.clone(),
        root_kind: root.kind,
        root_trigger_kind: root.trigger_kind.clone(),
    }))
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
        // No API route takes a preview's code together with a parent: a preview with a
        // parent runs its parent's own definition (a flow step, loop or branch body, agent
        // tool or workflow-as-code task). A flow deployed before flow nodes, or run with
        // `DISABLE_FLOW_SCRIPT`, runs its inline steps this way.
        JobKind::Preview | JobKind::FlowPreview => {
            parent.is_some_and(|(p, p_stored)| p_stored && path_within(job, p))
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
/// parent's path, claims nothing the parent did not already claim.
fn derives_from_claimed_parent(job: &LineageJob, parent: Option<(&LineageJob, bool)>) -> bool {
    parent.is_some_and(|(p, p_stored)| !p_stored && path_within(job, p))
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

fn claimed_path(job: &LineageJob) -> Option<ClaimedPath> {
    let path = job.runnable_path.as_deref().filter(|p| !p.is_empty())?;
    let (path, item) = match job.kind {
        // The server derives an app script's path as `<app path>/<component>`, with the
        // component a single segment.
        JobKind::AppScript => (
            path.rsplit_once('/').map_or(path, |(app, _)| app),
            ClaimedItem::App,
        ),
        JobKind::AppDependencies => (path, ClaimedItem::App),
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
    Some(ClaimedPath { path: path.to_string(), item })
}
