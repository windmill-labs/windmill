import { describe, expect, it } from 'vitest'

import type { OpenFlow } from '$lib/gen'
import type { FlowState } from './flowState'
import { nextId } from './flowModuleNextId'

function flowWith(ids: string[]): OpenFlow {
	return {
		summary: '',
		value: {
			modules: ids.map((id) => ({ id, value: { type: 'identity' } as any }))
		}
	} as OpenFlow
}

function stateWith(keys: string[]): FlowState {
	return Object.fromEntries(keys.map((k) => [k, {}])) as FlowState
}

describe('nextId', () => {
	it('produces a, b, c, ... for a fresh flow', () => {
		expect(nextId(stateWith(['failure']), flowWith([]))).toBe('a')
		expect(nextId(stateWith(['a', 'failure']), flowWith(['a']))).toBe('b')
		expect(nextId(stateWith(['a', 'b', 'c', 'failure']), flowWith(['a', 'b', 'c']))).toBe('d')
	})

	it('ignores the reserved failure/preprocessor keys always present in flowState', () => {
		expect(nextId(stateWith(['failure', 'preprocessor']), flowWith([]))).toBe('a')
	})

	// Regression: copy ids ("z2"), subflow result keys and other non-canonical keys land in
	// flowState; charsToNumber on them used to leak into the max and made new steps jump to
	// garbage ids like "bzw".
	it('is not poisoned by copy ids', () => {
		const ids = ['a', 'b', 'c']
		const state = stateWith([...ids, 'c2', 'a2', 'z2', 'c10', 'failure'])
		expect(nextId(state, flowWith(ids))).toBe('d')
	})

	it('is not poisoned by subflow result keys or user-renamed ids', () => {
		const ids = ['a', 'b']
		const state = stateWith([...ids, 'subflow:abcd', 'my_step', 'Result', 'failure'])
		expect(nextId(state, flowWith(ids))).toBe('c')
	})

	it('still accounts for real auto ids beyond length 3', () => {
		const ids = ['a', 'aaa', 'zzz']
		expect(nextId(stateWith([...ids, 'failure']), flowWith(ids))).toBe('aaaa')
	})
})
