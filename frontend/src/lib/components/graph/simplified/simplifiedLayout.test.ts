import { describe, expect, it } from 'vitest'
import type { FlowModule } from '$lib/gen'
import {
	layoutSimplifiedFlow,
	SIMPLIFIED,
	type SimplifiedEdgeData,
	type SimplifiedNodeData
} from './simplifiedLayout'

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
		for (const id of ['inner', 'one:empty-0']) expect(inside(byId[id], byId.one)).toBe(true)
		expect(byId['one:caption-0'].data.title).toBe('Default')

		expect(edges.map((e) => `${e.source}->${e.target}`).sort()).toEqual([
			'Input->inner',
			'Input->one:empty-0',
			'inner->Result',
			'one:empty-0->Result'
		])
	})

	it('joins two branch groups in a row through one junction instead of every exit-entry pair', () => {
		const group = (id: string, steps: string[]): FlowModule => ({
			id,
			value: { type: 'branchall', branches: steps.map((s) => ({ modules: [step(s, s)] })) }
		})
		const { nodes, edges } = layoutSimplifiedFlow(
			{ modules: [group('g1', ['a', 'b']), group('g2', ['c', 'd', 'e'])] },
			measure
		)
		const junction = nodes.find((n) => (n.data as { kind?: string }).kind === 'junction')!
		const pairs = edges.map((e) => `${e.source}->${e.target}`)
		expect(pairs).toEqual(expect.arrayContaining(['a', 'b'].map((s) => `${s}->${junction.id}`)))
		expect(pairs).toEqual(
			expect.arrayContaining(['c', 'd', 'e'].map((t) => `${junction.id}->${t}`))
		)
		expect(pairs.filter((p) => /^[ab]->[cde]$/.test(p))).toEqual([])
		// It sits in the gap between the two groups
		const byId = Object.fromEntries(nodes.map((n) => [n.id, n]))
		expect(junction.position.x).toBeGreaterThan(byId.a.position.x + byId.a.width!)
		expect(junction.position.x + junction.width!).toBeLessThan(byId.c.position.x)
	})

	it('gives every "+" the insert payload the editor resolves, branch-one default being branch 0', () => {
		const modules: FlowModule[] = [
			step('a', 'A'),
			{
				id: 'b',
				value: {
					type: 'branchone',
					branches: [{ expr: 'x', modules: [step('c', 'C')] }],
					default: []
				}
			}
		]
		const { nodes } = layoutSimplifiedFlow({ modules, editable: true }, measure)
		const inserts = nodes.flatMap((n) => {
			const { insert } = n.data as SimplifiedNodeData
			return insert ? [{ id: n.id, ...insert }] : []
		})
		const at = (index: number, rootId?: string, branch?: number) =>
			inserts.find(
				(i) => i.index === index && i.branch?.rootId === rootId && i.branch?.branch === branch
			)

		// Top level: between Input and a (the only place a trigger may go), and before Result
		expect(at(0)).toMatchObject({ sourceId: 'Input', targetId: 'a', allowTrigger: true })
		expect(at(2)).toMatchObject({ sourceId: 'b', targetId: 'Result', allowTrigger: false })
		// Inside the condition's lane, which the step tree numbers after the default
		expect(at(0, 'b', 1)).toMatchObject({ targetId: 'c', ancestors: ['b'] })
		expect(at(1, 'b', 1)).toMatchObject({ sourceId: 'c' })
		// The empty default lane's dot is its own "+"
		expect(inserts.find((i) => i.id === 'b:empty-0')).toMatchObject({
			index: 0,
			branch: { rootId: 'b', branch: 0 }
		})
	})
})
