import type { ChatCompletionSystemMessageParam } from 'openai/resources/chat/completions.mjs'

const ESCALATE_AFTER_BLOCKS = 3

const PLAN_MODE_INSTRUCTIONS = `# Plan mode active

You are in **plan mode**: a read-only research posture. You MUST NOT modify anything yet.

- Investigate freely with read-only tools (search, read, inspect, lint, list).
- Any tool that writes, edits, deletes, deploys, or runs code is blocked and returns an error until the plan is approved. Do not retry blocked tools.
- When you understand the task, call \`exit_plan_mode\` with the full plan as the \`summary\` (concise, well-structured markdown). The plan is shown to the user for approval there, so do not also repeat it in your message text — a one-line lead-in is enough. Only on approval are mutating tools unblocked.
- Your \`summary\` is saved as a markdown document and opened in the session preview for the user to read, so write it as a complete, self-contained plan. Do not call \`create_artifact\` for the plan — \`exit_plan_mode\` persists it for you.
- Do not call \`exit_plan_mode\` to ask a question or when the plan is still incomplete.`

const PLAN_MODE_ESCALATION = `\n\nSTOP retrying tools — they will stay blocked. Finalize your plan now and call \`exit_plan_mode\`.`

export const ENTER_PLAN_MODE_TOOL_DESCRIPTION = `Call this before starting a non-trivial change to research first and get the user's sign-off on your approach. Prefer it when the task adds meaningful new functionality, has several valid approaches, requires an architectural decision, will touch more than a couple of files, or is unclear enough that you need to explore before you understand the scope. Do NOT use it for small, well-specified edits (a typo, one obvious bug, a single function with clear requirements) or pure questions. On approval you enter a read-only posture; investigate, then call \`exit_plan_mode\` with your plan.`

export function derivePlanTitle(summary: string): string {
	// Unfenced, a `# comment` inside a snippet would win over the plan's real heading.
	const heading = summary
		.replace(/^(`{3,}|~{3,})[\s\S]*?^\1/gm, '')
		.match(/^#{1,3}[ \t]+(.+)$/m)?.[1]
		?.trim()
	return heading || 'Implementation plan'
}

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
