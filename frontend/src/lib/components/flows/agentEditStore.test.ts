import { describe, expect, it } from 'vitest'
import type { FlowModule } from '$lib/gen'
import {
	getAgentEditingPath,
	setAgentEditingPath,
	reanchorAgentEditsAcross
} from './agentEditStore.svelte'

function tool(id: string): object {
	return { id, value: { type: 'rawscript', tool_type: 'flowmodule', input_transforms: {} } }
}

function agentFork(id: string, tools: object[]): FlowModule {
	return { id, value: { type: 'aiagent', tools, input_transforms: {} } } as unknown as FlowModule
}

describe('reanchorAgentEditsAcross', () => {
	it('re-keys edit entries to the cloned tools arrays by module id', () => {
		const toolsA: object[] = [tool('t1')]
		const toolsB: object[] = [tool('t2')]
		let modules = [agentFork('a', toolsA), agentFork('b', toolsB)]
		setAgentEditingPath(toolsA, 'f/agents/one')
		setAgentEditingPath(toolsB, 'f/agents/two')

		reanchorAgentEditsAcross(
			() => modules,
			() => {
				// stand-in for refreshStateStore's $state.snapshot deep clone
				modules = JSON.parse(JSON.stringify(modules))
			}
		)

		expect(getAgentEditingPath(toolsA)).toBeUndefined()
		expect(getAgentEditingPath((modules[0].value as any).tools)).toBe('f/agents/one')
		expect(getAgentEditingPath((modules[1].value as any).tools)).toBe('f/agents/two')
	})

	it('drops the entry when the module is gone after the refresh', () => {
		const tools: object[] = [tool('t1')]
		let modules = [agentFork('a', tools)]
		setAgentEditingPath(tools, 'f/agents/one')

		reanchorAgentEditsAcross(
			() => modules,
			() => {
				modules = []
			}
		)

		expect(getAgentEditingPath(tools)).toBeUndefined()
	})
})
