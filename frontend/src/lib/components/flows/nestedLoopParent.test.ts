/*
 * Regression coverage for nested-loop "Test this step" arg resolution.
 *
 * stepsInputArgs (evalArg / updateStepArgs) feeds getStepPropPicker the step's parent
 * module, taken as modules[1] from dfs(stepId, flow, true) (which returns
 * [step, immediate parent, ..., root]). Using the outermost ancestor instead made
 * flow_input.iter resolve to the wrong (outer) loop for nested loops.
 *
 * These tests pin two properties:
 *   1. modules[1] is the immediate parent for every step, every container type, every
 *      nesting shape (this is the value the prop picker must receive).
 *   2. getStepPropPicker then resolves flow_input.iter to the INNERMOST enclosing loop.
 */
import { describe, expect, it } from 'vitest'
import type { FlowModule, OpenFlow } from '$lib/gen'
import { dfs, getStepPropPicker } from './previousResults'
import type { FlowState } from './flowState'

// ---- builders ---------------------------------------------------------------

function raw(id: string): FlowModule {
	return {
		id,
		summary: id,
		value: { type: 'rawscript', content: '', language: 'python3', input_transforms: {} } as any
	} as FlowModule
}
function forloop(id: string, modules: FlowModule[]): FlowModule {
	return {
		id,
		value: {
			type: 'forloopflow',
			iterator: { type: 'static', value: [] },
			skip_failures: false,
			modules
		} as any
	} as FlowModule
}
function whileloop(id: string, modules: FlowModule[]): FlowModule {
	return {
		id,
		value: { type: 'whileloopflow', skip_failures: false, modules } as any
	} as FlowModule
}
function branchone(id: string, def: FlowModule[], branches: FlowModule[][]): FlowModule {
	return {
		id,
		value: {
			type: 'branchone',
			default: def,
			branches: branches.map((modules, i) => ({ summary: `b${i}`, expr: 'true', modules }))
		} as any
	} as FlowModule
}
function branchall(id: string, branches: FlowModule[][]): FlowModule {
	return {
		id,
		value: {
			type: 'branchall',
			branches: branches.map((modules, i) => ({ summary: `b${i}`, skip_failure: false, modules }))
		} as any
	} as FlowModule
}
function tool(m: FlowModule): FlowModule {
	return { id: m.id, value: { tool_type: 'flowmodule', ...(m.value as any) } as any } as FlowModule
}
function aiagent(id: string, tools: FlowModule[]): FlowModule {
	return { id, value: { type: 'aiagent', tools, input_transforms: {} } as any } as FlowModule
}

// ---- independent ground-truth walker (does not call getChildModuleBranches) --

function childBranches(m: FlowModule): FlowModule[][] {
	const v: any = m.value
	if (v.type === 'forloopflow' || v.type === 'whileloopflow') return [v.modules]
	if (v.type === 'branchone') return [v.default, ...v.branches.map((b: any) => b.modules)]
	if (v.type === 'branchall') return v.branches.map((b: any) => b.modules)
	if (v.type === 'aiagent' && v.tools)
		return [
			(v.tools as FlowModule[]).filter((t) => {
				const tv = t.value as any
				return tv.tool_type === undefined || tv.tool_type === 'flowmodule'
			})
		]
	return []
}
function immediateParent(
	modules: FlowModule[],
	targetId: string,
	parent: FlowModule | undefined
): { found: boolean; parent?: FlowModule } {
	for (const m of modules) {
		if (m.id === targetId) return { found: true, parent }
		for (const branch of childBranches(m)) {
			const r = immediateParent(branch, targetId, m)
			if (r.found) return r
		}
	}
	return { found: false }
}
function allLeafIds(modules: FlowModule[]): string[] {
	const out: string[] = []
	for (const m of modules) {
		const branches = childBranches(m)
		if (branches.length === 0 && (m.value as any).type === 'rawscript') out.push(m.id)
		for (const b of branches) out.push(...allLeafIds(b))
	}
	return out
}
function allModuleIds(modules: FlowModule[]): string[] {
	const out: string[] = []
	for (const m of modules) {
		out.push(m.id)
		for (const b of childBranches(m)) out.push(...allModuleIds(b))
	}
	return out
}

// ---- exhaustive shape generator ---------------------------------------------

type Wrapper = (id: string, inner: FlowModule[]) => FlowModule
const wrappers: Record<string, Wrapper> = {
	for: (id, inner) => forloop(id, inner),
	while: (id, inner) => whileloop(id, inner),
	branchone_def: (id, inner) => branchone(id, inner, [[raw(id + '_o1')]]),
	branchone_arm: (id, inner) => branchone(id, [raw(id + '_def')], [inner, [raw(id + '_o2')]]),
	branchall: (id, inner) => branchall(id, [inner, [raw(id + '_o3')]]),
	aiagent: (id, inner) => aiagent(id, [tool(raw(id + '_t0')), ...inner.map(tool)])
}

