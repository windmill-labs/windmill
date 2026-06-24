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

	it('is not poisoned by subflow result keys', () => {
		const ids = ['a', 'b']
		const state = stateWith([...ids, 'subflow:abcd', 'Result', 'failure'])
		expect(nextId(state, flowWith(ids))).toBe('c')
	})

	// A step renamed to a long lowercase word ("process") is a valid base-26 string and would
	// otherwise inflate the max; the length cutoff keeps such renames out of the sequence.
	it('is not poisoned by renames to long lowercase words or underscored ids', () => {
		const ids = ['a', 'b']
		const state = stateWith([...ids, 'process', 'my_step', 'failure'])
		expect(nextId(state, flowWith(ids))).toBe('c')
	})
})
