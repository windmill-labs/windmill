import type { ChatCompletionSystemMessageParam } from 'openai/resources/chat/completions.mjs'

const ESCALATE_AFTER_BLOCKS = 3

const PLAN_MODE_INSTRUCTIONS = `# Plan mode active

You are in **plan mode**: a read-only research posture. You MUST NOT modify anything yet.

- Investigate freely with read-only tools (search, read, inspect, lint, list).
- Any tool that writes, edits, deletes, deploys, or runs code is blocked and returns an error until the plan is approved. Do not retry blocked tools.
- When you understand the task, call \`exit_plan_mode\` with the full plan as the \`summary\` (concise, well-structured markdown). The plan is shown to the user for approval there, so do not also repeat it in your message text — a one-line lead-in is enough. Only on approval are mutating tools unblocked.
- Do not call \`exit_plan_mode\` to ask a question or when the plan is still incomplete.`

const PLAN_MODE_ESCALATION = `\n\nSTOP retrying tools — they will stay blocked. Finalize your plan now and call \`exit_plan_mode\`.`

export function appendPlanModeInstructions(
	base: ChatCompletionSystemMessageParam,
	blocksThisTurn: number
): ChatCompletionSystemMessageParam {
	if (typeof base.content !== 'string') return base
	const block =
		blocksThisTurn >= ESCALATE_AFTER_BLOCKS
			? PLAN_MODE_INSTRUCTIONS + PLAN_MODE_ESCALATION
			: PLAN_MODE_INSTRUCTIONS
	return { ...base, content: `${base.content}\n\n${block}` }
}
