use std::{collections::HashMap, fmt, io, str::Utf8Error, string::FromUtf8Error};

use crate::{
    db::DB,
    jobs::{run_flow_by_path_inner, run_script_by_path_inner, RunJobQuery},
    users::fetch_api_authed,
};

use axum::{
    routing::{delete, get, post},
    Router,
};
use handler::{
    create_database_trigger, delete_database_trigger, exists_database_trigger,
    get_database_trigger, list_database_triggers, set_enabled, update_database_trigger,
    DatabaseTrigger,
};
use windmill_common::{db::UserDB, utils::StripPath};
use windmill_queue::PushArgsOwned;

mod handler;
mod replication_message;
mod trigger;

pub type SqlxJson<T> = sqlx::types::Json<T>;
pub use trigger::start_database;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create_database_trigger))
        .route("/list", get(list_database_triggers))
        .route("/get/*path", get(get_database_trigger))
        .route("/update/*path", post(update_database_trigger))
        .route("/delete/*path", delete(delete_database_trigger))
        .route("/exists/*path", get(exists_database_trigger))
        .route("/setenabled/*path", post(set_enabled))
}

async fn run_job(
    db: &DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    trigger: &DatabaseTrigger,
) -> anyhow::Result<()> {
    let args = PushArgsOwned { args: HashMap::new(), extra: None };

    let label_prefix = Some(format!("db-{}-", trigger.path));

    let authed = fetch_api_authed(
        trigger.edited_by.clone(),
        trigger.email.clone(),
        &trigger.workspace_id,
        db,
        "anonymous".to_string(),
    )
    .await?;

    let user_db = UserDB::new(db.clone());

    let run_query = RunJobQuery::default();

    if trigger.is_flow {
        run_flow_by_path_inner(
            authed,
            db.clone(),
            user_db,
            rsmq,
            trigger.workspace_id.clone(),
            StripPath(trigger.script_path.to_owned()),
            run_query,
            args,
            label_prefix,
        )
        .await?;
    } else {
        run_script_by_path_inner(
            authed,
            db.clone(),
            user_db,
            rsmq,
            trigger.workspace_id.clone(),
            StripPath(trigger.script_path.to_owned()),
            run_query,
            args,
            label_prefix,
        )
        .await?;
    }

    Ok(())
}
