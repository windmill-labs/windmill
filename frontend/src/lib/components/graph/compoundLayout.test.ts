import { describe, expect, it } from 'vitest'
import { compoundLayout } from './compoundLayout'
import { NODE } from './util'

describe('compoundLayout expanded subflows', () => {
	it('lays out subflows nested as each other first step once per level', () => {
		// in -> s0 -> s1 -> ... -> leaf -> ... -> s1-subflow-end -> s0-subflow-end
		const depth = 30
		const nodes: { id: string; parentIds?: string[]; subflowHead?: boolean }[] = [{ id: 'in' }]
		let prev = 'in'
		for (let i = 0; i < depth; i++) {
			nodes.push({ id: `s${i}`, parentIds: [prev], subflowHead: true })
			prev = `s${i}`
		}
		nodes.push({ id: 'leaf', parentIds: [prev] })
		prev = 'leaf'
		for (let i = depth - 1; i >= 0; i--) {
			nodes.push({ id: `s${i}-subflow-end`, parentIds: [prev] })
			prev = `s${i}-subflow-end`
		}

		const start = performance.now()
		const { positions } = compoundLayout(nodes)
		expect(performance.now() - start).toBeLessThan(1000)

		for (let i = 1; i < nodes.length; i++) {
			expect(positions.get(nodes[i].id)!.y).toBeGreaterThan(positions.get(nodes[i - 1].id)!.y)
		}
	})

	it('starts the subflow body below the space reserved above its start bound', () => {
		const nodes = [
			{ id: 'in' },
			{ id: 'b', parentIds: ['in'], subflowHead: true },
			{ id: 'c1', parentIds: ['b'] },
			{ id: 'b-subflow-end', parentIds: ['c1'] }
		]
		const extra = new Map([['b', { top: 76, bottom: 0, left: 0, right: 0 }]])
		const { positions } = compoundLayout(nodes, undefined, extra)

		const startBottom = positions.get('b')!.y + NODE.height
		expect(positions.get('c1')!.y - startBottom).toBe(NODE.gap.vertical)
	})

	it('leaves steps whose ids end in -subflow-end or -subflow alone', () => {
		const plain = compoundLayout([
			{ id: 'in' },
			{ id: 'a-subflow-end', parentIds: ['in'] },
			{ id: 'a', parentIds: ['a-subflow-end'] },
			{ id: 'out', parentIds: ['a'] }
		])
		const ys = ['in', 'a-subflow-end', 'a', 'out'].map((id) => plain.positions.get(id)?.y)
		expect(ys).toEqual([...ys].sort((p, q) => p! - q!))
		expect(new Set(ys).size).toBe(4)

		// A for loop whose step id ends in -subflow keeps its indented loop layout
		const loop = compoundLayout([
			{ id: 'x-subflow' },
			{ id: 'x-subflow-start', parentIds: ['x-subflow'] },
			{ id: 'c', parentIds: ['x-subflow-start'] },
			{ id: 'x-subflow-end', parentIds: ['c'] }
		])
		expect(loop.positions.get('c')!.x).toBeGreaterThan(loop.positions.get('x-subflow')!.x)
	})
})
