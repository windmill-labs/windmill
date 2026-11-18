import { describe, it, expect, vi } from 'vitest'

vi.mock('$lib/userNamespace', () => ({
	getUsernameForNamespace: () => 'alice'
}))

import { mintDraftPath, selectDraftStoragePath } from './mintDraftPath'

const MINTED = /^u\/alice\/draft_[0-9a-f_]+$/

describe('mintDraftPath', () => {
	it('mints a non-empty u/<username>/draft_<uuid> path', () => {
		const path = mintDraftPath()
		expect(path).not.toBe('')
		expect(path).toMatch(MINTED)
	})

	it('uses underscores, never dashes (path segments are word chars)', () => {
		expect(mintDraftPath()).not.toContain('-')
	})

	it('is unique per call', () => {
		expect(mintDraftPath()).not.toBe(mintDraftPath())
	})
})

describe('selectDraftStoragePath', () => {
	it('mints a non-empty path for a new item with no caller path', () => {
		// Regression: SDK create wrappers keyed autosave under '' → detached →
		// silently never POSTed. A new item must get a real, mintable key.
		const path = selectDraftStoragePath({ providedPaths: [undefined, ''], isNewItem: true })
		expect(path).toMatch(MINTED)
	})

	it('honors the first caller-provided path over minting (SDK stopgap wins)', () => {
		expect(
			selectDraftStoragePath({ providedPaths: ['u/bob/given', undefined], isNewItem: true })
		).toBe('u/bob/given')
	})

	it('falls through empties to the first non-empty provided path', () => {
		expect(
			selectDraftStoragePath({ providedPaths: ['', undefined, 'u/bob/existing'], isNewItem: false })
		).toBe('u/bob/existing')
	})

	it('stays detached ("") for a non-new item with no path (read-only view)', () => {
		expect(selectDraftStoragePath({ providedPaths: [undefined, ''], isNewItem: false })).toBe('')
	})
})
