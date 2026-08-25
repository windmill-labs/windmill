// Apart from the artifact size limit, import-free on purpose: shared.ts reads the gate's two
// refusals at module scope, so this has to stay outside the graph its shallow-import rule
// guards. artifactLimits imports nothing at all, for the same reason.
import { MAX_ARTIFACT_BYTES } from './artifacts/artifactLimits'

/** Plan mode's prose, in one place. The user-facing and model-facing strings for one event
 * are deliberately distinct: one is an instruction, the other a refusal. */
export const PLAN_MODE_MESSAGES = {
	blockedLabel: 'Blocked in plan mode',
	blockedResult:
		'Blocked: plan mode is active. Put this change in your plan; call exit_plan_mode when ready for approval.',
	/** Sits beside the autonomy picker while plan mode holds. The picker's tooltip carries
	 * the rest, so this states only the constraint. */
	modeNote: 'Read-only',
	// One pair for both artifact tools: the fact and the way forward are the same whether the
	// model tried to mint the plan or to rewrite it, and the generic refusal above ("put this
	// change in your plan") reads as nonsense for a call that writes a document.
	planWriteRefusedLabel: 'Plan document is read-only while planning',
	planWriteRefused:
		'The session plan document is not writable in plan mode — exit_plan_mode is what writes it. ' +
		'Keep researching, then hand the finished plan over with exit_plan_mode. Other artifacts ' +
		'(diagrams, design sketches, comparisons) are yours to create and revise here.',
	entered: 'Plan mode active.',
	approvedWithDoc: 'Plan approved and saved as a document. You may now execute it.',
	persistenceFailed:
		'Plan approval could not be saved. Plan mode remains active. Retry after artifact persistence is available.',
	enterPrompt:
		'Switch to plan mode? The assistant will research and draft a plan for your approval before changing anything.',
	exitPrompt: 'Ready to execute this plan?',
	enterDeclined:
		'The user declined plan mode. Continue with the task directly, requesting confirmation on changes as usual.',
	// Each pairs the row the user reads with the steer the model needs, which is far too
	// long to be that row.
	missingSummaryLabel: 'No plan to approve',
	missingSummary:
		'exit_plan_mode needs a non-empty `summary` holding the full plan — there is nothing to approve without it. Call it again with the plan as `summary`.',
	endedLabel: 'Plan already handed over',
	ended:
		"Plan mode has already ended — this hand-over was made and the posture is back to the user's own. There is nothing left to approve, so this call was refused rather than stamping the user's approval on a plan they never saw. To change the plan document now, rewrite it with update_artifact.",
	oversizedPlanLabel: 'Plan too large to save',
	// How far over decides whether the model trims or rewrites, and it cannot count bytes.
	oversizedPlan: (bytes: number) =>
		`The plan is too large to save (${bytes} bytes, limit ${MAX_ARTIFACT_BYTES}) and was not shown to the user. Cut it to the decisions and the steps — name the files you will touch instead of quoting them — and call exit_plan_mode again.`,
	// What the version picker shows for a revision the model did not label.
	revisionNote: 'Revised the plan',
	// States the outcome, not a decision: pressing Stop and moving the autonomy picker out of
	// plan mode both land here too, and this text persists in the transcript — asserting a
	// rejection would open the next turn interrogating a user who never made one.
	exitDeclined:
		'This plan was not approved. Stop here and hand the turn back to them: do not execute it, do not re-propose it, ' +
		'and do not start another round of research. Ask in one or two sentences what they want changed, and wait for their ' +
		'answer — they may have turned the plan down, interrupted you, or simply left plan mode, so do not assume which. ' +
		'Once you understand what they want, revise and propose again.'
} as const
