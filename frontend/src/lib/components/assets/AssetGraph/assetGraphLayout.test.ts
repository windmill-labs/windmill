import { describe, expect, it } from 'vitest'
import { layoutAssetGraph } from './assetGraphLayout'
import { NODE } from '$lib/components/graph/util'

const NODE_WIDTH = NODE.width

const n = (id: string) => ({ id, data: {} as any })
const e = (source: string, target: string) => ({ source, target })

describe('layoutAssetGraph (tidy-tree with join breaks)', () => {
	it('lays a chain out vertically on one column', () => {
		const pos = layoutAssetGraph({
			nodes: [n('a'), n('b'), n('c')],
			edges: [e('a', 'b'), e('b', 'c')]
		})
		expect(pos.get('a')!.x).toBe(pos.get('b')!.x)
		expect(pos.get('b')!.x).toBe(pos.get('c')!.x)
		expect(pos.get('a')!.y).toBeLessThan(pos.get('b')!.y)
		expect(pos.get('b')!.y).toBeLessThan(pos.get('c')!.y)
	})

	it('centers a parent over side-by-side children', () => {
		const pos = layoutAssetGraph({
			nodes: [n('p'), n('l'), n('r')],
			edges: [e('p', 'l'), e('p', 'r')]
		})
		const p = pos.get('p')!
		const l = pos.get('l')!
		const r = pos.get('r')!
		expect(l.y).toBe(r.y)
		expect(r.x - l.x).toBeGreaterThanOrEqual(NODE_WIDTH)
		expect(p.x).toBeCloseTo((l.x + r.x) / 2, 5)
	})

	it('keeps subtrees of different parents in disjoint horizontal bands', () => {
		// Two branches under one root; the left branch fans out into three
		// leaves. With per-band reservation, every node of the left branch
		// stays strictly left of every node of the right branch.
		const pos = layoutAssetGraph({
			nodes: [n('root'), n('a'), n('b'), n('a1'), n('a2'), n('a3'), n('b1')],
			edges: [
				e('root', 'a'),
				e('root', 'b'),
				e('a', 'a1'),
				e('a', 'a2'),
				e('a', 'a3'),
				e('b', 'b1')
			]
		})
		const leftMax = Math.max(...['a', 'a1', 'a2', 'a3'].map((id) => pos.get(id)!.x))
		const rightMin = Math.min(...['b', 'b1'].map((id) => pos.get(id)!.x))
		expect(rightMin - leftMax).toBeGreaterThanOrEqual(NODE_WIDTH)
	})

	it('a join roots its own subtree, centered under its parents', () => {
		//   s1   s2
		//    \   /
		//     j        j has 2 parents → excluded from both bands
		//     |
		//     t
		const pos = layoutAssetGraph({
			nodes: [n('s1'), n('s2'), n('j'), n('t')],
			edges: [e('s1', 'j'), e('s2', 'j'), e('j', 't')]
		})
		const s1 = pos.get('s1')!
		const s2 = pos.get('s2')!
		const j = pos.get('j')!
		// Sources keep single-node bands (the join didn't widen them).
		expect(Math.abs(s2.x - s1.x)).toBeLessThanOrEqual(NODE_WIDTH + NODE.gap.horizontal)
		// Join centered between its parents, one layer below the lowest.
		expect(j.x).toBeCloseTo((s1.x + s2.x) / 2, 5)
		expect(j.y).toBeGreaterThan(Math.max(s1.y, s2.y))
		// Its subtree hangs under it.
		expect(pos.get('t')!.x).toBe(j.x)
		expect(pos.get('t')!.y).toBeGreaterThan(j.y)
	})

	it('pushes a join tree sideways instead of overlapping a band in the same layers', () => {
		// Both sources also have a private child at the join's layer; the
		// join's band must not overlap those bands.
		const pos = layoutAssetGraph({
			nodes: [n('s1'), n('s2'), n('c1'), n('c2'), n('j')],
			edges: [e('s1', 'c1'), e('s2', 'c2'), e('s1', 'j'), e('s2', 'j')]
		})
		const j = pos.get('j')!
		for (const other of ['c1', 'c2']) {
			const o = pos.get(other)!
			expect(o.y).toBe(j.y)
			expect(Math.abs(o.x - j.x)).toBeGreaterThanOrEqual(NODE_WIDTH)
		}
	})

	it('lays a 2-cycle out as a chain (feedback edge dropped, no grid)', () => {
		const pos = layoutAssetGraph({
			nodes: [n('a'), n('b')],
			edges: [e('a', 'b'), e('b', 'a')]
		})
		// First-in-input wins the top slot; the b→a feedback edge is ignored.
		expect(pos.get('a')!.y).toBeLessThan(pos.get('b')!.y)
		expect(pos.get('a')!.x).toBe(pos.get('b')!.x)
	})

	it('keeps the acyclic part of a graph layered when one cycle exists', () => {
		// root → a ⇄ b → leaf: the a⇄b cycle must not degrade root/leaf layering.
		const pos = layoutAssetGraph({
			nodes: [n('root'), n('a'), n('b'), n('leaf')],
			edges: [e('root', 'a'), e('a', 'b'), e('b', 'a'), e('b', 'leaf')]
		})
		expect(pos.get('root')!.y).toBeLessThan(pos.get('a')!.y)
		expect(pos.get('a')!.y).toBeLessThan(pos.get('b')!.y)
		expect(pos.get('b')!.y).toBeLessThan(pos.get('leaf')!.y)
		// A linear chain stays in one column.
		expect(new Set(['root', 'a', 'b', 'leaf'].map((id) => pos.get(id)!.x)).size).toBe(1)
	})

	it('handles a longer cycle without dropping nodes', () => {
		const pos = layoutAssetGraph({
			nodes: [n('a'), n('b'), n('c')],
			edges: [e('a', 'b'), e('b', 'c'), e('c', 'a')]
		})
		expect(pos.size).toBe(3)
		const ys = ['a', 'b', 'c'].map((id) => pos.get(id)!.y)
		expect(new Set(ys).size).toBe(3)
	})

	it('packs disjoint components side by side without overlap', () => {
		const pos = layoutAssetGraph({
			nodes: [n('a'), n('b'), n('x'), n('y')],
			edges: [e('a', 'b'), e('x', 'y')]
		})
		const comp1Max = Math.max(pos.get('a')!.x, pos.get('b')!.x)
		const comp2Min = Math.min(pos.get('x')!.x, pos.get('y')!.x)
		expect(comp2Min).toBeGreaterThanOrEqual(comp1Max + NODE_WIDTH)
	})

	it('places the anchor centered above everything', () => {
		const pos = layoutAssetGraph(
			{
				nodes: [n('__add__'), n('a'), n('b')],
				edges: [e('__add__', 'a'), e('__add__', 'b')]
			},
			'__add__'
		)
		const anchor = pos.get('__add__')!
		expect(anchor.y).toBe(0)
		for (const id of ['a', 'b']) {
			expect(pos.get(id)!.y).toBeGreaterThan(anchor.y)
		}
	})
})
