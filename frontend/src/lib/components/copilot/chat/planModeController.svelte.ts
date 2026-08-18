import type { ChatCompletionSystemMessageParam } from 'openai/resources/chat/completions.mjs'
import type { ArtifactVersionTarget } from '$lib/components/sessions/previewRouter'
import { createToolDef, type Tool, type ToolCallbacks } from './shared'
import {
	appendPlanModeInstructions,
	derivePlanTitle,
	enterPlanModeArgs,
	exitPlanModeArgs,
	exitPlanModeRejection,
	planChangeNoteOf,
	planReasonOf,
	planSummaryOf,
	ENTER_PLAN_MODE_TOOL,
	ENTER_PLAN_MODE_TOOL_DESCRIPTION,
	EXIT_PLAN_MODE_TOOL,
	EXIT_PLAN_MODE_TOOL_DESCRIPTION
} from './planMode'
import { PLAN_MODE_MESSAGES } from './planModeMessages'
import { normalizeChangeNote } from './artifacts/artifactLimits'
import { currentVersion } from './artifacts/artifactsDB'
import { type SessionArtifactsStore } from './artifacts/artifactsState.svelte'

/** What plan mode needs from the chat it runs in: it reads the autonomy state and asks for
 * the two changes it can cause, rather than owning any of it. */
export interface PlanModeHost {
	/** Offered in this chat *and* selected. */
	readonly active: boolean
	/** Offered in this chat at all. */
	readonly available: boolean
	/** Auto-accepting confirmations, so there is no one to ask before entering. */
	readonly autoAccepting: boolean
	readonly isSessionChat: boolean
	readonly sessionId: string | undefined
	readonly chatId: string | undefined
	readonly artifacts: SessionArtifactsStore
	openArtifact(id: string, name: string, version: ArtifactVersionTarget): void
	enter(): void
	/** Hand the posture back to whatever preceded plan mode. */
	restore(): void
}

/** Read off the write that proposed the plan, so nothing re-reads and races a later one. */
type PlanSaveResult = { id: string; name: string; version: number }

/** The failure carries the model's message, so a lost slot and an unwritable store stay
 * distinguishable when `fn` reports them after the confirmation. */
type PlanSaveOutcome = { plan: PlanSaveResult } | { error: string }

/**
 * A round runs from entering plan mode to the proposal the user decides on. Nothing in it is
 * undone — a refused proposal stands as the newest version — so it only has to remember the
 * write it made, for an approval landing after the chat has moved on.
 */
export class PlanModeController {
	#host: PlanModeHost
	/** Keyed by tool call so a card's confirmation hook and the tool's `fn` share one write. */
	#save: { toolId: string; doc: Promise<PlanSaveOutcome> } | undefined
	/** Bumped only on *entering*, so it names the round, not the conversation: a chat rotation
	 * mid-approval still hands the posture back, a re-entered round must not. */
	#epoch = 0
	/** Drives the prompt's escalation once the model keeps retrying blocked tools. */
	blocksThisTurn = $state(0)

	constructor(host: PlanModeHost) {
		this.#host = host
	}

	/** The confirmation hook deliberately does not block on the write, so this is how a caller
	 * waits for it to settle. */
	get pendingSave(): Promise<PlanSaveOutcome> | undefined {
		return this.#save?.doc
	}

	/** A new round, whose approval is its own: one still in flight must not end this one. */
	startRound = () => {
		this.#save = undefined
		this.#epoch++
	}

	/** The conversation rotated. The save-dedup belongs to it; the plan document does not —
	 * that one is the session's, and rotating a chat leaves it exactly where it was. */
	resetRound = () => {
		this.#save = undefined
	}

	resetBlocks = () => {
		this.blocksThisTurn = 0
	}

	noteBlockedTool = () => {
		this.blocksThisTurn++
	}

	/** They have to leave the prompt on approval, or the model is still told it may not build
	 * while the gate has already opened. */
	decorateSystemMessage = (
		base: ChatCompletionSystemMessageParam
	): ChatCompletionSystemMessageParam =>
		this.#host.active ? appendPlanModeInstructions(base, this.blocksThisTurn) : base

