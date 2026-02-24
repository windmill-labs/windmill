use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};

use rand::Rng;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;
use sqlx::types::Json;
use sqlx::types::JsonRawValue;

use crate::{
    assets::AssetWithAltAccessType,
    more_serde::{default_empty_string, default_id, default_null, default_true, is_default},
    runnable_settings::{ConcurrencySettings, ConcurrencySettingsWithCustom, DebouncingSettings},
    scripts::{Schema, ScriptHash, ScriptLang},
    to_raw_value,
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

pub fn is_none_or_false(b: &Option<bool>) -> bool {
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
    pub fn parse_flow_value(&self) -> anyhow::Result<FlowValue> {
        serde_json::from_str(self.value.get())
            .map_err(|e| anyhow::anyhow!("Failed to parse flow value: {}", e))
    }
}

fn validate_retry(retry: &Retry, module_id: &str) -> anyhow::Result<()> {
    if retry.exponential.attempts > 0 && retry.exponential.seconds == 0 {
        return Err(anyhow::anyhow!(
            "Module '{}': Exponential backoff base (seconds) must be greater than 0. A base of 0 would cause immediate retries.",
            module_id
        ));
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
        use anyhow::Context;
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

    pub fn traverse_leafs<C: FnMut(&FlowModuleValue, &String) -> anyhow::Result<()>>(
        modules: Vec<&FlowModule>,
        cb: &mut C,
    ) -> anyhow::Result<()> {
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
                            ToolValue::Mcp(_) => {}
                            ToolValue::Websearch(_) => {}
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
    pub random_factor: Option<i8>,
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
        serde_json::from_str::<FlowModuleValue>(self.value.get())
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn get_value_with_skip_failures(&self) -> anyhow::Result<FlowModuleValueWithSkipFailures> {
        serde_json::from_str::<FlowModuleValueWithSkipFailures>(self.value.get())
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn get_branches_skip_failures(&self) -> anyhow::Result<FlowModuleWithBranches> {
        serde_json::from_str::<FlowModuleWithBranches>(self.value.get())
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn is_flow(&self) -> bool {
        self.get_type().is_ok_and(|x| x == "flow")
    }

    pub fn get_value_with_parallel(&self) -> anyhow::Result<FlowModuleValueWithParallel> {
        serde_json::from_str::<FlowModuleValueWithParallel>(self.value.get())
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn is_ai_agent(&self) -> bool {
        self.get_type().is_ok_and(|x| x == "aiagent")
    }

    pub fn is_simple(&self) -> bool {
        self.get_type()
            .is_ok_and(|x| x == "script" || x == "rawscript" || x == "flowscript")
    }

    pub fn get_type(&self) -> anyhow::Result<&str> {
        #[derive(Deserialize)]
        pub struct FlowModuleValueType<'a> {
            pub r#type: &'a str,
        }

        serde_json::from_str::<FlowModuleValueType>(self.value.get())
            .map_err(|e| anyhow::anyhow!("{}", e))
            .map(|x| x.r#type)
    }

    pub fn traverse_modules<C: FnMut(&FlowModule) -> anyhow::Result<()>>(
        modules: &Vec<FlowModule>,
        cb: &mut C,
    ) -> anyhow::Result<()> {
        for module in modules {
            cb(module)?;
            let module_value = module
                .get_value()
                .map_err(|e| anyhow::anyhow!("Module '{}': {}", module.id, e))?;
            Self::traverse_module_value(&module_value, cb)?;
        }
        Ok(())
    }

    fn traverse_module_value<C: FnMut(&FlowModule) -> anyhow::Result<()>>(
        module_value: &FlowModuleValue,
        cb: &mut C,
    ) -> anyhow::Result<()> {
        match module_value {
            FlowModuleValue::ForloopFlow { modules, .. }
            | FlowModuleValue::WhileloopFlow { modules, .. } => {
                Self::traverse_modules(modules, cb)?;
            }
            FlowModuleValue::BranchOne { branches, default, .. } => {
                for branch in branches {
                    Self::traverse_modules(&branch.modules, cb)?;
                }
                Self::traverse_modules(default, cb)?;
            }
            FlowModuleValue::BranchAll { branches, .. } => {
                for branch in branches {
                    Self::traverse_modules(&branch.modules, cb)?;
                }
            }
            FlowModuleValue::AIAgent { tools, .. } => {
                for tool in tools {
                    let Some(tool_module) = Option::<FlowModule>::from(tool) else {
                        continue;
                    };

                    cb(&tool_module)?;
                    let tool_value = tool_module
                        .get_value()
                        .map_err(|e| anyhow::anyhow!("Tool module '{}': {}", tool_module.id, e))?;
                    Self::traverse_module_value(&tool_value, cb)?;
                }
            }
            _ => {}
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
            ToolValue::Mcp(_) => None,
            ToolValue::Websearch(_) => None,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "tool_type", rename_all = "lowercase")]
pub enum ToolValue {
    FlowModule(FlowModuleValue),
    Mcp(McpToolValue),
    Websearch(WebsearchToolValue),
}

// Custom deserializer for backward compatibility with old flows
impl<'de> Deserialize<'de> for ToolValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let content = serde_json::Value::deserialize(deserializer)?;

        #[derive(Deserialize)]
        #[serde(tag = "tool_type", rename_all = "lowercase")]
        enum TaggedToolValue {
            FlowModule(FlowModuleValue),
            Mcp(McpToolValue),
            Websearch(WebsearchToolValue),
        }

        if let Ok(tagged) = TaggedToolValue::deserialize(&content) {
            return Ok(match tagged {
                TaggedToolValue::FlowModule(v) => ToolValue::FlowModule(v),
                TaggedToolValue::Mcp(v) => ToolValue::Mcp(v),
                TaggedToolValue::Websearch(v) => ToolValue::Websearch(v),
            });
        }

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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct WebsearchToolValue {}

fn is_none_or_empty_vec<T>(expr: &Option<Vec<T>>) -> bool {
    expr.is_none() || expr.as_ref().unwrap().is_empty()
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
        #[serde(skip_serializing_if = "Option::is_none")]
        tag_override: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_trigger: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pass_flow_input_directly: Option<bool>,
    },
    Flow {
        #[serde(default)]
        #[serde(alias = "input_transform")]
        input_transforms: HashMap<String, InputTransform>,
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pass_flow_input_directly: Option<bool>,
    },
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
    WhileloopFlow {
        modules: Vec<FlowModule>,
        #[serde(skip_serializing_if = "Option::is_none")]
        modules_node: Option<FlowNodeId>,
        #[serde(default = "default_false")]
        skip_failures: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        squash: Option<bool>,
    },
    BranchOne {
        branches: Vec<Branch>,
        default: Vec<FlowModule>,
        #[serde(skip_serializing_if = "Option::is_none")]
        default_node: Option<FlowNodeId>,
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
        #[serde(flatten)]
        concurrency_settings: ConcurrencySettingsWithCustom,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_trigger: Option<bool>,
        #[serde(skip_serializing_if = "is_none_or_empty_vec")]
        assets: Option<Vec<AssetWithAltAccessType>>,
    },
    Identity,
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

pub fn ordered_map<S>(
    value: &HashMap<String, InputTransform>,
    serializer: S,
) -> Result<S::Ok, S::Error>
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
    pub dedicated_worker: Option<bool>,
}

pub fn add_virtual_items_if_necessary(modules: &mut Vec<FlowModule>) {
    if modules.len() > 0
        && (modules[modules.len() - 1].sleep.is_some()
            || modules[modules.len() - 1].suspend.is_some())
    {
        modules.push(FlowModule {
            id: format!("{}-v", modules[modules.len() - 1].id),
            value: to_raw_value(&FlowModuleValue::Identity),
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
