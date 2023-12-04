/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};

use serde::{self, Deserialize, Serialize, Serializer};

use crate::{
    more_serde::{
        default_empty_string, default_false, default_id, default_null, default_true, is_default,
    },
    scripts::{Schema, ScriptHash, ScriptLang},
};

#[derive(Serialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Flow {
    pub workspace_id: String,
    pub path: String,
    pub summary: String,
    pub description: String,
    pub value: serde_json::Value,
    pub edited_by: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    pub archived: bool,
    pub schema: Option<Schema>,
    pub extra_perms: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedicated_worker: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_error_handler_muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct ListableFlow {
    pub workspace_id: String,
    pub path: String,
    pub summary: String,
    pub description: String,
    pub edited_by: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    pub archived: bool,
    pub extra_perms: serde_json::Value,
    pub starred: bool,
    pub has_draft: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_error_handler_muted: Option<bool>,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct NewFlow {
    pub path: String,
    pub summary: String,
    pub description: Option<String>,
    pub value: serde_json::Value,
    pub schema: Option<Schema>,
    pub draft_only: Option<bool>,
    pub tag: Option<String>,
    pub ws_error_handler_muted: Option<bool>,
    pub dedicated_worker: Option<bool>,
    pub timeout: Option<i32>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct FlowValue {
    pub modules: Vec<FlowModule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub failure_module: Option<FlowModule>,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default")]
    pub same_worker: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrent_limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_time_window_s: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_expr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_ttl: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub early_return: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // Priority at the flow level
    pub priority: Option<i16>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StopAfterIf {
    pub expr: String,
    pub skip_if_stopped: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default)]
pub struct Retry {
    pub constant: ConstantDelay,
    pub exponential: ExponentialDelay,
}

impl Retry {
    /// Takes the number of previous retries and returns the interval until the next retry if any.
    ///
    /// May return [`Duration::ZERO`] to retry immediately.
    pub fn interval(&self, previous_attempts: u16) -> Option<Duration> {
        let Self { constant, exponential } = self;

        if previous_attempts < constant.attempts {
            Some(Duration::from_secs(constant.seconds as u64))
        } else if previous_attempts - constant.attempts < exponential.attempts {
            let exp = previous_attempts.saturating_add(1) as u32;
            let secs = exponential.multiplier * exponential.seconds.saturating_pow(exp);
            Some(Duration::from_secs(secs as u64))
        } else {
            None
        }
    }

    pub fn has_attempts(&self) -> bool {
        self.constant.attempts != 0 || self.exponential.attempts != 0
    }

    pub fn max_attempts(&self) -> u16 {
        self.constant
            .attempts
            .saturating_add(self.exponential.attempts)
    }

    pub fn max_interval(&self) -> Option<Duration> {
        self.max_attempts()
            .checked_sub(1)
            .and_then(|p| self.interval(p))
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default)]
pub struct ConstantDelay {
    pub attempts: u16,
    pub seconds: u16,
}

/// multiplier * seconds ^ failures
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct ExponentialDelay {
    pub attempts: u16,
    pub multiplier: u16,
    pub seconds: u16,
}

impl Default for ExponentialDelay {
    fn default() -> Self {
        Self { attempts: 0, multiplier: 1, seconds: 0 }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Suspend {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_events: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_form: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_auth_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_groups_required: Option<InputTransform>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Mock {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_value: Option<serde_json::Value>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FlowModule {
    #[serde(default = "default_id")]
    pub id: String,
    pub value: FlowModuleValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_after_if: Option<StopAfterIf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspend: Option<Suspend>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock: Option<Mock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<Retry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sleep: Option<InputTransform>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_ttl: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // Priority at the flow step level
    pub priority: Option<i16>,
}

impl FlowModule {
    pub fn id_append(&mut self, s: &str) {
        self.id = format!("{}-{}", self.id, s);
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(
    tag = "type",
    rename_all(serialize = "lowercase", deserialize = "lowercase")
)]
pub enum InputTransform {
    Static {
        #[serde(default = "default_null")]
        value: serde_json::Value,
    },
    Javascript {
        #[serde(default = "default_empty_string")]
        expr: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BranchOneModules {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub expr: String,
    pub modules: Vec<FlowModule>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BranchAllModules {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub modules: Vec<FlowModule>,
    #[serde(default = "default_true")]
    pub skip_failure: bool,
    #[serde(default = "default_true")]
    pub parallel: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(
    tag = "type",
    rename_all(serialize = "lowercase", deserialize = "lowercase")
)]
pub enum FlowModuleValue {
    Script {
        #[serde(default)]
        #[serde(alias = "input_transform")]
        input_transforms: HashMap<String, InputTransform>,
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        hash: Option<ScriptHash>,
    },
    Flow {
        #[serde(default)]
        #[serde(alias = "input_transform")]
        input_transforms: HashMap<String, InputTransform>,
        path: String,
    },
    ForloopFlow {
        iterator: InputTransform,
        modules: Vec<FlowModule>,
        #[serde(default = "default_true")]
        skip_failures: bool,
        #[serde(default = "default_false")]
        parallel: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        parallelism: Option<u16>,
    },
    BranchOne {
        branches: Vec<BranchOneModules>,
        default: Vec<FlowModule>,
    },
    BranchAll {
        branches: Vec<BranchAllModules>,
        #[serde(default = "default_true")]
        parallel: bool,
    },
    RawScript {
        #[serde(default)]
        #[serde(alias = "input_transform", serialize_with = "ordered_map")]
        input_transforms: HashMap<String, InputTransform>,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        lock: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tag: Option<String>,
        language: ScriptLang,
        #[serde(skip_serializing_if = "Option::is_none")]
        concurrent_limit: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        concurrency_time_window_s: Option<i32>,
    },
    Identity,
}

impl FlowModuleValue {
    pub fn is_simple(&self) -> bool {
        match self {
            FlowModuleValue::Script { .. } => true,
            FlowModuleValue::Flow { .. } => true,
            FlowModuleValue::RawScript { .. } => true,
            _ => false,
        }
    }
}

fn ordered_map<S>(value: &HashMap<String, InputTransform>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}

#[derive(Deserialize)]
pub struct ListFlowQuery {
    pub path_start: Option<String>,
    pub path_exact: Option<String>,
    pub edited_by: Option<String>,
    pub show_archived: Option<bool>,
    pub order_by: Option<String>,
    pub order_desc: Option<bool>,
    pub starred_only: Option<bool>,
}

pub fn add_virtual_items_if_necessary(modules: &mut Vec<FlowModule>) {
    if modules.len() > 0
        && (modules[modules.len() - 1].sleep.is_some()
            || modules[modules.len() - 1].suspend.is_some())
    {
        modules.push(FlowModule {
            id: format!("{}-v", modules[modules.len() - 1].id),
            value: FlowModuleValue::Identity,
            stop_after_if: None,
            summary: Some("Virtual module needed for suspend/sleep when last module".to_string()),
            mock: None,
            retry: None,
            sleep: None,
            suspend: None,
            cache_ttl: None,
            timeout: None,
            priority: None,
        });
    }
}
