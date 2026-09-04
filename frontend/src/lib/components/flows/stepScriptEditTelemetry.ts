import { logFeatureUsage } from '$lib/utils/featureUsage'

// Anonymous counters for editing a workspace script's code without leaving the flow editor.
// Same rules as every other `logFeatureUsage` caller: aggregated counts only, and the two keys
// below are the whole vocabulary — no script path, hash, language or code ever reaches here.

export type StepScriptEditEvent =
	/** The drawer was opened on the step's script. */
	| 'opened'
	/** A new version of that script was deployed from the drawer. */
	| 'saved'

export function logStepScriptEdit(event: StepScriptEditEvent): void {
	logFeatureUsage('flow_step', 'script_edit', { key: event })
}
