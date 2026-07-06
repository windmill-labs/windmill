import { describe, it, expect } from 'vitest'
import { draftBaseIsStale } from './utils_draft_deploy'

// draftBaseIsStale compares a draft's base pointer against the deployed head
// of the item it was fetched with (`get_draft=true`). Shared by CompareDrafts
// and the session Edits drawer — a regression here silently hides (or
// fabricates) the "started from an older deployed version" warning.

describe('draftBaseIsStale', () => {
	it('script: stale iff the draft parent_hash differs from the deployed hash', () => {
		expect(draftBaseIsStale('script', { hash: 'v2', draft: { parent_hash: 'v1' } })).toBe(true)
		expect(draftBaseIsStale('script', { hash: 'v2', draft: { parent_hash: 'v2' } })).toBe(false)
	})

	it('script: no base pointer or no head → not stale (nothing to compare)', () => {
		expect(draftBaseIsStale('script', { hash: 'v2', draft: {} })).toBe(false)
		expect(draftBaseIsStale('script', { draft: { parent_hash: 'v1' } })).toBe(false)
	})

	it('flow: compares the pinned version_id against the deployed head', () => {
		expect(draftBaseIsStale('flow', { version_id: 7, draft: { version_id: 5 } })).toBe(true)
		expect(draftBaseIsStale('flow', { version_id: 7, draft: { version_id: 7 } })).toBe(false)
		expect(draftBaseIsStale('flow', { version_id: 7, draft: {} })).toBe(false)
	})

	it('app/raw_app: compares parent_version against the last of versions', () => {
		expect(draftBaseIsStale('app', { versions: [1, 2, 3], draft: { parent_version: 2 } })).toBe(
			true
		)
		expect(draftBaseIsStale('raw_app', { versions: [1, 2, 3], draft: { parent_version: 3 } })).toBe(
			false
		)
		expect(draftBaseIsStale('app', { versions: [], draft: { parent_version: 2 } })).toBe(false)
	})

	it('no draft on the response → not stale', () => {
		expect(draftBaseIsStale('script', { hash: 'v2' })).toBe(false)
		expect(draftBaseIsStale('script', undefined)).toBe(false)
	})
})
