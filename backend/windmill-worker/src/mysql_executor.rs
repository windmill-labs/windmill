use mysql_async::prelude::*;

use serde::Deserialize;
use serde_json::json;
use windmill_common::{
    error::{to_anyhow, Error},
    jobs::QueuedJob,
};
use windmill_parser_sql::parse_mysql_sig;

use crate::{get_content, transform_json_value, AuthedClient, JobCompleted};

#[derive(Deserialize)]
struct MysqlDatabase {
    host: String,
    user: Option<String>,
    password: Option<String>,
    port: Option<u16>,
    database: String,
}

pub async fn do_mysql(
    job: QueuedJob,
    client: &AuthedClient,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> windmill_common::error::Result<JobCompleted> {
    let args = if let Some(args) = &job.args {
        Some(transform_json_value("args", client, &job.workspace_id, args.clone()).await?)
    } else {
        None
    };

    let mysql_args: serde_json::Value = serde_json::from_value(args.unwrap_or_else(|| json!({})))
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;
    let database = serde_json::from_value::<MysqlDatabase>(
        mysql_args.get("database").unwrap_or(&json!({})).clone(),
    )
    .map_err(|e: serde_json::Error| Error::ExecutionErr(e.to_string()))?;
    let database = format!(
        "mysql://{user}:{password}@{host}:{port}/{dbname}",
        user = database.user.unwrap_or("mysql".to_string()),
        password = database.password.unwrap_or("".to_string()),
        host = database.host,
        port = database.port.unwrap_or(5432),
        dbname = database.database,
    );

    let pool = mysql_async::Pool::new(database.as_str());
    let mut conn = pool.get_conn().await.map_err(to_anyhow)?;

    let args = &job
        .args
        .clone()
        .unwrap_or_else(|| json!({}))
        .as_object()
        .map(|x| x.to_owned())
        .unwrap_or_else(|| json!({}).as_object().unwrap().to_owned());
    let mut statement_values: Vec<serde_json::Value> = vec![];

    let query: String = get_content(&job, db).await?;

    let sig = parse_mysql_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    for arg in &sig {
        statement_values.push(args.get(&arg.name).unwrap_or(&json!(null)).clone());
    }
    let rows: Vec<serde_json::Value> = conn.query(query).await.map_err(to_anyhow)?;

    drop(conn);

    pool.disconnect().await.map_err(to_anyhow)?;

    // And then check that we got back the same string we sent over.
    return Ok(JobCompleted { job: job, result: json!(rows), logs: "".to_string(), success: true });
}
