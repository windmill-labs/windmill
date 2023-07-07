use futures::TryStreamExt;
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio_postgres::{types::ToSql, NoTls, Row};
use windmill_common::{
    error::{to_anyhow, Error},
    jobs::QueuedJob,
};
use windmill_parser_sql::parse_pgsql_sig;

use crate::{
    common::postgres_row_to_json_value, get_content, transform_json_value, AuthedClient,
    JobCompleted,
};

#[derive(Deserialize)]
struct PgDatabase {
    host: String,
    user: Option<String>,
    password: Option<String>,
    port: Option<u16>,
    sslmode: Option<String>,
    dbname: String,
}

pub async fn do_postgresql(
    job: QueuedJob,
    client: &AuthedClient,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> windmill_common::error::Result<JobCompleted> {
    let args = if let Some(args) = &job.args {
        Some(transform_json_value("args", client, &job.workspace_id, args.clone()).await?)
    } else {
        None
    };

    let pg_args: serde_json::Value = serde_json::from_value(args.unwrap_or_else(|| json!({})))
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;
    let database =
        serde_json::from_value::<PgDatabase>(pg_args.get("database").unwrap_or(&json!({})).clone())
            .map_err(|e| Error::ExecutionErr(e.to_string()))?;
    let sslmode = database.sslmode.unwrap_or("prefer".to_string());
    let database = format!(
        "postgres://{user}:{password}@{host}:{port}/{dbname}?sslmode={sslmode}",
        user = database.user.unwrap_or("postgres".to_string()),
        password = database.password.unwrap_or("".to_string()),
        host = database.host,
        port = database.port.unwrap_or(5432),
        dbname = database.dbname,
        sslmode = sslmode
    );
    let (client, handle) = if sslmode == "require" {
        let (client, connection) = tokio_postgres::connect(
            &database,
            MakeTlsConnector::new(
                TlsConnector::builder()
                    .danger_accept_invalid_certs(true)
                    .danger_accept_invalid_hostnames(true)
                    .build()
                    .map_err(to_anyhow)?,
            ),
        )
        .await
        .map_err(to_anyhow)?;

        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        (client, handle)
    } else {
        let (client, connection) = tokio_postgres::connect(&database, NoTls)
            .await
            .map_err(to_anyhow)?;
        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        (client, handle)
    };

    let args = &job
        .args
        .clone()
        .unwrap_or_else(|| json!({}))
        .as_object()
        .map(|x| x.to_owned())
        .unwrap_or_else(|| json!({}).as_object().unwrap().to_owned());
    let mut i = 1;
    let mut statement_values: Vec<serde_json::Value> = vec![];

    loop {
        if args.get(&format!("${}", i)).is_none() {
            break;
        }
        statement_values.push(args.get(&format!("${}", i)).unwrap().to_owned());
        i += 1;
    }
    let query: String = get_content(&job, db).await?;

    let sig = parse_pgsql_sig(&query)
        .map_err(|x| Error::ExecutionErr(x.to_string()))?
        .args;

    let query_params = statement_values
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let arg_t = &sig[i]
                .otyp
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing otyp for pg arg"))?
                .to_owned();

            let boxed: windmill_common::error::Result<Box<dyn ToSql + Sync + Send>> = match value {
                Value::Null => Ok(Box::new(None::<bool>)),
                Value::Bool(b) => Ok(Box::new(b.clone())),
                Value::Number(n) if n.is_i64() && arg_t == "char" => {
                    Ok(Box::new(n.as_i64().unwrap() as i8))
                }
                Value::Number(n)
                    if n.is_i64() && (arg_t == "smallint" || arg_t == "smallserial") =>
                {
                    Ok(Box::new(n.as_i64().unwrap() as i16))
                }
                Value::Number(n) if n.is_i64() && (arg_t == "int" || arg_t == "serial") => {
                    Ok(Box::new(n.as_i64().unwrap() as i32))
                }
                Value::Number(n) if n.is_i64() => Ok(Box::new(n.as_i64().unwrap())),
                Value::Number(n) if n.is_u64() && arg_t == "oid" => {
                    Ok(Box::new(n.as_u64().unwrap() as u32))
                }
                Value::Number(n) if n.is_u64() && (arg_t == "bigint" || arg_t == "bigserial") => {
                    Ok(Box::new(n.as_u64().unwrap() as i64))
                }
                Value::Number(n) if n.is_f64() && arg_t == "real" => {
                    Ok(Box::new(n.as_f64().unwrap() as f32))
                }
                Value::Number(n) if n.is_f64() && arg_t == "double" => {
                    Ok(Box::new(n.as_f64().unwrap()))
                }
                Value::Number(n) => Ok(Box::new(n.as_f64().unwrap())),
                Value::String(s) => Ok(Box::new(s.clone())),
                _ => Err(Error::ExecutionErr(format!(
                    "Unsupported type in query: {:?} and signature {arg_t:?}",
                    value
                ))),
            };
            boxed
        })
        .collect::<windmill_common::error::Result<Vec<_>>>()?;
    // Now we can execute a simple statement that just returns its parameter.
    let rows = client
        .query_raw(&query, query_params)
        .await
        .map_err(to_anyhow)?;

    handle.abort();
    // And then check that we got back the same string we sent over.
    return Ok(JobCompleted {
        job: job,
        result: json!(rows
            .try_collect::<Vec<Row>>()
            .await
            .map_err(to_anyhow)?
            .into_iter()
            .map(postgres_row_to_json_value)
            .collect::<Result<Vec<_>, _>>()?),
        logs: "".to_string(),
        success: true,
    });
}
