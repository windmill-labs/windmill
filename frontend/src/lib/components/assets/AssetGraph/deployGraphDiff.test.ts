import { describe, expect, it } from 'vitest'
import type { AssetGraphResponse } from './types'
import { diffDeployedGraph, extractCascadeFacts, formatDrift } from './deployGraphDiff'

function g(
	writes: Array<[script: string, asset: string]>,
	subs: Array<[script: string, asset: string]> = [],
	reads: Array<[script: string, asset: string]> = []
): AssetGraphResponse {
	return {
		assets: [],
		runnables: [],
		edges: [
			...writes.map(([s, a]) => ({
				runnable_path: s,
				runnable_kind: 'script' as const,
				asset_kind: 'datatable' as const,
				asset_path: a,
				access_type: 'w' as const
			})),
			...reads.map(([s, a]) => ({
				runnable_path: s,
				runnable_kind: 'script' as const,
				asset_kind: 'datatable' as const,
				asset_path: a,
				access_type: 'r' as const
			}))
		],
		triggers: subs.map(([s, a]) => ({
			trigger_kind: 'asset' as const,
			asset_kind: 'datatable' as const,
			asset_path: a,
			runnable_kind: 'script' as const,
			runnable_path: s
		}))
	}
}

describe('extractCascadeFacts', () => {
	it('collects writes and subscriptions, ignoring reads', () => {
		const facts = extractCascadeFacts(g([['s', 'out']], [['s', 'in']], [['s', 'lookup']]), 's')
		expect([...facts.writes]).toEqual(['datatable:out'])
		expect([...facts.subs]).toEqual(['datatable:in'])
	})
})

describe('diffDeployedGraph', () => {
	it('no drift when deploy matches the prediction', () => {
		const predicted = new Map([['s', extractCascadeFacts(g([['s', 'out']], [['s', 'in']]), 's')]])
		const drift = diffDeployedGraph(predicted, g([['s', 'out']], [['s', 'in']]))
		expect(drift).toEqual([])
		expect(formatDrift(drift)).toBeUndefined()
	})

	it('reports a write promised by the preview but missing after deploy', () => {
		const predicted = new Map([['s', extractCascadeFacts(g([['s', 'out']]), 's')]])
		const drift = diffDeployedGraph(predicted, g([]))
		expect(drift).toHaveLength(1)
		expect(drift[0].missingWrites).toEqual(['datatable:out'])
		expect(formatDrift(drift)).toContain('s: output datatable:out')
	})

	it('reports a missing subscription', () => {
		const predicted = new Map([['s', extractCascadeFacts(g([], [['s', 'in']]), 's')]])
		const drift = diffDeployedGraph(predicted, g([]))
		expect(drift[0].missingSubs).toEqual(['datatable:in'])
	})

	it('extra server-side facts are not drift (deploy union direction)', () => {
		const predicted = new Map([['s', extractCascadeFacts(g([['s', 'out']]), 's')]])
		const drift = diffDeployedGraph(
			predicted,
			g(
				[
					['s', 'out'],
					['s', 'extra_detected_by_server']
				],
				[['s', 'extra_sub']]
			)
		)
		expect(drift).toEqual([])
	})
})
