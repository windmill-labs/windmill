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
    u8,
};

use rand::Rng;
use serde::{Deserialize, Serialize, Serializer};

use crate::{
    more_serde::{default_empty_string, default_id, default_null, default_true, is_default},
    scripts::{Schema, ScriptHash, ScriptLang},
};

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Flow {
    pub workspace_id: String,
    pub path: String,
    pub summary: String,
    pub description: String,
    pub value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
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
    #[serde(skip_serializing_if = "is_none_or_false")]
    pub ws_error_handler_muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,
    #[serde(skip_serializing_if = "is_none_or_false")]
    pub visible_to_runner_only: Option<bool>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct FlowWithStarred {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub flow: Flow,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred: Option<bool>,
}

fn is_none_or_false(b: &Option<bool>) -> bool {
    b.is_none() || !b.unwrap()
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ListableFlow {
    pub workspace_id: String,
    pub path: String,
    pub summary: String,
    pub description: String,
    pub edited_by: Option<String>,
    pub edited_at: Option<chrono::DateTime<chrono::Utc>>,
    pub archived: bool,
    pub extra_perms: serde_json::Value,
    pub starred: bool,
    pub has_draft: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_error_handler_muted: Option<bool>,
    #[sqlx(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_msg: Option<String>,
}

#[derive(Deserialize, sqlx::FromRow)]
pub struct NewFlow {
    pub path: String,
    pub summary: String,
    pub description: Option<String>,
    pub value: serde_json::Value,
    pub schema: Option<Schema>,
    pub draft_only: Option<bool>,
    pub tag: Option<String>,
    pub dedicated_worker: Option<bool>,
    pub timeout: Option<i32>,
    pub deployment_message: Option<String>,
    pub visible_to_runner_only: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct FlowValue {
    pub modules: Vec<FlowModule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub failure_module: Option<Box<FlowModule>>,
    #[serde(default)]
    pub preprocessor_module: Option<Box<FlowModule>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_key: Option<String>,
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
    pub fn interval(&self, previous_attempts: u16, silent: bool) -> Option<Duration> {
        let Self { constant, exponential } = self;

        if previous_attempts < constant.attempts {
            Some(Duration::from_secs(constant.seconds as u64))
        } else if previous_attempts - constant.attempts < exponential.attempts {
            let exp = previous_attempts.saturating_add(1) as u32;
            let mut secs = exponential.multiplier * exponential.seconds.saturating_pow(exp);
            if let Some(random_factor) = exponential.random_factor {
                if random_factor > 0 {
                    let random_component =
                        rand::thread_rng().gen_range(0..(std::cmp::min(random_factor, 100) as u16));
                    secs = match rand::thread_rng().gen_bool(1.0 / 2.0) {
                        true => secs.saturating_add(secs * random_component / 100),
                        false => secs.saturating_sub(secs * random_component / 100),
                    };
                }
            }
            if !silent {
                tracing::warn!("Rescheduling job in {} seconds due to failure", secs);
            }
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
            .and_then(|p| self.interval(p, true))
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default)]
pub struct ConstantDelay {
    pub attempts: u16,
    pub seconds: u16,
}

/// multiplier * seconds ^ failures (+/- jitter of the previous value, if any)
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct ExponentialDelay {
    pub attempts: u16,
    pub multiplier: u16,
    pub seconds: u16,
    pub random_factor: Option<i8>, // percentage, defaults to 0 for no jitter
}

impl Default for ExponentialDelay {
    fn default() -> Self {
        Self { attempts: 0, multiplier: 1, seconds: 0, random_factor: None }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_approval_disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_cancel: Option<bool>,
    #[serde(skip_serializing_if = "false_or_empty")]
    pub continue_on_disapprove_timeout: Option<bool>,
}

fn false_or_empty(v: &Option<bool>) -> bool {
    v.is_none() || v.as_ref().is_some_and(|x| !x)
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
    pub value: Box<serde_json::value::RawValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_after_if: Option<StopAfterIf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_after_all_iters_if: Option<StopAfterIf>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_after_use: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continue_on_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_if: Option<SkipIf>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SkipIf {
    pub expr: String,
}

#[derive(Deserialize)]
pub struct FlowModuleValueType {
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Deserialize)]
pub struct FlowModuleValueWithParallel {
    #[serde(rename = "type")]
    pub type_: String,
    pub parallel: Option<bool>,
    pub parallelism: Option<u16>,
}

#[derive(Deserialize)]
pub struct FlowModuleValueWithSkipFailures {
    pub skip_failures: Option<bool>,
    pub parallel: Option<bool>,
    pub parallelism: Option<u16>,
}

#[derive(Deserialize)]
pub struct BranchWithSkipFailures {
    pub skip_failure: Option<bool>,
}

#[derive(Deserialize)]
pub struct FlowModuleWithBranches {
    pub branches: Vec<BranchWithSkipFailures>,
}

impl FlowModule {
    pub fn id_append(&mut self, s: &str) {
        self.id = format!("{}-{}", self.id, s);
    }
    pub fn get_value(&self) -> anyhow::Result<FlowModuleValue> {
        serde_json::from_str::<FlowModuleValue>(self.value.get()).map_err(crate::error::to_anyhow)
    }

    pub fn get_value_with_skip_failures(&self) -> anyhow::Result<FlowModuleValueWithSkipFailures> {
        serde_json::from_str::<FlowModuleValueWithSkipFailures>(self.value.get())
            .map_err(crate::error::to_anyhow)
    }

    pub fn get_branches_skip_failures(&self) -> anyhow::Result<FlowModuleWithBranches> {
        serde_json::from_str::<FlowModuleWithBranches>(self.value.get())
            .map_err(crate::error::to_anyhow)
    }

    pub fn is_flow(&self) -> bool {
        self.get_type().is_ok_and(|x| x == "flow")
    }

    pub fn get_value_with_parallel(&self) -> anyhow::Result<FlowModuleValueWithParallel> {
        serde_json::from_str::<FlowModuleValueWithParallel>(self.value.get())
            .map_err(crate::error::to_anyhow)
    }

    pub fn is_simple(&self) -> bool {
        //todo: flow modules could also be simple execpt for the fact that the case of having single parallel flow approval step is not handled well (Create SuspendedTimeout)
        self.get_type()
            .is_ok_and(|x| x == "script" || x == "rawscript")
    }

    pub fn get_type(&self) -> anyhow::Result<String> {
        serde_json::from_str::<FlowModuleValueType>(self.value.get())
            .map_err(crate::error::to_anyhow)
            .map(|x| x.type_)
    }
}

#[derive(Deserialize)]
pub struct UntaggedInputTransform {
    #[serde(rename = "type")]
    pub type_: String,
    pub value: Option<Box<serde_json::value::RawValue>>,
    pub expr: Option<String>,
}

impl<'de> Deserialize<'de> for InputTransform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let untagged: UntaggedInputTransform = UntaggedInputTransform::deserialize(deserializer)?;

        match untagged.type_.as_str() {
            "static" => {
                let value = untagged.value.unwrap_or_else(default_null);
                Ok(InputTransform::Static { value })
            }
            "javascript" => {
                let expr = untagged.expr.unwrap_or_else(default_empty_string);
                Ok(InputTransform::Javascript { expr })
            }
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["static", "javascript"],
            )),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(
    tag = "type",
    rename_all(serialize = "lowercase", deserialize = "lowercase")
)]
pub enum InputTransform {
    Static {
        #[serde(default = "default_null")]
        value: Box<serde_json::value::RawValue>,
    },
    Javascript {
        #[serde(default = "default_empty_string")]
        expr: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Branch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default = "default_empty_string")]
    pub expr: String,
    pub modules: Vec<FlowModule>,
    #[serde(default = "default_true")]
    pub skip_failure: bool,
    #[serde(default = "default_true")]
    pub parallel: bool,
}

#[derive(Serialize, Debug, Clone)]
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
        tag_override: Option<String>,
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
        parallel: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        parallelism: Option<u16>,
    },
    WhileloopFlow {
        modules: Vec<FlowModule>,
        #[serde(default = "default_false")]
        skip_failures: bool,
    },
    BranchOne {
        branches: Vec<Branch>,
        default: Vec<FlowModule>,
    },
    BranchAll {
        branches: Vec<Branch>,
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
        #[serde(skip_serializing_if = "is_none_or_empty")]
        tag: Option<String>,
        language: ScriptLang,
        #[serde(skip_serializing_if = "Option::is_none")]
        custom_concurrency_key: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        concurrent_limit: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        concurrency_time_window_s: Option<i32>,
    },
    Identity,
}

