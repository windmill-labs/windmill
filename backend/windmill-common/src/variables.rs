/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::error;
use crate::worker::Connection;
use crate::{worker::WORKER_GROUP, BASE_URL, DB};
use chrono::{SecondsFormat, Utc};
use magic_crypt::{MagicCrypt256, MagicCryptError, MagicCryptTrait};
use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};

lazy_static::lazy_static! {
    pub static ref SECRET_SALT: Option<String> = std::env::var("SECRET_SALT").ok();
}

#[derive(Serialize, Clone)]

pub struct ContextualVariable {
    pub name: String,
    pub value: String,
    pub description: String,
    pub is_custom: bool,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]

pub struct ListableVariable {
    pub workspace_id: String,
    pub path: String,
    pub value: Option<String>,
    pub is_secret: bool,
    pub description: String,
    pub extra_perms: serde_json::Value,
    pub account: Option<i32>,
    pub is_oauth: Option<bool>,
    pub is_expired: Option<bool>,
    pub is_refreshed: Option<bool>,
    pub refresh_error: Option<String>,
    pub is_linked: Option<bool>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ExportableListableVariable {
    pub workspace_id: String,
    pub path: String,
    pub value: Option<String>,
    pub is_secret: bool,
    pub description: String,
    pub extra_perms: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<i32>,
    #[serde(skip_serializing_if = "is_none_or_false")]
    pub is_oauth: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<chrono::DateTime<Utc>>,
}

fn is_none_or_false(b: &Option<bool>) -> bool {
    b.is_none() || !b.unwrap()
}

#[derive(Deserialize)]
pub struct CreateVariable {
    pub path: String,
    pub value: String,
    pub is_secret: bool,
    pub description: String,
    pub account: Option<i32>,
    pub is_oauth: Option<bool>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
}

pub async fn build_crypt(db: &DB, w_id: &str) -> crate::error::Result<MagicCrypt256> {
    let key = get_workspace_key(w_id, db).await?;
    let crypt_key = if let Some(ref salt) = SECRET_SALT.as_ref() {
        format!("{}{}", key, salt)
    } else {
        key
    };
    Ok(magic_crypt::new_magic_crypt!(crypt_key, 256))
}

pub async fn build_crypt_with_key_suffix(
    db: &DB,
    w_id: &str,
    key_suffix: &str,
) -> crate::error::Result<MagicCrypt256> {
    let key = get_workspace_key(w_id, db).await?;
    let crypt_key = if let Some(ref salt) = SECRET_SALT.as_ref() {
        format!("{}{}{}", key, salt, key_suffix)
    } else {
        format!("{}{}", key, key_suffix)
    };
    Ok(magic_crypt::new_magic_crypt!(crypt_key, 256))
}

pub async fn get_workspace_key(w_id: &str, db: &DB) -> crate::error::Result<String> {
    let key = sqlx::query_scalar!(
        "SELECT key FROM workspace_key WHERE workspace_id = $1 AND kind = 'cloud'",
        w_id
    )
    .fetch_one(db)
    .await
    .map_err(|e| crate::Error::internal_err(format!("fetching workspace key: {e:#}")))?;
    Ok(key)
}

pub async fn get_secret_value_as_admin(
    db: &DB,
    w_id: &str,
    path: &str,
) -> crate::error::Result<String> {
    let variable_o = sqlx::query!(
        "SELECT value, is_secret, path from variable WHERE variable.path = $1 AND variable.workspace_id = $2", path, w_id
    )
    .fetch_optional(db)
    .await?;

    let variable = if let Some(variable) = variable_o {
        variable
    } else {
        return Err(crate::Error::NotFound(format!(
            "variable {} not found in workspace {}",
            path, w_id
        )));
    };

    let r = if variable.is_secret {
        let value = variable.value;
        if !value.is_empty() {
            let mc = build_crypt(db, w_id).await?;
            decrypt(&mc, value)?
        } else {
            "".to_string()
        }
    } else {
        variable.value
    };

    Ok(r)
}

pub fn encrypt(mc: &MagicCrypt256, value: &str) -> String {
    mc.encrypt_str_to_base64(value)
}

pub fn decrypt(mc: &MagicCrypt256, value: String) -> error::Result<String> {
    mc.decrypt_base64_to_string(value).map_err(|e| match e {
        MagicCryptError::DecryptError(_) => error::Error::internal_err(
            "Could not decrypt value. The value may have been encrypted with a different key."
                .to_string(),
        ),
        _ => error::Error::internal_err(e.to_string()),
    })
}

pub const WM_SCHEDULED_FOR: &str = "WM_SCHEDULED_FOR";

lazy_static::lazy_static! {
    pub static ref CUSTOM_ENVS_CACHE: Cache<String, (i64, Vec<(String, String)>)> = Cache::new(100);
}

pub async fn get_reserved_variables(
    conn: &Connection,
    w_id: &str,
    token: &str,
    email: &str,
    username: &str,
    job_id: &str,
    permissioned_as: &str,
    path: Option<String>,
    flow_id: Option<String>,
    flow_path: Option<String>,
    schedule_path: Option<String>,
    step_id: Option<String>,
    root_flow_id: Option<String>,
    scheduled_for: Option<chrono::DateTime<Utc>>,
) -> Vec<ContextualVariable> {
    let state_path = {
        let trigger = if schedule_path.is_some() {
            username.to_string()
        } else {
            "user".to_string()
        };

        if let Some(flow_path) = flow_path.clone() {
            format!(
                "{flow_path}/{}_{trigger}",
                step_id.clone().unwrap_or_else(|| "nostep".to_string())
            )
        } else if let Some(script_path) = path.clone() {
            let script_path = if script_path.ends_with("/") {
                format!("{script_path}state")
            } else {
                script_path
            };
            format!("{script_path}/{trigger}")
        } else {
            format!("u/{username}/tmp_state")
        }
    };

    let custom_envs = get_cached_workspace_envs(conn, w_id).await;

    let joined_schedule_path = schedule_path
        .clone()
        .unwrap_or("manual".to_string())
        .split("/")
        .collect::<Vec<&str>>()
        .join("_");
    let ts = chrono::Utc::now().timestamp_millis();
    let object_path = if let Some(flow_path) = flow_path.clone() {
        let flow_path = flow_path.split("/").collect::<Vec<&str>>().join("_");
        let step_id = step_id.clone().unwrap_or("".to_string());
        format!("{flow_path}/{joined_schedule_path}/{step_id}/{ts}_{job_id}")
    } else {
        let joined_script_path = path
            .clone()
            .unwrap_or("".to_string())
            .split("/")
            .collect::<Vec<&str>>()
            .join("_");
        format!("{joined_script_path}/{joined_schedule_path}/{ts}_{job_id}")
    };

    vec![
    ContextualVariable {
        name: "WM_WORKSPACE".to_string(),
        value: w_id.to_string(),
        description: "Workspace id of the current script".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_TOKEN".to_string(),
        value: token.to_string(),
        description: "Token ephemeral to the current script with equal permission to the \
                      permission of the run (Usable as a bearer token)"
            .to_string(),
            is_custom: false,
    },
    ContextualVariable {
        name: "WM_EMAIL".to_string(),
        value: email.to_string(),
        description: "Email of the user that executed the current script".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_USERNAME".to_string(),
        value: username.to_string(),
        description: "Username of the user that executed the current script".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_BASE_URL".to_string(),
        value: BASE_URL.read().await.clone(),
        description: "base url of this instance".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_JOB_ID".to_string(),
        value: job_id.to_string(),
        description: "Job id of the current script".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: WM_SCHEDULED_FOR.to_string(),
        value: scheduled_for
            .map(|ts| ts.to_rfc3339_opts(SecondsFormat::Secs, true))
            .unwrap_or_else(|| "".to_string()),
        description: "date-time in UTC (e.g: 2014-11-28T12:45:59.324310806Z) of when the job was scheduled".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_JOB_PATH".to_string(),
        value: path.unwrap_or_else(|| "".to_string()),
        description: "Path of the script or flow being run if any".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_FLOW_JOB_ID".to_string(),
        value: flow_id.unwrap_or_else(|| "".to_string()),
        description: "Job id of the encapsulating flow if the job is a flow step".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_ROOT_FLOW_JOB_ID".to_string(),
        value: root_flow_id.unwrap_or_else(|| "".to_string()),
        description: "Job id of the root flow if the job is a flow step".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_FLOW_PATH".to_string(),
        value: flow_path.unwrap_or_else(|| "".to_string()),
        description: "Path of the encapsulating flow if the job is a flow step".to_string(),
        is_custom: false,
    },

    ContextualVariable {
        name: "WM_SCHEDULE_PATH".to_string(),
        value: schedule_path.unwrap_or_else(|| "".to_string()),
        description: "Path of the schedule if the job of the step or encapsulating step has \
                      been triggered by a schedule"
            .to_string(),
            is_custom: false,
    },
    ContextualVariable {
        name: "WM_PERMISSIONED_AS".to_string(),
        value: permissioned_as.to_string(),
        description: "Fully Qualified (u/g) owner name of executor of the job".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_STATE_PATH".to_string(),
        value: state_path.clone(),
        description: "State resource path unique to a script and its trigger".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_FLOW_STEP_ID".to_string(),
        value: step_id.unwrap_or_else(|| "".to_string()),
        description: "The node id in a flow (like 'a', 'b', or 'f')".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_OBJECT_PATH".to_string(),
        value: object_path,
        description: "Script or flow step execution unique path, useful for storing results in an external service".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_WORKER_GROUP".to_string(),
        value: WORKER_GROUP.clone(),
        description: "name of the worker group the job is running on".to_string(),
        is_custom: false,
    },
].into_iter().chain(custom_envs.into_iter().map(|(name, value)| ContextualVariable {
    name,
    value,
    description: "Custom workspace environment variable".to_string(),
    is_custom: true,
})
).collect()
}

async fn get_cached_workspace_envs(conn: &Connection, w_id: &str) -> Vec<(String, String)> {
    let cached_envs_o = CUSTOM_ENVS_CACHE.get(w_id).and_then(|(ts, envs)| {
        if ts > chrono::Utc::now().timestamp() - (60 * 15) {
            Some(envs)
        } else {
            None
        }
    });

    let custom_envs = if let Some(cached_envs) = cached_envs_o {
        cached_envs
    } else {
        let custom_envs = match conn {
            Connection::Sql(db) => sqlx::query_as::<_, (String, String)>(
                "SELECT name, value FROM workspace_env WHERE workspace_id = $1",
            )
            .bind(w_id)
            .fetch_all(db)
            .await
            .unwrap_or_default(),
            Connection::Http(client) => client
                .get(&format!("/api/w/{w_id}/agent_workers/custom_envs"))
                .await
                .unwrap_or_default(),
        };
        CUSTOM_ENVS_CACHE.insert(
            w_id.to_string(),
            (chrono::Utc::now().timestamp(), custom_envs.clone()),
        );
        custom_envs
    };
    custom_envs
}
