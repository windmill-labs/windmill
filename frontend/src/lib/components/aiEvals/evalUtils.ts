import type {
	EvalCase,
	EvalCaseInput,
	EvalDataset,
	EvalExperiment,
	NewEvalCase,
	Scorer
} from '$lib/gen'

/** The case being edited in the drawer, before it is either run or saved to a dataset. */
export type CaseDraft = NewEvalCase & { id?: string }

/** A level the evals pane is on, and the way out of it. */
export type EvalsLocation = { label: string; back: () => void }

export type ScorerKind = Scorer['kind']

export function emptyCase(): CaseDraft {
	return { input: { user_message: '' } }
}

export function fromStoredCase(c: EvalCase): CaseDraft {
	const { created_at: _created_at, created_by: _created_by, ...rest } = c
	return rest
}

export function caseLabel(c: { input?: EvalCaseInput }): string {
	const message = c.input?.user_message?.trim()
	if (message) return message.length > 60 ? message.slice(0, 60) + '…' : message
	return 'Untitled case'
}

export function experimentName(experiment: EvalExperiment): string {
	return `Run ${experiment.run_number}`
}

/**
 * What ran: a deployed version, or a version with edits sitting on top of it.
 *
 * The list and the results endpoint restamp a draft run whose configuration was later deployed, so
 * the kind is usually enough; `deployedHash` and `currentVersion` resolve the one still unstamped.
 */
export function subjectLabel(
	experiment: EvalExperiment,
	deployedHash?: string,
	currentVersion?: number
): string {
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
	return experiment.subject.version ? `v${experiment.subject.version} + edits` : 'edits'
}

/** A scorer keeps its id when renamed, so its name is the column header and nothing else. */
export function scorerLabel(scorer: Scorer): string {
	return scorer.name || scorer.path.split('/').pop() || scorer.path
}

export function kindLabel(kind: ScorerKind): string {
	return kind === 'agent' ? 'Judge agent' : 'Script'
}

export function formatScore(score: number | undefined): string {
	return score == undefined ? '—' : score.toFixed(2)
}

export function formatDelta(delta: number): string {
	if (delta === 0) return '0.00'
	return `${delta > 0 ? '+' : '−'}${Math.abs(delta).toFixed(2)}`
}

/** What a dataset is for, where it says so: the path names it either way. */
export function datasetSummary(datasets: EvalDataset[], path: unknown): string | undefined {
	return datasets.find((d) => d.path === path)?.summary || undefined
}

/**
 * A pass threshold, as a field holds it. Empty is `''` or null, never a number: a number input
 * coerces the text, so a valid threshold of 0 would otherwise read as empty and be dropped. The
 * server refuses anything outside 0 to 1, caught here so the form blocks instead of the save.
 */
export function parseThreshold(text: string | number | null | undefined): {
	value?: number
	error: boolean
} {
	const trimmed = typeof text === 'string' ? text.trim() : text
	if (trimmed === '' || trimmed == undefined) return { error: false }
	const value = Number(trimmed)
	if (Number.isNaN(value) || value < 0 || value > 1) return { error: true }
	return { value, error: false }
}

export function summaryToName(summary: string): string {
	return summary
		.toLowerCase()
		.replace(/[^a-z0-9_]/g, '_')
		.replace(/_+/g, '_')
		.replace(/^_|_$/g, '')
}
