use uuid::Uuid;
use windmill_common::{agent_workers::QueueInitJob, worker::HttpClient};
use windmill_queue::{JobAndPerms, JobCompleted};

pub async fn queue_init_job(client: &HttpClient, content: &str) -> anyhow::Result<Uuid> {
    client
        .post(
            "/api/agent_workers/queue_init_job",
            &QueueInitJob { content: content.to_string() },
        )
        .await
        .and_then(|x: String| Uuid::parse_str(&x).map_err(|e| anyhow::anyhow!(e)))
}

pub async fn pull_job(client: &HttpClient) -> anyhow::Result<Option<JobAndPerms>> {
    client.post("/api/agent_workers/pull_job", &()).await
}

pub async fn send_result(client: &HttpClient, jc: JobCompleted) -> anyhow::Result<String> {
    client
        .post(
            &format!(
                "/api/w/{}/agent_workers/send_result/{}",
                jc.job.workspace_id, jc.job.id
            ),
            &jc,
        )
        .await
}

pub const UPDATE_PING_URL: &str = "/api/agent_workers/update_ping";
