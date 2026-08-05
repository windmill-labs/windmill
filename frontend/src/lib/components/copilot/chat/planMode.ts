import { z } from 'zod'
import type { ChatCompletionSystemMessageParam } from 'openai/resources/chat/completions.mjs'

const ESCALATE_AFTER_BLOCKS = 3

const PLAN_MODE_INSTRUCTIONS = `# Plan mode active

You are in **plan mode**: a read-only research posture. You MUST NOT modify anything yet.

- Investigate freely with read-only tools (search, read, inspect, lint, list).
- Any tool that writes, edits, deletes, deploys, or runs code is blocked and returns an error until the plan is approved. Do not retry blocked tools.
- When you understand the task, call \`exit_plan_mode\` with the full plan as the \`summary\` (concise, well-structured markdown). The plan is shown to the user for approval there, so do not also repeat it in your message text — a one-line lead-in is enough. Only on approval are mutating tools unblocked.
- Your \`summary\` is saved as a markdown document and opened in the session preview for the user to read, so write it as a complete, self-contained plan. Do not call \`create_artifact\` for the plan — \`exit_plan_mode\` persists it for you.
- A plan document may already exist for this conversation, and your \`summary\` **replaces it wholesale** — submitting only the new part erases the rest. So before planning an extension, a follow-up, or an amendment, call \`list_artifacts\`, find the entry whose \`role\` is \`plan\`, \`read_artifact\` it, and make your \`summary\` that plan with the new work folded in — marking what is already delivered as done. Start from a blank plan only when the user asks you to replace it.
- Open the \`summary\` with a markdown heading naming the change itself ("Add a daily cleanup schedule"). The document is titled from it and is already labelled as a plan, so do not prefix it with "Plan:".
- Do not call \`exit_plan_mode\` to ask a question or when the plan is still incomplete.`

const PLAN_MODE_ESCALATION = `\n\nSTOP retrying tools — they will stay blocked. Finalize your plan now and call \`exit_plan_mode\`.`

/** The tint every plan-mode surface shares. */
const PLAN_MODE_TINT = 'bg-teal-600/10 dark:bg-teal-500/10'

/** Plan mode's colour, on every surface that signals it: the autonomy pill, the composer
 * placeholder, a blocked tool call, and the plan's row in the artifact list. Teal, not the
 * house green: green is already the transcript's success colour — the check on a settled
 * tool call and the accept button sit a few rows above these surfaces — so a mode signal in
 * it reads as "this worked" rather than "this is held". */
export const PLAN_MODE_TEXT_COLOR = 'text-teal-600 dark:text-teal-500'

/** The `plan` pill, in the artifact list and on the preview header. */
export const PLAN_MODE_BADGE_CLASS = `${PLAN_MODE_TINT} ${PLAN_MODE_TEXT_COLOR}`

/** The autonomy pill: the shared tint plus the border and hover a button needs. Plan mode is
 * the only posture that refuses work, so it is the only one colouring the whole trigger
 * rather than just its icon — the user has to be able to tell from the composer why an edit
 * request went nowhere. `!` beats the Button variant's own colours. */
export const PLAN_MODE_TRIGGER_CLASS = `${PLAN_MODE_TINT} !border-teal-600/40 hover:bg-teal-600/[0.15] !text-teal-600 dark:!border-teal-500/40 dark:hover:bg-teal-500/[0.15] dark:!text-teal-500`

export const enterPlanModeArgs = z.object({
	reason: z
		.string()
		.describe(
			'One concise sentence on what you want to research/plan and why, shown to the user when asking to enter plan mode.'
		)
})

export const exitPlanModeArgs = z.object({
	summary: z
		.string()
		.min(1)
		.describe(
			'The plan to execute, as concise well-structured markdown. Shown verbatim to the user for approval.'
		)
})

