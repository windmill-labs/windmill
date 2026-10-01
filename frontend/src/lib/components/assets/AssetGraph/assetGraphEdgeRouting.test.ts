import { describe, it, expect } from 'vitest'
import { routeAssetGraphEdges, roundedPath, type RouteNode } from './assetGraphEdgeRouting'

const GAP = 80
const node = (id: string, cx: number, top: number): RouteNode => ({ id, cx, top, halfW: 100 })

describe('routeAssetGraphEdges', () => {
	// Four sources in one row feeding one target below: the screenshot shape
	// where every edge used to land on the same point and share one height.
	const sources = [node('a', 0, 0), node('b', 300, 0), node('c', 600, 0), node('d', 900, 0)]
	const target = node('t', 450, 200)
	const fanIn = ['a', 'b', 'c', 'd'].map((s) => ({ id: `${s}-t`, source: s, target: 't' }))
	const routes = routeAssetGraphEdges([...sources, target], fanIn, GAP)

	it('spreads a fan-in across the target, in the order of the sources', () => {
		const offsets = fanIn.map((e) => routes.get(e.id)!.targetOffset)
		expect(new Set(offsets).size).toBe(4)
		expect([...offsets].sort((x, y) => x - y)).toEqual(offsets)
	})

	it('keeps every run of a fan-in inside the gap above the target', () => {
		for (const e of fanIn) {
			const y = routes.get(e.id)!.y
			expect(y).toBeGreaterThan(200 - GAP)
			expect(y).toBeLessThan(200)
		}
	})

	it('stacks overlapping runs so none crosses another edge', () => {
		// Coming from the left, the farther source runs lower: its run then passes
		// under where the nearer source comes down, instead of through it.
		expect(routes.get('a-t')!.y).toBeGreaterThan(routes.get('b-t')!.y)
		// Mirrored on the right.
		expect(routes.get('d-t')!.y).toBeGreaterThan(routes.get('c-t')!.y)
	})

	it('offsets edges that share a lane', () => {
		const nodes = [node('s1', 0, 0), node('s2', 0, 0), node('mid', 0, 200), node('t', 0, 400)]
		const laned = routeAssetGraphEdges(
			nodes,
			[
				{ id: 'e1', source: 's1', target: 't', laneX: 150 },
				{ id: 'e2', source: 's2', target: 't', laneX: 150 }
			],
			GAP
		)
		expect(laned.get('e1')!.lane!.x).not.toBe(laned.get('e2')!.lane!.x)
	})

	it('gives edges joining the same two nodes one route', () => {
		const r = routeAssetGraphEdges(
			[node('s', 0, 0), node('t', 0, 200), node('u', 300, 200)],
			[
				{ id: 'read', source: 's', target: 't' },
				{ id: 'trigger', source: 's', target: 't' },
				{ id: 'other', source: 's', target: 'u' }
			],
			GAP
		)
		expect(r.get('trigger')).toEqual(r.get('read'))
		expect(r.get('other')!.sourceOffset).not.toBe(r.get('read')!.sourceOffset)
	})

	it('leaves an upward edge unrouted', () => {
		const r = routeAssetGraphEdges(
			[node('up', 0, 0), node('down', 0, 200)],
			[{ id: 'back', source: 'down', target: 'up' }],
			GAP
		)
		expect(r.has('back')).toBe(false)
	})
})

describe('roundedPath', () => {
	it('rounds each corner and ends on the last point', () => {
		const d = roundedPath(
			[
				[0, 0],
				[0, 50],
				[100, 50],
				[100, 100]
			],
			10
		)
		expect(d.startsWith('M 0 0')).toBe(true)
		expect(d.match(/Q/g)).toHaveLength(2)
		expect(d.endsWith('L 100 100')).toBe(true)
	})
})
