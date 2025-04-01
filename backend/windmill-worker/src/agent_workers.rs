use uuid::Uuid;
use windmill_common::{
    agent_workers::{PullJobRequest, QueueInitJob, AGENT_HTTP_CLIENT, BASE_INTERNAL_URL},
    utils::HTTP_CLIENT,
    BASE_URL,
};
use windmill_queue::PulledJobResult;

pub async fn queue_init_job(worker_name: &str, content: &str) -> anyhow::Result<Uuid> {
    let client = HTTP_CLIENT.clone();
    let url = format!("{}/api/agent_workers/queue_init_job", BASE_URL.read().await);
    let response = client
        .post(url)
        .json(&QueueInitJob { worker_name: worker_name.to_string(), content: content.to_string() })
        .send()
        .await?;
    let status = response.status();
    if status.is_success() {
        Ok(Uuid::parse_str(&response.text().await?)?)
    } else {
        Err(anyhow::anyhow!("Failed to create initial job"))
    }
}

pub async fn pull_job(worker_name: &str) -> anyhow::Result<PulledJobResult> {
    let client = AGENT_HTTP_CLIENT.clone();
    let url = format!("{}/api/agent_workers/pull_job", *BASE_INTERNAL_URL);
    tracing::error!("pulled 1");
    let response = client
        .post(url)
        // .body(serde_json::to_string(&PullJobRequest {
        //     worker_name: worker_name.to_string(),
        // })?)
        // .header("Content-Type", "application/json")
        .send()
        .await?;
    let status = response.status();
    tracing::error!("pulled 2");
    if status.is_success() {
        Ok(response.json().await?)
    } else {
        Err(anyhow::anyhow!(format!(
            "Job pull http request failed {}",
            response.status()
        )))
    }
}
