import { describe, expect, it } from 'vitest'

import { getUnusedInstanceDbName } from './utils.svelte'

describe('getUnusedInstanceDbName', () => {
	it('scopes the name to the workspace with the given prefix', () => {
		expect(getUnusedInstanceDbName('dt', 'myworkspace', [])).toBe('dt_myworkspace')
		expect(getUnusedInstanceDbName('dl', 'myworkspace', [])).toBe('dl_myworkspace')
	})

	it('lowercases and replaces hyphens with underscores', () => {
		expect(getUnusedInstanceDbName('dt', 'My-Team', [])).toBe('dt_my_team')
	})

	it('appends an incrementing suffix when the name is already used', () => {
		expect(getUnusedInstanceDbName('dt', 'abc', ['dt_abc'])).toBe('dt_abc_1')
		expect(getUnusedInstanceDbName('dt', 'abc', ['dt_abc', 'dt_abc_1'])).toBe('dt_abc_2')
	})

	it('skips over already-used suffixed names', () => {
		expect(getUnusedInstanceDbName('dt', 'abc', ['dt_abc', 'dt_abc_2'])).toBe('dt_abc_1')
		expect(getUnusedInstanceDbName('dt', 'abc', ['dt_abc', 'dt_abc_1', 'dt_abc_2'])).toBe(
			'dt_abc_3'
		)
	})

	it('accepts any iterable of used names', () => {
		expect(getUnusedInstanceDbName('dt', 'abc', new Set(['dt_abc']))).toBe('dt_abc_1')
	})

	it('truncates the base name to the postgres 63-char identifier limit', () => {
		const longId = 'w'.repeat(100)
		const name = getUnusedInstanceDbName('dt', longId, [])
		expect(name.length).toBe(63)
		expect(name.startsWith('dt_')).toBe(true)
	})

	it('keeps the result within 63 chars even when appending a suffix', () => {
		const longId = 'w'.repeat(100)
		const base = getUnusedInstanceDbName('dt', longId, []) // length 63
		const name = getUnusedInstanceDbName('dt', longId, [base])
		expect(name.length).toBeLessThanOrEqual(63)
		expect(name.endsWith('_1')).toBe(true)
	})
})
