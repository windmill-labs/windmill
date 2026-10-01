import { describe, it, expect } from 'vitest'
import {
	detourLane,
	routeAssetGraphEdges,
	roundedPath,
	type EdgeRoute,
	type RouteEdge,
	type RouteNode
} from './assetGraphEdgeRouting'

const GAP = 80
const node = (id: string, cx: number, top: number): RouteNode => ({
	id,
	cx,
	top,
	halfW: 100,
	height: 60
})
const edge = (source: string, target: string, style = 'read'): RouteEdge => ({
	id: `${source}-${target}`,
	source,
	target,
	style
})
type Pt = [number, number]
const segments = (pts: Pt[]) => pts.slice(1).map((p, i) => [pts[i], p] as const)
// Two axis-aligned segments drawn over each other for a positive length.
function shareLength(a: readonly [Pt, Pt], b: readonly [Pt, Pt]): boolean {
	const vertical = (s: readonly [Pt, Pt]) => s[0][0] === s[1][0]
	if (vertical(a) !== vertical(b)) return false
	const [axis, along] = vertical(a) ? [0, 1] : [1, 0]
	if (a[0][axis] !== b[0][axis]) return false
	const lo = Math.max(Math.min(a[0][along], a[1][along]), Math.min(b[0][along], b[1][along]))
	const hi = Math.min(Math.max(a[0][along], a[1][along]), Math.max(b[0][along], b[1][along]))
	return hi - lo > 0.5
}
/** The rule that keeps merged lines readable: a segment two edges draw over
 * each other belongs to edges sharing a source or a target. */
function expectNoAmbiguousSegment(edges: RouteEdge[], routes: Map<string, EdgeRoute>) {
	for (const e of edges)
		for (const f of edges) {
			if (e === f || e.source === f.source || e.target === f.target) continue
			for (const a of segments(routes.get(e.id)!.points))
				for (const b of segments(routes.get(f.id)!.points))
					expect(shareLength(a, b), `${e.id} and ${f.id} share a segment`).toBe(false)
		}
}
const endX = (r: Map<string, EdgeRoute>, id: string) => r.get(id)!.points.at(-1)![0]

