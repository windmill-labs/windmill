import { formatDateRange } from '$lib/utils'

function computeMinMaxInc(inc: number) {
	let minTs = new Date(new Date().getTime() - inc).toISOString()
	let maxTs = new Date().toISOString()
	return { minTs, maxTs }
}

export type Timeframe =
	| {
			label: string
			computeMinMax: () => { minTs: string | null; maxTs: string | null }
			type: 'dynamic'
	  }
	| {
			label: string
			computeMinMax: () => { minTs: string | null; maxTs: string | null }
			minTs: string | null
			maxTs: string | null
			type: 'manual'
	  }

export function buildManualTimeframe(minTs: string | null, maxTs: string | null): Timeframe {
	return {
		label: formatDateRange(minTs ?? undefined, maxTs ?? undefined),
		minTs,
		maxTs,
		type: 'manual',
		computeMinMax: () => ({ minTs, maxTs })
	}
}

export const serviceLogsTimeframes: Timeframe[] = [
	{ label: '1000 last service logs', computeMinMax: () => ({ minTs: null, maxTs: null }) },
	{ label: 'Within last 5 minutes', computeMinMax: () => computeMinMaxInc(5 * 60 * 1000) },
	{ label: 'Within last 30 minutes', computeMinMax: () => computeMinMaxInc(30 * 60 * 1000) },
	{ label: 'Within last 24 hours', computeMinMax: () => computeMinMaxInc(24 * 60 * 60 * 1000) },
	{ label: 'Within last 7 days', computeMinMax: () => computeMinMaxInc(7 * 24 * 60 * 60 * 1000) },
	{ label: 'Within last month', computeMinMax: () => computeMinMaxInc(30 * 24 * 60 * 60 * 1000) }
].map((item) => ({ ...item, type: 'dynamic' }))

export const runsTimeframes: Timeframe[] = [
	{ label: 'Latest runs', computeMinMax: () => ({ minTs: null, maxTs: null }) },
	{ label: 'Within 30 seconds', computeMinMax: () => computeMinMaxInc(30 * 1000) },
	{ label: 'Within last minute', computeMinMax: () => computeMinMaxInc(60 * 1000) },
	{ label: 'Within last 5 minutes', computeMinMax: () => computeMinMaxInc(5 * 60 * 1000) },
	{ label: 'Within last 30 minutes', computeMinMax: () => computeMinMaxInc(30 * 60 * 1000) },
	{ label: 'Within last 24 hours', computeMinMax: () => computeMinMaxInc(24 * 60 * 60 * 1000) },
	{ label: 'Within last 7 days', computeMinMax: () => computeMinMaxInc(7 * 24 * 60 * 60 * 1000) },
	{ label: 'Within last month', computeMinMax: () => computeMinMaxInc(30 * 24 * 60 * 60 * 1000) }
].map((item) => ({ ...item, type: 'dynamic' }))
