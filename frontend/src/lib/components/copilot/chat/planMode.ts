import { z } from 'zod'
import type { ChatCompletionSystemMessageParam } from 'openai/resources/chat/completions.mjs'
import { artifactOverflowBytes } from './artifacts/artifactLimits'
import { PLAN_MODE_MESSAGES } from './planModeMessages'
import type { ArtifactVersionTarget } from '$lib/components/sessions/previewRouter'

const ESCALATE_AFTER_BLOCKS = 3

const PLAN_MODE_INSTRUCTIONS = `# Plan mode active

This is a research posture. Inspect freely; changes to the workspace, execution, and deployment stay blocked until approval.

- You may create and revise ordinary session artifacts here — a diagram (\`\`\`mermaid fences render), a design sketch, a comparison of options — for work the plan should point at rather than swallow. The plan document is not one of them: while plan mode is active, \`create_artifact\` with \`role: "plan"\` and \`update_artifact\` on the plan are both refused, whatever the general artifact guidance says about revising a plan. \`exit_plan_mode\` is what writes it.
- When the plan is complete, call \`exit_plan_mode\` with the full, self-contained markdown plan. It persists and opens the document for approval, so do not create a separate plan artifact or repeat it in chat.
- The summary replaces the whole document. For revisions, first find the session's \`role: "plan"\` artifact, read its current text, and merge the new work into the complete replacement.
- If \`approvedVersion\` is behind the current version, the current text is an unapproved draft. Revise that draft; read the numbered approved version only when recovering what the user accepted. Missing \`approvedVersion\` means nothing was approved.
- Do not call \`exit_plan_mode\` for questions or incomplete plans.`

const PLAN_MODE_ESCALATION = `\n\nSTOP retrying tools — they will stay blocked. Finalize your plan now and call \`exit_plan_mode\`.`

/** The tint every plan-mode surface shares. */
const PLAN_MODE_TINT = 'bg-teal-600/10 dark:bg-teal-500/10'

/** Teal, not the house green: green is the transcript's success colour a few rows above,
 * so a mode signal in it would read as "this worked". */
export const PLAN_MODE_TEXT_COLOR = 'text-teal-600 dark:text-teal-500'

/** The `plan` pill, in the artifact list and on the preview header. */
export const PLAN_MODE_BADGE_CLASS = `${PLAN_MODE_TINT} ${PLAN_MODE_TEXT_COLOR}`

/** Renders what `planVersionView` decided. */
export function planBadge(
	state: 'plan' | 'draft' | undefined
): { label: string; class: string } | undefined {
	if (state === undefined) return undefined
	return state === 'plan'
		? { label: 'plan', class: `font-medium ${PLAN_MODE_BADGE_CLASS}` }
		: { label: 'draft', class: 'font-normal bg-surface-secondary text-tertiary' }
}

/**
 * How one version reads, for the pill and the bar above it. Judged against the version the
 * user approved, never the newest: latest is only where the model stopped. So the approved
 * version is never stale, the one in front of it is a draft, and anything behind is history
 * that is neither. The list and the header both ask here so they cannot disagree.
 */
export function planVersionView(
	a: { role?: 'plan'; approvedVersion?: number; version?: number },
	/** Undefined while unpinned, which means the latest. */
	shown: number | undefined
): {
	badge: 'plan' | 'draft' | undefined
	bar: 'approved-with-newer' | 'unapproved-head' | undefined
	/** The version the history bar offers, when the plan is not what is on screen. Undefined
	 * when that is simply the latest, which is reached by clearing the pin rather than by
	 * pinning it — the same rule `planVersionTarget` follows. */
	backToPlan: number | undefined
} {
	const latest = a.version ?? 1
	const at = shown ?? latest
	const isPlan = a.role === 'plan'
	const approvedHere = isPlan && a.approvedVersion === at
	const approvedElsewhere = isPlan && !approvedHere ? a.approvedVersion : undefined
	return {
		badge: !isPlan ? undefined : approvedHere ? 'plan' : at === latest ? 'draft' : undefined,
		bar: approvedHere
			? latest > at
				? 'approved-with-newer'
				: undefined
			: approvedElsewhere !== undefined && at === latest
				? 'unapproved-head'
				: undefined,
		backToPlan: approvedElsewhere === latest ? undefined : approvedElsewhere
	}
}

/**
 * How to open a plan at a particular version — the card's own proposal, or the approved one.
 * Pins it only while it is behind the document: pinning the current version dresses it as
 * history, banner and all, and omitting a version would strand the reader wherever they were.
 */
export function planVersionTarget(
	doc: { version?: number } | undefined,
	wanted: number | undefined
): ArtifactVersionTarget {
	return doc && wanted !== undefined && wanted < (doc.version ?? 1) ? wanted : 'latest'
}

/**
 * What a click in the artifact list should open. Only a plan names a version, because only a
 * plan has one the reader did not pick: for anything else, naming `'latest'` would throw away
 * the version they pinned on that tab, which omitting it is what preserves.
 */