function buildShapes(): { name: string; flow: OpenFlow }[] {
	const keys = Object.keys(wrappers)
	const shapes: { name: string; flow: OpenFlow }[] = []
	const emit = (name: string, target: FlowModule) =>
		shapes.push({
			name,
			flow: { summary: name, value: { modules: [raw('pre'), target, raw('post')] } } as OpenFlow
		})

	for (const k of keys)
		emit(k, wrappers[k]('c0', [raw('sib_before'), raw('leaf'), raw('sib_after')]))
	for (const k1 of keys)
		for (const k2 of keys)
			emit(
				`${k1}>${k2}`,
				wrappers[k1]('c0', [
					raw('x'),
					wrappers[k2]('c1', [raw('s_before'), raw('leaf'), raw('s_after')])
				])
			)
	const k3 = ['for', 'while', 'branchone_arm', 'branchall']
	for (const k1 of k3)
		for (const k2 of k3)
			for (const k3i of k3)
				emit(
					`${k1}>${k2}>${k3i}`,
					wrappers[k1]('c0', [
						wrappers[k2]('c1', [raw('m'), wrappers[k3i]('c2', [raw('leaf')])]),
						raw('n')
					])
				)
	emit(
		'for>for>for>for',
		forloop('c0', [forloop('c1', [forloop('c2', [forloop('c3', [raw('leaf')])])])])
	)
	return shapes
}

// ---- synthetic flow state with loop previewArgs carrying chained iter --------

function resolveIter(flow: OpenFlow, leafId: string): string | undefined {
	const modules = dfs(leafId, flow, true)
	const parent = modules.length > 1 ? modules[1] : undefined

	const fs: FlowState = {} as any
	for (const id of allModuleIds(flow.value.modules)) {
		fs[id] = { schema: { properties: {}, required: [] } } as any
	}
	const loopsOuterToInner = modules
		.slice(1)
		.filter((m) => ['forloopflow', 'whileloopflow'].includes((m.value as any).type))
		.reverse()
	loopsOuterToInner.forEach((loop, i) => {
		const previewArgs: any = { iter: { value: `L${i}`, index: i } }
		if (i > 0) previewArgs.iter_parent = { value: `L${i - 1}`, index: i - 1 }
		fs[loop.id] = { ...(fs[loop.id] as any), previewArgs }
	})

	const picker = getStepPropPicker(fs, parent, undefined, leafId, flow, {}, false)
	return (picker.pickableProperties.flow_input as any)?.iter?.value
}

describe('nested-loop parent selection', () => {
	const shapes = buildShapes()

	it('modules[1] is the immediate parent for every step in every generated shape', () => {
		const mismatches: string[] = []
		let checked = 0
		for (const { name, flow } of shapes) {
			for (const leaf of allLeafIds(flow.value.modules)) {
				const modules = dfs(leaf, flow, true)
				expect(modules[0]?.id, `${name}/${leaf}`).toBe(leaf)
				const newParent = modules.length > 1 ? modules[1] : undefined
				const truth = immediateParent(flow.value.modules, leaf, undefined)
				checked++
				if ((newParent?.id ?? undefined) !== (truth.parent?.id ?? undefined)) {
					mismatches.push(`${name}/${leaf}: got ${newParent?.id} want ${truth.parent?.id}`)
				}
			}
		}
		expect(checked).toBeGreaterThan(400)
		expect(mismatches).toEqual([])
	})

	it('flow_input.iter resolves to the innermost enclosing loop', () => {
		const cases: [string, OpenFlow, string | undefined][] = [
			['single for', { value: { modules: [forloop('a', [raw('leaf')])] } } as OpenFlow, 'L0'],
			[
				'for>for',
				{ value: { modules: [forloop('a', [forloop('b', [raw('leaf')])])] } } as OpenFlow,
				'L1'
			],
			[
				'for>for>for',
				{
					value: { modules: [forloop('a', [forloop('b', [forloop('c', [raw('leaf')])])])] }
				} as OpenFlow,
				'L2'
			],
			[
				'for>while',
				{ value: { modules: [forloop('a', [whileloop('b', [raw('leaf')])])] } } as OpenFlow,
				'L1'
			],
			[
				'for>for>branch arm',
				{
					value: {
						modules: [forloop('a', [forloop('b', [branchone('br', [raw('leaf')], [[raw('o')]])])])]
					}
				} as OpenFlow,
				'L1'
			],
			[
				'branch inside single loop',
				{
					value: { modules: [forloop('a', [branchone('br', [raw('leaf')], [[raw('o')]])])] }
				} as OpenFlow,
				'L0'
			],
			['no loop', { value: { modules: [raw('leaf')] } } as OpenFlow, undefined]
		]
		for (const [name, flow, expected] of cases) {
			expect(resolveIter(flow, 'leaf'), name).toBe(expected)
		}
	})
})
