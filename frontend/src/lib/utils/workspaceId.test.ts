import { describe, expect, it } from 'vitest'

import { WORKSPACE_ID_MAX_LENGTH, toWorkspaceId, validateWorkspaceId } from './workspaceId'

// This module was extracted so the import wizard and the create-workspace form
// could not drift on what a workspace id is. These pin the contract they share.

describe('validateWorkspaceId', () => {
	it('accepts dash-separated groups of word characters', () => {
		for (const id of ['a', 'prod', 'my_ws', 'a-b-c', 'a1-2b', 'wm-fork-x']) {
			expect(validateWorkspaceId(id), id).toBeUndefined()
		}
	})

	it('rejects leading, trailing and doubled dashes, and anything outside \\w', () => {
		for (const id of ['-a', 'a-', 'a--b', '', 'a b', 'a.b', 'a/b', 'é']) {
			expect(validateWorkspaceId(id), id).toMatch(/letters, numbers and dashes/)
		}
	})

	it('measures length against effectiveId, but the character rule against what was typed', () => {
		const typed = 'a'.repeat(45)
		// The typed id is fine on its own...
		expect(validateWorkspaceId(typed)).toBeUndefined()
		// ...but a fork submits it prefixed, and that is what the backend stores.
		const prefixed = `wm-fork-${typed}`
		expect(prefixed.length).toBeGreaterThan(WORKSPACE_ID_MAX_LENGTH)
		expect(validateWorkspaceId(typed, prefixed)).toMatch(/too long/)
	})

	it('reports the character problem before the length one', () => {
		expect(validateWorkspaceId('a b'.padEnd(80, 'c'))).toMatch(/letters, numbers and dashes/)
	})

	it('allows exactly the maximum length', () => {
		const id = 'a'.repeat(WORKSPACE_ID_MAX_LENGTH)
		expect(validateWorkspaceId(id)).toBeUndefined()
		expect(validateWorkspaceId(id + 'a')).toMatch(/too long/)
	})
})

describe('toWorkspaceId', () => {
	it('produces something validateWorkspaceId accepts', () => {
		for (const raw of [
			'Support automation',
			'  GitHub  Release   Dashboard  ',
			'a//b__c',
			'Ünïcødé nåme',
			'---leading and trailing---',
			'MiXeD CaSe'
		]) {
			const id = toWorkspaceId(raw)
			expect(validateWorkspaceId(id), `${raw} -> ${id}`).toBeUndefined()
		}
	})

	it('lowercases, collapses runs into single dashes, and trims them', () => {
		expect(toWorkspaceId('Support automation')).toBe('support-automation')
		expect(toWorkspaceId('a//b')).toBe('a-b')
		expect(toWorkspaceId('  spaced  out  ')).toBe('spaced-out')
	})

	it('clips to the maximum length without leaving a trailing dash', () => {
		// Slicing at 50 lands mid-separator here; the result must still be valid.
		const raw = `${'a'.repeat(WORKSPACE_ID_MAX_LENGTH - 1)} tail`
		const id = toWorkspaceId(raw)
		expect(id.length).toBeLessThanOrEqual(WORKSPACE_ID_MAX_LENGTH)
		expect(id.endsWith('-')).toBe(false)
		expect(validateWorkspaceId(id)).toBeUndefined()
	})
})

describe('validateWorkspaceId — the reserved id', () => {
	// `check_w_id_conflict` refuses it, and `existsWorkspace` reports it free, so without
	// this the wizard walks the user to the last step before the create fails.
	it('refuses `global`', () => {
		expect(validateWorkspaceId('global')).toMatch(/not allowed/i)
	})

	it('refuses it as the effective id too', () => {
		expect(validateWorkspaceId('wm-fork-x', 'global')).toMatch(/not allowed/i)
	})

	it('allows a fork named `global`, which reaches the backend as `wm-fork-global`', () => {
		expect(validateWorkspaceId('global', 'wm-fork-global')).toBeUndefined()
	})

	it('still accepts ids that merely contain it', () => {
		expect(validateWorkspaceId('global-ops')).toBeUndefined()
		expect(validateWorkspaceId('my-global')).toBeUndefined()
	})
})