fn is_none_or_empty(expr: &Option<String>) -> bool {
    expr.is_none() || expr.as_ref().unwrap().is_empty()
}

#[derive(Deserialize)]
struct UntaggedFlowModuleValue {
    #[serde(rename = "type")]
    type_: String,
    #[serde(alias = "input_transform")]
    input_transforms: Option<HashMap<String, InputTransform>>,
    path: Option<String>,
    hash: Option<ScriptHash>,
    tag_override: Option<String>,
    iterator: Option<InputTransform>,
    modules: Option<Vec<FlowModule>>,
    skip_failures: Option<bool>,
    parallel: Option<bool>,
    parallelism: Option<u16>,
    branches: Option<Vec<Branch>>,
    default: Option<Vec<FlowModule>>,
    content: Option<String>,
    lock: Option<String>,
    tag: Option<String>,
    language: Option<ScriptLang>,
    custom_concurrency_key: Option<String>,
    concurrent_limit: Option<i32>,
    concurrency_time_window_s: Option<i32>,
}

impl<'de> Deserialize<'de> for FlowModuleValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let untagged: UntaggedFlowModuleValue = UntaggedFlowModuleValue::deserialize(deserializer)?;

        match untagged.type_.as_str() {
            "script" => Ok(FlowModuleValue::Script {
                input_transforms: untagged.input_transforms.unwrap_or_default(),
                path: untagged
                    .path
                    .ok_or_else(|| serde::de::Error::missing_field("path"))?,
                hash: untagged.hash,
                tag_override: untagged.tag_override,
            }),
            "flow" => Ok(FlowModuleValue::Flow {
                input_transforms: untagged.input_transforms.unwrap_or_default(),
                path: untagged
                    .path
                    .ok_or_else(|| serde::de::Error::missing_field("path"))?,
            }),
            "forloopflow" => Ok(FlowModuleValue::ForloopFlow {
                iterator: untagged
                    .iterator
                    .ok_or_else(|| serde::de::Error::missing_field("iterator"))?,
                modules: untagged
                    .modules
                    .ok_or_else(|| serde::de::Error::missing_field("modules"))?,
                skip_failures: untagged.skip_failures.unwrap_or(true),
                parallel: untagged.parallel.unwrap_or(false),
                parallelism: untagged.parallelism,
            }),
            "whileloopflow" => Ok(FlowModuleValue::WhileloopFlow {
                modules: untagged
                    .modules
                    .ok_or_else(|| serde::de::Error::missing_field("modules"))?,
                skip_failures: untagged.skip_failures.unwrap_or(false),
            }),
            "branchone" => Ok(FlowModuleValue::BranchOne {
                branches: untagged
                    .branches
                    .ok_or_else(|| serde::de::Error::missing_field("branches"))?,
                default: untagged
                    .default
                    .ok_or_else(|| serde::de::Error::missing_field("default"))?,
            }),
            "branchall" => Ok(FlowModuleValue::BranchAll {
                branches: untagged
                    .branches
                    .ok_or_else(|| serde::de::Error::missing_field("branches"))?,
                parallel: untagged.parallel.unwrap_or(true),
            }),
            "rawscript" => Ok(FlowModuleValue::RawScript {
                input_transforms: untagged.input_transforms.unwrap_or_default(),
                content: untagged
                    .content
                    .ok_or_else(|| serde::de::Error::missing_field("content"))?,
                lock: untagged.lock,
                path: untagged.path,
                tag: untagged.tag,
                language: untagged
                    .language
                    .ok_or_else(|| serde::de::Error::missing_field("language"))?,
                custom_concurrency_key: untagged.custom_concurrency_key,
                concurrent_limit: untagged.concurrent_limit,
                concurrency_time_window_s: untagged.concurrency_time_window_s,
            }),
            "identity" => Ok(FlowModuleValue::Identity),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &[
                    "script",
                    "flow",
                    "forloopflow",
                    "whileloopflow",
                    "branchone",
                    "branchall",
                    "rawscript",
                    "identity",
                ],
            )),
        }
    }
}

impl Into<Box<serde_json::value::RawValue>> for FlowModuleValue {
    fn into(self) -> Box<serde_json::value::RawValue> {
        crate::worker::to_raw_value(&self)
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
    pub include_draft_only: Option<bool>,
    pub with_deployment_msg: Option<bool>,
}

pub fn add_virtual_items_if_necessary(modules: &mut Vec<FlowModule>) {
    if modules.len() > 0
        && (modules[modules.len() - 1].sleep.is_some()
            || modules[modules.len() - 1].suspend.is_some())
    {
        modules.push(FlowModule {
            id: format!("{}-v", modules[modules.len() - 1].id),
            value: crate::worker::to_raw_value(&FlowModuleValue::Identity),
            stop_after_if: None,
            stop_after_all_iters_if: None,
            summary: Some("Virtual module needed for suspend/sleep when last module".to_string()),
            mock: None,
            retry: None,
            sleep: None,
            suspend: None,
            cache_ttl: None,
            timeout: None,
            priority: None,
            delete_after_use: None,
            continue_on_error: None,
            skip_if: None,
        });
    }
}
