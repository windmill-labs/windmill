use crate::{
    db::DB,
    jobs::{run_flow_by_path_inner, run_script_by_path_inner, RunJobQuery},
    users::fetch_api_authed,
};
use serde_json::value::RawValue;
use std::collections::HashMap;

use axum::{
    routing::{delete, get, post},
    Router,
};
use handler::{
    alter_publication, create_postgres_trigger, create_publication, create_slot,
    create_template_script, delete_postgres_trigger, delete_publication, drop_slot_name,
    exists_postgres_trigger, get_postgres_trigger, get_publication_info, get_template_script,
    is_database_in_logical_level, list_database_publication, list_postgres_triggers,
    list_slot_name, set_enabled, update_postgres_trigger,
};
use windmill_common::{db::UserDB, utils::StripPath};
use windmill_queue::PushArgsOwned;

mod bool;
mod converter;
mod handler;
mod hex;
mod mapper;
mod relation;
mod replication_message;
mod trigger;

pub use handler::PostgresTrigger;
pub use trigger::start_database;

fn publication_service() -> Router {
    Router::new()
        .route("/get/:publication_name/*path", get(get_publication_info))
        .route("/create/:publication_name/*path", post(create_publication))
        .route("/update/:publication_name/*path", post(alter_publication))
        .route(
            "/delete/:publication_name/*path",
            delete(delete_publication),
        )
        .route("/list/*path", get(list_database_publication))
}

fn slot_service() -> Router {
    Router::new()
        .route("/list/*path", get(list_slot_name))
        .route("/create/*path", post(create_slot))
        .route("/delete/*path", delete(drop_slot_name))
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create_postgres_trigger))
        .route("/list", get(list_postgres_triggers))
        .route("/get/*path", get(get_postgres_trigger))
        .route("/update/*path", post(update_postgres_trigger))
        .route("/delete/*path", delete(delete_postgres_trigger))
        .route("/exists/*path", get(exists_postgres_trigger))
        .route("/setenabled/*path", post(set_enabled))
        .route("/get_template_script/:id", get(get_template_script))
        .route("/create_template_script", post(create_template_script))
        .route(
            "/is_valid_postgres_configuration/*path",
            get(is_database_in_logical_level),
        )
        .nest("/publication", publication_service())
        .nest("/slot", slot_service())
}

async fn run_job(
    args: Option<HashMap<String, Box<RawValue>>>,
    extra: Option<HashMap<String, Box<RawValue>>>,
    db: &DB,
    trigger: &PostgresTrigger,
) -> anyhow::Result<()> {
    let args = PushArgsOwned { args: args.unwrap_or_default(), extra };

    let authed = fetch_api_authed(
        trigger.edited_by.clone(),
        trigger.email.clone(),
        &trigger.workspace_id,
        db,
        Some(format!("postgres-{}", trigger.path)),
    )
    .await?;

    let user_db = UserDB::new(db.clone());

    let run_query = RunJobQuery::default();

    if trigger.is_flow {
        run_flow_by_path_inner(
            authed,
            db.clone(),
            user_db,
            trigger.workspace_id.clone(),
            StripPath(trigger.script_path.to_owned()),
            run_query,
            args,
            None,
        )
        .await?;
    } else {
        run_script_by_path_inner(
            authed,
            db.clone(),
            user_db,
            trigger.workspace_id.clone(),
            StripPath(trigger.script_path.to_owned()),
            run_query,
            args,
            None,
        )
        .await?;
    }

    Ok(())
}
