/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{collections::HashMap, time::Duration};

use serde::{self, Deserialize, Serialize};

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
}

#[derive(Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct NewFlow {
    pub path: String,
    pub summary: String,
    pub description: String,
    pub value: serde_json::Value,
    pub schema: Option<Schema>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct FlowValue {
    pub modules: Vec<FlowModule>,
    #[serde(default)]
    pub failure_module: Option<FlowModule>,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default")]
    pub same_worker: bool,
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
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FlowModule {
    #[serde(default = "default_id")]
    pub id: String,
    #[serde(default)]
    #[serde(alias = "input_transform")]
    pub input_transforms: HashMap<String, InputTransform>,
    pub value: FlowModuleValue,
    pub stop_after_if: Option<StopAfterIf>,
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspend: Option<Suspend>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<Retry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sleep: Option<InputTransform>,
}

impl FlowModule {
    pub fn id_append(&mut self, s: &str) {
        self.id = format!("{}-{}", self.id, s);
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
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
        #[serde(alias = "input_transform")]
        input_transforms: HashMap<String, InputTransform>,
        content: String,
        lock: Option<String>,
        path: Option<String>,
        language: ScriptLang,
    },
    Identity,
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
