//! Summary-based partial compaction for AI agent steps running with
//! `Memory::AutoCompacted`.
//!
//! When the prompt of an agent-loop iteration approaches the model's context window,
//! the older prefix of the conversation is replaced by a single LLM-written summary
//! while the recent tail is kept verbatim. The summary is written "up to" the point of
//! compaction: messages the summarizing model does not see follow it in the
//! conversation.

use windmill_ai::{
    ai_providers::AIProvider,
    credentials::ProviderCredentials,
    proxy::{common_outbound_headers, retain_effective_credentials},
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder},
    types::{OpenAIContent, OpenAIMessage, OutputType, TokenUsage},
    utils::pinned_ai_client_for,
};
use windmill_common::{client::AuthedClient, error::Error};

use crate::ai::stream_event_processor::StreamEventProcessor;

/// Fraction of the context window at which a compaction is taken.
const COMPACTION_TRIGGER_RATIO: f64 = 0.8;
/// Fraction of the context window the kept tail plus the summary should fit into.
const COMPACTION_TARGET_RATIO: f64 = 0.7;
/// Room left inside the target for the summary itself, which is prepended to the tail.
const SUMMARY_OUTPUT_RESERVE_TOKENS: usize = 8000;
/// Ceiling on that reserve as a share of the window. A flat 8000 eats the whole target
/// on a small window, which would collapse the tail to one message and summarize the
/// rest — a context wipe rather than a compaction.
const MAX_SUMMARY_RESERVE_SHARE: usize = 10;
/// Summarizing fewer messages than this costs a full model call to save almost nothing.
const MIN_PREFIX_MESSAGES_TO_SUMMARIZE: usize = 4;
/// After this many failed summarizations the run stops trying, so a provider that
/// rejects the summarization request does not add a wasted call to every iteration. The
/// count is per run: a chat turn is its own job, and its conversation has grown since
/// the last one, so it is worth one attempt of its own.
const MAX_CONSECUTIVE_COMPACTION_FAILURES: usize = 3;

// Reinforce text-only output. The summarization call carries no tools, but a strong
// instruction keeps weaker models from narrating a tool call instead of summarizing.
const NO_TOOLS_PREAMBLE: &str = "CRITICAL: Respond with TEXT ONLY. Do NOT call any tools.

- You already have all the context you need in the conversation above.
- Your entire response must be plain text: an <analysis> block followed by a <summary> block.

";

const NO_TOOLS_TRAILER: &str =
    "\n\nREMINDER: Respond with plain text only — an <analysis> block followed by a <summary> block.";

// The <analysis> block is a drafting scratchpad that `format_compact_summary` strips
// before the summary reaches the conversation.
const SUMMARY_PROMPT: &str = r#"Your task is to create a detailed summary of the conversation so far. This summary will be placed at the start of a continuing session; newer messages that build on this context will follow after it (you do not see them here). Summarize thoroughly so that someone reading only your summary and then the newer messages can fully understand what happened and continue the work without losing context.

This is a conversation with an AI assistant that uses tools to accomplish tasks. Frame the summary in those terms.

Before providing your final summary, wrap your analysis in <analysis> tags to organize your thoughts. In your analysis:

1. Chronologically analyze each message and section of the conversation. For each section thoroughly identify:
   - The user's explicit requests and intents
   - Your approach to addressing the user's requests
   - Key decisions, technical concepts and patterns
   - Specific details: the tools called, the arguments they were called with, and what they returned
   - Errors you ran into and how you fixed them
   - Specific user feedback, especially if the user told you to do something differently
2. Double-check for technical accuracy and completeness.

Your summary should include the following sections:

