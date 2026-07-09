use reqwest::header::HeaderMap;
use uuid::Uuid;
use windmill_common::{
    agent_workers::QueueInitJob, worker::HttpClient, workspaces::DucklakeWithConnData,
};
use windmill_queue::{JobAndPerms, JobCompleted};

pub async fn queue_init_job(client: &HttpClient, content: &str) -> anyhow::Result<Uuid> {
    client
        .post(
            "/api/agent_workers/queue_init_job",
            None,
            &QueueInitJob { content: content.to_string() },
        )
        .await
        .and_then(|x: String| Uuid::parse_str(&x).map_err(|e| anyhow::anyhow!(e)))
}

pub async fn queue_periodic_job(client: &HttpClient, content: &str) -> anyhow::Result<Uuid> {
    client
        .post(
            "/api/agent_workers/queue_periodic_job",
            None,
            &QueueInitJob { content: content.to_string() },
        )
        .await
        .and_then(|x: String| Uuid::parse_str(&x).map_err(|e| anyhow::anyhow!(e)))
}

pub async fn pull_job(
    client: &HttpClient,
    headers: Option<HeaderMap>,
    body: Option<bool>,
) -> anyhow::Result<Option<JobAndPerms>> {
    client
        .post("/api/agent_workers/pull_job", headers, &body)
        .await
}

pub async fn send_result(client: &HttpClient, jc: JobCompleted) -> anyhow::Result<String> {
    client
        .post(
            &format!(
                "/api/w/{}/agent_workers/send_result/{}",
                jc.job.workspace_id, jc.job.id
            ),
            None,
            &jc,
        )
        .await
}

#[allow(dead_code)]
pub async fn get_ducklake_from_agent_http(
    client: &HttpClient,
    name: &str,
    w_id: &str,
) -> anyhow::Result<DucklakeWithConnData> {
    client
        .get(&format!(
            "/api/w/{}/agent_workers/get_ducklake/{}",
            w_id, &name
        ))
        .await
}

#[allow(dead_code)]
pub async fn get_datatable_resource_from_agent_http(
    client: &HttpClient,
    name: &str,
    w_id: &str,
) -> anyhow::Result<serde_json::Value> {
    client
        .get(&format!(
            "/api/w/{}/agent_workers/get_datatable_resource/{}",
            w_id, &name
        ))
        .await
}

/// Record a materialization outcome from an agent worker (no direct DB) via the
/// API, so `materialized_partition` state lands the same as on a Sql worker.
// Only called from the duckdb executor, which is itself `#[cfg(feature = "duckdb")]`.
#[cfg(feature = "duckdb")]
pub async fn record_materialization_from_agent_http(
    client: &HttpClient,
    w_id: &str,
    req: &windmill_common::materialization::RecordMaterializationRequest,
) -> anyhow::Result<()> {
    client
        .post(
            &format!("/api/w/{}/agent_workers/record_materialization", w_id),
            None,
            req,
        )
        .await
}

pub const UPDATE_PING_URL: &str = "/api/agent_workers/update_ping";