describe('routeAssetGraphEdges', () => {
	it('merges a fan-in into one trunk entering the target', () => {
		const nodes = [node('a', 0, 0), node('b', 300, 0), node('c', 600, 0), node('t', 300, 200)]
		const edges = [edge('a', 't'), edge('b', 't'), edge('c', 't')]
		const r = routeAssetGraphEdges(nodes, edges, GAP)
		expect(new Set(edges.map((e) => endX(r, e.id))).size).toBe(1)
		// The bus is marked where the outer edges join it.
		expect(r.get('b-t')!.junctions.length).toBeGreaterThan(0)
		expectNoAmbiguousSegment(edges, r)
	})

	it('merges a fan-out into one drop leaving the source', () => {
		const nodes = [node('s', 300, 0), node('a', 0, 200), node('b', 300, 200), node('c', 600, 200)]
		const edges = [edge('s', 'a'), edge('s', 'b'), edge('s', 'c')]
		const r = routeAssetGraphEdges(nodes, edges, GAP)
		expect(new Set(edges.map((e) => r.get(e.id)!.points[0][0])).size).toBe(1)
		expectNoAmbiguousSegment(edges, r)
	})

	it('merges long edges into one target even when their sources fan out', () => {
		// Two sources each feed a node right below and, past it, a node further
		// down: the long edges share a target, so they may share their lane.
		const nodes = [
			node('events', 0, 0),
			node('persons', 600, 0),
			node('merged', 300, 200),
			node('rollup', 300, 400)
		]
		const edges = [
			edge('events', 'merged'),
			edge('persons', 'merged'),
			{ ...edge('events', 'rollup'), laneX: 150 },
			{ ...edge('persons', 'rollup'), laneX: 150 },
			edge('merged', 'rollup')
		]
		const r = routeAssetGraphEdges(nodes, edges, GAP)
		const lane = (id: string) => r.get(id)!.points.find(([x, y]) => y > 200 && y < 400)?.[0]
		expect(lane('events-rollup')).toBe(lane('persons-rollup'))
		expect(endX(r, 'events-rollup')).toBe(endX(r, 'persons-rollup'))
		expect(endX(r, 'events-merged')).toBe(endX(r, 'persons-merged'))
		expectNoAmbiguousSegment(edges, r)
	})

	it('never merges edges with different sources and different targets', () => {
		const nodes = [node('s1', 0, 0), node('s2', 600, 0), node('t1', 0, 200), node('t2', 600, 200)]
		const edges = [edge('s1', 't1'), edge('s1', 't2'), edge('s2', 't1'), edge('s2', 't2')]
		expectNoAmbiguousSegment(edges, routeAssetGraphEdges(nodes, edges, GAP))
	})

	it('keeps edges drawn differently apart', () => {
		const nodes = [node('a', 0, 0), node('b', 600, 0), node('t', 300, 200)]
		const r = routeAssetGraphEdges(nodes, [edge('a', 't', 'read'), edge('b', 't', 'trigger')], GAP)
		expect(endX(r, 'a-t')).not.toBe(endX(r, 'b-t'))
	})

	it('draws a lone edge between overlapping nodes as one straight line', () => {
		const r = routeAssetGraphEdges([node('s', 0, 0), node('t', 40, 200)], [edge('s', 't')], GAP)
		const pts = r.get('s-t')!.points
		expect(pts).toHaveLength(2)
		expect(pts[0][0]).toBe(pts[1][0])
	})

	it('enters a side to save a turn when nothing is in the way', () => {
		const r = routeAssetGraphEdges([node('s', 0, 0), node('t', 600, 200)], [edge('s', 't')], GAP)
		const route = r.get('s-t')!
		// Two turns through the gap become one, by a side of either node.
		expect(route.points).toHaveLength(3)
		expect(route.sourceSide ?? route.targetSide).toBeDefined()
	})

	it('takes no side shortcut through another node', () => {
		// Leaving s sideways would run through b; entering t sideways would
		// come down through c.
		const r = routeAssetGraphEdges(
			[node('s', 0, 0), node('b', 300, 0), node('c', 600, 0), node('t', 300, 200)],
			[edge('s', 't')],
			GAP
		)
		const route = r.get('s-t')!
		expect(route.sourceSide).toBeUndefined()
	})

	it('offsets lanes of unrelated edges', () => {
		const nodes = [
			node('s1', 0, 0),
			node('s2', 0, 0),
			node('mid', 0, 200),
			node('t1', -50, 400),
			node('t2', 50, 400)
		]
		const r = routeAssetGraphEdges(
			nodes,
			[
				{ ...edge('s1', 't1'), laneX: 150 },
				{ ...edge('s2', 't2'), laneX: 150 }
			],
			GAP
		)
		expect(r.get('s1-t1')!.points[2][0]).not.toBe(r.get('s2-t2')!.points[2][0])
	})

	it('gives edges joining the same two nodes one route', () => {
		const r = routeAssetGraphEdges(
			[node('s', 0, 0), node('t', 0, 200)],
			[edge('s', 't', 'read'), { ...edge('s', 't', 'trigger'), id: 'trigger' }],
			GAP
		)
		expect(r.get('trigger')).toEqual(r.get('s-t'))
	})

	it('leaves an upward edge unrouted', () => {
		const r = routeAssetGraphEdges(
			[node('up', 0, 0), node('down', 0, 200)],
			[edge('down', 'up')],
			GAP
		)
		expect(r.has('down-up')).toBe(false)
	})
})

describe('detourLane', () => {
	it('clears every node in the way, not only the first one found', () => {
		// The target's column is blocked by two nodes on different rows; the lane
		// past the nearer one alone would still run through the other.
		const source = node('s', -200, 0)
		const target = node('t', 0, 600)
		const a = { ...node('a', 150, 200), halfW: 180 }
		const b = { ...node('b', 80, 400), halfW: 180 }
		const lane = detourLane(source, target, [source, target, a, b], 25)!
		for (const n of [a, b]) expect(Math.abs(lane - n.cx)).toBeGreaterThan(n.halfW)
	})

	it('needs no lane when the column is clear', () => {
		const source = node('s', 0, 0)
		const target = node('t', 0, 400)
		expect(detourLane(source, target, [source, target, node('a', 500, 200)], 25)).toBeUndefined()
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
