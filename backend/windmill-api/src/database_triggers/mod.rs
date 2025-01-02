use serde_json::value::RawValue;
use std::collections::HashMap;

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
    alter_publication, create_database_trigger, create_publication, create_slot,
    delete_database_trigger, delete_publication, drop_slot_name, exists_database_trigger,
    get_database_trigger, get_publication_info, get_template_script, list_database_publication,
    list_database_triggers, list_slot_name, set_enabled, update_database_trigger, DatabaseTrigger,
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
        .route("/create", post(create_database_trigger))
        .route("/list", get(list_database_triggers))
        .route("/get/*path", get(get_database_trigger))
        .route("/update/*path", post(update_database_trigger))
        .route("/delete/*path", delete(delete_database_trigger))
        .route("/exists/*path", get(exists_database_trigger))
        .route("/setenabled/*path", post(set_enabled))
        .route("/get-template-script", post(get_template_script))
        .nest("/publication", publication_service())
        .nest("/slot", slot_service())
}

async fn run_job(
    args: Option<HashMap<String, Box<RawValue>>>,
    extra: Option<HashMap<String, Box<RawValue>>>,
    db: &DB,
    trigger: &DatabaseTrigger,
) -> anyhow::Result<()> {
    let args = PushArgsOwned { args: args.unwrap_or_default(), extra };
    let label_prefix = Some(format!("db-{}-", trigger.path));

    let authed = fetch_api_authed(
        trigger.edited_by.clone(),
        trigger.email.clone(),
        &trigger.workspace_id,
        db,
        Some("anonymous".to_string()),
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
            label_prefix,
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
            label_prefix,
        )
        .await?;
    }

    Ok(())
}
