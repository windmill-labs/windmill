use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use windmill_common::{
    agent_workers::{AgentWorkerData, QueueInitJob},
    worker::{HttpClient, JobCancelled},
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

pub async fn pull_job(
    client: &HttpClient,
    headers: Option<HeaderMap>,
    body: Option<AgentWorkerData>,
) -> anyhow::Result<Option<JobAndPerms>> {
    client
        .post("/api/agent_workers/pull_job", headers, &body)
        .await
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessAgentWorkerResult {
    pub job_completed: JobCompleted,
    pub agent_worker_data: Option<AgentWorkerData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CancelJob {
    pub job_cancelled: JobCancelled,
    pub agent_worker_data: Option<AgentWorkerData>,
    
}
impl ProcessAgentWorkerResult {
    pub fn new(job_completed: JobCompleted, agent_worker_data: Option<AgentWorkerData>) -> Self {
        Self { job_completed, agent_worker_data }
    }
}

pub async fn send_result(
    client: &HttpClient,
    data: ProcessAgentWorkerResult,
) -> anyhow::Result<String> {
    client
        .post(
            &format!(
                "/api/w/{}/agent_workers/send_result/{}",
                data.job_completed.job.workspace_id, data.job_completed.job.id
            ),
            None,
            &data,
        )
        .await
}

pub const UPDATE_PING_URL: &str = "/api/agent_workers/update_ping";
