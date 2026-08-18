import type { Scorer } from '$lib/gen'

export type ScorerKind = Scorer['kind']

/** The column header. A scorer keeps its id when renamed, so this is display only. */
export function scorerLabel(scorer: Scorer): string {
	return scorer.name || scorer.path.split('/').pop() || scorer.path
}

export function kindLabel(kind: ScorerKind): string {
	return kind === 'agent' ? 'Judge agent' : 'Script'
}

/** Where the column's runnable is edited. Editing a scorer is editing the thing itself. */
export function scorerHref(scorer: Scorer, workspace: string | undefined): string {
	const ws = workspace ? `?workspace=${encodeURIComponent(workspace)}` : ''
	return scorer.kind === 'agent'
		? `/resources${ws}${ws ? '&' : '?'}path=${encodeURIComponent(scorer.path)}`
		: `/scripts/get/${scorer.path}${ws}`
}

/** Scores are read across a row, so they are padded to a fixed width rather than trimmed. */
export function formatScore(score: number | undefined): string {
	return score == undefined ? '—' : score.toFixed(2)
}

export function formatDelta(delta: number): string {
	if (delta === 0) return '0.00'
	return `${delta > 0 ? '+' : '−'}${Math.abs(delta).toFixed(2)}`
}

/** Which side of a column's threshold a score fell on. `undefined` when the column has no
 *  threshold, which is what keeps a column of plain numbers from being read as pass or fail. */
export function passedBy(scorer: Scorer, score: number | undefined): boolean | undefined {
	return scorer.pass_if == undefined || score == undefined ? undefined : score >= scorer.pass_if
}
