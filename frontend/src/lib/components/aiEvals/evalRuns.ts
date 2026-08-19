import type { EvalExperiment } from '$lib/gen'

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
	return experiment.subject.version ? `v${experiment.subject.version} + edits` : 'unsaved edits'
}
