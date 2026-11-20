import { describe, expect, it } from 'vitest'
import { compoundLayout } from './compoundLayout'
import { NODE } from './util'

describe('compoundLayout expanded subflows', () => {
	it('lays out subflows nested as each other first step once per level', () => {
		// in -> s0 -> s1 -> ... -> leaf -> ... -> s1-subflow-end -> s0-subflow-end
		const depth = 30
		const nodes: { id: string; parentIds?: string[] }[] = [{ id: 'in' }]
		let prev = 'in'
		for (let i = 0; i < depth; i++) {
			nodes.push({ id: `s${i}`, parentIds: [prev] })
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
			{ id: 'b', parentIds: ['in'] },
			{ id: 'c1', parentIds: ['b'] },
			{ id: 'b-subflow-end', parentIds: ['c1'] }
		]
		const extra = new Map([['b', { top: 76, bottom: 0, left: 0, right: 0 }]])
		const { positions } = compoundLayout(nodes, undefined, extra)

		const startBottom = positions.get('b')!.y + NODE.height
		expect(positions.get('c1')!.y - startBottom).toBe(NODE.gap.vertical)
	})
})
