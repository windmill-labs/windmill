import { describe, expect, it } from 'vitest'
import { isChartResult, parseFence } from './displayCatalog'

const rows = [
	{ status: 'success', runs: 12 },
	{ status: 'failure', runs: 3 }
]

describe('display catalog', () => {
	it('accepts a chart and reports a missing column by name', () => {
		expect(
			parseFence(JSON.stringify({ chart: { type: 'bar', data: rows, x: 'status', y: 'runs' } })).ok
		).toBe(true)
		const bad = parseFence(
			JSON.stringify({ chart: { type: 'bar', data: rows, x: 'status', y: 'count' } })
		)
		expect(bad.ok).toBe(false)
		expect(!bad.ok && bad.error).toContain('column "count" is not in any row of data')
		const pie = parseFence(
			JSON.stringify({ chart: { type: 'pie', data: rows, x: 'status', y: ['runs', 'status'] } })
		)
		expect(!pie.ok && pie.error).toContain('a pie chart takes exactly one y column')
		expect(!pie.ok && pie.error).toContain('y column "status" must hold numbers')
	})

	// The fence renders on the public shared-session page, which must issue no request.
	it('rejects kinds that fetch or run markup, alone or inside render_all', () => {
		for (const item of [
			{ html: '<img src="https://example.com/x.png">' },
			{ markdown: '![x](https://example.com/x.png)' },
			{ s3: 'a/b.png' },
			{ map: { lat: 0, lon: 0 } }
		]) {
			expect(parseFence(JSON.stringify(item)).ok).toBe(false)
			expect(parseFence(JSON.stringify({ render_all: [item] })).ok).toBe(false)
		}
	})

	it('does not classify an arbitrary `chart` result as a chart', () => {
		expect(isChartResult({ chart: 'https://example.com/chart.png' })).toBe(false)
		expect(isChartResult({ chart: { type: 'bar', data: rows, x: 'status', y: 'runs' } })).toBe(true)
	})
})