	/** Only the transition the current posture allows; auto-accepting exposes neither, since
	 * entering is the user's choice. */
	get tools(): Tool<any>[] {
		if (!this.#host.available) return []
		if (this.#host.autoAccepting) return []
		return this.#host.active ? [this.exitTool] : [this.enterTool]
	}

	// This safety tag is what keeps plan mode escapable through its handoff tool.
	exitTool: Tool<any> = {
		def: createToolDef(exitPlanModeArgs, EXIT_PLAN_MODE_TOOL, EXIT_PLAN_MODE_TOOL_DESCRIPTION),
		planModeSafe: true,
		requiresConfirmation: true,
		// A batch's tool list is snapshotted before its calls are run, so a second hand-over in
		// one response still finds this tool after the first restored the posture. Refused here
		// rather than in `fn`, because `onConfirmationRequested` writes the document too — and
		// under YOLO nothing asks: the tool would confer the user's approval on a plan no card
		// ever showed them.
		validateBeforeConfirmation: ({ args }) =>
			this.#host.active
				? exitPlanModeRejection(args)
				: { label: PLAN_MODE_MESSAGES.endedLabel, result: PLAN_MODE_MESSAGES.ended },
		confirmationMessage: (args) => planSummaryOf(args) ?? PLAN_MODE_MESSAGES.exitPrompt,
		cancellationMessage: PLAN_MODE_MESSAGES.exitDeclined,
		showDetails: true,
		onConfirmationRequested: (p) => {
			void this.#ensurePlanDoc(p)
		},
		fn: async ({ args, toolCallbacks, toolId }) => {
			const save = this.#ensurePlanDoc({ args, toolCallbacks, toolId })
			// Captured before the await: the approval belongs to the round that proposed this
			// plan, and the user can leave and re-enter plan mode while the write is in flight.
			const epoch = this.#epoch
			const saved = await save
			// Reporting the failure is `fn`'s to do and no earlier: the write settles while the
			// card is still waiting to be confirmed, and clearing that card from underneath the
			// wait would take away the only control left that resolves it.
			if ('error' in saved) {
				toolCallbacks.setToolStatus(toolId, {
					content: 'Plan was not saved',
					error: saved.error
				})
				return saved.error
			}
			if (!(await this.#markApproved(saved.plan.id, saved.plan.version))) {
				toolCallbacks.setToolStatus(toolId, {
					content: 'Plan approval was not saved',
					error: PLAN_MODE_MESSAGES.persistenceFailed
				})
				return PLAN_MODE_MESSAGES.persistenceFailed
			}
			// Only to the round this approval belongs to: ending a re-entered round would drop the
			// user out of a planning posture they just chose.
			if (this.#host.active && epoch === this.#epoch) this.#host.restore()
			return PLAN_MODE_MESSAGES.approvedWithDoc
		}
	}

	enterTool: Tool<any> = {
		def: createToolDef(enterPlanModeArgs, ENTER_PLAN_MODE_TOOL, ENTER_PLAN_MODE_TOOL_DESCRIPTION),
		planModeSafe: true,
		requiresConfirmation: true,
		confirmationMessage: (args) => planReasonOf(args) ?? PLAN_MODE_MESSAGES.enterPrompt,
		cancellationMessage: PLAN_MODE_MESSAGES.enterDeclined,
		showDetails: true,
		fn: async () => {
			this.#host.enter()
			return PLAN_MODE_MESSAGES.entered
		}
	}

	// A round spans the whole posture, so re-proposing revises the same document.
	#ensurePlanDoc = (p: { args: any; toolCallbacks: ToolCallbacks; toolId: string }) => {
		if (this.#save?.toolId !== p.toolId) {
			this.#save = { toolId: p.toolId, doc: this.#savePlanDoc(p) }
		}
		return this.#save.doc
	}

	#savePlanDoc = async (p: {
		args: any
		toolCallbacks: ToolCallbacks
		toolId: string
	}): Promise<PlanSaveOutcome> => {
		const host = this.#host
		// Read once, before any await: every write below is about the session that proposed the
		// plan, not whichever one the getter would answer with by the time they land.
		const sessionId = host.sessionId
		const chatId = host.chatId
		const unsaved = { error: PLAN_MODE_MESSAGES.persistenceFailed }
		if (!host.isSessionChat || !sessionId) return unsaved
		const summary = planSummaryOf(p.args)
		if (!summary) return unsaved
		try {
			// No approval field: the write bumps the version and leaves `approvedVersion` where
			// it was, which is what makes the new text read as an undecided proposal. The note
			// keeps the picker a sequence of decisions rather than a stack of dates.
			const plan = await host.artifacts.savePlan(
				sessionId,
				{
					name: derivePlanTitle(summary),
					content: summary,
					note: normalizeChangeNote(planChangeNoteOf(p.args)) ?? PLAN_MODE_MESSAGES.revisionNote
				},
				chatId
			)
			const version = currentVersion(plan)
			// The write stands either way; only what the user would *see* is held back, so a
			// session swapped in mid-save gets neither another session's plan nor a dead card.
			if (sessionId === host.sessionId) {
				// 'latest', not the version just written: this plan is being put up for approval,
				// so it has to be readable — and pinning it would strand the reader there when
				// the next round revises the document.
				host.openArtifact(plan.id, plan.name, 'latest')
				p.toolCallbacks.setToolStatus(p.toolId, { planArtifactId: plan.id, planVersion: version })
			}
			return { plan: { id: plan.id, name: plan.name, version } }
		} catch (e) {
			console.error('Failed to persist plan artifact', e)
			return unsaved
		}
	}

	// Lands on the document rather than being inferred from a transcript read long after the
	// card scrolled away. Unguarded: the approval holds whichever chat is current.
	#markApproved = async (id: string, version: number): Promise<boolean> => {
		try {
			return await this.#host.artifacts.approve(id, version)
		} catch (e) {
			console.error('Failed to mark plan artifact approved', e)
			return false
		}
	}
}
