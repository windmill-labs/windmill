import { describe, it, expect, vi, beforeEach } from 'vitest'

// Mock the generated client so the migration runs against in-memory drafts.
const listDrafts = vi.fn()
const getOwnDraft = vi.fn()
const updateDraft = vi.fn()
vi.mock('$lib/gen', () => ({
	DraftService: {
		listDrafts: (...a: any[]) => listDrafts(...a),
		getOwnDraft: (...a: any[]) => getOwnDraft(...a),
		updateDraft: (...a: any[]) => updateDraft(...a)
	}
}))

import { migrateOwnDraftPaths } from './draftPathMigration'

beforeEach(() => {
	localStorage.clear()
	listDrafts.mockReset()
	getOwnDraft.mockReset()
	updateDraft.mockReset()
	updateDraft.mockResolvedValue({ status: 'saved', current_timestamp: 'now' })
})

describe('migrateOwnDraftPaths', () => {
	it('rewrites a flow draft_path into path, drops draft_path, preserves created_at', async () => {
		listDrafts.mockResolvedValue([{ kind: 'flow', path: 'u/a/dep', mine: true }])
		getOwnDraft.mockResolvedValue({
			value: { path: 'u/a/dep', draft_path: 'u/a/renamed', summary: 's', value: { modules: [] } },
			created_at: '2020-01-01T00:00:00Z'
		})

		const changed = await migrateOwnDraftPaths('ws')

		expect(changed).toBe(true)
		const body = updateDraft.mock.calls[0][0].requestBody
		expect(body.value.path).toBe('u/a/renamed')
		expect('draft_path' in body.value).toBe(false)
		expect(body.force).toBe(true)
		expect(body.created_at).toBe('2020-01-01T00:00:00Z')
		// run-once flag set
		expect(localStorage.getItem('wm:draftPathMigration:v1:ws')).not.toBeNull()
	})

	it('drops a redundant draft_path even when it equals path (e.g. scripts)', async () => {
		listDrafts.mockResolvedValue([{ kind: 'script', path: 'u/a/s', mine: true }])
		getOwnDraft.mockResolvedValue({
			value: { path: 'u/a/typed', draft_path: 'u/a/typed', content: 'x' },
			created_at: 't'
		})
		await migrateOwnDraftPaths('ws')
		const body = updateDraft.mock.calls[0][0].requestBody
		expect(body.value.path).toBe('u/a/typed')
		expect('draft_path' in body.value).toBe(false)
	})

	it('leaves drafts without draft_path untouched', async () => {
		listDrafts.mockResolvedValue([{ kind: 'flow', path: 'u/a/f', mine: true }])
		getOwnDraft.mockResolvedValue({
			value: { path: 'u/a/f', value: { modules: [] } },
			created_at: 't'
		})
		const changed = await migrateOwnDraftPaths('ws')
		expect(changed).toBe(false)
		expect(updateDraft).not.toHaveBeenCalled()
	})

	it('skips drawer kinds (never carried draft_path) without fetching them', async () => {
		listDrafts.mockResolvedValue([
			{ kind: 'variable', path: 'u/a/secret', mine: true },
			{ kind: 'trigger_http', path: 'u/a/route', mine: true }
		])
		await migrateOwnDraftPaths('ws')
		expect(getOwnDraft).not.toHaveBeenCalled()
		expect(updateDraft).not.toHaveBeenCalled()
	})

	it('is a no-op once the flag is set (run-once)', async () => {
		localStorage.setItem('wm:draftPathMigration:v1:ws', 't')
		const changed = await migrateOwnDraftPaths('ws')
		expect(changed).toBe(false)
		expect(listDrafts).not.toHaveBeenCalled()
	})

	it('does not set the flag when listing fails (so it retries later)', async () => {
		listDrafts.mockRejectedValue(new Error('network'))
		const changed = await migrateOwnDraftPaths('ws')
		expect(changed).toBe(false)
		expect(localStorage.getItem('wm:draftPathMigration:v1:ws')).toBeNull()
	})
})
