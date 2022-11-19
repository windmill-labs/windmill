/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]

pub struct ContextualVariable {
    pub name: String,
    pub value: String,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]

pub struct ListableVariable {
    pub workspace_id: String,
    pub path: String,
    pub value: Option<String>,
    pub is_secret: bool,
    pub description: String,
    pub extra_perms: serde_json::Value,
    pub account: Option<i32>,
    pub is_oauth: bool,
    pub is_expired: Option<bool>,
}

#[derive(Deserialize)]
pub struct CreateVariable {
    pub path: String,
    pub value: String,
    pub is_secret: bool,
    pub description: String,
    pub account: Option<i32>,
    pub is_oauth: Option<bool>,
}

pub fn get_reserved_variables(
    w_id: &str,
    token: &str,
    email: &str,
    username: &str,
    job_id: &str,
    permissioned_as: &str,
    base_url: &str,
    path: Option<String>,
    flow_id: Option<String>,
    flow_path: Option<String>,
    schedule_path: Option<String>,
) -> [ContextualVariable; 12] {
    let state_path = {
        let flow_path = flow_path
            .clone()
            .unwrap_or_else(|| "NO_FLOW_PATH".to_string());
        let script_path = path.clone().unwrap_or_else(|| "NO_JOB_PATH".to_string());
        let schedule_path = schedule_path
            .clone()
            .map(|x| format!("/{x}"))
            .unwrap_or_else(String::new);

        let script_path = if script_path.ends_with("/") {
            "NO_NAME".to_string()
        } else {
            script_path
        };
        format!("{permissioned_as}/{flow_path}/{script_path}{schedule_path}")
    };
    [
        ContextualVariable {
            name: "WM_WORKSPACE".to_string(),
            value: w_id.to_string(),
            description: "Workspace id of the current script".to_string(),
        },
        ContextualVariable {
            name: "WM_TOKEN".to_string(),
            value: token.to_string(),
            description: "Token ephemeral to the current script with equal permission to the \
                          permission of the run (Usable as a bearer token)"
                .to_string(),
        },
        ContextualVariable {
            name: "WM_EMAIL".to_string(),
            value: email.to_string(),
            description: "Email of the user that executed the current script".to_string(),
        },
        ContextualVariable {
            name: "WM_USERNAME".to_string(),
            value: username.to_string(),
            description: "Username of the user that executed the current script".to_string(),
        },
        ContextualVariable {
            name: "WM_BASE_URL".to_string(),
            value: base_url.to_string(),
            description: "base url of this instance".to_string(),
        },
        ContextualVariable {
            name: "WM_JOB_ID".to_string(),
            value: job_id.to_string(),
            description: "Job id of the current script".to_string(),
        },
        ContextualVariable {
            name: "WM_JOB_PATH".to_string(),
            value: path.unwrap_or_else(|| "".to_string()),
            description: "Path of the script or flow being run if any".to_string(),
        },
        ContextualVariable {
            name: "WM_FLOW_JOB_ID".to_string(),
            value: flow_id.unwrap_or_else(|| "".to_string()),
            description: "Job id of the encapsulating flow if the job is a flow step".to_string(),
        },
        ContextualVariable {
            name: "WM_FLOW_PATH".to_string(),
            value: flow_path.unwrap_or_else(|| "".to_string()),
            description: "Path of the encapsulating flow if the job is a flow step".to_string(),
        },
        ContextualVariable {
            name: "WM_SCHEDULE_PATH".to_string(),
            value: schedule_path.unwrap_or_else(|| "".to_string()),
            description: "Path of the schedule if the job of the step or encapsulating step has \
                          been triggered by a schedule"
                .to_string(),
        },
        ContextualVariable {
            name: "WM_PERMISSIONED_AS".to_string(),
            value: permissioned_as.to_string(),
            description: "Fully Qualified (u/g) owner name of executor of the job".to_string(),
        },
        ContextualVariable {
            name: "WM_STATE_PATH".to_string(),
            value: state_path,
            description: "State resource path unique to a script and its trigger".to_string(),
        },
    ]
}