export const ENTER_PLAN_MODE_TOOL_DESCRIPTION = `Call this before starting a non-trivial change to research first and get the user's sign-off on your approach. Prefer it when the task adds meaningful new functionality, has several valid approaches, requires an architectural decision, will touch more than a couple of files, or is unclear enough that you need to explore before you understand the scope. Do NOT use it for small, well-specified edits (a typo, one obvious bug, a single function with clear requirements) or pure questions. On approval you enter a read-only posture; investigate, then call \`exit_plan_mode\` with your plan.`

export const EXIT_PLAN_MODE_TOOL_DESCRIPTION = `Call once your plan is ready and you want to start executing it. Shows the plan to the user for approval; only on approval are mutating tools unblocked. Do not call it to ask a question — use it only to hand over a complete plan.`

/** Plan mode's prose, in one place so the gate, the cards, the composer and the tool
 * results stay consistent. The user-facing and model-facing strings for one event are
 * deliberately distinct: the model needs the instruction, the user needs to know the
 * product refused and why. */
export const PLAN_MODE_MESSAGES = {
	blockedLabel: 'Blocked in plan mode',
	blockedResult:
		'Blocked: plan mode is active. Put this change in your plan; call exit_plan_mode when ready for approval.',
	composerHint: ' — read-only, it will propose a plan first',
	entered: 'Plan mode active.',
	approved: 'Plan approved. You may now execute it.',
	approvedWithDoc: 'Plan approved and saved as a document. You may now execute it.',
	enterPrompt:
		'Switch to plan mode? The assistant will research and draft a plan for your approval before changing anything.',
	exitPrompt: 'Ready to execute this plan?',
	enterDeclined:
		'The user declined plan mode. Continue with the task directly, requesting confirmation on changes as usual.',
	missingSummary:
		'exit_plan_mode needs a non-empty `summary` holding the full plan — there is nothing to approve without it. Call it again with the plan as `summary`.',
	exitDeclined:
		'The user rejected this plan. Stop here and hand the turn back to them: do not execute it, do not re-propose it, ' +
		'and do not start another round of research. Reply in one or two sentences asking what is wrong, missing, or unwanted in it, ' +
		'and wait for their answer. Keep discussing until you understand the objection, then revise and propose again.'
} as const

/** The two plan tools' names. Exported because the autonomy picker resolves their pending
 * cards by name, far from the `createToolDef` calls that declare them — a rename that misses
 * one of those sites would go silently inert rather than failing. */
export const ENTER_PLAN_MODE_TOOL = 'enter_plan_mode'
export const EXIT_PLAN_MODE_TOOL = 'exit_plan_mode'

/** What each plan card says, keyed by the tool call it renders. `settled` / `declined` /
 * `pending` are the three states a card can be read in. `declined` is not only the reject
 * button: switching posture out of plan mode resolves the card, and a call refused before
 * any card was offered renders here too, so it must not name a decision the user made. */
export const PLAN_CARD_COPY = {
	[ENTER_PLAN_MODE_TOOL]: {
		settled: 'Planning started',
		declined: 'Continuing without planning',
		pending: 'Start planning?',
		reject: 'Not now',
		confirm: 'Start planning'
	},
	[EXIT_PLAN_MODE_TOOL]: {
		settled: 'Plan approved',
		declined: 'Plan not approved',
		pending: 'Proposed plan',
		reject: 'Keep planning',
		confirm: 'Approve and implement'
	}
} as const

export type PlanCardTool = keyof typeof PLAN_CARD_COPY

export function isPlanCardTool(name: string | undefined): name is PlanCardTool {
	// hasOwn, not `in`: tool names come from the model, and `in` would accept
	// `toString` or `__proto__` and render an unknown call as a plan card.
	return name !== undefined && Object.hasOwn(PLAN_CARD_COPY, name)
}

/** The model-facing refusal for an `exit_plan_mode` with no usable plan, or undefined
 * when the call is fine. Plan mode is a safety posture, so an unusable call must be
 * refused before a card offers to approve it — not swallowed into a blank approval. */
export function exitPlanModeRejection(args: unknown): string | undefined {
	const summary = exitPlanModeArgs.safeParse(args).data?.summary
	return summary?.trim() ? undefined : PLAN_MODE_MESSAGES.missingSummary
}

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