1. Primary Request and Intent: Capture all of the user's explicit requests and intents in detail.
2. Key Technical Concepts: List important technical concepts, technologies, and frameworks discussed.
3. Actions and Results: Enumerate the tool calls that were made — each by tool name — noting the arguments used, what came back, and why it mattered. Include verbatim excerpts of any data, identifiers or code that later steps depend on.
4. Errors and fixes: List errors encountered and how they were fixed, including any user feedback.
5. Problem Solving: Document problems solved and any ongoing troubleshooting efforts.
6. All user messages: List ALL user messages that are not tool results. These are critical for understanding the user's feedback and changing intent.
7. Pending Tasks: Outline any pending tasks you have explicitly been asked to work on.
8. Current Work: Describe precisely what was being worked on immediately before this summary, paying special attention to the most recent messages.
9. Context for Continuing Work: Summarize any context, decisions, or state needed to understand and continue the work in subsequent messages. If there is a clear next step directly in line with the user's most recent explicit request, state it and include a direct quote from the most recent conversation showing where you left off.

Structure your output like this:

<analysis>
[Your thought process, ensuring all points are covered thoroughly and accurately]
</analysis>

<summary>
1. Primary Request and Intent:
   [Detailed description]

2. Key Technical Concepts:
   - [Concept]

3. Actions and Results:
   - [Tool name]
      - [Arguments used]
      - [What came back]
      - [Why it matters]

4. Errors and fixes:
   - [Error]: [How you fixed it]

5. Problem Solving:
   [Description]

6. All user messages:
   - [Non-tool-result user message]

7. Pending Tasks:
   - [Task]

8. Current Work:
   [Precise description of current work]

9. Context for Continuing Work:
   [Key context, decisions, or state needed to continue]
</summary>

Please provide your summary following this structure, ensuring precision and thoroughness."#;

/// Rough per-message token count, used only to choose where to cut the conversation.
/// The trigger itself runs off the provider's own count; this only has to rank
/// messages by size consistently.
fn estimate_message_tokens(message: &OpenAIMessage) -> usize {
    let content_len = match message.content.as_ref() {
        Some(OpenAIContent::Text(text)) => text.len(),
        Some(OpenAIContent::Parts(parts)) => {
            serde_json::to_string(parts).map(|s| s.len()).unwrap_or(0)
        }
        None => 0,
    };
    let tool_calls_len = message
        .tool_calls
        .as_ref()
        .and_then(|calls| serde_json::to_string(calls).ok())
        .map(|s| s.len())
        .unwrap_or(0);
    (content_len + tool_calls_len) / 4
}

/// Whole-conversation estimate, used only when the provider reported no usage for the
/// last request; without it the mode would be silently inert on such providers.
fn estimate_conversation_tokens(messages: &[OpenAIMessage]) -> usize {
    messages.iter().map(estimate_message_tokens).sum()
}

/// What the provider said about the most recent request of the agent loop.
#[derive(Clone, Copy)]
pub struct LastRequest {
    /// Prompt tokens the provider counted, `None` when it reported no usage.
    pub prompt_tokens: Option<i32>,
    /// How many messages that request carried. The assistant turn it produced and the
    /// tool results that followed sit past this index and are not in `prompt_tokens`.
    pub message_count: usize,
}

/// Size of the prompt the *next* request would carry: what the provider counted for the
/// last one plus an estimate of everything appended since. A single large tool result
/// can be most of a context window, so measuring only the last request would let the
/// conversation blow past the window without ever tripping the trigger.
fn projected_prompt_tokens(messages: &[OpenAIMessage], last_request: LastRequest) -> usize {
    match last_request.prompt_tokens {
        Some(counted) => {
            let appended = last_request.message_count.min(messages.len());
            (counted.max(0) as usize) + estimate_conversation_tokens(&messages[appended..])
        }
        None => estimate_conversation_tokens(messages),
    }
}

/// How much of the window the summary may occupy. Both the split that decides what to
/// keep and the cap the summarization request asks for read this: if they disagree, a
/// summary can come back larger than the room made for it and put the conversation
/// straight back over the trigger, compacting its own previous summary every response.
fn summary_reserve_tokens(context_window: usize) -> usize {
    SUMMARY_OUTPUT_RESERVE_TOKENS.min(context_window / MAX_SUMMARY_RESERVE_SHARE)
}

/// First message that may be summarized away. The leading system messages carry the
/// step's own system prompt, which has to survive compaction.
fn summarizable_start(messages: &[OpenAIMessage]) -> usize {
    messages
        .iter()
        .position(|message| message.role != "system")
        .unwrap_or(messages.len())
}

