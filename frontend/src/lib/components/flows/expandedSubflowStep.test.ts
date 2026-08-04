import { describe, it, expect } from 'vitest'
import type { FlowModule } from '$lib/gen'
import { resolveExpandedSubflowStep } from './expandedSubflowStep'

const script = (id: string): FlowModule =>
	({ id, value: { type: 'rawscript', language: 'bun', content: '', input_transforms: {} } }) as any
const subflow = (id: string, path: string): FlowModule =>
	({ id, value: { type: 'flow', path } }) as any
const forloop = (id: string, modules: FlowModule[]): FlowModule =>
	({ id, value: { type: 'forloopflow', modules, iterator: { type: 'static', value: [] } } }) as any

// `outer` inlines `f/inner`, which inlines `f/innermost`; each subflow step also sits
// inside a for loop, which the graph node id does not record.
const flows: Record<string, FlowModule[]> = {
	'f/inner': [forloop('l', [subflow('b', 'f/innermost')])],
	'f/innermost': [forloop('m', [script('leaf')])]
}
const rootModules = [forloop('k', [subflow('a', 'f/inner')])]
const loadFlowModules = async (path: string) => flows[path]

describe('resolveExpandedSubflowStep', () => {
	it('follows every subflow boundary down to the selected step', async () => {
		const resolved = await resolveExpandedSubflowStep(
			'subflow:a:b:leaf',
			rootModules,
			loadFlowModules
		)
		expect(resolved?.pathChain).toEqual(['f/inner', 'f/innermost'])
		expect(resolved?.containingFlowPath).toBe('f/innermost')
		expect(resolved?.module?.id).toBe('leaf')
	})

	it('still reports the containing flow when the step is not one of its modules', async () => {
		const resolved = await resolveExpandedSubflowStep(
			'subflow:a:b:leaf-tool-0',
			rootModules,
			loadFlowModules
		)
		expect(resolved?.containingFlowPath).toBe('f/innermost')
		expect(resolved?.module).toBeUndefined()
	})
})
