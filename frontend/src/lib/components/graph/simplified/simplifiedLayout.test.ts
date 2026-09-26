import { describe, expect, it } from 'vitest'
import type { FlowModule } from '$lib/gen'
import { layoutSimplifiedFlow, SIMPLIFIED, type SimplifiedEdgeData } from './simplifiedLayout'

const measure = (text: string) => text.length * 7

function step(id: string, summary: string): FlowModule {
	return {
		id,
		summary,
		value: { type: 'rawscript', language: 'bun', content: '', input_transforms: {} }
	}
}

describe('layoutSimplifiedFlow', () => {
	it('sizes each column by its widest node and stacks branches in one column', () => {
		const modules: FlowModule[] = [
			{
				id: 'b',
				value: {
					type: 'branchall',
					branches: [{ modules: [step('long', 'x'.repeat(40))] }, { modules: [step('short', 'x')] }]
				}
			},
			step('after', 'x')
		]
		const { nodes, edges } = layoutSimplifiedFlow({ modules }, measure)
		const byId = Object.fromEntries(nodes.map((n) => [n.id, n]))

		// Branches of different widths merge on one trunk, placed before the merge target.
		const merging = edges.filter((e) => e.target === 'after')
		expect(merging.map((e) => e.source).sort()).toEqual(['long', 'short'])
		const trunks = new Set(merging.map((e) => (e.data as SimplifiedEdgeData).trunkX))
		expect(trunks.size).toBe(1)
		expect([...trunks][0]!).toBeLessThan(byId.after.position.x)
		expect([...trunks][0]!).toBeGreaterThan(byId.long.position.x + byId.long.width!)

		expect(byId.long.position.x).toBe(byId.short.position.x)
		expect(byId.long.position.y).not.toBe(byId.short.position.y)
		expect(byId.long.width!).toBeGreaterThan(byId.short.width!)
		// The next column starts after the widest node of the branch column, not the narrow one.
		expect(byId.after.position.x).toBeGreaterThanOrEqual(
			byId.long.position.x + byId.long.width! + SIMPLIFIED.colGap
		)
	})

	it('draws loop and branch steps as boxes, with edges straight into and out of their contents', () => {
		const modules: FlowModule[] = [
			{
				id: 'loop',
				value: {
					type: 'forloopflow',
					iterator: { type: 'javascript', expr: 'x' },
					skip_failures: false,
					modules: [
						{
							id: 'one',
							value: {
								type: 'branchone',
								branches: [{ expr: 'true', modules: [step('inner', 'Inner')] }],
								default: []
							}
						}
					]
				}
			}
		]
		const { nodes, edges } = layoutSimplifiedFlow({ modules }, measure)
		const byId = Object.fromEntries(nodes.map((n) => [n.id, n]))
		const inside = (n: (typeof nodes)[number], box: (typeof nodes)[number]) =>
			n.position.x > box.position.x &&
			n.position.x + n.width! < box.position.x + box.width! &&
			n.position.y > box.position.y &&
			n.position.y + n.height! < box.position.y + box.height!
		expect(byId.loop.type).toBe('simplifiedBox')
		expect(byId.one.type).toBe('simplifiedBox')
		expect(inside(byId.one, byId.loop)).toBe(true)
		for (const id of ['inner', 'one:empty-1']) expect(inside(byId[id], byId.one)).toBe(true)
		expect(byId['one:caption-1'].data.title).toBe('Default')

		expect(edges.map((e) => `${e.source}->${e.target}`).sort()).toEqual([
			'Input->inner',
			'Input->one:empty-1',
			'inner->Result',
			'one:empty-1->Result'
		])
	})
})
