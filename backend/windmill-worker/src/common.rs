use sqlx::{Pool, Postgres};
use tokio::{fs::File, io::AsyncReadExt};
use windmill_common::error::{self, Error};
use windmill_queue::CLOUD_HOSTED;

use crate::MAX_RESULT_SIZE;

pub async fn read_result(job_dir: &str) -> error::Result<serde_json::Value> {
    let mut file = File::open(format!("{job_dir}/result.json")).await?;
    let mut content = "".to_string();
    file.read_to_string(&mut content).await?;
    if *CLOUD_HOSTED && content.len() > MAX_RESULT_SIZE {
        return Err(Error::ExecutionErr("Result is too large for the cloud app (limit 2MB). 
        If using this script as part of the flow, use the shared folder to pass heavy data between steps.".to_owned()));
    }
    serde_json::from_str(&content)
        .map_err(|e| Error::ExecutionErr(format!("Error parsing result: {e}")))
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn set_logs(logs: &str, id: &uuid::Uuid, db: &Pool<Postgres>) {
    if sqlx::query!(
        "UPDATE queue SET logs = $1 WHERE id = $2",
        logs.to_owned(),
        id
    )
    .execute(db)
    .await
    .is_err()
    {
        tracing::error!(%id, "error updating logs for id {id}")
    };
}

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
