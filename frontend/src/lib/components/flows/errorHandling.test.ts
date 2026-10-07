import { describe, it, expect } from 'vitest'
import { writable } from 'svelte/store'
import type { FlowModule } from '$lib/gen'
import type { StateStore } from '$lib/utils'
import type { ExtendedOpenFlow } from './types'
import { addBranch } from './branchOps'
import {
	catchAllIndex,
	errorHandlerIds,
	errorHandlerOf,
	suggestsErrorHandling,
	testsError
} from './errorHandling'

function step(id: string, continueOnError = false): FlowModule {
	return { id, value: { type: 'identity' }, continue_on_error: continueOnError } as FlowModule
}

function handler(id: string, exprs: string[]): FlowModule {
	return {
		id,
		value: {
			type: 'branchone',
			default_summary: 'On success',
			default: [],
			branches: exprs.map((expr) => ({ summary: '', expr, modules: [] }))
		}
	} as FlowModule
}

describe('errorHandlerOf', () => {
	it('recognises the branchone right after a step that continues on error and tests it', () => {
		const flow = { modules: [step('g', true), handler('b', ['!!results.g.error']), step('c')] }
		expect(errorHandlerOf(flow, 'g')?.id).toBe('b')
	})

	it('ignores a branchone that is not right after the step', () => {
		const flow = { modules: [step('g', true), step('c'), handler('b', ['!!results.g.error'])] }
		expect(errorHandlerOf(flow, 'g')).toBeUndefined()
	})

	it('ignores a step that does not continue on error', () => {
		const flow = { modules: [step('g'), handler('b', ['!!results.g.error'])] }
		expect(errorHandlerOf(flow, 'g')).toBeUndefined()
	})
})

describe('errorHandlerIds', () => {
	it('maps the handlers in nested step lists to the steps they guard', () => {
		const loop = {
			id: 'l',
			value: {
				type: 'forloopflow',
				modules: [step('g', true), handler('b', ['!!results.g?.error'])],
				skip_failures: false
			}
		} as FlowModule
		const flow = { modules: [step('a'), handler('x', ['!!results.a.error']), loop] }
		expect([...errorHandlerIds(flow)]).toEqual([['b', 'g']])
	})
})

describe('suggestsErrorHandling', () => {
	it('suggests it on a step whose failure stops the flow', () => {
		expect(suggestsErrorHandling({ modules: [step('g'), step('c')] }, 'g')).toBe(true)
	})

	it('does not suggest it when a loop around the step skips failures, or on a container', () => {
		const loop = (skip: boolean) =>
			({
				id: 'l',
				value: { type: 'forloopflow', modules: [step('g')], skip_failures: skip }
			}) as FlowModule
		expect(suggestsErrorHandling({ modules: [loop(false)] }, 'g')).toBe(true)
		expect(suggestsErrorHandling({ modules: [loop(true)] }, 'g')).toBe(false)
		expect(suggestsErrorHandling({ modules: [loop(false)] }, 'l')).toBe(false)
	})

	it('does not suggest a second handler when error branches already follow the step', () => {
		const flow = { modules: [step('g'), handler('b', ['!!results.g.error'])] }
		expect(suggestsErrorHandling(flow, 'g')).toBe(false)
	})
})

describe('addBranch on an error handler', () => {
	it('lands before the catch-all, pre-filled to test the error name', () => {
		const flow = {
			summary: '',
			value: { modules: [step('g', true), handler('b', ['!!results.g.error'])] }
		} as ExtendedOpenFlow
		const flowStore: StateStore<ExtendedOpenFlow> = { val: flow }
		const history = writable({ history: [] as ExtendedOpenFlow[], index: -1 })

		addBranch('b', { flowStore, history })

		const branches = (flow.value.modules[1].value as { branches: { expr: string }[] }).branches
		expect(branches.map((b) => b.expr)).toEqual([
			"results.g?.error?.name === ''",
			'!!results.g.error'
		])
	})
})

describe('testsError', () => {
	it('recognises the null-safe and bracket accessors', () => {
		expect(testsError('!!results.g?.error', 'g')).toBe(true)
		expect(testsError(`results?.["g"]?.['error']?.name === 'X'`, 'g')).toBe(true)
		expect(testsError('results.g.error_count > 0 || results.gg.error', 'g')).toBe(false)
	})

	it('does not read a test for success as one for the error', () => {
		expect(testsError('!results.g.error && results.g.count > 0', 'g')).toBe(false)
		expect(testsError('results.g?.error == null', 'g')).toBe(false)
		expect(testsError('results.g?.error != null', 'g')).toBe(true)
	})
})

describe('catchAllIndex', () => {
	it('finds a catch-all written in any truthiness form', () => {
		for (const expr of [
			'results.g?.error != null',
			'Boolean(results.g.error)',
			'( !! results.g?.error )'
		]) {
			expect(catchAllIndex(handler('b', ["results.g?.error?.name === 'X'", expr]), 'g')).toBe(1)
		}
		expect(catchAllIndex(handler('b', ['results.g?.error !== null']), 'g')).toBeUndefined()
	})
})
