import { describe, it, expect, vi } from 'vitest'

// Importing the syncer pulls in the generated client and its localStorage /
// pagehide wiring; stub the same surfaces the other syncer test does so the
// pure-function import stays side-effect free.
vi.mock('./gen', () => ({
	DraftService: { updateDraft: vi.fn() }
}))
vi.mock('./gen/core/OpenAPI', () => ({ OpenAPI: { BASE: '' } }))
vi.mock('./localDraftHints.svelte', () => ({ setLocalDraftHint: vi.fn() }))

import { sanitizeDraftValueForSave } from './userDraftDbSyncer.svelte'

describe('sanitizeDraftValueForSave', () => {
	it('strips hash and assets from a script draft', () => {
		const value = {
			path: 'u/me/foo',
			summary: 'hi',
			content: 'export function main() {}',
			hash: '123456789',
			assets: [{ path: 's3://bucket/x', kind: 's3object' }]
		}
		const cleaned = sanitizeDraftValueForSave('script', value) as any
		expect(cleaned).not.toHaveProperty('hash')
		expect(cleaned).not.toHaveProperty('assets')
		expect(cleaned.content).toBe('export function main() {}')
		expect(cleaned.summary).toBe('hi')
	})

	it('does not mutate the input object', () => {
		const value = { path: 'u/me/foo', hash: 'h', assets: [] }
		sanitizeDraftValueForSave('script', value)
		expect(value).toHaveProperty('hash', 'h')
		expect(value).toHaveProperty('assets')
	})

	it('returns the same reference when no omitted field is present', () => {
		const value = { path: 'u/me/foo', summary: 'hi' }
		expect(sanitizeDraftValueForSave('script', value)).toBe(value)
	})

	it('leaves non-script kinds untouched even if they carry hash/assets', () => {
		const value = { hash: 'h', assets: [] }
		expect(sanitizeDraftValueForSave('flow', value)).toBe(value)
		expect(sanitizeDraftValueForSave('app', value)).toBe(value)
	})

	it('passes through null (the delete signal) and non-objects', () => {
		expect(sanitizeDraftValueForSave('script', null)).toBeNull()
		expect(sanitizeDraftValueForSave('script', 'hello')).toBe('hello')
		const arr = [1, 2, 3]
		expect(sanitizeDraftValueForSave('script', arr)).toBe(arr)
	})
})