export function listOpenTarget(artifact: {
	role?: 'plan'
	version?: number
	approvedVersion?: number
}): ArtifactVersionTarget | undefined {
	if (artifact.role !== 'plan') return undefined
	return planVersionTarget(artifact, artifact.approvedVersion)
}

/**
 * The pill on a list row. Read at the version `listOpenTarget` opens rather than at the
 * newest, so a row cannot label one version and open another.
 */
export function listBadge(artifact: {
	role?: 'plan'
	version?: number
	approvedVersion?: number
}): 'plan' | 'draft' | undefined {
	const target = listOpenTarget(artifact)
	return planVersionView(artifact, typeof target === 'number' ? target : undefined).badge
}

/** The only posture that refuses work, so the only one colouring the whole trigger: the
 * user has to see from the composer why an edit went nowhere. `!` beats the Button. */
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
		),
	change_note: z
		.string()
		.optional()
		.describe(
			'Only when revising a plan the user has already seen: what changed since the last proposal, as a short label they will read in the version picker — under 60 characters, no trailing period, starting with a verb ("Dropped the migration step", "Split phase 2 in two"). Omit on a first proposal.'
		)
})

/**
 * One argument, never a parse of the whole call: `change_note` is optional and cosmetic, so
 * a model sending it as `null` would fail the object parse and take the plan down with it.
 */
function stringArg(args: unknown, key: 'summary' | 'change_note' | 'reason'): string | undefined {
	const value = (args as Record<string, unknown> | null | undefined)?.[key]
	return typeof value === 'string' ? value : undefined
}

export const planSummaryOf = (args: unknown) => stringArg(args, 'summary')
export const planChangeNoteOf = (args: unknown) => stringArg(args, 'change_note')
export const planReasonOf = (args: unknown) => stringArg(args, 'reason')

export const ENTER_PLAN_MODE_TOOL_DESCRIPTION = `Call this before starting a non-trivial change to research first and get the user's sign-off on your approach. Prefer it when the task adds meaningful new functionality, has several valid approaches, requires an architectural decision, will touch more than a couple of files, or is unclear enough that you need to explore before you understand the scope. Do NOT use it for small, well-specified edits (a typo, one obvious bug, a single function with clear requirements) or pure questions. On approval you enter a research posture where nothing in the workspace changes; investigate, then call \`exit_plan_mode\` with your plan.`

export const EXIT_PLAN_MODE_TOOL_DESCRIPTION = `Call once your plan is ready and you want to start executing it. Shows the plan to the user for approval; only on approval are the workspace-changing tools unblocked. Do not call it to ask a question — use it only to hand over a complete plan. Valid only while plan mode is active: once the plan is approved there is nothing left to approve, and a revision goes into the plan document with \`update_artifact\`.`

/** Exported because the autonomy picker resolves pending cards by name, far from the
 * `createToolDef` calls — a rename missing one site would go silently inert. */
export const ENTER_PLAN_MODE_TOOL = 'enter_plan_mode'
export const EXIT_PLAN_MODE_TOOL = 'exit_plan_mode'

/** `declined` is not only the reject button: a Stop and leaving plan mode both resolve a
 * pending card, so it must name the outcome rather than the button. */
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

/** The six fields of a tool message this decision reads, named rather than imported so the
 * contract is the parameter and a case is one object literal. */
type PlanCardStatus = {
	error?: string
	declinedByUser?: boolean
	needsConfirmation?: boolean
	isLoading?: boolean
	isQueued?: boolean
	isStreamingArguments?: boolean
}

/** Undefined renders as an ordinary tool error. Keyed off the decision, not off each error
 * path, so an error added later cannot read as a plan the user turned down. */
export function planCardState(
	status: PlanCardStatus
): 'settled' | 'declined' | 'pending' | undefined {
	if (status.error) return status.declinedByUser ? 'declined' : undefined
	// isQueued matters: a card waiting its turn has no error and no confirmation pending
	// yet, so without it a queued call reads as already resolved.
	return status.needsConfirmation ||
		status.isLoading ||
		status.isQueued ||
		status.isStreamingArguments
		? 'pending'
		: 'settled'
}

/** An unusable call must be refused before a card offers to approve it, not swallowed
 * into a blank approval. */
export function exitPlanModeRejection(
	args: unknown
): { label: string; result: string } | undefined {
	const summary = planSummaryOf(args)
	if (!summary?.trim()) {
		return {
			label: PLAN_MODE_MESSAGES.missingSummaryLabel,
			result: PLAN_MODE_MESSAGES.missingSummary
		}
	}
	// The plan reaches the store through the save path, never create_artifact, so the cap
	// applies here or not at all — otherwise the card offers a plan the document never got.
	const bytes = artifactOverflowBytes(summary)
	if (bytes !== undefined) {
		return {
			label: PLAN_MODE_MESSAGES.oversizedPlanLabel,
			result: PLAN_MODE_MESSAGES.oversizedPlan(bytes)
		}
	}
	return undefined
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
