// Summary-based compaction: when a conversation approaches the model's context
// window, the older prefix is replaced by an LLM-generated structured summary
// while the recent tail is kept verbatim. The summary precedes the kept tail,
// so it is written "up to" the point of compaction — newer messages the model
// does not see here will follow it.
//
// Prompt structure and the analysis/summary split are adapted from the
// reference compaction prompt used by coding agents.

// Reinforce text-only output. The summary call is issued without tools, but a
// strong instruction keeps weaker models from narrating a tool call instead of
// producing the summary.
const NO_TOOLS_PREAMBLE = `CRITICAL: Respond with TEXT ONLY. Do NOT call any tools.

- You already have all the context you need in the conversation above.
- Your entire response must be plain text: an <analysis> block followed by a <summary> block.

`

const NO_TOOLS_TRAILER =
	'\n\nREMINDER: Respond with plain text only — an <analysis> block followed by a <summary> block.'

// The <analysis> block is a drafting scratchpad that formatCompactSummary()
// strips before the summary reaches context.
const SUMMARY_PROMPT = `Your task is to create a detailed summary of the conversation so far. This summary will be placed at the start of a continuing session; newer messages that build on this context will follow after it (you do not see them here). Summarize thoroughly so that someone reading only your summary and then the newer messages can fully understand what happened and continue the work without losing context.

This is a conversation with Windmill's global workspace assistant. It inspects workspace items and authors them as per-user drafts — scripts, flows, apps, resources, variables, triggers, and schedules — then deploys those drafts and test-runs scripts and flows. It works with items by their workspace path (e.g. \`u/alice/sync_orders\`, \`f/team/my_flow\`); it does NOT edit files on a filesystem. Frame the summary in those terms.

Before providing your final summary, wrap your analysis in <analysis> tags to organize your thoughts. In your analysis:

1. Chronologically analyze each message and section of the conversation. For each section thoroughly identify:
   - The user's explicit requests and intents
   - Your approach to addressing the user's requests
   - Key decisions, technical concepts and code patterns
   - Specific details: workspace item paths and kinds (script / flow / app / resource / variable / trigger / schedule), code snippets for runnables, and the exact actions taken (drafts created or updated, items deployed, test runs and their results)
   - Errors you ran into and how you fixed them
   - Specific user feedback, especially if the user told you to do something differently
2. Double-check for technical accuracy and completeness.

Your summary should include the following sections:

1. Primary Request and Intent: Capture all of the user's explicit requests and intents in detail.
2. Key Technical Concepts: List important technical concepts, technologies, and frameworks discussed.
3. Workspace Items and Code: Enumerate the workspace items inspected, drafted, updated, deployed, or test-run — each by its path and kind (script / flow / app / resource / variable / trigger / schedule) — noting what was done and why. Include full code snippets for runnables (script code, flow inline scripts, app inline runnables) wherever code was written or changed.
4. Errors and fixes: List errors encountered and how they were fixed, including any user feedback.
5. Problem Solving: Document problems solved and any ongoing troubleshooting efforts.
6. All user messages: List ALL user messages that are not tool results. These are critical for understanding the user's feedback and changing intent.
7. Pending Tasks: Outline any pending tasks you have explicitly been asked to work on.
8. Current Work: Describe precisely what was being worked on immediately before this summary, paying special attention to the most recent messages. Include item paths and code snippets where applicable.
9. Context for Continuing Work: Summarize any context, decisions, or state needed to understand and continue the work in subsequent messages — including which items are still drafts versus deployed and the exact paths involved. If there is a clear next step directly in line with the user's most recent explicit request, state it and include a direct quote from the most recent conversation showing where you left off.

Structure your output like this:

<analysis>
[Your thought process, ensuring all points are covered thoroughly and accurately]
</analysis>

<summary>
1. Primary Request and Intent:
   [Detailed description]

2. Key Technical Concepts:
   - [Concept]

3. Workspace Items and Code:
   - [path + kind, e.g. u/alice/sync_orders (script)]
      - [What was done: read / draft created / draft updated / deployed / test-run result]
      - [Why it matters]
      - [Code snippet, for runnables]

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

Please provide your summary following this structure, ensuring precision and thoroughness.`

/** Prompt sent as the final user message of the summarization request. */
export function getCompactionSummaryPrompt(): string {
	return NO_TOOLS_PREAMBLE + SUMMARY_PROMPT + NO_TOOLS_TRAILER
}

/**
 * Strips the <analysis> drafting scratchpad and unwraps the <summary> block.
 * Falls back to the trimmed raw text when the model didn't use the tags, so a
 * well-formed-but-untagged summary is still usable.
 */
export function formatCompactSummary(raw: string): string {
	// Strip the analysis scratchpad first: it precedes the summary and may itself
	// mention <summary>/<analysis> tokens that would otherwise be mistaken for the
	// real summary boundary.
	let formatted = raw.replace(/<analysis>[\s\S]*?<\/analysis>/gi, '')

	const summaryMatch = formatted.match(/<summary>([\s\S]*?)<\/summary>/i)
	if (summaryMatch) {
		formatted = (summaryMatch[1] ?? '').trim()
	} else {
		// A truncated response or a weaker model sometimes opens <summary> without
		// closing it. The text after the opener is still the summary, so keep it
		// rather than leak the bare tag.
		const openIdx = formatted.search(/<summary>/i)
		if (openIdx !== -1) {
			formatted = formatted.slice(openIdx)
		}
	}

	// An orphaned opener or closer left by either branch must never reach the user.
	formatted = formatted.replace(/<\/?(?:analysis|summary)>/gi, '')

	// Collapse the blank-line runs left behind by stripping the analysis block.
	return formatted.replace(/\n{3,}/g, '\n\n').trim()
}

/**
 * Wraps a formatted summary as the content of the user message that replaces
 * the summarized prefix in the conversation.
 */
export function buildSummaryMessageContent(formattedSummary: string): string {
	return `This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation. Recent messages after the summary are preserved verbatim.

${formattedSummary}

Continue the conversation from where it left off. Do not re-introduce the summary or recap it to the user; pick up the work as if the break never happened.`
}
