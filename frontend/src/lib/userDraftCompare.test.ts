import { describe, it, expect, vi } from 'vitest'

// Importing the module pulls in the generated client and the syncer's
// localStorage / pagehide wiring; stub the same surfaces the syncer tests do
// so the pure-function import stays side-effect free.
vi.mock('./gen', () => ({ DraftService: { updateDraft: vi.fn() } }))
vi.mock('./gen/core/OpenAPI', () => ({ OpenAPI: { BASE: '' } }))
vi.mock('./userDraftDbSyncer.svelte', () => ({ UserDraftDbSyncer: { save: vi.fn() } }))

import { draftValuesEqual } from './userDraft.svelte'

describe('draftValuesEqual', () => {
	it('ignores a key the schema added but nobody filled in', () => {
		const deployed = { args: { host: 'example.com' } }
		const settled = { args: { host: 'example.com', port: '', ssl: false, tags: [] } }
		expect(draftValuesEqual(settled, deployed)).toBe(true)
	})

	it('still sees the same key once it holds something', () => {
		const deployed = { args: { host: 'example.com' } }
		expect(draftValuesEqual({ args: { host: 'example.com', port: '5432' } }, deployed)).toBe(false)
	})

	it('sees a value being cleared, because the baseline had one', () => {
		expect(draftValuesEqual({ args: { host: '' } }, { args: { host: 'example.com' } })).toBe(false)
	})

	it('treats 0 as a real value, not as empty', () => {
		expect(draftValuesEqual({ timeout: 0 }, {})).toBe(false)
	})

	it('drops a nested branch that empties out entirely', () => {
		expect(draftValuesEqual({ args: { auth: { token: '' } } }, {})).toBe(true)
	})

	it('keeps array positions when an element empties out', () => {
		expect(draftValuesEqual({ xs: [{ a: '' }, { a: 'x' }] }, { xs: [{}, { a: 'x' }] })).toBe(true)
		expect(draftValuesEqual({ xs: [{ a: 'x' }, { a: '' }] }, { xs: [{}, { a: 'x' }] })).toBe(false)
	})

	it('ignores server-managed metadata riding along on the deployed payload', () => {
		const deployed = {
			path: 'u/me/r',
			value: { host: 'h' },
			created_by: 'admin',
			created_at: '2026-01-01T00:00:00Z',
			is_oauth: false,
			inherited_labels: ['from-folder'],
			starred: true
		}
		expect(draftValuesEqual({ path: 'u/me/r', value: { host: 'h' } }, deployed)).toBe(true)
	})

	it('still compares labels, which are edited here', () => {
		expect(draftValuesEqual({ labels: ['a'] }, { labels: ['b'] })).toBe(false)
	})

	it('only ignores metadata names at the top level', () => {
		const a = { value: { created_by: 'someone' } }
		const b = { value: { created_by: 'someone else' } }
		expect(draftValuesEqual(a, b)).toBe(false)
	})
})
