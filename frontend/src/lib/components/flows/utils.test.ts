import { describe, expect, it } from 'vitest'

import type { FlowValue } from '$lib/gen'
import { modulesWithRetryOrSleep, normalizeAgentHistory } from './utils.svelte'

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

describe('normalizeAgentHistory', () => {
	const legacy = () => ({
		memory: {
			type: 'static',
			value: { kind: 'auto', context_length: 10, memory_id: '0f5c3a8e-1d2b-4c6a-9e7f-3b8d2a1c4e6f' }
		}
	})

	// Outside chat the baked id is still read for runs that pass none, and an older worker must keep
	// accepting the step, so a save leaves it exactly as it was.
	it('keeps a legacy baked memory id outside chat mode', () => {
		const transforms = legacy()
		normalizeAgentHistory(transforms, false)
		expect(transforms).toEqual(legacy())
	})

	it('drops a legacy baked memory id in chat mode, where it was never read', () => {
		const transforms = legacy()
		normalizeAgentHistory(transforms, true)
		expect(transforms.memory.value).toEqual({ kind: 'auto', context_length: 10 })
	})

	it('drops an empty baked memory id, which names no memory', () => {
		const transforms = {
			memory: { type: 'static', value: { kind: 'auto', context_length: 10, memory_id: '' } }
		}
		normalizeAgentHistory(transforms, false)
		expect(transforms.memory.value).toEqual({ kind: 'auto', context_length: 10 })
	})

	it('saves managed memory that keeps no messages as off, which is how it runs', () => {
		for (const context_length of [0, null, undefined]) {
			const transforms: Record<string, any> = {
				memory: { type: 'static', value: { kind: 'window', context_length } }
			}
			normalizeAgentHistory(transforms, false)
			expect(transforms.memory.value).toEqual({ kind: 'off' })
		}
	})

	it('does not persist an empty static memory id or message list', () => {
		const transforms: Record<string, any> = {
			memory_id: { type: 'static', value: ' ' },
			previous_messages: { type: 'static', value: [] }
		}
		normalizeAgentHistory(transforms, false)
		expect(transforms).toEqual({})
	})
})
