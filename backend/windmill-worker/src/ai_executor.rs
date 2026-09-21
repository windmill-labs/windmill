use crate::ai::compaction::{CompactionRequest, Compactor, LastRequest};
use crate::ai::tools::{execute_tool_calls, ToolAbortHandles, ToolExecutionContext};
use crate::ai::utils::{
    add_message_to_conversation, any_tool_needs_previous_result, cleanup_mcp_clients,
    filter_schema_by_input_transforms, find_unique_tool_name, get_flow_context,
    get_flow_job_runnable_and_raw_flow, get_step_name_from_flow, load_mcp_tools,
    parse_raw_script_schema, update_flow_status_module_with_actions,
    update_flow_status_module_with_actions_success,
};
use crate::memory_common::MAX_MEMORY_SIZE_BYTES;
use crate::memory_oss::{memory_storage_capacity_bytes, read_from_memory, write_to_memory};
use crate::worker_flow::{get_previous_job_result, get_transform_context};
use async_recursion::async_recursion;
use regex::Regex;
use serde_json::value::RawValue;
use sha2::Digest;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;
#[cfg(feature = "bedrock")]
use windmill_ai::ai_bedrock::check_env_credentials;
#[cfg(feature = "mcp")]
use windmill_mcp::McpClient;

#[cfg(not(feature = "mcp"))]
use crate::ai::tools::McpClientStub as McpClient;
use windmill_ai::{
    ai_providers::AIProvider,
    image_handler::upload_image_to_s3,
    model_context::model_context_window,
    providers::{
        create_chat_completions_query_builder, create_query_builder, is_chat_completions_only,
        openai::{
            is_reasoning_summary_unavailable, rejects_reasoning_summary,
            remember_reasoning_summary_unavailable,
        },
        remember_chat_completions_only,
    },
    proxy::{
        common_outbound_headers, needs_unavailable_oauth_exchange, retain_effective_credentials,
    },
    query_builder::{BuildRequestArgs, ParsedResponse},
    types::*,
    utils::{pinned_ai_client_for, should_use_structured_output_tool},
};
use windmill_common::{
    cache,
    client::AuthedClient,
    db::DB,
    error::{self, Error},
    flow_conversations::{memory_key, MessageExtras, MessageType},
    flow_status::AgentAction,
    flows::{AgentTool, FlowModule, FlowModuleValue, InputTransform, ToolValue},
    get_latest_hash_for_path,
    jobs::JobKind,
    scripts::get_full_hub_script_by_path,
    utils::{StripPath, HTTP_CLIENT},
    worker::{to_raw_value, Connection},
};
use windmill_queue::{append_logs, cancel_single_job, CanceledBy, MiniPulledJob};

use crate::{
    ai::stream_event_processor::StreamEventProcessor,
    common::{
        build_args_map, resolve_job_timeout, transform_json_value, OccupancyMetrics, StreamNotifier,
    },
    handle_child::{run_future_with_polling_update_job_poller_graceful, GracefulPollOutcome},
};

lazy_static::lazy_static! {
    static ref TOOL_NAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();

    static ref AI_AGENT_TOOL_SCHEMA: Box<RawValue> = to_raw_value(&serde_json::json!({
        "type": "object",
        "properties": {
            "user_message": { "type": "string" },
        },
        "required": ["user_message"],
        "additionalProperties": false,
    }));
}

const DEFAULT_MAX_AGENT_ITERATIONS: usize = 10;
const HARD_MAX_AGENT_ITERATIONS: usize = 1000;

/// What a run stopped by `max_iterations` reports back. `Message` rather than
/// `OpenAIMessage` is load-bearing: `agent_action` is `skip_serializing` on the
/// latter and reaches JSON only through this wrapper, so serializing these raw
/// drops every tool name and job id and leaves the partial run unreadable.
#[derive(serde::Serialize)]
struct MaxIterPartialResult<'a> {
    messages: Vec<Message<'a>>,
}

fn strip_system_messages(messages: &[OpenAIMessage]) -> Vec<OpenAIMessage> {
    messages
        .iter()
        .filter(|message| message.role != "system")
        .cloned()
        .collect()
}

fn strip_leading_tool_messages(messages: Vec<OpenAIMessage>) -> Vec<OpenAIMessage> {
    match messages.iter().position(|message| message.role != "tool") {
        Some(first_non_tool_index) => messages.into_iter().skip(first_non_tool_index).collect(),
        None => Vec::new(),
    }
}

fn prepare_auto_memory_messages_for_request(
    loaded_messages: &[OpenAIMessage],
    context_length: usize,
) -> Vec<OpenAIMessage> {
    let start_idx = loaded_messages.len().saturating_sub(context_length);
    strip_leading_tool_messages(loaded_messages[start_idx..].to_vec())
}

fn prepare_auto_memory_messages_for_persistence(
    all_messages: &[OpenAIMessage],
    context_length: usize,
) -> Vec<OpenAIMessage> {
    let non_system_messages = strip_system_messages(all_messages);
    let start_idx = non_system_messages.len().saturating_sub(context_length);
    non_system_messages[start_idx..].to_vec()
}

/// Everything the two compaction sites of the agent loop share. Only the query builder
/// and `include_usage` differ between them, since both can change mid-run.
struct CompactionContext<'a> {
    compactor: Option<&'a mut Compactor>,
    timeout: Option<std::time::Duration>,
    credentials: &'a windmill_ai::credentials::ProviderCredentials,
    args: &'a AIAgentArgs,
    client: &'a AuthedClient,
    workspace_id: &'a str,
}

/// Compacts when the conversation has outgrown the window, billing the summarization
/// call to the step. Returns whether it did.
async fn compact_if_needed(
    ctx: CompactionContext<'_>,
    query_builder: &dyn windmill_ai::query_builder::QueryBuilder,
    include_usage: bool,
    messages: &mut Vec<OpenAIMessage>,
    last_request: &mut LastRequest,
    final_usage: &mut Option<TokenUsage>,
) -> bool {
    let (Some(compactor), Some(timeout)) = (ctx.compactor, ctx.timeout) else {
        return false;
    };
    let pass = compactor
        .maybe_compact(
            messages,
            *last_request,
            &CompactionRequest {
                query_builder,
                credentials: ctx.credentials,
                model: ctx.args.provider.get_model(),
                timeout,
                client: ctx.client,
                workspace_id: ctx.workspace_id,
                include_usage,
            },
        )
        .await;
    if let Some(usage) = pass.usage {
        match final_usage {
            Some(existing) => existing.accumulate(&usage),
            None => *final_usage = Some(usage),
        }
    }
    // A rewritten conversation no longer matches the provider's count for the request
    // that produced it: the count indexed the old message list. Drop it so any later
    // pass measures the estimate over the actual messages rather than a stale, larger
    // prompt — reusing it can decline a summary that already fits, then drop it.
    if pass.compacted {
        *last_request = LastRequest { prompt_tokens: None, message_count: 0 };
    }
    pass.compacted
}

/// The inputs a linked step supplies for itself; the resource holds the rest of the brain.
const FLOW_LOCAL_AGENT_KEYS: [&str; 5] = [
    "user_message",
    "user_attachments",
    "enabled_tools",
    "memory_id",
    "previous_messages",
];

/// The flow-local inputs that name a conversation, which a saved agent never carries.
const STEP_HISTORY_KEYS: [&str; 2] = ["memory_id", "previous_messages"];

/// Where one agent invocation's history comes from.
#[derive(Debug)]
enum HistorySource<'a> {
    /// Supplied by the flow and replayed as is: memory is neither read nor written.
    Messages(&'a [OpenAIMessage]),
    Managed {
        memory_id: Uuid,
        bound: MemoryBound,
    },
    Stateless,
}

/// What keeps a managed conversation inside the model's context.
#[derive(Debug, Clone, Copy)]
enum MemoryBound {
    /// Only the most recent messages are sent and kept.
    LastMessages(usize),
    /// Everything is sent and kept; compaction summarizes what no longer fits.
    Compaction { context_window: usize },
}

impl MemoryBound {
    /// How many of the memory's messages a run reads and writes back. Compaction keeps all of
    /// them: the summary it writes is what bounds the conversation, so truncating on top of it
    /// would drop the tail that summary was written to precede.
    fn messages_to_keep(self) -> usize {
        match self {
            MemoryBound::LastMessages(context_length) => context_length,
            MemoryBound::Compaction { .. } => usize::MAX,
        }
    }
}

/// A step's memory id counts only as the step authored it. A static empty value is a form
/// placeholder, so it reads as unset rather than as an expression that evaluated to nothing, which
/// runs without memory; an AI-filled value would let the model choose which memory the agent reads.
fn keep_authored_memory_id(
    args: &mut AIAgentArgs,
    step_input_transforms: &HashMap<String, InputTransform>,
) {
    match step_input_transforms.get("memory_id") {
        Some(InputTransform::Javascript { .. }) => {}
        Some(InputTransform::Static { .. }) if args.memory_id.as_deref() != Some("") => {}
        _ => args.memory_id = None,
    }
}

/// Reconciles the step's history inputs, the agent's memory policy and the run's memory id. A step
/// holds one of two shapes: an older `auto` or `manual` memory, read as the editor that wrote it
/// meant it, or the current setting plus the step's own history inputs. Also returns lines for the
/// job log: an input that went unused, or a policy that remembers ending up stateless.
fn resolve_history_source<'a>(
    args: &'a AIAgentArgs,
    run_memory_id: Option<Uuid>,
    workspace_id: &str,
    flow_path: &str,
) -> (HistorySource<'a>, Vec<&'static str>) {
    let mut notes = Vec::new();
    let managed = |bound: MemoryBound, notes: &mut Vec<&'static str>| {
        managed_memory_id(args, run_memory_id, workspace_id, flow_path, notes)
            .map_or(HistorySource::Stateless, |memory_id| {
                HistorySource::Managed { memory_id, bound }
            })
    };
    match &args.memory {
        // The step's own history inputs came after these, so a step that still holds one reads it
        // alone: what it did before the editor offered them is what it keeps doing.
        Some(Memory::Manual { messages }) => {
            note_unread_step_inputs(&mut notes, args);
            (HistorySource::Messages(messages), notes)
        }
        Some(Memory::Auto { context_length, memory_id }) => {
            note_unread_step_inputs(&mut notes, args);
            // An id baked in at save time only ever applied when the run carried none.
            match run_memory_id.or(*memory_id) {
                Some(memory_id) => (
                    HistorySource::Managed {
                        memory_id,
                        bound: MemoryBound::LastMessages(*context_length),
                    },
                    notes,
                ),
                None => {
                    notes.push(NO_RUN_MEMORY_ID);
                    (HistorySource::Stateless, notes)
                }
            }
        }
        Some(Memory::Window { context_length }) => {
            let source = managed(MemoryBound::LastMessages(*context_length), &mut notes);
            (source, notes)
        }
        Some(Memory::Compaction { context_window }) => {
            let source = managed(
                MemoryBound::Compaction { context_window: *context_window },
                &mut notes,
            );
            (source, notes)
        }
        Some(Memory::Off) | None => {
            if args.memory_id.as_deref().is_some_and(|id| !id.is_empty()) {
                notes.push("Managed memory is off, so this step's memory id is ignored.");
            }
            match &args.previous_messages {
                Some(messages) => (HistorySource::Messages(messages), notes),
                None => (HistorySource::Stateless, notes),
            }
        }
    }
}

const NO_RUN_MEMORY_ID: &str =
    "No memory id was passed to this run, so the agent runs without memory.";

/// The memory a step under a current setting reads and writes: its own id where it set one,
/// otherwise the run's. `None` is a step that ends up stateless, with the reason noted.
fn managed_memory_id(
    args: &AIAgentArgs,
    run_memory_id: Option<Uuid>,
    workspace_id: &str,
    flow_path: &str,
    notes: &mut Vec<&'static str>,
) -> Option<Uuid> {
    if args
        .previous_messages
        .as_ref()
        .is_some_and(|messages| !messages.is_empty())
    {
        notes.push("Managed memory is on, so this step's previous messages are ignored.");
    }
    match args.memory_id.as_deref() {
        Some("") => {
            notes.push(
                "This step's memory id evaluated to an empty value, so the agent runs without memory.",
            );
            None
        }
        Some(step_memory_id) => Some(memory_key(workspace_id, flow_path, step_memory_id)),
        None => run_memory_id.or_else(|| {
            notes.push(NO_RUN_MEMORY_ID);
            None
        }),
    }
}

/// An older memory setting reads neither history input, which is only visible in the job log: the
/// editor offers them on a step that has been moved to the current settings.
fn note_unread_step_inputs(notes: &mut Vec<&'static str>, args: &AIAgentArgs) {
    if args.memory_id.as_deref().is_some_and(|id| !id.is_empty()) {
        notes.push("This step uses an older memory setting, so its memory id is not read.");
    }
    if args
        .previous_messages
        .as_ref()
        .is_some_and(|messages| !messages.is_empty())
    {
        notes
            .push("This step uses an older memory setting, so its previous messages are not read.");
    }
}