/// Index at which the kept tail starts, or `None` when there is nothing worth
/// summarizing.
///
/// The tail grows backwards from the newest message until it fills the target budget,
/// then backs up over any leading `tool` messages so it never opens with a tool result
/// whose `tool_calls` message was summarized away — every provider rejects that.
fn plan_tail_start(messages: &[OpenAIMessage], context_window: usize) -> Option<usize> {
    let prefix_start = summarizable_start(messages);

    let budget = ((context_window as f64 * COMPACTION_TARGET_RATIO) as usize)
        - summary_reserve_tokens(context_window);

    let mut tail_start = messages.len();
    let mut tail_tokens = 0usize;
    for index in (prefix_start..messages.len()).rev() {
        let tokens = estimate_message_tokens(&messages[index]);
        // The newest message is always kept, however large: dropping it would discard
        // the tool results this iteration just produced.
        if tail_start < messages.len() && tail_tokens + tokens > budget {
            break;
        }
        tail_tokens += tokens;
        tail_start = index;
    }

    while tail_start > prefix_start && messages[tail_start].role == "tool" {
        tail_start -= 1;
    }

    (tail_start.saturating_sub(prefix_start) >= MIN_PREFIX_MESSAGES_TO_SUMMARIZE)
        .then_some(tail_start)
}

/// Strips the `<analysis>` drafting scratchpad and unwraps the `<summary>` block.
/// Falls back to the trimmed raw text when the model did not use the tags, so a
/// well-formed-but-untagged summary is still usable, and to the empty string — which
/// the caller counts as a failure — when the response stopped inside the scratchpad.
fn format_compact_summary(raw: &str) -> String {
    // Strip the analysis scratchpad first: it precedes the summary and may itself
    // mention <summary>/<analysis> tokens that would otherwise be mistaken for the real
    // summary boundary.
    let without_analysis = ANALYSIS_BLOCK_REGEX.replace_all(raw, "");

    let summary = if let Some(captures) = SUMMARY_BLOCK_REGEX.captures(&without_analysis) {
        captures
            .get(1)
            .map(|m| m.as_str())
            .unwrap_or_default()
            .to_string()
    } else if let Some(opener) = SUMMARY_OPENER_REGEX.find(&without_analysis) {
        // A truncated response or a weaker model sometimes opens <summary> without
        // closing it. The text after the opener is still the summary.
        without_analysis[opener.end()..].to_string()
    } else if ANALYSIS_OPENER_REGEX.is_match(&without_analysis) {
        // An unclosed <analysis> means the response ran out before it reached the
        // summary. Its draft notes are not one, and handing them over would put the
        // model's own scratchpad into the conversation as fact.
        return String::new();
    } else {
        without_analysis.to_string()
    };

    // An orphaned opener or closer left by either branch must never reach the model.
    let summary = STRAY_TAG_REGEX.replace_all(&summary, "");
    // Collapse the blank-line runs left behind by stripping the analysis block.
    BLANK_LINE_RUN_REGEX
        .replace_all(&summary, "\n\n")
        .trim()
        .to_string()
}

lazy_static::lazy_static! {
    static ref ANALYSIS_BLOCK_REGEX: regex::Regex =
        regex::Regex::new(r"(?is)<analysis>.*?</analysis>").unwrap();
    static ref ANALYSIS_OPENER_REGEX: regex::Regex =
        regex::Regex::new(r"(?i)<analysis>").unwrap();
    static ref SUMMARY_BLOCK_REGEX: regex::Regex =
        regex::Regex::new(r"(?is)<summary>(.*?)</summary>").unwrap();
    static ref SUMMARY_OPENER_REGEX: regex::Regex =
        regex::Regex::new(r"(?i)<summary>").unwrap();
    static ref STRAY_TAG_REGEX: regex::Regex =
        regex::Regex::new(r"(?i)</?(?:analysis|summary)>").unwrap();
    static ref BLANK_LINE_RUN_REGEX: regex::Regex = regex::Regex::new(r"\n{3,}").unwrap();
}

