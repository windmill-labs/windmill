import type { EvalExperiment } from '$lib/gen'

/** A level the evals pane is on, and the way out of it. The pane knows where it is; the surface
 *  around it decides where that is shown. */
export type EvalsLocation = { label: string; back: () => void }

/** An experiment is called by the run it is, which is short enough to say in a row or a menu. */
export function experimentName(experiment: EvalExperiment): string {
	return `Run ${experiment.run_number}`
}

/**
 * What ran: a deployed version, or a version with edits sitting on top of it.
 *
 * A draft run whose configuration was later deployed is a run of that version. The list and the
 * results endpoint both restamp such a run, so the kind is usually enough; `deployedHash` and
 * `currentVersion` resolve the one that has not been restamped yet, which is what the table has
 * on hand.
 */
export function subjectLabel(
	experiment: EvalExperiment,
	deployedHash?: string,
	currentVersion?: number
): string {
	// A pinned run names the version it inlined, which is the whole of what it was asked to run.
	if (experiment.subject.kind === 'agent_version') {
		return experiment.subject.version ? `v${experiment.subject.version}` : 'a past version'
	}
	const deployed =
		experiment.subject.kind === 'agent' ||
		(experiment.subject.draft_hash != undefined && experiment.subject.draft_hash === deployedHash)
	if (deployed) {
		const version =
			experiment.subject.kind === 'agent' ? experiment.subject.version : currentVersion
		return version ? `v${version}` : 'deployed'
	}
	// Version first, as every other badge reads, and "+ draft" for what is sitting on it: the
	// word is what the rest of Windmill calls unsaved edits, and the plus is what says they are on
	// top of that version rather than a version of their own.
	return experiment.subject.version ? `v${experiment.subject.version} + draft` : 'draft'
}
