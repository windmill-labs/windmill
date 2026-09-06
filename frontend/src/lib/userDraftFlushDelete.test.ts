import { describe, it, expect, beforeEach, vi } from 'vitest'

// The flush stands in for the POST it drives: whatever write actually lands is
// what publishes the hint, which is the only thing that tells a delete apart
// from an edit that displaced it.
let landed: 'delete' | 'upsert' | 'nothing' = 'delete'
vi.mock('./userDraftDbSyncer.svelte', () => ({
	UserDraftDbSyncer: {
		flush: vi.fn(async ({ workspace, itemKind, path }: any) => {
			if (landed === 'nothing') return
			const { setLocalDraftHint } = await import('./localDraftHints.svelte')
			setLocalDraftHint(workspace, itemKind, path, landed === 'upsert')
		}),
		save: vi.fn()
	}
}))
vi.mock('./gen', () => ({ DraftService: { updateDraft: vi.fn() } }))
vi.mock('./gen/core/OpenAPI', () => ({ OpenAPI: { BASE: '' } }))

import { flushDraftDelete } from './userDraft.svelte'

let n = 0
let path = ''
beforeEach(() => {
	// A fresh key per case: hints persist by design, so a reused one would carry
	// the previous case's answer.
	path = `u/me/v${n++}`
})

describe('flushDraftDelete', () => {
	it('confirms a delete that landed', async () => {
		landed = 'delete'
		expect(await flushDraftDelete('variable', path, { workspace: 'ws' })).toBe(true)
	})

	// Discard leaves the form editable, so typing after it can replace the queued
	// `value: null` with an upsert. The pipeline settles either way; only the item
	// still being there tells the caller not to leave the editor.
	it('rejects a delete displaced by a later edit', async () => {
		landed = 'upsert'
		expect(await flushDraftDelete('variable', path, { workspace: 'ws' })).toBe(false)
	})

	it('rejects a delete that never landed', async () => {
		landed = 'nothing'
		expect(await flushDraftDelete('variable', path, { workspace: 'ws' })).toBe(false)
	})
})