/// Wraps a formatted summary as the user message that replaces the summarized prefix.
fn build_summary_message(formatted_summary: &str) -> OpenAIMessage {
    OpenAIMessage {
        role: "user".to_string(),
        content: Some(OpenAIContent::Text(format!(
            "This conversation is being continued from an earlier portion that ran out of context. \
             The summary below covers that earlier portion. Recent messages after the summary are \
             preserved verbatim.\n\n{formatted_summary}\n\nContinue from where it left off. Do not \
             re-introduce the summary or recap it; pick up the work as if the break never happened."
        ))),
        ..Default::default()
    }
}

/// Everything the summarization call needs that the agent loop already has in hand.
pub struct CompactionRequest<'a> {
    pub query_builder: &'a dyn QueryBuilder,
    pub credentials: &'a ProviderCredentials,
    pub model: &'a str,
    pub temperature: Option<f32>,
    pub timeout: std::time::Duration,
    pub client: &'a AuthedClient,
    pub workspace_id: &'a str,
    /// Whether the endpoint accepts the usage-tracking request shape. The agent loop
    /// learns this from a rejection; sending it again here would make every
    /// summarization fail on an endpoint the agent itself runs fine against.
    pub include_usage: bool,
}

/// Per-step compaction state: the configured window and how many summarizations in a
/// row have failed.
pub struct Compactor {
    context_window: usize,
    consecutive_failures: usize,
    /// Whether the current conversation has already been acted on. A compaction pass is
    /// worth taking once per provider response: a loop that exits without issuing
    /// another request — a structured-output turn does — otherwise reaches the post-loop
    /// pass on the same conversation and summarizes it a second time, or retries a
    /// failure with nothing changed.
    measurement_spent: bool,
}

impl Compactor {
    pub fn new(context_window: usize) -> Self {
        Self { context_window, consecutive_failures: 0, measurement_spent: false }
    }

    /// Arms the next compaction pass. Called for each provider response, whose token
    /// count is what that pass measures.
    pub fn record_response(&mut self) {
        self.measurement_spent = false;
    }

    /// Summarizes the older part of `messages` in place when the conversation has
    /// crossed the trigger threshold. Returns the summarization call's own token usage
    /// so the step can bill it.
    ///
    /// A failed summarization is never fatal: the step keeps running on the
    /// uncompacted conversation and hits the provider's own context limit if it has to.
    pub async fn maybe_compact(
        &mut self,
        messages: &mut Vec<OpenAIMessage>,
        last_request: LastRequest,
        request: &CompactionRequest<'_>,
    ) -> Option<TokenUsage> {
        if self.consecutive_failures >= MAX_CONSECUTIVE_COMPACTION_FAILURES
            || self.measurement_spent
        {
            return None;
        }

        if (projected_prompt_tokens(messages, last_request) as f64)
            < self.context_window as f64 * COMPACTION_TRIGGER_RATIO
        {
            return None;
        }

        let Some(tail_start) = plan_tail_start(messages, self.context_window) else {
            // The trigger runs off the provider's count and the split off a character
            // estimate; when they disagree the step keeps growing with nothing done, so
            // say so rather than leaving the mode looking broken.
            tracing::info!(
                "AI agent is over its {} token context window but has nothing worth summarizing \
                 ({} messages)",
                self.context_window,
                messages.len()
            );
            return None;
        };

        self.measurement_spent = true;

        match summarize_prefix(
            &messages[..tail_start],
            summary_reserve_tokens(self.context_window),
            request,
        )
        .await
        {
            Ok((summary, usage)) => {
                let formatted = format_compact_summary(&summary);
                if formatted.is_empty() {
                    self.consecutive_failures += 1;
                    tracing::warn!("AI agent compaction produced an empty summary, skipping");
                    return usage;
                }
                self.consecutive_failures = 0;
                let prefix_start = summarizable_start(messages);
                let compacted = messages.drain(prefix_start..tail_start).count();
                messages.insert(prefix_start, build_summary_message(&formatted));
                tracing::info!(
                    "AI agent compacted {} messages into a summary, {} messages left",
                    compacted,
                    messages.len()
                );
                usage
            }
            Err(e) => {
                self.consecutive_failures += 1;
                tracing::warn!(
                    "AI agent compaction failed ({}/{}): {e}",
                    self.consecutive_failures,
                    MAX_CONSECUTIVE_COMPACTION_FAILURES
                );
                None
            }
        }
    }
}

