use uuid::Uuid;

use crate::{db::DB, error::Result, jobs::JobKind};

/// Where a job's code comes from, as far as its chain of parents can prove it.
pub struct JobProvenance {
    /// True only when the job and every ancestor run code at a path taken from a
    /// deployed version, never from the request that created the job.
    pub deployed: bool,
    pub parent_path: Option<String>,
    pub root_id: Uuid,
    pub root_path: Option<String>,
    pub root_kind: JobKind,
    pub root_trigger_kind: Option<String>,
}

struct LineageJob {
    id: Uuid,
    parent_job: Option<Uuid>,
    kind: JobKind,
    runnable_path: Option<String>,
    trigger_kind: Option<String>,
    version_matches_path: bool,
}

pub async fn job_provenance(db: &DB, job_id: &Uuid, w_id: &str) -> Result<Option<JobProvenance>> {
    let lineage = sqlx::query_as!(
        LineageJob,
        r#"WITH RECURSIVE lineage AS (
            SELECT id, parent_job, kind, runnable_path, runnable_id, trigger_kind, 0 AS depth
            FROM v2_job WHERE id = $1 AND workspace_id = $2
          UNION ALL
            SELECT p.id, p.parent_job, p.kind, p.runnable_path, p.runnable_id, p.trigger_kind,
                l.depth + 1
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
            END AS "version_matches_path!"
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
    let deployed = root.parent_job.is_none()
        && lineage
            .iter()
            .all(|j| runs_stored_code(j.kind) && j.version_matches_path);
    Ok(Some(JobProvenance {
        deployed,
        parent_path: lineage.get(1).and_then(|j| j.runnable_path.clone()),
        root_id: root.id,
        root_path: root.runnable_path.clone(),
        root_kind: root.kind,
        root_trigger_kind: root.trigger_kind.clone(),
    }))
}

/// Whether the job's code and its path are both fixed by what was deployed (or, for a
/// hub script, published), rather than supplied by the request that created the job.
fn runs_stored_code(kind: JobKind) -> bool {
    match kind {
        JobKind::Script
        | JobKind::Script_Hub
        | JobKind::Flow
        | JobKind::SingleStepFlow
        | JobKind::FlowScript
        | JobKind::FlowNode
        | JobKind::AIAgent
        | JobKind::AppScript => true,
        JobKind::Preview
        | JobKind::FlowPreview
        | JobKind::Dependencies
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
