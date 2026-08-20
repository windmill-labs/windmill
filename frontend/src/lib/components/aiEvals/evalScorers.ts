import type { Scorer } from '$lib/gen'

export type ScorerKind = Scorer['kind']

/** The column header. A scorer keeps its id when renamed, so this is display only. */
export function scorerLabel(scorer: Scorer): string {
	return scorer.name || scorer.path.split('/').pop() || scorer.path
}

export function kindLabel(kind: ScorerKind): string {
	return kind === 'agent' ? 'Judge agent' : 'Script'
}

/** Scores are read across a row, so they are padded to a fixed width rather than trimmed. */
export function formatScore(score: number | undefined): string {
	return score == undefined ? '—' : score.toFixed(2)
}

export function formatDelta(delta: number): string {
	if (delta === 0) return '0.00'
	return `${delta > 0 ? '+' : '−'}${Math.abs(delta).toFixed(2)}`
}