/// Sends the prefix to the same model with the compaction prompt appended and no tools.
async fn summarize_prefix(
    prefix: &[OpenAIMessage],
    reserve_tokens: usize,
    request: &CompactionRequest<'_>,
) -> Result<(String, Option<TokenUsage>), Error> {
    let mut summary_messages = prefix.to_vec();
    // A trailing assistant message whose tool results were kept in the tail would reach
    // the provider as an unanswered tool call, which every provider rejects.
    while summary_messages
        .last()
        .is_some_and(|message| message.tool_calls.is_some())
    {
        summary_messages.pop();
    }
    summary_messages.push(OpenAIMessage {
        role: "user".to_string(),
        content: Some(OpenAIContent::Text(format!(
            "{NO_TOOLS_PREAMBLE}{SUMMARY_PROMPT}{NO_TOOLS_TRAILER}"
        ))),
        ..Default::default()
    });

    let build_args = BuildRequestArgs {
        messages: &summary_messages,
        tools: None,
        model: request.model,
        temperature: request.temperature,
        // The step's reasoning effort is deliberately dropped. Every provider counts
        // thinking against this same budget, so a high-effort model can spend the whole
        // reserve before writing anything and hand back a summary cut off inside its
        // scratchpad — which counts as a failure. The prompt asks for an `<analysis>`
        // block, which is the reasoning this call needs.
        reasoning_effort: None,
        // Exactly the room the split set aside. Asking for more lets a summary land the
        // conversation back over the trigger; leaving it to the provider default gives
        // 64000 on Anthropic (over several Claude models' output ceiling) and the model's
        // own small default on Bedrock, short enough to truncate the response.
        max_tokens: Some(reserve_tokens as u32),
        output_schema: None,
        output_type: &OutputType::Text,
        system_prompt: None,
        user_message: "",
        attachments: None,
        has_websearch: false,
        // The summarization request carries no tools, so its prefix does not match the
        // agent's own requests: routing it onto the step's cache key would only evict.
        prompt_cache_key: None,
    };

    let parsed = if request.credentials.provider == AIProvider::AWSBedrock {
        #[cfg(feature = "bedrock")]
        {
            windmill_ai::providers::bedrock::BedrockQueryBuilder::default()
                .execute_request(
                    &summary_messages,
                    None,
                    request.model,
                    request.temperature,
                    build_args.reasoning_effort,
                    build_args.max_tokens,
                    request.credentials.api_key.as_deref().unwrap_or(""),
                    request
                        .credentials
                        .region
                        .as_deref()
                        .unwrap_or(windmill_ai::ai_providers::USE_ENV_REGION),
                    Some(StreamEventProcessor::new_silent().boxed_sink()),
                    request.client,
                    request.workspace_id,
                    None,
                    request.credentials.aws_access_key_id.as_deref(),
                    request.credentials.aws_secret_access_key.as_deref(),
                    request.credentials.aws_session_token.as_deref(),
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
        let base_url = &request.credentials.base_url;
        let api_key = request.credentials.api_key.as_deref().unwrap_or("");
        let body = if request.include_usage {
            request
                .query_builder
                .build_request(&build_args, request.client, request.workspace_id)
                .await?
        } else {
            request
                .query_builder
                .build_request_without_usage(&build_args, request.client, request.workspace_id)
                .await?
        };
        let endpoint =
            request
                .query_builder
                .get_endpoint(base_url, request.model, &OutputType::Text);
        let auth_headers = retain_effective_credentials(
            request.credentials,
            request
                .query_builder
                .get_auth_headers(api_key, base_url, &OutputType::Text),
        );

        // As in the agent loop, `endpoint` derives from the user-controlled provider
        // base_url, so DNS is pinned to the SSRF-validated address.
        let mut http_request = pinned_ai_client_for(base_url)
            .await?
            .post(&endpoint)
            .timeout(request.timeout)
            .header("Content-Type", "application/json");
        for (header_name, header_value) in &auth_headers {
            http_request = http_request.header(*header_name, header_value.clone());
        }
        for (header_name, header_value) in common_outbound_headers(request.credentials) {
            http_request = http_request.header(header_name.as_str(), header_value.as_str());
        }

        let resp = http_request
            .body(body)
            .send()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to call API: {e}")))?;

        if let Err(e) = resp.error_for_status_ref() {
            let text = resp
                .text()
                .await
                .unwrap_or_else(|_| "<failed to read body>".to_string());
            return Err(Error::internal_err(format!(
                "API error calling {endpoint}: {e} - {text}"
            )));
        }

        request
            .query_builder
            .parse_streaming_response(resp, StreamEventProcessor::new_silent().boxed_sink())
            .await?
    };

    match parsed {
        ParsedResponse::Text { content, usage, .. } => Ok((content.unwrap_or_default(), usage)),
        ParsedResponse::Image { .. } => Err(Error::internal_err(
            "Compaction summary came back as an image".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(role: &str, content: &str) -> OpenAIMessage {
        OpenAIMessage {
            role: role.to_string(),
            content: Some(OpenAIContent::Text(content.to_string())),
            ..Default::default()
        }
    }

    #[test]
    fn format_compact_summary_drops_the_analysis_scratchpad() {
        let raw = "<analysis>mentions <summary> as a token</analysis>\n\n<summary>the real summary</summary>";
        assert_eq!(format_compact_summary(raw), "the real summary");
    }

    #[test]
    fn format_compact_summary_keeps_an_unclosed_summary() {
        let raw = "<analysis>x</analysis><summary>cut off mid";
        assert_eq!(format_compact_summary(raw), "cut off mid");
    }

    /// Draft notes are not a summary: passing them on would insert the model's own
    /// scratchpad into the conversation as the record of what happened.
    #[test]
    fn format_compact_summary_rejects_a_response_cut_off_inside_the_analysis() {
        assert_eq!(format_compact_summary("<analysis>ran out of tokens"), "");
    }

    /// A tail opening on a tool message loses the `tool_calls` that produced it, which
    /// every provider rejects.
    #[test]
    fn plan_tail_start_never_opens_the_tail_on_a_tool_result() {
        let mut messages = vec![message("system", "sys"), message("user", "hi")];
        for _ in 0..6 {
            messages.push(OpenAIMessage {
                role: "assistant".to_string(),
                tool_calls: Some(vec![]),
                ..Default::default()
            });
            messages.push(message("tool", &"x".repeat(40000)));
        }

        let tail_start = plan_tail_start(&messages, 32000).expect("should compact");
        assert_ne!(messages[tail_start].role, "tool");
        assert!(tail_start >= 1, "the system message is never summarized");
    }

    /// A single oversized tool result can fill the window on its own; measuring only
    /// the last request would leave the trigger below threshold until the next request
    /// has already been refused.
    #[test]
    fn projected_prompt_tokens_counts_what_arrived_after_the_last_request() {
        let messages = vec![
            message("system", "sys"),
            message("user", "hi"),
            message("assistant", "calling a tool"),
            message("tool", &"x".repeat(40000)),
        ];
        let last_request = LastRequest { prompt_tokens: Some(500), message_count: 2 };

        assert_eq!(
            projected_prompt_tokens(&messages, last_request),
            500 + 10003
        );
    }

    /// The reserve has to leave the tail room to grow on a small window, and the
    /// summarization request asks for exactly this number: a larger cap lets a summary
    /// land the conversation back over the trigger and compact its own summary next
    /// response.
    #[test]
    fn summary_reserve_scales_with_the_window() {
        assert_eq!(
            summary_reserve_tokens(128000),
            SUMMARY_OUTPUT_RESERVE_TOKENS
        );
        assert_eq!(summary_reserve_tokens(20000), 2000);
        assert_eq!(summary_reserve_tokens(8000), 800);
    }

    #[test]
    fn plan_tail_start_declines_when_the_prefix_is_short() {
        let messages = vec![
            message("system", "sys"),
            message("user", "hi"),
            message("assistant", "hello"),
        ];
        assert_eq!(plan_tail_start(&messages, 1000), None);
    }
}
