import { describe, expect, it } from 'vitest'

import type { FlowValue, MemoryConfig, OpenFlow } from '$lib/gen'
import { cleanFlow, modulesWithRetryOrSleep } from './utils.svelte'

const constantRetry = { constant: { attempts: 1, seconds: 5 } }

function step(id: string, extra: Record<string, unknown> = {}) {
	return { id, value: { type: 'identity' }, ...extra } as any
}

describe('modulesWithRetryOrSleep', () => {
	it('reports retries and sleeps everywhere same_worker applies', () => {
		const flow: FlowValue = {
			modules: [
				step('a', { retry: constantRetry }),
				step('b', { sleep: { type: 'static', value: 3 } }),
				step('c'),
				{
					id: 'd',
					value: {
						type: 'forloopflow',
						modules: [step('e', { retry: constantRetry })],
						iterator: { type: 'static', value: [] },
						skip_failures: false
					}
				} as any
			],
			failure_module: step('failure', { retry: constantRetry }),
			preprocessor_module: step('preprocessor', { sleep: { type: 'static', value: 1 } })
		}

		expect(modulesWithRetryOrSleep(flow)).toEqual(['a', 'b', 'e', 'failure', 'preprocessor'])
	})

	it('ignores what same_worker does not govern: agent tools and attempt-less retries', () => {
		const flow: FlowValue = {
			modules: [
				step('a', { retry: { constant: { attempts: 0, seconds: 5 } } }),
				{
					id: 'b',
					value: { type: 'aiagent', tools: [step('tool', { retry: constantRetry })] }
				} as any
			]
		}

		expect(modulesWithRetryOrSleep(flow)).toEqual([])
	})
})

describe('cleanFlow memory_id stamping', () => {
	function agentFlow(memory: MemoryConfig): OpenFlow {
		return {
			summary: '',
			value: {
				modules: [
					{
						id: 'a',
						value: {
							type: 'aiagent',
							tools: [],
							input_transforms: { memory: { type: 'static', value: memory } }
						}
					} as any
				]
			}
		} as OpenFlow
	}

	function memoryOf(flow: OpenFlow): any {
		return (flow.value.modules[0].value as any).input_transforms.memory.value
	}

	// Two steps left without one share a memory key, which silently merges their
	// conversations.
	it('stamps one on the modes that persist, and nowhere else', () => {
		expect(memoryOf(cleanFlow(agentFlow({ kind: 'autocompacted' })))).toHaveProperty('memory_id')
		expect(memoryOf(cleanFlow(agentFlow({ kind: 'auto', context_length: 5 })))).toHaveProperty(
			'memory_id'
		)
		expect(memoryOf(cleanFlow(agentFlow({ kind: 'off' })))).not.toHaveProperty('memory_id')
	})
})
