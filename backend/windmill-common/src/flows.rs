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

use anyhow::Context;
use rand::Rng;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;
use sqlx::types::Json;
use sqlx::types::JsonRawValue;

use crate::{
    assets::AssetWithAltAccessType,
    cache,
    db::DB,
    error::{Error, Result as WindmillResult},
    jobs::{ConcurrencySettings, ConcurrencySettingsWithCustom, DebouncingSettings},
    more_serde::{default_empty_string, default_id, default_null, default_true, is_default},
    scripts::{Schema, ScriptHash, ScriptLang},
    worker::{to_raw_value, Connection},
};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Flow {
    pub workspace_id: String,
    pub path: String,
    pub summary: String,
    pub description: String,
    pub value: Json<Box<JsonRawValue>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_behalf_of_email: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct FlowWithStarred {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub flow: Flow,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_error_logs: Option<String>,
    pub version_id: i64,
}

fn is_none_or_false(b: &Option<bool>) -> bool {
    b.is_none() || !b.unwrap()
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ListableFlow {
    pub workspace_id: String,
    pub path: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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

fn validate_retry(retry: &Retry, module_id: &str) -> WindmillResult<()> {
    if retry.exponential.attempts > 0 && retry.exponential.seconds == 0 {
        return Err(Error::BadRequest(format!(
            "Module '{}': Exponential backoff base (seconds) must be greater than 0. A base of 0 would cause immediate retries.",
            module_id
        )));
    }
    Ok(())
}

fn validate_flow_value<'de, D>(deserializer: D) -> Result<Box<RawValue>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw_value = Box::<RawValue>::deserialize(deserializer)?;

    let flow_value: FlowValue = serde_json::from_str(raw_value.get())
        .map_err(|e| serde::de::Error::custom(format!("Invalid flow value: {}", e)))?;

    FlowModule::traverse_modules(&flow_value.modules, &mut |module| {
        if let Some(ref retry) = module.retry {
            validate_retry(retry, &module.id)?;
        }
        return Ok(());
    })
    .map_err(|e| serde::de::Error::custom(e.to_string()))?;

    if let Some(ref _failure_module) = flow_value.failure_module {
        //add validation logic here for failure module
    }

    if let Some(ref _preprocessor_module) = flow_value.preprocessor_module {
        //add validation logic here for preprocessor module
    }

    Ok(raw_value)
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct NewFlow {
    pub path: String,
    pub summary: String,
    pub description: Option<String>,
    #[serde(deserialize_with = "validate_flow_value")]
    pub value: Box<RawValue>,
    pub schema: Option<Schema>,
    pub draft_only: Option<bool>,
    pub tag: Option<String>,
    pub dedicated_worker: Option<bool>,
    pub timeout: Option<i32>,
    pub deployment_message: Option<String>,
    pub visible_to_runner_only: Option<bool>,
    pub on_behalf_of_email: Option<String>,
    pub ws_error_handler_muted: Option<bool>,
}

impl NewFlow {
    pub fn parse_flow_value(&self) -> crate::error::Result<FlowValue> {
        serde_json::from_str(self.value.get()).map_err(|e| {
            crate::error::Error::InternalErr(format!("Failed to parse flow value: {}", e))
        })
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct FlowValue {
    pub modules: Vec<FlowModule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub failure_module: Option<Box<FlowModule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub preprocessor_module: Option<Box<FlowModule>>,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default")]
    pub same_worker: bool,
    #[serde(flatten)]
    pub concurrency_settings: ConcurrencySettings,
    #[serde(flatten)]
    pub debouncing_settings: DebouncingSettings,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_expr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_ttl: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_ignore_s3_path: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub early_return: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // Priority at the flow level
    pub priority: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_input_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_env: Option<HashMap<String, Box<RawValue>>>,
}

impl FlowValue {
    pub fn get_flow_module_at_step(&self, step: Step) -> anyhow::Result<&FlowModule> {
        let flow_module = match step {
            Step::PreprocessorStep => self
                .preprocessor_module
                .as_deref()
                .with_context(|| format!("no preprocessor module")),
            Step::Step { idx, .. } => self
                .modules
                .get(idx)
                .with_context(|| format!("no module found at index: {idx}")),
            Step::FailureStep => self
                .failure_module
                .as_deref()
                .with_context(|| format!("no failure module")),
        };

        flow_module
    }

    /// Traverse FlowValue while invoking provided by caller callback on leafs
    // #[async_recursion::async_recursion(?Send)]
    // TODO: We may be want this async.
    pub fn traverse_leafs<C: FnMut(&FlowModuleValue, &String) -> crate::error::Result<()>>(
        modules: Vec<&FlowModule>,
        cb: &mut C,
    ) -> crate::error::Result<()> {
        use FlowModuleValue::*;
        for module in modules {
            match serde_json::from_str::<FlowModuleValue>(module.value.get())? {
                s @ (Script { .. }
                | RawScript { .. }
                | Flow { .. }
                | FlowScript { .. }
                | Identity) => cb(&s, &module.id)?,
                ForloopFlow { modules, .. } | WhileloopFlow { modules, .. } => {
                    Self::traverse_leafs(modules.iter().collect(), cb)?
                }
                AIAgent { tools, .. } => {
                    for tool in tools {
                        match &tool.value {
                            ToolValue::FlowModule(module_value) => cb(module_value, &tool.id)?,
                            ToolValue::Mcp(_) => {
                                // MCP tools don't have a FlowModuleValue to traverse
                            }
                        }
                    }
                }
                BranchOne { default, branches, .. } => {
                    Self::traverse_leafs(default.iter().collect(), cb)?;
                    for branch in branches {
                        Self::traverse_leafs(branch.modules.iter().collect(), cb)?;
                    }
                }
                BranchAll { branches, .. } => {
                    for branch in branches {
                        Self::traverse_leafs(branch.modules.iter().collect(), cb)?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Step {
    Step { idx: usize, len: usize },
    PreprocessorStep,
    FailureStep,
}

impl Step {
    pub fn from_i32_and_len(step: i32, len: usize) -> Self {
        if step < 0 {
            Step::PreprocessorStep
        } else if (step as usize) < len {
            Step::Step { idx: step as usize, len }
        } else {
            Step::FailureStep
        }
    }

    pub fn get_step_index(&self) -> Option<usize> {
        match self {
            Step::Step { idx, .. } => Some(*idx),
            _ => None,
        }
    }

    pub fn is_index_step(&self) -> bool {
        matches!(self, Step::Step { .. })
    }

    pub fn is_preprocessor_step(&self) -> bool {
        matches!(self, Step::PreprocessorStep)
    }

    pub fn is_failure_step(&self) -> bool {
        matches!(self, Step::FailureStep)
    }

    pub fn is_last_step(&self) -> bool {
        matches!(self, Step::Step { idx, len } if *idx == len - 1)
    }
}

#[derive(Default, Deserialize, Serialize, Debug, Clone)]
pub struct StopAfterIf {
    pub expr: String,
    pub skip_if_stopped: bool,
    pub error_message: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
pub struct RetryIf {
    pub expr: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default)]
pub struct Retry {
    pub constant: ConstantDelay,
    pub exponential: ExponentialDelay,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_if: Option<RetryIf>,
}

impl Retry {
    /// Takes the number of previous retries and returns the interval until the next retry if any.
    ///
    /// May return [`Duration::ZERO`] to retry immediately.
    pub fn interval(&self, previous_attempts: u32, silent: bool) -> Option<Duration> {
        let Self { constant, exponential, .. } = self;

        if previous_attempts < constant.attempts {
            Some(Duration::from_secs(constant.seconds as u64))
        } else if previous_attempts - constant.attempts < exponential.attempts {
            let exp = previous_attempts.saturating_add(1) as u32;
            let mut secs = exponential.multiplier * exponential.seconds.saturating_pow(exp);
            if let Some(random_factor) = exponential.random_factor {
                if random_factor > 0 {
                    let random_component =
                        rand::rng().random_range(0..(std::cmp::min(random_factor, 100) as u16));
                    secs = match rand::rng().random_bool(1.0 / 2.0) {
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

    pub fn max_attempts(&self) -> u32 {
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
    pub attempts: u32,
    pub seconds: u16,
}

/// multiplier * seconds ^ failures (+/- jitter of the previous value, if any)
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct ExponentialDelay {
    pub attempts: u32,
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

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct FlowModule {
    #[serde(default = "default_id")]
    pub id: String,
    pub value: Box<RawValue>,
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
    pub cache_ignore_s3_path: Option<bool>,
    #[serde(
        default,
        deserialize_with = "raw_value_to_input_transform::<_, i32>",
        skip_serializing_if = "Option::is_none"
    )]
    pub timeout: Option<InputTransform>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // Priority at the flow step level
    pub priority: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_after_use: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continue_on_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_if: Option<SkipIf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_preprocessor: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pass_flow_input_directly: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SkipIf {
    pub expr: String,
}

#[derive(Deserialize)]
pub struct FlowModuleValueWithParallel {
    #[serde(rename = "type")]
    pub type_: String,
    pub parallel: Option<bool>,
    #[serde(
        default,
        deserialize_with = "raw_value_to_input_transform::<_, u16>",
        skip_serializing_if = "Option::is_none"
    )]
    pub parallelism: Option<InputTransform>,
}

#[derive(Deserialize)]
pub struct FlowModuleValueWithSkipFailures {
    pub skip_failures: Option<bool>,
    pub parallel: Option<bool>,
    #[serde(
        default,
        deserialize_with = "raw_value_to_input_transform::<_, u16>",
        skip_serializing_if = "Option::is_none"
    )]
    pub parallelism: Option<InputTransform>,
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

    pub fn is_ai_agent(&self) -> bool {
        self.get_type().is_ok_and(|x| x == "aiagent")
    }

    pub fn is_simple(&self) -> bool {
        //todo: flow modules could also be simple execpt for the fact that the case of having single parallel flow approval step is not handled well (Create SuspendedTimeout)
        self.get_type()
            .is_ok_and(|x| x == "script" || x == "rawscript" || x == "flowscript")
    }

    pub fn get_type(&self) -> anyhow::Result<&str> {
        #[derive(Deserialize)]
        pub struct FlowModuleValueType<'a> {
            pub r#type: &'a str,
        }

        serde_json::from_str::<FlowModuleValueType>(self.value.get())
            .map_err(crate::error::to_anyhow)
            .map(|x| x.r#type)
    }

    pub fn traverse_modules<C: FnMut(&FlowModule) -> crate::error::Result<()>>(
        modules: &Vec<FlowModule>,
        cb: &mut C,
    ) -> crate::error::Result<()> {
        for module in modules {
            cb(module)?;
            match module
                .get_value()
                .map_err(|e| Error::BadRequest(format!("Module '{}': {}", module.id, e)))?
            {
                FlowModuleValue::ForloopFlow { modules, .. }
                | FlowModuleValue::WhileloopFlow { modules, .. } => {
                    Self::traverse_modules(&modules, cb)?;
                }
                FlowModuleValue::BranchOne { branches, default, .. } => {
                    for branch in branches {
                        Self::traverse_modules(&branch.modules, cb)?;
                    }
                    Self::traverse_modules(&default, cb)?;
                }
                FlowModuleValue::BranchAll { branches, .. } => {
                    for branch in branches {
                        Self::traverse_modules(&branch.modules, cb)?;
                    }
                }
                FlowModuleValue::AIAgent { tools, .. } => {
                    for tool in tools {
                        match &tool.value {
                            ToolValue::FlowModule(module_value) => match module_value {
                                FlowModuleValue::ForloopFlow { modules, .. }
                                | FlowModuleValue::WhileloopFlow { modules, .. } => {
                                    Self::traverse_modules(&modules, cb)?;
                                }
                                FlowModuleValue::BranchOne { branches, default, .. } => {
                                    for branch in branches {
                                        Self::traverse_modules(&branch.modules, cb)?;
                                    }
                                    Self::traverse_modules(&default, cb)?;
                                }
                                FlowModuleValue::BranchAll { branches, .. } => {
                                    for branch in branches {
                                        Self::traverse_modules(&branch.modules, cb)?;
                                    }
                                }
                                _ => {}
                            },
                            ToolValue::Mcp(_) => {
                                // MCP tools don't have a FlowModule to traverse
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct UntaggedInputTransform {
    #[serde(rename = "type")]
    pub type_: String,
    pub value: Option<Box<RawValue>>,
    pub expr: Option<String>,
}

impl<'de> Deserialize<'de> for InputTransform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let untagged: UntaggedInputTransform = UntaggedInputTransform::deserialize(deserializer)?;

        let input_transform = TryInto::<InputTransform>::try_into(untagged)
            .map_err(|e| serde::de::Error::custom(e))?;

        Ok(input_transform)
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
        value: Box<RawValue>,
    },
    Javascript {
        #[serde(default = "default_empty_string")]
        expr: String,
    },
    Ai,
}

impl InputTransform {
    pub fn new_static_value(value: Box<RawValue>) -> InputTransform {
        InputTransform::Static { value }
    }

    pub fn new_javascript_expr(expr: &str) -> InputTransform {
        InputTransform::Javascript { expr: expr.to_owned() }
    }
}

impl TryFrom<UntaggedInputTransform> for InputTransform {
    type Error = anyhow::Error;
    fn try_from(value: UntaggedInputTransform) -> Result<Self, Self::Error> {
        let input_transform = match value.type_.as_str() {
            "static" => InputTransform::new_static_value(value.value.unwrap_or_else(default_null)),
            "javascript" => InputTransform::new_javascript_expr(&value.expr.unwrap_or_default()),
            "ai" => InputTransform::Ai,
            other => {
                return Err(anyhow::anyhow!(
                    "got value: {other} for field `type`, expected value: `static` or `javascript`"
                ))
            }
        };

        Ok(input_transform)
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawValueOrFormatted<T> {
    RawValue(T),
    Formatted { r#type: String, value: Option<T>, expr: Option<String> },
}

fn raw_value_to_input_transform<'de, D, T>(
    deserializer: D,
) -> Result<Option<InputTransform>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Serialize,
{
    let val = Option::<RawValueOrFormatted<T>>::deserialize(deserializer)?;
    let input_tranform = match val {
        Some(RawValueOrFormatted::RawValue(v)) => {
            Some(InputTransform::new_static_value(to_raw_value(&v)))
        }
        Some(RawValueOrFormatted::Formatted { r#type, expr, value }) => {
            let untaged_input_transform = UntaggedInputTransform {
                type_: r#type,
                expr,
                value: value.map(|val| to_raw_value(&val)),
            };
            let input_transform = TryInto::<InputTransform>::try_into(untaged_input_transform)
                .map_err(|e| serde::de::Error::custom(e))?;
            Some(input_transform)
        }
        _ => None,
    };
    Ok(input_tranform)
}

/// Id in the `flow_node` table.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, Hash, Eq, PartialEq)]
#[serde(transparent)]
pub struct FlowNodeId(pub i64);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Branch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default = "default_empty_string")]
    pub expr: String,
    pub modules: Vec<FlowModule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modules_node: Option<FlowNodeId>,
    #[serde(default = "default_true")]
    pub skip_failure: bool,
    #[serde(default = "default_true")]
    pub parallel: bool,
}

// Tool types for AI Agent
#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct AgentTool {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub value: ToolValue,
}

// Convert FlowModule -> AgentTool
impl From<FlowModule> for AgentTool {
    fn from(flow_module: FlowModule) -> Self {
        let module_value = serde_json::from_str::<FlowModuleValue>(flow_module.value.get())
            .unwrap_or(FlowModuleValue::Identity);

        AgentTool {
            id: flow_module.id,
            summary: flow_module.summary,
            value: ToolValue::FlowModule(module_value),
        }
    }
}

// Convert AgentTool -> FlowModule (only for FlowModule type tools)
impl From<&AgentTool> for Option<FlowModule> {
    fn from(tool: &AgentTool) -> Self {
        match &tool.value {
            ToolValue::FlowModule(module_value) => Some(FlowModule {
                id: tool.id.clone(),
                value: to_raw_value(module_value),
                summary: tool.summary.clone(),
                ..Default::default()
            }),
            ToolValue::Mcp(_) => None, // MCP tools can't be converted to FlowModule
        }
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "tool_type", rename_all = "lowercase")]
pub enum ToolValue {
    FlowModule(FlowModuleValue),
    Mcp(McpToolValue),
}

// Custom deserializer for backward compatibility with old flows
impl<'de> Deserialize<'de> for ToolValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let content = serde_json::Value::deserialize(deserializer)?;

        // First, try to deserialize as the new tagged format (with tool_type field)
        #[derive(Deserialize)]
        #[serde(tag = "tool_type", rename_all = "lowercase")]
        enum TaggedToolValue {
            FlowModule(FlowModuleValue),
            Mcp(McpToolValue),
        }

        if let Ok(tagged) = TaggedToolValue::deserialize(&content) {
            return Ok(match tagged {
                TaggedToolValue::FlowModule(v) => ToolValue::FlowModule(v),
                TaggedToolValue::Mcp(v) => ToolValue::Mcp(v),
            });
        }

        // Fall back to legacy format (direct FlowModuleValue without tool_type)
        FlowModuleValue::deserialize(&content)
            .map(ToolValue::FlowModule)
            .map_err(|_| {
                D::Error::custom(
                    "expected ToolValue with tool_type field or legacy FlowModuleValue",
                )
            })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct McpToolValue {
    pub resource_path: String,
    #[serde(default)]
    pub include_tools: Vec<String>,
    #[serde(default)]
    pub exclude_tools: Vec<String>,
}

fn is_none_or_empty_vec<T>(expr: &Option<Vec<T>>) -> bool {
    expr.is_none() || expr.as_ref().unwrap().is_empty()
}

#[derive(Serialize, Debug, Clone)]
#[serde(
    tag = "type",
    rename_all(serialize = "lowercase", deserialize = "lowercase")
)]
pub enum FlowModuleValue {
    /// Reference to another script on the workspace
    Script {
        #[serde(default)]
        #[serde(alias = "input_transform")]
        input_transforms: HashMap<String, InputTransform>,
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        hash: Option<ScriptHash>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tag_override: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_trigger: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pass_flow_input_directly: Option<bool>,
    },

    /// Reference to another flow on the workspace
    Flow {
        #[serde(default)]
        #[serde(alias = "input_transform")]
        input_transforms: HashMap<String, InputTransform>,
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pass_flow_input_directly: Option<bool>,
    },

    /// For loop node
    ForloopFlow {
        iterator: InputTransform,
        modules: Vec<FlowModule>,
        #[serde(skip_serializing_if = "Option::is_none")]
        modules_node: Option<FlowNodeId>,
        #[serde(default = "default_true")]
        skip_failures: bool,
        parallel: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        parallelism: Option<InputTransform>,
        #[serde(skip_serializing_if = "Option::is_none")]
        squash: Option<bool>,
    },

    /// While loop node
    WhileloopFlow {
        modules: Vec<FlowModule>,
        #[serde(skip_serializing_if = "Option::is_none")]
        modules_node: Option<FlowNodeId>,
        #[serde(default = "default_false")]
        skip_failures: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        squash: Option<bool>,
    },

    /// Branch-one node
    BranchOne {
        branches: Vec<Branch>,
        default: Vec<FlowModule>,
        #[serde(skip_serializing_if = "Option::is_none")]
        default_node: Option<FlowNodeId>,
    },

    /// Branch-all node
    BranchAll {
        branches: Vec<Branch>,
        #[serde(default = "default_true")]
        parallel: bool,
    },

    /// Inline script node
    /// Only exists if parsed from value from `flow_version` | `flow` table.
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
        #[serde(flatten)]
        concurrency_settings: ConcurrencySettingsWithCustom,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_trigger: Option<bool>,
        #[serde(skip_serializing_if = "is_none_or_empty_vec")]
        assets: Option<Vec<AssetWithAltAccessType>>,
    },

    /// Just a placeholder
    Identity,

    /// Also Inline script node, but instead of being baked into flow, it references `flow_node`
    /// Internal only, never exposed to the frontend.
    /// Only exists if parsed from value from `flow_version_lite` table.
    FlowScript {
        #[serde(default)]
        #[serde(alias = "input_transform", serialize_with = "ordered_map")]
        input_transforms: HashMap<String, InputTransform>,
        id: FlowNodeId,
        #[serde(skip_serializing_if = "is_none_or_empty")]
        tag: Option<String>,
        language: ScriptLang,
        #[serde(flatten)]
        concurrency_settings: ConcurrencySettingsWithCustom,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_trigger: Option<bool>,
        #[serde(skip_serializing_if = "is_none_or_empty_vec")]
        assets: Option<Vec<AssetWithAltAccessType>>,
    },

    // AI agent node
    AIAgent {
        input_transforms: HashMap<String, InputTransform>,
        tools: Vec<AgentTool>,
    },
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
    #[serde(default, deserialize_with = "raw_value_to_input_transform::<_, u16>")]
    parallelism: Option<InputTransform>,
    branches: Option<Vec<Branch>>,
    default: Option<Vec<FlowModule>>,
    content: Option<String>,
    lock: Option<String>,
    tag: Option<String>,
    language: Option<ScriptLang>,
    is_trigger: Option<bool>,
    id: Option<FlowNodeId>,
    default_node: Option<FlowNodeId>,
    modules_node: Option<FlowNodeId>,
    assets: Option<Vec<AssetWithAltAccessType>>,
    tools: Option<Vec<AgentTool>>,
    pass_flow_input_directly: Option<bool>,
    squash: Option<bool>,
    #[serde(flatten)]
    concurrency_settings: ConcurrencySettingsWithCustom,
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
                is_trigger: untagged.is_trigger,
                pass_flow_input_directly: untagged.pass_flow_input_directly,
            }),
            "flow" => Ok(FlowModuleValue::Flow {
                input_transforms: untagged.input_transforms.unwrap_or_default(),
                path: untagged
                    .path
                    .ok_or_else(|| serde::de::Error::missing_field("path"))?,
                pass_flow_input_directly: untagged.pass_flow_input_directly,
            }),
            "forloopflow" => Ok(FlowModuleValue::ForloopFlow {
                iterator: untagged
                    .iterator
                    .ok_or_else(|| serde::de::Error::missing_field("iterator"))?,
                modules: untagged
                    .modules
                    .ok_or_else(|| serde::de::Error::missing_field("modules"))?,
                modules_node: untagged.modules_node,
                skip_failures: untagged.skip_failures.unwrap_or(true),
                parallel: untagged.parallel.unwrap_or(false),
                parallelism: untagged.parallelism,
                squash: untagged.squash,
            }),
            "whileloopflow" => Ok(FlowModuleValue::WhileloopFlow {
                modules: untagged
                    .modules
                    .ok_or_else(|| serde::de::Error::missing_field("modules"))?,
                modules_node: untagged.modules_node,
                skip_failures: untagged.skip_failures.unwrap_or(false),
                squash: untagged.squash,
            }),
            "branchone" => Ok(FlowModuleValue::BranchOne {
                branches: untagged
                    .branches
                    .ok_or_else(|| serde::de::Error::missing_field("branches"))?,
                default: untagged
                    .default
                    .ok_or_else(|| serde::de::Error::missing_field("default"))?,
                default_node: untagged.default_node,
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
                concurrency_settings: untagged.concurrency_settings,
                is_trigger: untagged.is_trigger,
                assets: untagged.assets,
            }),
            "flowscript" => Ok(FlowModuleValue::FlowScript {
                input_transforms: untagged.input_transforms.unwrap_or_default(),
                id: untagged
                    .id
                    .ok_or_else(|| serde::de::Error::missing_field("id"))?,
                tag: untagged.tag,
                language: untagged
                    .language
                    .ok_or_else(|| serde::de::Error::missing_field("language"))?,
                concurrency_settings: untagged.concurrency_settings,
                is_trigger: untagged.is_trigger,
                assets: untagged.assets,
            }),
            "identity" => Ok(FlowModuleValue::Identity),
            "aiagent" => Ok(FlowModuleValue::AIAgent {
                input_transforms: untagged.input_transforms.unwrap_or_default(),
                tools: untagged
                    .tools
                    .ok_or_else(|| serde::de::Error::missing_field("tools"))?,
            }),
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
                    "aiagent",
                ],
            )),
        }
    }
}

impl Into<Box<RawValue>> for FlowModuleValue {
    fn into(self) -> Box<RawValue> {
        to_raw_value(&self)
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
    pub without_description: Option<bool>,
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
            cache_ignore_s3_path: None,
            timeout: None,
            priority: None,
            delete_after_use: None,
            continue_on_error: None,
            skip_if: None,
            apply_preprocessor: None,
            pass_flow_input_directly: None,
        });
    }
}

/// Resolve the value of a flow if any.
pub async fn resolve_maybe_value<T>(
    e: &sqlx::PgPool,
    workspace_id: &str,
    with_code: bool,
    maybe: Option<T>,
    value_mut: impl FnOnce(&mut T) -> Option<&mut Json<Box<JsonRawValue>>>,
) -> Result<Option<T>, Error> {
    let Some(mut container) = maybe else {
        return Ok(None);
    };
    let Some(value) = value_mut(&mut container) else {
        return Ok(Some(container));
    };
    resolve_value(e, workspace_id, &mut value.0, with_code).await?;
    Ok(Some(container))
}

/// Resolve modules recursively.
async fn resolve_value(
    e: &sqlx::PgPool,
    workspace_id: &str,
    value: &mut Box<JsonRawValue>,
    with_code: bool,
) -> Result<(), Error> {
    let mut val = serde_json::from_str::<FlowValue>(value.get()).map_err(|err| {
        Error::internal_err(format!("resolve: Failed to parse flow value: {}", err))
    })?;
    for module in &mut val.modules {
        resolve_module(e, workspace_id, &mut module.value, with_code).await?;
    }
    *value = to_raw_value(&val);
    Ok(())
}

/// Resolve module value recursively.
pub async fn resolve_module(
    db: &DB,
    workspace_id: &str,
    value: &mut Box<JsonRawValue>,
    with_code: bool,
) -> Result<(), Error> {
    use FlowModuleValue::*;

    let mut val = serde_json::from_str::<FlowModuleValue>(value.get()).map_err(|err| {
        Error::internal_err(format!(
            "resolve: Failed to parse flow module value: {}",
            err
        ))
    })?;
    match &mut val {
        FlowScript { .. } => {
            // In order to avoid an unnecessary `.clone()` of `val`, take ownership of it's content
            // using `std::mem::replace`.
            let FlowScript {
                input_transforms,
                id,
                tag,
                language,
                is_trigger,
                assets,
                concurrency_settings,
            } = std::mem::replace(&mut val, Identity)
            else {
                unreachable!()
            };
            // Load script lock file and code content.
            let (lock, content) = if !with_code {
                (Some("...".to_string()), "...".to_string())
            } else {
                cache::flow::fetch_script(&Connection::Sql(db.clone()), id)
                    .await
                    .map(|data| (data.lock.clone(), data.code.clone()))?
            };
            val = RawScript {
                input_transforms,
                content,
                lock,
                path: None,
                tag,
                language,
                is_trigger,
                assets,
                concurrency_settings,
            };
        }
        ForloopFlow { modules, modules_node, .. } | WhileloopFlow { modules, modules_node, .. } => {
            resolve_modules(db, workspace_id, modules, modules_node.take(), with_code).await?;
        }
        BranchOne { branches, default, default_node } => {
            resolve_modules(db, workspace_id, default, default_node.take(), with_code).await?;
            for branch in branches {
                resolve_modules(
                    db,
                    workspace_id,
                    &mut branch.modules,
                    branch.modules_node.take(),
                    with_code,
                )
                .await?;
            }
        }
        BranchAll { branches, .. } => {
            for branch in branches {
                resolve_modules(
                    db,
                    workspace_id,
                    &mut branch.modules,
                    branch.modules_node.take(),
                    with_code,
                )
                .await?;
            }
        }
        _ => {}
    }
    *value = to_raw_value(&val);
    Ok(())
}

pub async fn resolve_modules(
    e: &sqlx::PgPool,
    workspace_id: &str,
    modules: &mut Vec<FlowModule>,
    modules_node: Option<FlowNodeId>,
    with_code: bool,
) -> Result<(), Error> {
    // Replace the `modules_node` with the actual modules.
    if let Some(id) = modules_node {
        *modules = cache::flow::fetch_flow(e, id)
            .await
            .map(|data| data.value().modules.clone())?;
    }
    for module in modules.iter_mut() {
        Box::pin(resolve_module(
            e,
            workspace_id,
            &mut module.value,
            with_code,
        ))
        .await?;
    }
    Ok(())
}