/// Whether a request has something to ask the model. Only text output sends previous messages, so
/// an image prompt comes from the user message alone. An empty list is no conversation, except
/// under a legacy `manual` memory, which ran on whatever list it held.
fn has_prompt(
    history: &HistorySource,
    has_user_message: bool,
    is_text_output: bool,
    legacy_list: bool,
) -> bool {
    has_user_message
        || (is_text_output
            && (legacy_list || matches!(history, HistorySource::Messages(m) if !m.is_empty())))
}

fn find_module_by_id(
    modules: &Vec<FlowModule>,
    target_id: &str,
) -> Result<Option<FlowModule>, Error> {
    let mut found: Option<FlowModule> = None;
    FlowModule::traverse_modules(modules, &mut |module| {
        if found.is_none() && module.id == target_id {
            found = Some(module.clone());
        }
        Ok(())
    })
    .map_err(|e| Error::internal_err(format!("Failed to traverse flow modules: {e}")))?;
    Ok(found)
}

async fn find_ai_agent_tool_module_in_parent_agent(
    modules: &Vec<FlowModule>,
    parent_agent_step_id: &str,
    tool_module_id: &str,
    client: &AuthedClient,
) -> Result<Option<FlowModule>, Error> {
    let Some(parent_agent_module) = find_module_by_id(modules, parent_agent_step_id)? else {
        return Ok(None);
    };

    let FlowModuleValue::AIAgent { tools, agent, tool_inputs, .. } =
        parent_agent_module.get_value()?
    else {
        return Ok(None);
    };

    // A linked parent carries no tools on the module (they live in the resource, resolved only in
    // the main execution branch). Resolve them from the resource here too, so a nested agent tool
    // of a saved+linked agent can still be located when it runs as its own job.
    let mut tools = if let Some(agent_ref) = agent.as_deref() {
        let agent_path = agent_ref
            .trim_start_matches("$res:")
            .trim_start_matches("res://");
        // Definitions only: resolving their defaults here would hit the same inaccessible resources.
        let resource_value = client
            .get_resource_value::<serde_json::Value>(agent_path)
            .await
            .map_err(|e| {
                Error::internal_err(format!(
                    "failed to load ai_agent resource {agent_path}: {e}"
                ))
            })?;
        match resource_value {
            serde_json::Value::Object(mut map) => match map.remove("tools") {
                Some(t) => serde_json::from_value::<Vec<AgentTool>>(t).map_err(|e| {
                    Error::internal_err(format!(
                        "invalid tools in ai_agent resource {agent_path}: {e}"
                    ))
                })?,
                None => Vec::new(),
            },
            _ => Vec::new(),
        }
    } else {
        tools
    };
    // The nested job reads its history inputs from the tool's transforms, which must carry the
    // host flow's bindings as the parent evaluated them.
    overlay_tool_inputs(&mut tools, &tool_inputs);

    for tool in tools {
        if tool.id == tool_module_id {
            return Ok(Option::<FlowModule>::from(&tool));
        }
    }

    Ok(None)
}

