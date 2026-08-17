import { describe, expect, it } from 'vitest'

import type { FlowValue } from '$lib/gen'
import { modulesWithRetryOrSleep } from './utils.svelte'

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