/// Resolve the `description` sent to the model for an AI agent tool, in priority order:
/// an explicit per-tool description, then one auto-derived from the underlying runnable,
/// then the tool name as the historical last-resort fallback. Blank/whitespace-only values
/// at each level are skipped so a lower-priority source can still apply.
fn resolve_tool_description(
    user_description: Option<String>,
    derived_description: Option<String>,
    tool_name: &str,
) -> String {
    fn non_empty(value: Option<String>) -> Option<String> {
        value
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    non_empty(user_description)
        .or_else(|| non_empty(derived_description))
        .unwrap_or_else(|| tool_name.to_string())
}

/// Fetch a workspace script's stored description by hash, used to auto-derive an AI agent
/// tool's description when the user did not provide an explicit one. Returns `None` when the
/// script has no description or on any lookup error, so the caller falls back to the tool name.
async fn fetch_script_description(db: &DB, w_id: &str, hash: i64) -> Option<String> {
    sqlx::query_scalar!(
        "SELECT description FROM script WHERE hash = $1 AND workspace_id = $2",
        hash,
        w_id,
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .map(|d| d.trim().to_string())
    .filter(|d| !d.is_empty())
}

/// Overlay a linked step's host-local tool wiring onto the agent resource's tools. For each tool
/// id present in `tool_inputs`, merge its per-input transforms into that tool's `input_transforms`
/// (step wins). Only `FlowModule` tools carry input transforms; MCP/websearch tools are skipped.
fn overlay_tool_inputs(
    tools: &mut [AgentTool],
    tool_inputs: &HashMap<String, HashMap<String, InputTransform>>,
) {
    if tool_inputs.is_empty() {
        return;
    }
    for tool in tools.iter_mut() {
        let Some(overrides) = tool_inputs.get(&tool.id) else {
            continue;
        };
        let ToolValue::FlowModule(fmv) = &mut tool.value else {
            continue;
        };
        let input_transforms = match fmv {
            FlowModuleValue::Script { input_transforms, .. }
            | FlowModuleValue::RawScript { input_transforms, .. }
            | FlowModuleValue::FlowScript { input_transforms, .. }
            | FlowModuleValue::AIAgent { input_transforms, .. } => input_transforms,
            _ => continue,
        };
        for (key, transform) in overrides {
            input_transforms.insert(key.clone(), transform.clone());
        }
    }
}

/// What every websearch entry is named by, whatever label it carries. Web search reaches the model
/// as a provider capability rather than a tool, so it has no model-facing name of its own, and the
/// editor's label is not one: a flow module tool could carry the same one, and enabling that tool
/// would then silently turn web search on with it.
///
/// Reserved, on the `__wm_` prefix this codebase uses for names it keeps for itself, and held that
/// way by `flow_module_tool_name` refusing to advertise a tool that takes it.
const WEBSEARCH_ENABLED_NAME: &str = "__wm_web_search";

/// The name a flow module tool is advertised to the model under.
///
/// Rejected rather than skipped: a tool the model is never shown is a tool the agent silently does
/// not have, and a run that quietly drops one is harder to explain than a run that will not start.
fn flow_module_tool_name(summary: Option<&str>) -> Result<&str, Error> {
    match summary {
        Some(name) if name == WEBSEARCH_ENABLED_NAME => Err(Error::internal_err(format!(
            "Invalid tool name: {name:?} is reserved for enabling web search"
        ))),
        Some(name) if TOOL_NAME_REGEX.is_match(name) => Ok(name),
        other => Err(Error::internal_err(format!("Invalid tool name: {other:?}"))),
    }
}

/// The name a run enables a roster entry by: the name the model is shown, except for an entry the
/// model is shown nothing of, which cannot be named by a label others may share. An MCP server is
/// named by the resource it points at, web search by `WEBSEARCH_ENABLED_NAME`.
///
/// The MCP path is bare. The roster stores it as authored, `$res:` and all, but a name is an
/// argument value and one carrying that prefix is resolved to the resource itself before the worker
/// is handed its args, so the prefixed form is not something the list can hold.
fn tool_enabled_name(tool: &AgentTool) -> Option<&str> {
    match &tool.value {
        ToolValue::Mcp(mcp) => Some(mcp.resource_path.trim_start_matches("$res:")),
        ToolValue::Websearch(_) => Some(WEBSEARCH_ENABLED_NAME),
        _ => tool.summary.as_deref(),
    }
}

/// The roster a run advertises, given the names it enabled.
///
/// `None` advertises the whole roster, which is what every agent written before the field existed
/// relies on; an empty list advertises nothing.
///
/// Whole entries, decided before any of them is resolved: an MCP server a run switched off is never
/// contacted, and reading its resource, refreshing its token and opening a client are each things
/// that can fail a run. Which tools a server exposes stays its entry's own `include_tools` /
/// `exclude_tools`, the only place that choice is made.
fn narrow_roster(tools: Vec<AgentTool>, enabled_tools: Option<&[String]>) -> Vec<AgentTool> {
    let Some(enabled) = enabled_tools else {
        return tools;
    };
    tools
        .into_iter()
        .filter(|t| tool_enabled_name(t).is_some_and(|name| enabled.iter().any(|n| n == name)))
        .collect()
}

/// The log line for names in `enabled_tools` that name nothing on the agent, if any. The list can
/// be computed per run, so a name that has since been renamed away must not fail the step — but it
/// would otherwise silently narrow the agent, so the run says how many of its names matched nothing.
///
/// Counted, never quoted: a name is an argument value, and one written as `$var:path` reaches the
/// worker already replaced by the variable's own value. Quoting it would write that value to the
/// job log, where the masks a job registers never reach it — they are applied to a subprocess's
/// output in `handle_child` and nowhere else, and `append_logs` stores what it is given verbatim.
fn unmatched_enabled_tools_message(
    enabled_tools: &[String],
    advertised: &[&str],
) -> Option<String> {
    let unmatched = enabled_tools
        .iter()
        .filter(|name| !advertised.contains(&name.as_str()))
        .count();
    if unmatched == 0 {
        return None;
    }
    let subject = if unmatched == 1 { "name" } else { "names" };
    Some(format!(
        "--- ENABLED TOOLS: {unmatched} {subject} named no tool of this agent and had no effect ---\n"
    ))
}

pub async fn handle_ai_agent_job(
    // connection
    conn: &Connection,
    db: &DB,

    // agent job
    job: &MiniPulledJob,

    // job execution context
    client: &AuthedClient,
    canceled_by: &mut Option<CanceledBy>,
    mem_peak: &mut i32,
    occupancy_metrics: &mut OccupancyMetrics,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    hostname: &str,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
    has_stream: &mut bool,
) -> Result<Box<RawValue>, Error> {
    // build_args_map returns None if no $res:/$var: transforms needed, in which case use original args
    let local_args = match build_args_map(job, client, conn).await? {
        Some(transformed) => transformed,
        None => job.args.as_ref().map(|a| a.0.clone()).unwrap_or_default(),
    };

    // Handle dry_run mode - check credentials without making API calls.
    // The credentials check is always invoked inline (provider present, no agent link and no
    // parent flow), so it resolves before any flow/agent-resource context is fetched.
    let is_credentials_check = local_args
        .get("credentials_check")
        .map(|v| v.get().trim() == "true")
        .unwrap_or(false);
    if is_credentials_check {
        let args = serde_json::from_str::<AIAgentArgs>(&serde_json::to_string(&local_args)?)?;
        return handle_credentials_check(&args.provider).await;
    }

    // flow_step_id is set by the flow executor for top-level AI agents.
    // For nested AI agent tools, it's not set (to avoid triggering flow step
    // machinery on a parent that has no v2_job_status row), so we extract the
    // tool module ID from the runnable_path which has the form ".../tools/{id}".
    let flow_step_id = job
        .flow_step_id
        .as_deref()
        .or_else(|| job.runnable_path().rsplit_once("/tools/").map(|(_, id)| id))
        .ok_or_else(|| Error::internal_err("AI agent job has no flow step id".to_string()))?
        .to_string();
    let flow_step_id = &flow_step_id;

    let Some(immediate_parent_job) = &job.parent_job else {
        return Err(Error::internal_err(
            "AI agent job has no parent job".to_string(),
        ));
    };

    let mut flow_job_id = *immediate_parent_job;
    let mut flow_job = get_flow_job_runnable_and_raw_flow(db, &flow_job_id).await?;
    let direct_parent_job_kind = flow_job.kind;
    let direct_parent_job_flow_step_id = flow_job.flow_step_id.clone();

    // If the direct parent is an AI agent (nested tool case), go one level up to the flow.
    if flow_job.kind == JobKind::AIAgent {
        let Some(parent_job_id) = flow_job.parent_job else {
            return Err(Error::internal_err(
                "AI agent parent has no parent job".to_string(),
            ));
        };
        flow_job_id = parent_job_id;
        flow_job = get_flow_job_runnable_and_raw_flow(db, &flow_job_id).await?;

        if !matches!(
            flow_job.kind,
            JobKind::Flow | JobKind::FlowNode | JobKind::FlowPreview
        ) {
            return Err(Error::internal_err(
                "AI agent nesting beyond 2 levels is not supported. \
                 Only flow → agent → nested agent tool is allowed."
                    .to_string(),
            ));
        }
    }

    let flow_data = match flow_job.kind {
        JobKind::Flow | JobKind::FlowNode => {
            cache::job::fetch_flow(db, &flow_job.kind, flow_job.runnable_id).await?
        }
        JobKind::FlowPreview => {
            cache::job::fetch_preview_flow(db, &flow_job_id, flow_job.raw_flow).await?
        }
        _ => {
            return Err(Error::internal_err(
                "expected parent flow, flow preview or flow node for ai agent job".to_string(),
            ));
        }
    };

    let value = flow_data.value();

    let module = if direct_parent_job_kind == JobKind::AIAgent {
        let parent_agent_step_id = direct_parent_job_flow_step_id.as_deref().ok_or_else(|| {
            Error::internal_err("Parent AI agent job has no flow_step_id".to_string())
        })?;
        find_ai_agent_tool_module_in_parent_agent(
            &value.modules,
            parent_agent_step_id,
            flow_step_id,
            client,
        )
        .await?
    } else {
        find_module_by_id(&value.modules, flow_step_id)?
    };

    let Some(module) = module else {
        return Err(Error::internal_err(
            "AI agent module not found in flow".to_string(),
        ));
    };

    let summary = module.summary.clone();

    let FlowModuleValue::AIAgent {
        tools: module_tools,
        omit_output_from_conversation,
        agent,
        tool_inputs,
        input_transforms: step_input_transforms,
        ..
    } = module.get_value()?
    else {
        return Err(Error::internal_err(
            "AI agent module is not an AI agent".to_string(),
        ));
    };

    // A linked step takes its brain and tools from the resource and keeps only its own flow-local
    // inputs. The brain and the roster stay rigid; what the step binds to this flow is the message
    // it asks, which of those tools this use may call, the conversation it is part of (its memory
    // id and previous messages), and the tools' own inputs — the last overlaid from `tool_inputs`
    // below.
    let (mut args, tools): (AIAgentArgs, Vec<AgentTool>) = if let Some(agent_ref) = agent.as_deref()
    {
        let agent_path = agent_ref
            .trim_start_matches("$res:")
            .trim_start_matches("res://");
        // Read raw and interpolate only the brain below. Interpolating the whole resource would also
        // resolve each tool's default `$res:`/`$var:`, which a host flow may be overriding and which
        // may be unreadable to whoever runs this flow — an unused tool could then fail the agent.
        let resource_value = client
            .get_resource_value::<serde_json::Value>(agent_path)
            .await
            .map_err(|e| {
                Error::internal_err(format!(
                    "failed to load ai_agent resource {agent_path}: {e}"
                ))
            })?;
        let mut config = match resource_value {
            serde_json::Value::Object(map) => map,
            _ => {
                return Err(Error::internal_err(format!(
                    "ai_agent resource {agent_path} must be a JSON object"
                )))
            }
        };
        let mut tools = match config.remove("tools") {
            Some(t) => serde_json::from_value::<Vec<AgentTool>>(t).map_err(|e| {
                Error::internal_err(format!(
                    "invalid tools in ai_agent resource {agent_path}: {e}"
                ))
            })?,
            None => Vec::new(),
        };
        overlay_tool_inputs(&mut tools, &tool_inputs);
        // The resource is not validated against a schema, so a history input it happens to carry
        // is dropped before interpolation, where a bad `$res:` in it would fail the step. The
        // other flow-local keys stay: a resource's own user message is the step's fallback.
        for key in STEP_HISTORY_KEYS {
            config.remove(key);
        }
        let brain = transform_json_value(
            "ai_agent",
            client,
            &job.workspace_id,
            serde_json::Value::Object(config),
            job,
            conn,
            0,
        )
        .await?;
        let mut brain = match brain {
            serde_json::Value::Object(map) => map,
            _ => {
                return Err(Error::internal_err(format!(
                    "ai_agent resource {agent_path} must be a JSON object"
                )))
            }
        };
        // Only after interpolating the resource: these are caller-controlled and already resolved by
        // build_args_map, so passing them through it again would expand contextual values —
        // `$WM_TOKEN` in a user message would reach the model provider.
        for key in FLOW_LOCAL_AGENT_KEYS {
            if let Some(v) = local_args.get(key) {
                brain.insert(
                    key.to_string(),
                    serde_json::from_str(v.get()).unwrap_or(serde_json::Value::Null),
                );
            }
        }
        let args = serde_json::from_value::<AIAgentArgs>(serde_json::Value::Object(brain))
            .map_err(|e| {
                Error::internal_err(format!(
                    "invalid ai_agent resource config {agent_path}: {e}"
                ))
            })?;
        (args, tools)
    } else {
        let args = serde_json::from_str::<AIAgentArgs>(&serde_json::to_string(&local_args)?)?;
        // "Edit" on a linked step clears `agent` but keeps the host's `tool_inputs`, so overlay them
        // here too: a flow persisted mid-edit must still bind its tools to this flow's context
        // rather than the agent author's.
        let mut tools = module_tools;
        overlay_tool_inputs(&mut tools, &tool_inputs);
        (args, tools)
    };

    keep_authored_memory_id(&mut args, &step_input_transforms);

    // Nesting is capped at flow → agent → nested agent. When this job is itself a nested tool,
    // a linked resource's tool set may still contain AIAgent tools (the editor can't constrain a
    // shared resource); don't advertise them — invoking one would only fail the depth check as a
    // third-level agent.
    let tools = if direct_parent_job_kind == JobKind::AIAgent {
        tools
            .into_iter()
            .filter(|t| {
                !matches!(
                    &t.value,
                    ToolValue::FlowModule(FlowModuleValue::AIAgent { .. })
                )
            })
            .collect()
    } else {
        tools
    };

    // Narrow the roster to the tools this run enabled, before the loop below pays a script or hub
    // fetch per tool.
    let enabled_tools = args.enabled_tools.as_deref();
    // Taken before the narrowing consumes the roster, and only by a run that narrows: they are what
    // its names are matched against, so they are also what tells it a name matched nothing.
    let roster_names: Vec<String> = match enabled_tools {
        Some(_) => tools
            .iter()
            .filter_map(|t| tool_enabled_name(t).map(str::to_string))
            .collect(),
        None => Vec::new(),
    };
    let tools = narrow_roster(tools, enabled_tools);

    // Separate Windmill tools from MCP tools, websearch, and extract MCP resource configs
    let mut windmill_modules: Vec<FlowModule> = Vec::new();
    // Explicit per-tool descriptions keyed by tool id. When set, these override the
    // description auto-derived from the underlying runnable when building tool definitions.
    let mut tool_descriptions: HashMap<String, String> = HashMap::new();
    #[allow(unused_mut)]
    let mut mcp_configs: Vec<crate::ai::utils::McpResourceConfig> = Vec::new();
    let mut has_websearch = false;

    for tool in tools {
        match &tool.value {
            #[allow(unused_variables)]
            ToolValue::Mcp(mcp_config) => {
                #[cfg(feature = "mcp")]
                {
                    // This is an MCP tool - extract config
                    tracing::debug!(
                        "MCP server module: path={}, include={:?}, exclude={:?}",
                        mcp_config.resource_path,
                        mcp_config.include_tools,
                        mcp_config.exclude_tools
                    );
                    mcp_configs.push(crate::ai::utils::McpResourceConfig {
                        resource_path: mcp_config.resource_path.clone(),
                        include_tools: Some(mcp_config.include_tools.clone()),
                        exclude_tools: Some(mcp_config.exclude_tools.clone()),
                    });
                }

                #[cfg(not(feature = "mcp"))]
                {
                    tracing::warn!("MCP tool detected but MCP feature is not enabled");
                }
            }
            ToolValue::FlowModule(_) => {
                // Regular Windmill flow module (script, flow, etc.) - convert to FlowModule
                tracing::debug!("Windmill module: {:?}", tool.id);
                if let Some(description) = tool
                    .description
                    .as_ref()
                    .map(|d| d.trim())
                    .filter(|d| !d.is_empty())
                {
                    tool_descriptions.insert(tool.id.clone(), description.to_string());
                }
                if let Some(flow_module) = Option::<FlowModule>::from(&tool) {
                    windmill_modules.push(flow_module);
                }
            }
            ToolValue::Websearch(_) => {
                // WebSearch tool - mark as enabled
                tracing::debug!("WebSearch tool enabled");
                has_websearch = true;
            }
        }
    }

    // Process Windmill flow modules into Tool definitions
    let tools = futures::future::try_join_all(windmill_modules.into_iter().map(|mut t| {
        let conn = conn;
        let db = db;
        let job = job;
        let user_description = tool_descriptions.get(&t.id).cloned();
        async move {
            let summary = flow_module_tool_name(t.summary.as_deref())?;

            // Extract schema, input_transforms, and an auto-derived description from the module value
            let module_value = t.get_value()?;
            let (schema, input_transforms, derived_description) = match &module_value {
                FlowModuleValue::Script {
                    hash,
                    path,
                    tag_override,
                    input_transforms,
                    is_trigger,
                    pass_flow_input_directly,
                } => {
                    let derived_description: Option<String>;
                    let schema = match hash {
                        Some(hash) => {
                            let (_, metadata) = cache::script::fetch(conn, hash.clone()).await?;
                            derived_description =
                                fetch_script_description(db, &job.workspace_id, hash.0).await;
                            Ok::<_, Error>(
                                metadata
                                    .schema
                                    .clone()
                                    .map(|s| RawValue::from_string(s).ok())
                                    .flatten(),
                            )
                        }
                        None => {
                            if path.starts_with("hub/") {
                                let hub_script = get_full_hub_script_by_path(
                                    StripPath(path.to_string()),
                                    &HTTP_CLIENT,
                                    None,
                                )
                                .await?;
                                // Hub scripts carry their free-text description in `summary`.
                                derived_description = hub_script
                                    .summary
                                    .as_ref()
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty());
                                Ok(Some(hub_script.schema))
                            } else {
                                let hash = get_latest_hash_for_path(
                                    db,
                                    db,
                                    &job.workspace_id,
                                    path.as_str(),
                                    true,
                                )
                                .await?
                                .0;
                                // update module definition to use a fixed hash so all tool calls match the same schema
                                t.value = to_raw_value(&FlowModuleValue::Script {
                                    hash: Some(hash),
                                    path: path.clone(),
                                    tag_override: tag_override.clone(),
                                    input_transforms: input_transforms.clone(),
                                    is_trigger: *is_trigger,
                                    pass_flow_input_directly: *pass_flow_input_directly,
                                });
                                derived_description =
                                    fetch_script_description(db, &job.workspace_id, hash.0).await;
                                let (_, metadata) = cache::script::fetch(conn, hash).await?;
                                Ok(metadata
                                    .schema
                                    .clone()
                                    .map(|s| RawValue::from_string(s).ok())
                                    .flatten())
                            }
                        }
                    }?;
                    (schema, input_transforms, derived_description)
                }
                FlowModuleValue::RawScript { content, language, input_transforms, .. } => {
                    let schema = Some(parse_raw_script_schema(&content, &language).await?);
                    (schema, input_transforms, None)
                }
                FlowModuleValue::AIAgent { input_transforms, .. } => {
                    // By convention for AIAgent tools, only user_message is expected to be AI-filled.
                    (
                        Some(
                            RawValue::from_string(AI_AGENT_TOOL_SCHEMA.get().to_string())
                                .expect("AI_AGENT_TOOL_SCHEMA should always be valid JSON"),
                        ),
                        input_transforms,
                        None,
                    )
                }
                _ => {
                    return Err(Error::internal_err(format!(
                        "Unsupported tool: {}",
                        summary
                    )));
                }
            };

            // Filter schema based on user given input transforms
            let schema = if let Some(s) = schema {
                Some(filter_schema_by_input_transforms(s, input_transforms)?)
            } else {
                None
            };

            let description =
                resolve_tool_description(user_description, derived_description, summary);

            Ok(Tool {
                def: ToolDef {
                    r#type: "function".to_string(),
                    function: ToolDefFunction {
                        name: summary.to_string(),
                        description: Some(description),
                        parameters: schema.unwrap_or_else(|| {
                            to_raw_value(&serde_json::json!({
                                "type": "object",
                                "properties": {},
                                "required": [],
                            }))
                        }),
                    },
                },
                module: Some(t),
                mcp_source: None,
            })
        }
    }))
    .await?;

    // Load MCP tools if configured
    let mut tools = tools;

    let mcp_clients = if !mcp_configs.is_empty() {
        let (clients, mcp_tools) =
            load_mcp_tools(db, &job.workspace_id, mcp_configs, client).await?;
        tools.extend(mcp_tools);
        clients
    } else {
        HashMap::new()
    };

    if let Some(enabled) = enabled_tools {
        let matchable: Vec<&str> = roster_names.iter().map(|s| s.as_str()).collect();
        windmill_common::feature_usage::log_feature_usage(
            "ai_agent",
            "dynamic_tools",
            if tools.is_empty() && !has_websearch {
                "no_tools"
            } else {
                "tools"
            },
        );
        if let Some(message) = unmatched_enabled_tools_message(enabled, &matchable) {
            append_logs(&job.id, &job.workspace_id, message, conn).await;
        }
    }

    let mut inner_occupancy_metrics = occupancy_metrics.clone();

    let stream_notifier = StreamNotifier::new(conn, job);

    if let Some(stream_notifier) = stream_notifier {
        stream_notifier.update_flow_status_with_stream_job();
    }

    let flow_status_job = if direct_parent_job_kind == JobKind::AIAgent {
        None
    } else {
        Some(flow_job_id)
    };

    // Create cancellation signal for graceful shutdown
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    let tool_abort_handles: ToolAbortHandles = Arc::new(std::sync::Mutex::new(Vec::new()));

    /// Grace period for in-flight tool calls to complete after cancellation.
    const CANCEL_GRACE_PERIOD: std::time::Duration = std::time::Duration::from_secs(30);

    let outcome = {
        let agent_fut = run_agent(
            db,
            conn,
            job,
            flow_status_job.as_ref(),
            Some(flow_step_id.as_str()),
            &args,
            &tools,
            &mcp_clients,
            summary.as_deref(),
            client,
            &mut inner_occupancy_metrics,
            worker_dir,
            base_internal_url,
            worker_name,
            hostname,
            killpill_rx,
            has_stream,
            has_websearch,
            omit_output_from_conversation,
            cancel_rx,
            tool_abort_handles.clone(),
        );

        let mut occupancy_opt = Some(occupancy_metrics);

        run_future_with_polling_update_job_poller_graceful(
            job.id,
            job.timeout,
            conn,
            mem_peak,
            canceled_by,
            agent_fut,
            worker_name,
            &job.workspace_id,
            &mut occupancy_opt,
            Box::pin(futures::stream::once(async { 0 })),
            cancel_tx,
            CANCEL_GRACE_PERIOD,
        )
        .await?
    };
    // agent_fut and update_job are now dropped — borrows on mcp_clients and canceled_by released

    // Cleanup MCP clients
    cleanup_mcp_clients(mcp_clients).await;

    let format_cancel_info = |cb: &Option<CanceledBy>| {
        cb.as_ref()
            .map_or(("unknown".to_string(), "unknown".to_string()), |x| {
                (
                    x.username.clone().unwrap_or_default(),
                    x.reason.clone().unwrap_or_default(),
                )
            })
    };

    match outcome {
        GracefulPollOutcome::Ok(result) => Ok(result),
        GracefulPollOutcome::Timeout(ms) => {
            tracing::error!("AI agent timeout after {}s", ms / 1000);
            Err(Error::ExecutionErr(format!(
                "AI agent timeout after (>{}s)",
                ms / 1000
            )))
        }
        GracefulPollOutcome::Cancelled { canceled_by: cb } => {
            let (by, reason) = format_cancel_info(&cb);
            Err(Error::ExecutionErr(format!(
                "Job cancelled by {by} (reason: {reason})"
            )))
        }
        GracefulPollOutcome::CancelledTimeout { canceled_by: cb } => {
            let (by, reason) = format_cancel_info(&cb);
            // Abort any still-running spawned tool tasks
            // unwrap safe: lock is only held briefly for push/drain, no panic possible inside
            for handle in tool_abort_handles.lock().unwrap().drain(..) {
                handle.abort();
            }
            // Hard timeout: clean up orphaned jobs still stuck in v2_job_queue
            cleanup_orphaned_tool_jobs(db, &job.id, &job.workspace_id, cb).await;
            Err(Error::ExecutionErr(format!(
                "Job cancelled by {by} (reason: {reason}, timed out waiting for tool calls)"
            )))
        }
        GracefulPollOutcome::AlreadyCompleted => {
            Err(Error::AlreadyCompleted("Job already completed".to_string()))
        }
    }
}

/// OpenAI rejects a `prompt_cache_key` over 64 characters
/// (`Invalid 'prompt_cache_key': string too long`), and a runnable path alone can pass
/// that. Fold an over-long key into a digest of itself: same step still yields the same
/// key across runs, which is the whole property that routes them to one cache.
fn bounded_prompt_cache_key(raw: &str) -> String {
    const MAX_LEN: usize = 64;
    if raw.len() <= MAX_LEN {
        return raw.to_string();
    }
    let suffix = hex::encode(&sha2::Sha256::digest(raw.as_bytes())[..16]);
    // Keep a readable head so a key stays traceable to its workspace in provider logs.
    let mut head = MAX_LEN - suffix.len() - 1;
    while head > 0 && !raw.is_char_boundary(head) {
        head -= 1;
    }
    format!("{}:{}", &raw[..head], suffix)
}

#[async_recursion]
pub async fn run_agent(
    // connection
    db: &DB,
    conn: &Connection,

    // agent job and flow data
    job: &MiniPulledJob,
    parent_job: Option<&Uuid>,
    flow_step_id_override: Option<&str>,
    args: &AIAgentArgs,
    tools: &[Tool],
    mcp_clients: &HashMap<String, Arc<McpClient>>,
    summary: Option<&str>,

    // job execution context
    client: &AuthedClient,
    occupancy_metrics: &mut OccupancyMetrics,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    hostname: &str,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
    has_stream: &mut bool,
    has_websearch: bool,
    omit_output_from_conversation: bool,

    // cancellation signal from parent
    cancel_rx: tokio::sync::watch::Receiver<bool>,

    // abort handles for spawned tool tasks
    tool_abort_handles: ToolAbortHandles,
) -> error::Result<Box<RawValue>> {
    let output_type = args.output_type.as_ref().unwrap_or(&OutputType::Text);
    let credentials = args.provider.to_provider_credentials(db).await?;
    let base_url = &credentials.base_url;
    let api_key = credentials.api_key.as_deref().unwrap_or("");

    // Create the query builder for the provider
    let mut query_builder = create_query_builder(&credentials, args.provider.get_model());
    if query_builder.supports_chat_completions_fallback(base_url)
        && is_chat_completions_only(base_url, args.provider.get_model())
    {
        query_builder = create_chat_completions_query_builder(&credentials);
    }
    // These outlive the iteration that discovers them: a request shape or a route the
    // endpoint rejected once stays rejected for the whole step.
    let mut include_usage = true;
    let mut include_prompt_cache_key = true;

    // Initialize messages
    let mut messages =
        if let Some(system_prompt) = args.system_prompt.clone().filter(|s| !s.is_empty()) {
            vec![OpenAIMessage {
                role: "system".to_string(),
                content: Some(OpenAIContent::Text(system_prompt)),
                ..Default::default()
            }]
        } else {
            vec![]
        };

    // Effective flow_step_id: override for nested agents, otherwise from job
    let effective_flow_step_id: Option<&str> =
        flow_step_id_override.or(job.flow_step_id.as_deref());

    // Keyed on the step, not the run: every run of this step opens with the same system
    // prompt and tool definitions, and each agent-loop iteration extends the previous
    // one's prefix. Above ~15 requests/minute one key starts missing again, which is a
    // reason to split it further, never to make it per-run.
    let prompt_cache_key = bounded_prompt_cache_key(&format!(
        "{}:{}:{}",
        job.workspace_id,
        job.runnable_path(),
        effective_flow_step_id.unwrap_or_default()
    ));

    // Fetch flow context for input transforms context, chat and memory
    let mut flow_context = get_flow_context(db, job).await;

    // The run's memory id is also the chat conversation id, which a step's own memory id never
    // replaces.
    let conversation_id = flow_context
        .flow_status
        .as_ref()
        .and_then(|fs| fs.memory_id);
    let (history, history_notes) = resolve_history_source(
        args,
        conversation_id,
        &job.workspace_id,
        flow_context.flow_path.as_deref().unwrap_or_default(),
    );

    // Check if user_message is provided and non-empty
    let has_user_message = args
        .user_message
        .as_ref()
        .map(|m| !m.is_empty())
        .unwrap_or(false);

    let is_text_output = output_type == &OutputType::Text;

    if is_text_output {
        for note in &history_notes {
            append_logs(&job.id, &job.workspace_id, format!("{note}\n"), conn).await;
        }
    } else if !matches!(args.memory, None | Some(Memory::Off))
        || args.memory_id.is_some()
        || args.previous_messages.is_some()
    {
        append_logs(
            &job.id,
            &job.workspace_id,
            "Image output sends no history, so memory and previous messages are not read.\n",
            conn,
        )
        .await;
    }

    // A `manual` memory sent whatever list it held, an empty one included, so a step that still has
    // one keeps running without a user message.
    let legacy_list = matches!(args.memory, Some(Memory::Manual { .. }));
    if !has_prompt(&history, has_user_message, is_text_output, legacy_list) {
        let missing = if !is_text_output {
            "'user_message' must be provided for image output"
        } else if matches!(
            args.memory,
            Some(Memory::Window { .. } | Memory::Compaction { .. } | Memory::Auto { .. })
        ) {
            "'user_message' must be provided while managed memory is on"
        } else {
            "Either 'previous_messages' or 'user_message' must be provided"
        };
        return Err(Error::internal_err(missing.to_string()));
    }

    if matches!(output_type, OutputType::Text) {
        match &history {
            HistorySource::Messages(provided) => messages.extend(provided.iter().cloned()),
            HistorySource::Managed { memory_id, bound } => {
                if let Some(step_id) = effective_flow_step_id {
                    match read_from_memory(db, &job.workspace_id, *memory_id, step_id).await {
                        Ok(Some(loaded_messages)) => {
                            let messages_to_load = prepare_auto_memory_messages_for_request(
                                &loaded_messages,
                                bound.messages_to_keep(),
                            );
                            messages.extend(messages_to_load);
                        }
                        Ok(None) => {}
                        Err(e) => {
                            tracing::error!("Failed to read memory for step {}: {}", step_id, e);
                        }
                    }
                }
            }
            HistorySource::Stateless => {}
        }
    }

    // Extract previous step result only if any tool needs it
    let previous_result = {
        if any_tool_needs_previous_result(&tools) {
            if let Some(ref flow_status) = flow_context.flow_status {
                get_previous_job_result(db, &job.workspace_id, flow_status)
                    .await
                    .ok()
                    .flatten()
            } else {
                None
            }
        } else {
            None
        }
    };

    // Build IdContext for results.stepId syntax
    let id_context = {
        if let Some(ref flow_status) = flow_context.flow_status {
            // Get the step ID from the AI agent's flow step
            let previous_id = effective_flow_step_id
                .map(str::to_string)
                .unwrap_or_else(|| "unknown".to_string());

            Some(get_transform_context(job, &previous_id, flow_status))
        } else {
            None
        }
    };

    // Add user message and attachments as a single user message
    // (Bedrock requires a text block alongside document blocks in the same message)
    {
        let has_message = args
            .user_message
            .as_ref()
            .map(|m| !m.is_empty())
            .unwrap_or(false);
        let has_attachments = args
            .user_attachments
            .as_ref()
            .map(|a| !a.is_empty())
            .unwrap_or(false);

        if has_message && has_attachments {
            let mut parts = vec![ContentPart::Text { text: args.user_message.clone().unwrap() }];
            for attachment in args.user_attachments.as_ref().unwrap() {
                if !attachment.s3.is_empty() {
                    parts.push(ContentPart::S3Object { s3_object: attachment.clone() });
                }
            }
            messages.push(OpenAIMessage {
                role: "user".to_string(),
                content: Some(OpenAIContent::Parts(parts)),
                ..Default::default()
            });
        } else if has_message {
            messages.push(OpenAIMessage {
                role: "user".to_string(),
                content: Some(OpenAIContent::Text(args.user_message.clone().unwrap())),
                ..Default::default()
            });
        } else if has_attachments {
            let mut parts = vec![];
            for attachment in args.user_attachments.as_ref().unwrap() {
                if !attachment.s3.is_empty() {
                    parts.push(ContentPart::S3Object { s3_object: attachment.clone() });
                }
            }
            messages.push(OpenAIMessage {
                role: "user".to_string(),
                content: Some(OpenAIContent::Parts(parts)),
                ..Default::default()
            });
        }
    }

    let mut actions = vec![];
    let mut content = None;
    let mut final_usage: Option<TokenUsage> = None;

    // Check if this provider supports tools with the current output type
    let supports_tools = query_builder.supports_tools_with_output_type(output_type);

    let mut tool_defs: Option<Vec<ToolDef>> = if tools.is_empty() || !supports_tools {
        None
    } else {
        Some(tools.iter().map(|t| t.def.clone()).collect())
    };

    // Handle structured output schema
    let has_output_properties = args
        .output_schema
        .as_ref()
        .and_then(|schema| schema.properties.as_ref())
        .map(|props| !props.is_empty())
        .unwrap_or(false);

    let should_use_structured_output_tool =
        should_use_structured_output_tool(&args.provider.kind, &args.provider.model);
    let mut used_structured_output_tool = false;
    let mut structured_output_tool_name: Option<String> = None;

    // For text output with schema, handle structured output
    if has_output_properties && is_text_output {
        let schema = args.output_schema.as_ref().unwrap();
        if should_use_structured_output_tool {
            // Anthropic uses a tool for structured output
            let unique_tool_name = find_unique_tool_name("structured_output", tool_defs.as_deref());
            structured_output_tool_name = Some(unique_tool_name.clone());

            let output_tool = ToolDef {
                r#type: "function".to_string(),
                function: ToolDefFunction {
                    name: unique_tool_name,
                    description: Some(
                        "This tool MUST be used last to return a structured JSON object as the final output."
                            .to_string(),
                    ),
                    parameters: to_raw_value(&schema),
                },
            };
            if let Some(ref mut existing_tools) = tool_defs {
                existing_tools.push(output_tool);
            } else {
                tool_defs = Some(vec![output_tool]);
            }
        }
        // For non-Anthropic providers, response_format is handled by the query builder
    }

    let user_wants_streaming = streaming_requested(args.streaming);
    *has_stream = user_wants_streaming && is_text_output;

    let mut final_events_str = String::new();
    let mut final_reasoning = String::new();

    // Always create a StreamEventProcessor for text output (use silent mode if user doesn't want streaming)
    let stream_event_processor = if is_text_output {
        if user_wants_streaming {
            Some(StreamEventProcessor::new(conn, job))
        } else {
            Some(StreamEventProcessor::new_silent())
        }
    } else {
        None
    };

    let chat_enabled = flow_context
        .flow_status
        .as_ref()
        .and_then(|fs| fs.chat_input_enabled)
        .unwrap_or(false);
    let persist_output_to_conversation = chat_enabled && !omit_output_from_conversation;

    let step_name = get_step_name_from_flow(summary.as_deref(), effective_flow_step_id);

    let max_iterations = args
        .max_iterations
        .map(|m| m.clamp(1, HARD_MAX_AGENT_ITERATIONS))
        .unwrap_or(DEFAULT_MAX_AGENT_ITERATIONS);

    let mut compactor = match &args.memory {
        Some(Memory::Compaction { context_window }) if is_text_output => {
            let tool_schema_tokens = tool_defs
                .as_ref()
                .and_then(|defs| serde_json::to_string(defs).ok())
                .map(|schemas| schemas.len() / 4)
                .unwrap_or(0);
            // An unset window — the usual case — is looked up from the model. The field
            // is the override for what the lookup cannot serve: a Custom AI deployment,
            // or an id the table does not list.
            let context_window = match context_window {
                0 => model_context_window(args.provider.get_model()),
                declared => *declared,
            };
            Some(Compactor::new(context_window, tool_schema_tokens))
        }
        _ => None,
    };
    // The window a persisted conversation has to fit in, when the storage is smaller
    // than the model. The database cuts what it cannot hold from the oldest message,
    // summary first, so a step that persists there summarizes down to it before writing.
    let persist_capacity = match (&compactor, &history, effective_flow_step_id) {
        (Some(_), HistorySource::Managed { .. }, Some(_)) => memory_storage_capacity_bytes().await,
        _ => None,
    };
    // The summarization call runs under the agent's own request timeout, which resolves
    // from the job alone and so is the same for every iteration.
    let compaction_timeout = match compactor {
        Some(_) => Some(
            resolve_job_timeout(conn, &job.workspace_id, job.id, job.timeout)
                .await
                .0,
        ),
        None => None,
    };
    // A memory loaded from an earlier run can already be over the window — the step was
    // switched to a smaller model, or a run under a wider one persisted more than this
    // window holds. Compaction is otherwise reactive, taken after a request the endpoint
    // accepted, because the fallbacks the loop learns from a rejection (`stream_options`,
    // the chat/completions reroute) are not known before the first request. But an
    // oversized load would make that first request overflow and fail the run, and every
    // retry reload the same history, so one pass is taken up front off the character
    // estimate. It uses the default request shape: an endpoint that needs a fallback may
    // reject this one summary, which is non-fatal — the loop then proceeds as it would
    // have. Mainstream providers need no fallback, so it lands.
    let mut last_request = LastRequest { prompt_tokens: None, message_count: 0 };
    if compactor.is_some() {
        compact_if_needed(
            CompactionContext {
                compactor: compactor.as_mut(),
                timeout: compaction_timeout,
                credentials: &credentials,
                args,
                client,
                workspace_id: &job.workspace_id,
            },
            query_builder.as_ref(),
            include_usage,
            &mut messages,
            &mut last_request,
            &mut final_usage,
        )
        .await;
    }

    // Main agent loop
    for i in 0..max_iterations {
        // Check if parent was canceled — stop iterating but let current tool calls finish
        if *cancel_rx.borrow() {
            return Err(Error::ExecutionErr("Job cancelled".to_string()));
        }

        if used_structured_output_tool {
            break;
        }

        // How many messages this request carries, so the provider's prompt count can
        // later be told apart from what the response and its tool results add.
        let request_message_count = messages.len();

        // Handle AWS Bedrock provider specially using the official SDK
        let parsed = if credentials.provider == AIProvider::AWSBedrock {
            #[cfg(feature = "bedrock")]
            {
                let region = credentials
                    .region
                    .as_deref()
                    .unwrap_or(windmill_ai::ai_providers::USE_ENV_REGION);
                // Use Bedrock SDK via dedicated query builder
                windmill_ai::providers::bedrock::BedrockQueryBuilder::default()
                    .execute_request(
                        &messages,
                        tool_defs.as_deref(),
                        args.provider.get_model(),
                        args.temperature,
                        args.provider.get_reasoning_effort(),
                        args.max_completion_tokens,
                        api_key,
                        region,
                        stream_event_processor.as_ref().map(|p| p.boxed_sink()),
                        client,
                        &job.workspace_id,
                        structured_output_tool_name.as_deref(),
                        credentials.aws_access_key_id.as_deref(),
                        credentials.aws_secret_access_key.as_deref(),
                        credentials.aws_session_token.as_deref(),
                    )
                    .await?
            }
            #[cfg(not(feature = "bedrock"))]
            {
                return Err(Error::internal_err(
                    "AWS Bedrock support is not enabled. Build with 'bedrock' feature.".to_string(),
                ));
            }
        } else {
            // For all other providers, use the HTTP client approach
            let mut build_args = BuildRequestArgs {
                messages: &messages,
                tools: tool_defs.as_deref(),
                model: args.provider.get_model(),
                temperature: args.temperature,
                reasoning_effort: args.provider.get_reasoning_effort(),
                max_tokens: args.max_completion_tokens,
                output_schema: args.output_schema.as_ref(),
                output_type,
                system_prompt: args.system_prompt.as_deref(),
                user_message: args.user_message.as_deref().unwrap_or(""),
                attachments: args.user_attachments.as_deref(),
                has_websearch,
                prompt_cache_key: include_prompt_cache_key.then_some(prompt_cache_key.as_str()),
                reasoning_summary: !is_reasoning_summary_unavailable(
                    &credentials,
                    args.provider.get_model(),
                ),
            };

            // A worker cannot run the client credentials exchange, so an OAuth resource
            // has no token here: the request would carry an empty credential and come
            // back 401.
            if needs_unavailable_oauth_exchange(
                &credentials,
                args.provider.resource.token_url.as_deref(),
                &query_builder.get_auth_headers(api_key, base_url, output_type),
            ) {
                return Err(Error::ExecutionErr(format!(
                    "The {:?} resource authenticates with OAuth, which AI agent steps do not \
                     support. Set an API key on the resource, or carry the provider's credential \
                     header in its `headers`.",
                    credentials.provider
                )));
            }

            let timeout = resolve_job_timeout(conn, &job.workspace_id, job.id, job.timeout)
                .await
                .0;

            let trailing_headers = common_outbound_headers(&credentials).collect::<Vec<_>>();

            // `endpoint` derives from the user-controlled provider base_url, so pin
            // DNS to the SSRF-validated address: the connect must not rebind to an
            // internal IP between the check and the request (TOCTOU).
            let pinned_ai_client = pinned_ai_client_for(base_url).await?;

            // Helper to build HTTP request with headers
            let build_http_request =
                |endpoint: &str, auth_headers: &[(&'static str, String)], body: String| {
                    let mut req = pinned_ai_client
                        .post(endpoint)
                        .timeout(timeout)
                        .header("Content-Type", "application/json");

                    for (header_name, header_value) in auth_headers {
                        req = req.header(*header_name, header_value.clone());
                    }

                    for (header_name, header_value) in &trailing_headers {
                        req = req.header(header_name.as_str(), header_value.as_str());
                    }

                    req.body(body)
                };

            // An endpoint can reject the request shape rather than the model:
            // `stream_options` and `prompt_cache_key`, which not every OpenAI-compatible
            // gateway accepts, a reasoning summary, which OpenAI refuses to unverified
            // organizations, and the route itself, when an Azure resource is outside
            // the Responses API's model/region matrix. Each is retried once with that
            // part dropped.
            // Set where the route is found to be absent, and read once the fallback has
            // answered: a rejection it did not resolve says nothing about the deployment.
            let mut rerouted_by_a_route_rejection = false;
            let resp = loop {
                let request_body = if include_usage {
                    query_builder
                        .build_request(&build_args, client, &job.workspace_id)
                        .await?
                } else {
                    query_builder
                        .build_request_without_usage(&build_args, client, &job.workspace_id)
                        .await?
                };
                let endpoint =
                    query_builder.get_endpoint(base_url, args.provider.get_model(), output_type);
                let auth_headers = retain_effective_credentials(
                    &credentials,
                    query_builder.get_auth_headers(api_key, base_url, output_type),
                );

                let resp = build_http_request(&endpoint, &auth_headers, request_body)
                    .send()
                    .await
                    .map_err(|e| Error::internal_err(format!("Failed to call API: {}", e)))?;

                match resp.error_for_status_ref() {
                    Ok(_) => {
                        if rerouted_by_a_route_rejection {
                            remember_chat_completions_only(base_url, args.provider.get_model());
                        }
                        break resp;
                    }
                    Err(e) => {
                        let status = resp.status();
                        let text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "<failed to read body>".to_string());

                        // Common error patterns: 400 Bad Request with mentions of stream_options or include_usage
                        let rejects_usage_tracking = include_usage
                            && query_builder.supports_retry_without_usage()
                            && status.as_u16() == 400
                            && (text.contains("stream_options")
                                || text.contains("include_usage")
                                || text.contains("Additional properties are not allowed"));

                        // An OpenAI-compatible gateway that validates the body strictly
                        // names the offending field, whether it calls it an unrecognized
                        // argument or an unexpected additional property.
                        let rejects_prompt_cache_key = build_args.prompt_cache_key.is_some()
                            && status.as_u16() == 400
                            && text.contains("prompt_cache_key");

                        let summary_refused = build_args.reasoning_summary
                            && rejects_reasoning_summary(status.as_u16(), &text);

                        // Only the first call of the step may re-route: an endpoint that
                        // does not serve this API rejects that one already, whereas a
                        // rejection once the conversation is under way is about the
                        // conversation (context length, content filter, tool schema).
                        let route_unserved = i == 0
                            && query_builder.supports_chat_completions_fallback(base_url)
                            && matches!(status.as_u16(), 400 | 404)
                            && *output_type == OutputType::Text;

                        if rejects_usage_tracking {
                            tracing::info!(
                                "Retrying request without stream_options due to provider incompatibility"
                            );
                            include_usage = false;
                        } else if rejects_prompt_cache_key {
                            // Checked before the route fallback: the endpoint serves this
                            // route, it just refuses one optional field, and re-routing
                            // the whole step over that would give up far more.
                            tracing::info!(
                                "Retrying request without prompt_cache_key due to provider incompatibility"
                            );
                            include_prompt_cache_key = false;
                            build_args.prompt_cache_key = None;
                        } else if summary_refused {
                            tracing::info!(
                                "Retrying request without the reasoning summary the endpoint refused"
                            );
                            remember_reasoning_summary_unavailable(
                                &credentials,
                                args.provider.get_model(),
                            );
                            build_args.reasoning_summary = false;
                        } else if route_unserved {
                            tracing::info!(
                                "Endpoint rejected the request ({}), falling back to chat/completions",
                                status
                            );
                            // Only a 404 says the route is absent. A 400 is ambiguous —
                            // a deployment that does serve the route rejects tool
                            // schemas, blocked hosted tools and filtered content the
                            // same way — so it re-routes this step and nothing more.
                            rerouted_by_a_route_rejection = status.as_u16() == 404;
                            query_builder = create_chat_completions_query_builder(&credentials);
                            include_usage = true;
                        } else {
                            return Err(Error::internal_err(format!(
                                "API error calling {}: {} - {}",
                                endpoint, e, text
                            )));
                        }
                    }
                }
            };

            if let Some(ref stream_event_processor) = stream_event_processor {
                query_builder
                    .parse_streaming_response(resp, stream_event_processor.boxed_sink())
                    .await?
            } else {
                query_builder.parse_image_response(resp).await?
            }
        };

        match parsed {
            ParsedResponse::Text {
                content: response_content,
                reasoning: response_reasoning,
                tool_calls,
                events_str,
                annotations,
                used_websearch,
                usage,
            } => {
                // What this one request's prompt occupied. `final_usage` sums every
                // iteration, so it says nothing about how full the context is.
                last_request = LastRequest {
                    prompt_tokens: usage.as_ref().and_then(|u| u.input_tokens),
                    message_count: request_message_count,
                };
                if let Some(compactor) = compactor.as_mut() {
                    compactor.record_response();
                }

                // Accumulate usage from this iteration
                if let Some(u) = usage {
                    match &mut final_usage {
                        Some(existing) => existing.accumulate(&u),
                        None => final_usage = Some(u),
                    }
                }
                if let Some(events_str) = events_str {
                    final_events_str.push_str(&events_str);
                }
                append_reasoning(&mut final_reasoning, response_reasoning.as_deref());

                // Add websearch tool message if websearch was used
                if used_websearch {
                    actions.push(AgentAction::WebSearch {});
                    if let Some(parent_job) = parent_job {
                        update_flow_status_module_with_actions(db, parent_job, &actions).await?;
                        update_flow_status_module_with_actions_success(db, parent_job, true)
                            .await?;
                    }
                    messages.push(OpenAIMessage {
                        role: "tool".to_string(),
                        content: Some(OpenAIContent::Text(
                            "Used websearch tool successfully".to_string(),
                        )),
                        agent_action: Some(AgentAction::WebSearch {}),
                        ..Default::default()
                    });
                    if persist_output_to_conversation {
                        if let Some(conversation_id) = conversation_id {
                            // The search ran inside the provider's call, so this job's args
                            // describe the agent, not the search: its sources reach the row
                            // only if they are written here.
                            let extras = (!annotations.is_empty()).then(|| MessageExtras {
                                tool_result: serde_json::to_string(&annotations).ok(),
                                ..Default::default()
                            });
                            // Awaited like every row of the loop, so rows commit in turn order.
                            // Worded like every other tool row, so a reader recovers the tool
                            // name from the sentence.
                            if let Err(e) = add_message_to_conversation(
                                db,
                                &conversation_id,
                                Some(job.id),
                                "Used websearch tool",
                                MessageType::Tool,
                                &step_name,
                                true,
                                extras.as_ref(),
                            )
                            .await
                            {
                                tracing::warn!(
                                    "Failed to add websearch tool message to conversation {}: {}",
                                    conversation_id,
                                    e
                                );
                            }
                        }
                    }
                }

                if let Some(ref response_content) = response_content {
                    actions.push(AgentAction::Message {});
                    messages.push(OpenAIMessage {
                        role: "assistant".to_string(),
                        content: Some(OpenAIContent::Text(response_content.clone())),
                        agent_action: Some(AgentAction::Message {}),
                        annotations: if annotations.is_empty() {
                            None
                        } else {
                            Some(annotations.clone())
                        },
                        ..Default::default()
                    });

                    if let Some(parent_job) = parent_job {
                        update_flow_status_module_with_actions(db, parent_job, &actions).await?;
                        update_flow_status_module_with_actions_success(db, parent_job, true)
                            .await?;
                    }

                    content = Some(OpenAIContent::Text(response_content.clone()));

                    // Add assistant message to conversation if chat_input_enabled
                    if persist_output_to_conversation && !response_content.is_empty() {
                        if let Some(conversation_id) = conversation_id {
                            // This iteration's thinking goes on the answer's row; the job
                            // result only keeps the turn's thinking as one string.
                            let extras = response_reasoning.clone().map(|reasoning| {
                                MessageExtras { reasoning: Some(reasoning), ..Default::default() }
                            });
                            if let Err(e) = add_message_to_conversation(
                                db,
                                &conversation_id,
                                Some(job.id),
                                response_content,
                                MessageType::Assistant,
                                &step_name,
                                true,
                                extras.as_ref(),
                            )
                            .await
                            {
                                tracing::warn!(
                                    "Failed to add assistant message to conversation {}: {}",
                                    conversation_id,
                                    e
                                );
                            }
                        }
                    }
                }

                if tool_calls.is_empty() {
                    break;
                } else if i == max_iterations - 1 {
                    #[derive(serde::Serialize)]
                    struct MaxIterError<'a> {
                        message: String,
                        name: &'static str,
                        #[serde(skip_serializing_if = "Option::is_none")]
                        step_id: Option<&'a str>,
                        result: MaxIterPartialResult<'a>,
                    }
                    return Err(Error::ExecutionRawError(
                        serde_json::value::to_raw_value(&MaxIterError {
                            message: format!(
                                "AI agent reached max iterations ({}), you can either increase max_iterations or enable the \"continue on error\" option from the advanced options of the step.",
                                max_iterations
                            ),
                            name: "ExecutionErr",
                            step_id: effective_flow_step_id,
                            result: MaxIterPartialResult {
                                messages: messages
                                    .iter()
                                    .map(|m| Message {
                                        message: m,
                                        agent_action: m.agent_action.as_ref(),
                                    })
                                    .collect(),
                            },
                        })?,
                    ));
                }

                messages.push(OpenAIMessage {
                    role: "assistant".to_string(),
                    tool_calls: Some(tool_calls.clone()),
                    ..Default::default()
                });

                // A round's thinking is stored on one row, the first the round writes, which is
                // where the stream shows it: its text row when it wrote text, else the row of
                // its first call — the answer row below when that call is the structured-output
                // tool. Two rows carrying it would show it twice after a reload.
                let call_reasoning = response_reasoning
                    .clone()
                    .filter(|_| response_content.as_deref().unwrap_or("").is_empty());
                let structured_output_first = structured_output_tool_name
                    .as_ref()
                    .zip(tool_calls.first())
                    .map_or(false, |(name, tc)| tc.function.name == *name);

                // Handle tool calls using extracted tools module
                let tool_execution_ctx = ToolExecutionContext {
                    db,
                    conn,
                    job,
                    parent_job,
                    summary: &summary,
                    flow_step_id_override,
                    client,
                    worker_dir,
                    base_internal_url,
                    worker_name,
                    hostname,
                    occupancy_metrics,
                    killpill_rx,
                    stream_event_processor: stream_event_processor.as_ref(),
                    flow_context: &mut flow_context,
                    omit_output_from_conversation,
                    reasoning: if structured_output_first {
                        None
                    } else {
                        call_reasoning.clone()
                    },
                    previous_result: &previous_result,
                    id_context: &id_context,
                    tool_abort_handles: tool_abort_handles.clone(),
                };

                let (tool_messages, tool_content, tool_used_structured_output) =
                    execute_tool_calls(
                        tool_execution_ctx,
                        &tool_calls,
                        &tools,
                        mcp_clients,
                        &mut actions,
                        &mut final_events_str,
                        &structured_output_tool_name,
                    )
                    .await?;

                messages.extend(tool_messages);

                // A structured answer is the arguments of the structured-output tool call,
                // on which the loop ends without a text iteration, so its row is written here.
                if tool_used_structured_output && persist_output_to_conversation {
                    if let (Some(conversation_id), Some(OpenAIContent::Text(answer))) =
                        (conversation_id, tool_content.as_ref())
                    {
                        let extras = call_reasoning
                            .clone()
                            .filter(|_| structured_output_first)
                            .map(|reasoning| MessageExtras {
                                reasoning: Some(reasoning),
                                ..Default::default()
                            });
                        if let Err(e) = add_message_to_conversation(
                            db,
                            &conversation_id,
                            Some(job.id),
                            answer,
                            MessageType::Assistant,
                            &step_name,
                            true,
                            extras.as_ref(),
                        )
                        .await
                        {
                            tracing::warn!(
                                "Failed to add structured answer to conversation {}: {}",
                                conversation_id,
                                e
                            );
                        }
                    }
                }

                if let Some(tc) = tool_content {
                    content = Some(tc);
                }
                used_structured_output_tool = tool_used_structured_output;

                // Check cancellation after tool calls complete to avoid a wasted LLM call
                if *cancel_rx.borrow() {
                    return Err(Error::ExecutionErr("Job cancelled".to_string()));
                }

                // Compact between iterations rather than mid-request: the next iteration
                // is what would carry the grown conversation to the provider.
                compact_if_needed(
                    CompactionContext {
                        compactor: compactor.as_mut(),
                        timeout: compaction_timeout,
                        credentials: &credentials,
                        args,
                        client,
                        workspace_id: &job.workspace_id,
                    },
                    query_builder.as_ref(),
                    include_usage,
                    &mut messages,
                    &mut last_request,
                    &mut final_usage,
                )
                .await;
            }
            ParsedResponse::Image { base64_data } => {
                // For image output, upload to S3 and track in conversation
                let s3_object =
                    upload_image_to_s3(&base64_data, &job.workspace_id, &job.id, client).await?;

                let content = to_raw_value(&s3_object);

                // Add assistant message to conversation if chat_input_enabled
                if persist_output_to_conversation {
                    if let Some(conversation_id) = conversation_id {
                        // Create extended version with type discriminator for conversation storage
                        // This avoids conflicts with outputs that are of the same format as S3 objects
                        let s3_with_type = S3ObjectWithType {
                            s3_object: s3_object.clone(),
                            r#type: "windmill_s3_object".to_string(),
                        };

                        let message_content = serde_json::to_string(&s3_with_type)
                            .unwrap_or_else(|_| content.get().to_string());

                        if let Err(e) = add_message_to_conversation(
                            db,
                            &conversation_id,
                            Some(job.id),
                            &message_content,
                            MessageType::Assistant,
                            &step_name,
                            true,
                            None,
                        )
                        .await
                        {
                            tracing::warn!(
                                "Failed to add assistant message to conversation {}: {}",
                                conversation_id,
                                e
                            );
                        }
                    }
                }

                // Return early since image generation is complete
                return Ok(content);
            }
        }
    }

    // A turn the model answers without calling a tool leaves the loop on its first
    // iteration, so this is the only compaction a chat-shaped step ever gets: without it
    // the conversation is persisted whole and every later turn reloads it, until the
    // provider refuses the request outright.
    //
    // Two passes, in order. First the model window, off the provider's count for the last
    // request: a chat-shaped turn never reached the in-loop check, so this is where an
    // attachment that fills the model context is caught — it is a few bytes in the row,
    // invisible to the storage measure below. Then, when the row is smaller than the
    // model, a second pass bounds what is written to the database, measured in bytes.
    compact_if_needed(
        CompactionContext {
            compactor: compactor.as_mut(),
            timeout: compaction_timeout,
            credentials: &credentials,
            args,
            client,
            workspace_id: &job.workspace_id,
        },
        query_builder.as_ref(),
        include_usage,
        &mut messages,
        &mut last_request,
        &mut final_usage,
    )
    .await;
    let storage_bound = match (compactor.as_mut(), persist_capacity) {
        (Some(compactor), Some(bytes)) => compactor.bound_by_storage(bytes),
        _ => false,
    };
    let compacted = storage_bound
        && compact_if_needed(
            CompactionContext {
                compactor: compactor.as_mut(),
                timeout: compaction_timeout,
                credentials: &credentials,
                args,
                client,
                workspace_id: &job.workspace_id,
            },
            query_builder.as_ref(),
            include_usage,
            &mut messages,
            &mut last_request,
            &mut final_usage,
        )
        .await;
    if compacted {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!(
                "No instance object storage, so the memory was summarized to fit the {}KB \
                 the database holds rather than the model's context window.\n",
                MAX_MEMORY_SIZE_BYTES / 1000
            ),
            conn,
        )
        .await;
    }

    // Return the final result
    let final_messages: Vec<Message> = messages
        .iter()
        .map(|m| Message { message: m, agent_action: m.agent_action.as_ref() })
        .collect();

    // Parse content as JSON for structured output, fallback to string if it fails
    let output_value = match content {
        Some(content_str) => match has_output_properties {
            true => match content_str {
                OpenAIContent::Text(text) => {
                    serde_json::from_str::<Box<RawValue>>(&text).map_err(|_e| {
                        Error::internal_err(format!("Failed to parse structured output: {}", text))
                    })
                }
                OpenAIContent::Parts(_parts) => Err(Error::internal_err(
                    "Failed to parse structured output".to_string(),
                )),
            },
            false => Ok(match content_str {
                OpenAIContent::Text(text) => to_raw_value(&text),
                OpenAIContent::Parts(parts) => to_raw_value(&parts),
            }),
        }?,
        None => to_raw_value(&""),
    };

    // Wait for stream event processor to finish persisting events (if any)
    if let Some(handle) = {
        if let Some(stream_event_processor) = stream_event_processor {
            stream_event_processor.to_handle()
        } else {
            None
        }
    } {
        if let Err(e) = handle.await {
            return Err(Error::internal_err(format!(
                "Error waiting for stream event processor: {}",
                e
            )));
        }
    }

    // final_messages holds the complete history: what was loaded plus this run's messages
    if matches!(output_type, OutputType::Text) {
        if let HistorySource::Managed { memory_id, bound } = &history {
            if let Some(step_id) = effective_flow_step_id {
                let all_messages: Vec<OpenAIMessage> =
                    final_messages.iter().map(|m| m.message.clone()).collect();

                if !all_messages.is_empty() {
                    let messages_to_persist = prepare_auto_memory_messages_for_persistence(
                        &all_messages,
                        bound.messages_to_keep(),
                    );

                    match write_to_memory(
                        db,
                        &job.workspace_id,
                        *memory_id,
                        step_id,
                        &messages_to_persist,
                    )
                    .await
                    {
                        // The conversation outgrew what the database holds and was cut
                        // from its oldest message. Only the worker's own log says so
                        // otherwise, so from the flow's side the agent simply starts the
                        // next run having forgotten how this one began.
                        Ok(dropped) if dropped > 0 => {
                            append_logs(
                                &job.id,
                                &job.workspace_id,
                                format!(
                                    "Memory does not fit the {}KB the database holds, so its \
                                     {dropped} oldest messages were dropped. Configure instance \
                                     object storage to keep the whole conversation.\n",
                                    MAX_MEMORY_SIZE_BYTES / 1000
                                ),
                                conn,
                            )
                            .await;
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!(
                                "Failed to persist {} messages to memory for step {}: {}",
                                messages_to_persist.len(),
                                step_id,
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(to_raw_value(&AIAgentResult {
        output: output_value,
        messages: final_messages,
        wm_stream: if !final_events_str.is_empty() {
            Some(final_events_str)
        } else {
            None
        },
        reasoning: (!final_reasoning.is_empty()).then_some(final_reasoning),
        usage: if final_usage.as_ref().map(|u| u.is_empty()).unwrap_or(true) {
            None
        } else {
            final_usage
        },
    }))
}

/// Whether the step asked for its answer as it is generated. Absence means on, matching the
/// schema's own default: a step that never wrote the key never had an opinion, and an answer
/// arriving as it is written is what people expect. Only an explicit `false` holds it back.
///
/// The chat surfaces decide whether to open a stream from their own reading of the same config,
/// and a surface that opens one for an answer sent in a single piece re-runs the flow when the
/// connection times out. So this default is half of a contract, not a local preference.
fn streaming_requested(streaming: Option<bool>) -> bool {
    streaming.unwrap_or(true)
}

/// Add one iteration's thinking to the step's. Every iteration thinks, and a tool-call
/// iteration's thinking is what led to the call, so the result keeps all of them in order,
/// blank-line separated, rather than only the answering turn's.
fn append_reasoning(accumulated: &mut String, reasoning: Option<&str>) {
    let Some(reasoning) = reasoning.map(str::trim).filter(|r| !r.is_empty()) else {
        return;
    };
    if !accumulated.is_empty() {
        accumulated.push_str("\n\n");
    }
    accumulated.push_str(reasoning);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_message(role: &str, content: &str) -> OpenAIMessage {
        OpenAIMessage {
            role: role.to_string(),
            content: Some(OpenAIContent::Text(content.to_string())),
            ..Default::default()
        }
    }

    #[derive(Debug, PartialEq)]
    enum Resolved {
        Messages(usize),
        Window(Uuid, usize),
        Compaction(Uuid, usize),
        Stateless { noted: bool },
    }

    /// Every memory shape a worker may still read, resolved against a run with or without a
    /// memory id. The hashed id is pinned: changing it detaches memories stored under string ids.
    #[test]
    fn history_source_resolves_every_memory_shape() {
        use serde_json::json;
        let run = Uuid::from_u128(1);
        let baked = Uuid::from_u128(2);
        let cust_1 = Uuid::parse_str("0168fcea-ffa7-5c15-bdb0-7709bb5f540d").unwrap();
        let window = json!({ "kind": "window", "context_length": 10 });
        let message = json!([{ "role": "user", "content": "earlier" }]);
        let two_messages = json!([
            { "role": "user", "content": "earlier" },
            { "role": "assistant", "content": "reply" }
        ]);
        let cases = [
            (
                "absent memory is off",
                json!({}),
                Some(run),
                Resolved::Stateless { noted: false },
            ),
            (
                "legacy off",
                json!({ "memory": { "kind": "off" } }),
                Some(run),
                Resolved::Stateless { noted: false },
            ),
            (
                "legacy auto prefers the run's id",
                json!({ "memory": { "kind": "auto", "context_length": 4, "memory_id": baked } }),
                Some(run),
                Resolved::Window(run, 4),
            ),
            (
                "legacy auto falls back to its baked id",
                json!({ "memory": { "kind": "auto", "context_length": 4, "memory_id": baked } }),
                None,
                Resolved::Window(baked, 4),
            ),
            (
                "legacy auto with an empty baked id uses the run's",
                json!({ "memory": { "kind": "auto", "context_length": 4, "memory_id": "" } }),
                Some(run),
                Resolved::Window(run, 4),
            ),
            (
                "legacy auto with an empty baked id and no run id is stateless",
                json!({ "memory": { "kind": "auto", "context_length": 4, "memory_id": " " } }),
                None,
                Resolved::Stateless { noted: true },
            ),
            (
                "legacy auto without a length is off",
                json!({ "memory": { "kind": "auto", "memory_id": baked } }),
                Some(run),
                Resolved::Stateless { noted: false },
            ),
            (
                "a cleared count is off",
                json!({ "memory": { "kind": "window", "context_length": null } }),
                Some(run),
                Resolved::Stateless { noted: false },
            ),
            (
                "legacy manual replays its messages",
                json!({ "memory": { "kind": "manual", "messages": message } }),
                Some(run),
                Resolved::Messages(1),
            ),
            (
                "window keeps the run's memory",
                json!({ "memory": window }),
                Some(run),
                Resolved::Window(run, 10),
            ),
            (
                "window without a memory id is stateless",
                json!({ "memory": window }),
                None,
                Resolved::Stateless { noted: true },
            ),
            (
                "compaction keeps the run's memory",
                json!({ "memory": { "kind": "compaction", "context_window": 32000 } }),
                Some(run),
                Resolved::Compaction(run, 32000),
            ),
            (
                "a cleared context window is left for the model lookup to fill",
                json!({ "memory": { "kind": "compaction", "context_window": null } }),
                Some(run),
                Resolved::Compaction(run, 0),
            ),
            (
                "a step memory id overrides the run's",
                json!({ "memory": window, "memory_id": "cust_1" }),
                Some(run),
                Resolved::Window(cust_1, 10),
            ),
            (
                "a uuid step memory id is used as is",
                json!({ "memory": window, "memory_id": baked.to_string() }),
                Some(run),
                Resolved::Window(baked, 10),
            ),
            (
                "a step memory id evaluating to null is stateless",
                json!({ "memory": window, "memory_id": null }),
                Some(run),
                Resolved::Stateless { noted: true },
            ),
            (
                "an off policy ignores the step memory id, and says so",
                json!({ "memory": { "kind": "off" }, "memory_id": "cust_1" }),
                Some(run),
                Resolved::Stateless { noted: true },
            ),
            (
                "managed memory ignores the step's previous messages",
                json!({ "memory": window, "memory_id": "cust_1", "previous_messages": message }),
                Some(run),
                Resolved::Window(cust_1, 10),
            ),
            (
                "memory that is off sends the step's previous messages",
                json!({ "previous_messages": message }),
                Some(run),
                Resolved::Messages(1),
            ),
            (
                "a previous messages expression that evaluated to null is no history",
                json!({ "previous_messages": null }),
                Some(run),
                Resolved::Stateless { noted: false },
            ),
            (
                "a legacy manual list ignores the step's previous messages",
                json!({ "memory": { "kind": "manual", "messages": message }, "previous_messages": two_messages }),
                Some(run),
                Resolved::Messages(1),
            ),
            (
                "legacy auto ignores a step memory id",
                json!({ "memory": { "kind": "auto", "context_length": 4, "memory_id": baked }, "memory_id": "cust_1" }),
                None,
                Resolved::Window(baked, 4),
            ),
        ];
        for (name, history, run_memory_id, expected) in cases {
            let mut raw = json!({ "provider": { "kind": "openai", "resource": {}, "model": "m" } });
            raw.as_object_mut()
                .unwrap()
                .extend(history.as_object().unwrap().clone());
            let args: AIAgentArgs = serde_json::from_value(raw).unwrap();
            let resolved = match resolve_history_source(&args, run_memory_id, "ws", "f/flow") {
                (HistorySource::Messages(m), _) => Resolved::Messages(m.len()),
                (
                    HistorySource::Managed {
                        memory_id,
                        bound: MemoryBound::LastMessages(context_length),
                    },
                    _,
                ) => Resolved::Window(memory_id, context_length),
                (
                    HistorySource::Managed {
                        memory_id,
                        bound: MemoryBound::Compaction { context_window },
                    },
                    _,
                ) => Resolved::Compaction(memory_id, context_window),
                (HistorySource::Stateless, notes) => {
                    Resolved::Stateless { noted: !notes.is_empty() }
                }
            };
            assert_eq!(resolved, expected, "{name}");
        }
    }

    /// A placeholder the form seeds must not read as a memory id that evaluated to nothing, which
    /// would turn memory off for the step.
    #[test]
    fn only_an_expression_can_set_an_empty_step_memory_id() {
        let transforms = |memory_id: &str| -> HashMap<String, InputTransform> {
            HashMap::from([(
                "memory_id".to_string(),
                serde_json::from_str(memory_id).unwrap(),
            )])
        };
        let args = || -> AIAgentArgs {
            serde_json::from_value(serde_json::json!({
                "provider": { "kind": "openai", "resource": {}, "model": "m" },
                "memory_id": null,
            }))
            .unwrap()
        };
        for (transform, expected) in [
            (r#"{ "type": "static" }"#, None),
            (r#"{ "type": "static", "value": "" }"#, None),
            (r#"{ "type": "ai" }"#, None),
            (
                r#"{ "type": "javascript", "expr": "flow_input.customer_id" }"#,
                Some(""),
            ),
        ] {
            let mut args = args();
            keep_authored_memory_id(&mut args, &transforms(transform));
            assert_eq!(args.memory_id.as_deref(), expected, "{transform}");
        }
    }

    /// Only text output sends previous messages, so they never stand in for an image prompt.
    #[test]
    fn previous_messages_never_stand_in_for_an_image_prompt() {
        let args: AIAgentArgs = serde_json::from_value(serde_json::json!({
            "provider": { "kind": "openai", "resource": {}, "model": "m" },
            "previous_messages": [{ "role": "user", "content": "earlier" }],
        }))
        .unwrap();
        let (history, _) = resolve_history_source(&args, None, "ws", "f/flow");
        assert!(has_prompt(&history, false, true, false));
        assert!(!has_prompt(&history, false, false, false));
        assert!(has_prompt(&history, true, false, false));
        assert!(!has_prompt(
            &HistorySource::Messages(&[]),
            false,
            true,
            false
        ));
        // A legacy `manual` memory ran on an empty list alone, and still does for text output.
        assert!(has_prompt(&HistorySource::Messages(&[]), false, true, true));
        assert!(!has_prompt(
            &HistorySource::Messages(&[]),
            false,
            false,
            true
        ));
    }

    #[test]
    fn reasoning_keeps_every_iteration_in_order() {
        let mut acc = String::new();
        append_reasoning(&mut acc, Some("I need both cities.\n\n"));
        append_reasoning(&mut acc, None);
        append_reasoning(&mut acc, Some("   "));
        append_reasoning(&mut acc, Some("Paris is closer."));
        assert_eq!(acc, "I need both cities.\n\nParis is closer.");
    }

    #[test]
    fn an_unwritten_streaming_field_streams() {
        assert!(streaming_requested(None));
        assert!(streaming_requested(Some(true)));
        assert!(!streaming_requested(Some(false)));
    }

    #[test]
    fn max_iterations_partial_result_keeps_the_action_tags() {
        let messages = vec![OpenAIMessage {
            role: "tool".to_string(),
            content: Some(OpenAIContent::Text("{\"rows\":2}".to_string())),
            tool_call_id: Some("call_1".to_string()),
            agent_action: Some(AgentAction::ToolCall {
                job_id: uuid::Uuid::nil(),
                function_name: "list_payouts".to_string(),
                module_id: "b".to_string(),
            }),
            ..Default::default()
        }];

        let partial = MaxIterPartialResult {
            messages: messages
                .iter()
                .map(|m| Message { message: m, agent_action: m.agent_action.as_ref() })
                .collect(),
        };
        let json = serde_json::to_value(&partial).unwrap();

        let action = &json["messages"][0]["agent_action"];
        assert_eq!(action["type"], "tool_call");
        assert_eq!(action["function_name"], "list_payouts");
    }

    /// Over 64 characters OpenAI rejects the key outright, which costs a wasted round
    /// trip per run and silently leaves that step with no prompt caching at all.
    #[test]
    fn prompt_cache_key_stays_within_the_provider_bound() {
        let long = format!("my-workspace:f/{}/agent:step_12", "nested_folder".repeat(8));
        assert!(long.len() > 64);

        let bounded = bounded_prompt_cache_key(&long);

        assert!(
            bounded.len() <= 64,
            "got {} chars: {bounded}",
            bounded.len()
        );
        // Stable for the same step, or every run would land on a different cache.
        assert_eq!(bounded, bounded_prompt_cache_key(&long));
        assert_ne!(
            bounded,
            bounded_prompt_cache_key(&long.replace("step_12", "step_13"))
        );
    }

    #[test]
    fn prompt_cache_key_passes_short_keys_through_unchanged() {
        let short = "admins:f/agent/step:a";
        assert_eq!(bounded_prompt_cache_key(short), short);
    }

    /// Truncation on a byte index would panic mid-character.
    #[test]
    fn prompt_cache_key_truncates_on_a_char_boundary() {
        let long = format!("workspace:f/{}/agent:step", "é".repeat(80));
        assert!(bounded_prompt_cache_key(&long).len() <= 64);
    }

    #[test]
    fn overlay_tool_inputs_binds_matching_flowmodule_tool_only() {
        fn js(expr: &str) -> InputTransform {
            InputTransform::Javascript { expr: expr.to_string() }
        }
        fn script_tool(id: &str, key: &str, expr: &str) -> AgentTool {
            let mut its = HashMap::new();
            its.insert(key.to_string(), js(expr));
            AgentTool {
                id: id.to_string(),
                summary: None,
                description: None,
                value: ToolValue::FlowModule(FlowModuleValue::Script {
                    input_transforms: its,
                    path: "u/test/tool".to_string(),
                    hash: None,
                    tag_override: None,
                    is_trigger: None,
                    pass_flow_input_directly: None,
                }),
            }
        }
        fn script_its(tool: &AgentTool) -> &HashMap<String, InputTransform> {
            let ToolValue::FlowModule(FlowModuleValue::Script { input_transforms, .. }) =
                &tool.value
            else {
                panic!("expected script tool")
            };
            input_transforms
        }

        // "a" gets rebound, "b" is left alone, the MCP tool is skipped even though it has an override.
        let mut tools = vec![
            script_tool("a", "x", "authoring_flow_expr"),
            script_tool("b", "y", "keep_me"),
            AgentTool {
                id: "m".to_string(),
                summary: None,
                description: None,
                value: ToolValue::Mcp(windmill_common::flows::McpToolValue {
                    resource_path: "u/test/mcp".to_string(),
                    include_tools: vec![],
                    exclude_tools: vec![],
                }),
            },
        ];

        let mut tool_inputs: HashMap<String, HashMap<String, InputTransform>> = HashMap::new();
        tool_inputs.insert(
            "a".to_string(),
            HashMap::from([
                ("x".to_string(), js("flow_input.tenant")),
                ("z".to_string(), js("results.step1")),
            ]),
        );
        tool_inputs.insert(
            "m".to_string(),
            HashMap::from([("q".to_string(), js("ignored"))]),
        );

        overlay_tool_inputs(&mut tools, &tool_inputs);

        // "a": existing key replaced, new key added.
        let a = script_its(&tools[0]);
        assert!(
            matches!(a.get("x"), Some(InputTransform::Javascript { expr }) if expr == "flow_input.tenant")
        );
        assert!(
            matches!(a.get("z"), Some(InputTransform::Javascript { expr }) if expr == "results.step1")
        );
        // "b": no override for it, untouched.
        let b = script_its(&tools[1]);
        assert!(
            matches!(b.get("y"), Some(InputTransform::Javascript { expr }) if expr == "keep_me")
        );
        // MCP tool: not a FlowModule, left as-is.
        assert!(matches!(&tools[2].value, ToolValue::Mcp(_)));
    }

    #[test]
    fn narrow_roster_keeps_the_entries_a_run_named() {
        fn named(id: &str, summary: &str) -> AgentTool {
            AgentTool {
                id: id.to_string(),
                summary: Some(summary.to_string()),
                description: None,
                value: ToolValue::FlowModule(FlowModuleValue::Script {
                    input_transforms: HashMap::new(),
                    path: "u/test/tool".to_string(),
                    hash: None,
                    tag_override: None,
                    is_trigger: None,
                    pass_flow_input_directly: None,
                }),
            }
        }
        fn mcp(id: &str, summary: &str, path: &str) -> AgentTool {
            AgentTool {
                id: id.to_string(),
                summary: Some(summary.to_string()),
                description: None,
                value: ToolValue::Mcp(windmill_common::flows::McpToolValue {
                    resource_path: path.to_string(),
                    include_tools: vec![],
                    exclude_tools: vec![],
                }),
            }
        }
        fn websearch(id: &str, summary: Option<&str>) -> AgentTool {
            AgentTool {
                id: id.to_string(),
                summary: summary.map(str::to_string),
                description: None,
                value: ToolValue::Websearch(windmill_common::flows::WebsearchToolValue {}),
            }
        }
        let roster = || {
            vec![
                named("a", "get_user"),
                named("b", "send_email"),
                mcp("c", "github", "$res:u/test/gh"),
            ]
        };
        let names = |tools: &[AgentTool]| -> Vec<String> {
            tools.iter().filter_map(|t| t.summary.clone()).collect()
        };
        let ids =
            |tools: &[AgentTool]| -> Vec<String> { tools.iter().map(|t| t.id.clone()).collect() };

        // No list at all: the whole roster, as every agent written before the field expects.
        assert_eq!(
            names(&narrow_roster(roster(), None)),
            ["get_user", "send_email", "github"]
        );

        // An empty list is a list: nothing is advertised, and no server is resolved to find that
        // out — one that is down must not fail a run that switched it off.
        assert!(names(&narrow_roster(roster(), Some(&[]))).is_empty());

        let enabled = ["get_user".to_string(), "renamed_away".to_string()];
        assert_eq!(
            names(&narrow_roster(roster(), Some(&enabled))),
            ["get_user"]
        );
        // Counted, not quoted: a name is an argument value, and one holding `$var:` arrives as the
        // variable's own value, which this log is not masked for.
        assert_eq!(
            unmatched_enabled_tools_message(&enabled, &["get_user", "send_email", "u/test/gh"])
                .unwrap(),
            "--- ENABLED TOOLS: 1 name named no tool of this agent and had no effect ---\n"
        );

        // A server is named by the resource it points at, bare, and never by its summary: that
        // label is shown to nobody and two entries may carry the same one.
        let named_server = ["u/test/gh".to_string()];
        assert_eq!(
            names(&narrow_roster(roster(), Some(&named_server))),
            ["github"]
        );
        assert!(unmatched_enabled_tools_message(&named_server, &["u/test/gh"]).is_none());
        // Alongside a name that does match, so the summary being rejected is what empties it.
        let summary_and_tool = ["get_user".to_string(), "github".to_string()];
        assert_eq!(
            names(&narrow_roster(roster(), Some(&summary_and_tool))),
            ["get_user"]
        );
        assert!(narrow_roster(roster(), Some(&["u/test/other".to_string()])).is_empty());

        // Web search reaches the model as a provider capability rather than a tool, so it has no
        // name of its own and is enabled by a reserved one, whatever label it was authored with.
        for label in [None, Some("Web Search")] {
            let mut with_websearch = roster();
            with_websearch.push(websearch("w", label));
            assert_eq!(
                ids(&narrow_roster(
                    with_websearch,
                    Some(&[WEBSEARCH_ENABLED_NAME.to_string()])
                )),
                ["w"]
            );
        }
    }

    #[test]
    fn a_tool_cannot_take_the_name_web_search_is_enabled_by() {
        // Nothing else in a roster may answer to the reserved name, or enabling that tool would
        // switch web search on beside it. Held here rather than by the shape of the name, which is
        // an ordinary identifier: the run refuses to start instead.
        assert!(TOOL_NAME_REGEX.is_match(WEBSEARCH_ENABLED_NAME));
        assert!(flow_module_tool_name(Some(WEBSEARCH_ENABLED_NAME)).is_err());

        assert_eq!(flow_module_tool_name(Some("get_user")).unwrap(), "get_user");
        assert!(flow_module_tool_name(Some("get user")).is_err());
        assert!(flow_module_tool_name(None).is_err());
    }

    #[test]
    fn tool_description_prefers_explicit_over_derived_and_name() {
        assert_eq!(
            resolve_tool_description(
                Some("  Use to look up a user by id  ".to_string()),
                Some("derived from script".to_string()),
                "get_user"
            ),
            "Use to look up a user by id"
        );
    }

    #[test]
    fn tool_description_falls_back_to_derived_when_no_explicit() {
        assert_eq!(
            resolve_tool_description(None, Some("Sync resources".to_string()), "sync_tool"),
            "Sync resources"
        );
        // A blank explicit description must not shadow a usable derived one.
        assert_eq!(
            resolve_tool_description(
                Some("   ".to_string()),
                Some("Sync resources".to_string()),
                "sync_tool"
            ),
            "Sync resources"
        );
    }

    #[test]
    fn tool_description_falls_back_to_name_when_nothing_usable() {
        assert_eq!(resolve_tool_description(None, None, "my_tool"), "my_tool");
        assert_eq!(
            resolve_tool_description(Some("  ".to_string()), Some("".to_string()), "my_tool"),
            "my_tool"
        );
    }

    #[test]
    fn auto_memory_request_preserves_messages_within_context_window() {
        let loaded_messages = vec![
            text_message("system", "instructions-a"),
            text_message("user", "first-user"),
            text_message("assistant", "first-assistant"),
            text_message("system", "instructions-b"),
            text_message("user", "second-user"),
            text_message("assistant", "second-assistant"),
        ];

        let prepared = prepare_auto_memory_messages_for_request(&loaded_messages, 3);
        let roles: Vec<&str> = prepared
            .iter()
            .map(|message| message.role.as_str())
            .collect();
        let contents: Vec<&str> = prepared
            .iter()
            .map(|message| match message.content.as_ref() {
                Some(OpenAIContent::Text(text)) => text.as_str(),
                _ => "",
            })
            .collect();

        assert_eq!(roles, vec!["system", "user", "assistant"]);
        assert_eq!(
            contents,
            vec!["instructions-b", "second-user", "second-assistant"]
        );
    }

    #[test]
    fn auto_memory_request_drops_leading_tool_messages() {
        let loaded_messages = vec![
            text_message("tool", "stale-tool-result"),
            text_message("user", "hello"),
            text_message("assistant", "hi"),
        ];

        let prepared = prepare_auto_memory_messages_for_request(&loaded_messages, 10);
        let roles: Vec<&str> = prepared
            .iter()
            .map(|message| message.role.as_str())
            .collect();

        assert_eq!(roles, vec!["user", "assistant"]);
    }

    #[test]
    fn auto_memory_persistence_excludes_system_messages() {
        let all_messages = vec![
            text_message("system", "instructions"),
            text_message("user", "hello"),
            text_message("assistant", "hi"),
            text_message("system", "duplicate-instructions"),
            text_message("user", "follow-up"),
        ];

        let persisted = prepare_auto_memory_messages_for_persistence(&all_messages, 10);
        let roles: Vec<&str> = persisted
            .iter()
            .map(|message| message.role.as_str())
            .collect();

        assert_eq!(roles, vec!["user", "assistant", "user"]);
    }
}

/// Handle credentials check mode - check credentials without making API calls
async fn handle_credentials_check(provider: &ProviderWithResource) -> Result<Box<RawValue>, Error> {
    let result = match &provider.kind {
        #[cfg(feature = "bedrock")]
        AIProvider::AWSBedrock => {
            let check = check_env_credentials().await;
            serde_json::json!({
                "credentials_check": true,
                "provider": "aws_bedrock",
                "credentials": {
                    "available": check.available,
                    "access_key_id_prefix": check.access_key_id_prefix,
                    "region": check.region,
                    "error": check.error
                }
            })
        }
        #[cfg(not(feature = "bedrock"))]
        AIProvider::AWSBedrock => {
            serde_json::json!({
                "credentials_check": true,
                "provider": "aws_bedrock",
                "error": "AWS Bedrock support is not enabled. Build with 'bedrock' feature."
            })
        }
        other => {
            serde_json::json!({
                "credentials_check": true,
                "provider": format!("{:?}", other),
                "message": "Credentials check not implemented for this provider"
            })
        }
    };

    serde_json::value::to_raw_value(&result).map_err(|e| Error::internal_err(e.to_string()))
}

/// Hard-timeout fallback: force-cancel any descendant jobs still in v2_job_queue
/// so they don't stay as zombies.
async fn cleanup_orphaned_tool_jobs(
    db: &DB,
    parent_job_id: &Uuid,
    w_id: &str,
    canceled_by: Option<CanceledBy>,
) {
    let username = canceled_by
        .as_ref()
        .and_then(|cb| cb.username.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let reason = canceled_by
        .as_ref()
        .and_then(|cb| cb.reason.clone())
        .unwrap_or_else(|| {
            format!(
                "parent AI agent {} was cancelled and tool call did not complete in time",
                parent_job_id
            )
        });

    // Find direct child jobs still in v2_job_queue (agent tool jobs are always direct children)
    let orphaned_ids: Vec<Uuid> = match sqlx::query_scalar!(
        r#"SELECT j.id FROM v2_job j
            JOIN v2_job_queue q ON q.id = j.id
            WHERE j.parent_job = $1 AND j.workspace_id = $2"#,
        parent_job_id,
        w_id,
    )
    .fetch_all(db)
    .await
    {
        Ok(ids) => ids,
        Err(e) => {
            tracing::error!(
                "Failed to find orphaned tool jobs for {}: {}",
                parent_job_id,
                e
            );
            return;
        }
    };

    if orphaned_ids.is_empty() {
        return;
    }

    tracing::warn!(
        "Cleaning up {} orphaned tool jobs for cancelled AI agent {}",
        orphaned_ids.len(),
        parent_job_id,
    );

    for job_id in &orphaned_ids {
        let queued_job = match windmill_queue::get_queued_job_v2(db, job_id).await {
            Ok(Some(j)) => j,
            Ok(None) => continue,
            Err(e) => {
                tracing::error!("Failed to fetch orphaned tool job {}: {}", job_id, e);
                continue;
            }
        };

        let tx = match db.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!(
                    "Failed to begin transaction for orphaned job {}: {}",
                    job_id,
                    e
                );
                continue;
            }
        };

        match cancel_single_job(
            &username,
            Some(reason.clone()),
            queued_job,
            w_id,
            tx,
            db,
            true,
        )
        .await
        {
            Ok((tx, _)) => {
                if let Err(e) = tx.commit().await {
                    tracing::error!(
                        "Failed to commit cancel for orphaned tool job {}: {}",
                        job_id,
                        e
                    );
                }
            }
            Err(e) => {
                // warn not error: job may have completed between fetch and cancel (expected race)
                tracing::warn!("Failed to force-cancel orphaned tool job {}: {}", job_id, e);
            }
        }
    }
}
