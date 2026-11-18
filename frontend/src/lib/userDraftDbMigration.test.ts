import { describe, it, expect, beforeEach, vi } from 'vitest'

// Service layer is mocked: the migration must dedup against the deployed value
// without making real network calls.
const updateDraft = vi.fn(async (..._args: any[]) => ({ status: 'created' as const }))
const getScriptByPath = vi.fn()
const getFlowByPath = vi.fn()
const getAppByPath = vi.fn()

vi.mock('./gen', () => ({
	DraftService: { updateDraft: (...a: unknown[]) => updateDraft(...(a as [])) },
	ScriptService: { getScriptByPath: (...a: unknown[]) => getScriptByPath(...(a as [])) },
	FlowService: { getFlowByPath: (...a: unknown[]) => getFlowByPath(...(a as [])) },
	AppService: { getAppByPath: (...a: unknown[]) => getAppByPath(...(a as [])) }
}))

// `migrateApp` mutates an App in place; the deployed fixtures below are already
// in migrated shape, so a no-op keeps the dedup comparison exact.
vi.mock('./components/apps/migrateApp', () => ({ migrateApp: vi.fn() }))
vi.mock('./toast', () => ({ sendUserToast: vi.fn() }))
vi.mock('./userNamespace', () => ({ getUsernameForNamespace: () => 'me' }))
vi.mock('./utils/uuid', () => ({ randomUUID: () => 'fixed-uuid' }))

import { migrateUserDraftsToDb } from './userDraftDbMigration'

function lsKey(kind: string, path: string): string {
	return `userdraft/w/main/${kind}/${path}`
}

function setDraft(kind: string, path: string, value: unknown): string {
	const key = lsKey(kind, path)
	localStorage.setItem(key, JSON.stringify({ value, lastWrittenAt: 123 }))
	return key
}

beforeEach(() => {
	localStorage.clear()
	vi.clearAllMocks()
	updateDraft.mockResolvedValue({ status: 'created' })
})

describe('migrateUserDraftsToDb dedup', () => {
	it('drops a draft deep-equal to the deployed script without uploading it', async () => {
		const deployed = { path: 'u/me/s', summary: 'hi', content: 'x', language: 'bun' }
		getScriptByPath.mockResolvedValue(deployed)
		const key = setDraft('script', 'u/me/s', { ...deployed })

		await migrateUserDraftsToDb()

		expect(getScriptByPath).toHaveBeenCalledWith({
			workspace: 'main',
			path: 'u/me/s',
			getDraft: false
		})
		expect(updateDraft).not.toHaveBeenCalled()
		expect(localStorage.getItem(key)).toBeNull()
	})

	it('uploads a draft that differs from the deployed script', async () => {
		getScriptByPath.mockResolvedValue({
			path: 'u/me/s',
			summary: 'hi',
			content: 'x',
			language: 'bun'
		})
		const key = setDraft('script', 'u/me/s', {
			path: 'u/me/s',
			summary: 'hi',
			content: 'EDITED',
			language: 'bun'
		})

		await migrateUserDraftsToDb()

		expect(updateDraft).toHaveBeenCalledTimes(1)
		expect(localStorage.getItem(key)).toBeNull()
	})

	it('treats `{ field: undefined }` and an absent field as equal (json normalization)', async () => {
		// The draft table stores JSON, which strips `undefined` keys — the
		// comparison must too, or a draft that only differs by an undefined key
		// would never dedup.
		getFlowByPath.mockResolvedValue({ summary: 'f', value: { modules: [] } })
		const key = setDraft('flow', 'u/me/f', {
			summary: 'f',
			value: { modules: [] },
			labels: undefined
		})

		await migrateUserDraftsToDb()

		expect(updateDraft).not.toHaveBeenCalled()
		expect(localStorage.getItem(key)).toBeNull()
	})

	it('ignores server-managed metadata fields on the deployed flow payload', async () => {
		// The deployed flow carries read-time metadata (workspace_id, edited_by,
		// version_id, is_draft, timestamps) that the editor's draft content never
		// holds — they must not block the dedup.
		getFlowByPath.mockResolvedValue({
			workspace_id: 'admins',
			path: 'u/me/f',
			summary: 'f',
			value: { modules: [] },
			edited_by: 'admin@windmill.dev',
			edited_at: '2026-01-01T00:00:00Z',
			archived: false,
			schema: {},
			extra_perms: {},
			version_id: 2,
			is_draft: false,
			draft_saved_at: '2026-01-01T00:00:01Z'
		})
		const key = setDraft('flow', 'u/me/f', {
			path: 'u/me/f',
			summary: 'f',
			value: { modules: [] },
			archived: false,
			schema: {}
		})

		await migrateUserDraftsToDb()

		expect(updateDraft).not.toHaveBeenCalled()
		expect(localStorage.getItem(key)).toBeNull()
	})

	it('compares app drafts against the deployed `.value`', async () => {
		const appValue = {
			grid: [],
			fullscreen: false,
			unusedInlineScripts: [],
			hiddenInlineScripts: []
		}
		getAppByPath.mockResolvedValue({ value: { ...appValue } })
		const key = setDraft('app', 'u/me/a', { ...appValue })

		await migrateUserDraftsToDb()

		expect(getAppByPath).toHaveBeenCalledWith({
			workspace: 'main',
			path: 'u/me/a',
			getDraft: false
		})
		expect(updateDraft).not.toHaveBeenCalled()
		expect(localStorage.getItem(key)).toBeNull()
	})

	it('uploads when there is no deployed item (fetch rejects)', async () => {
		getScriptByPath.mockRejectedValue(new Error('404'))
		const key = setDraft('script', 'u/me/new', { path: 'u/me/new', content: 'x' })

		await migrateUserDraftsToDb()

		expect(updateDraft).toHaveBeenCalledTimes(1)
		expect(localStorage.getItem(key)).toBeNull()
	})

	it('skips the deployed fetch for a pathless /add draft and uploads at a minted path', async () => {
		// A legacy `/add` autosave has an empty path; there is no deployed item to
		// dedup against, so it uploads to a freshly minted `u/{user}/draft_{uuid}`.
		setDraft('script', '', { path: '', content: 'x' })

		await migrateUserDraftsToDb()

		expect(getScriptByPath).not.toHaveBeenCalled()
		expect(updateDraft).toHaveBeenCalledTimes(1)
		expect(updateDraft.mock.calls[0][0]).toMatchObject({
			kind: 'script',
			// `mintDraftAddPath` dashes→underscores (path segments are word chars).
			path: 'u/me/draft_fixed_uuid'
		})
	})

	it('does not dedup unsupported kinds (e.g. variable) — uploads as before', async () => {
		const key = setDraft('variable', 'u/me/v', { value: 'secret' })

		await migrateUserDraftsToDb()

		expect(getScriptByPath).not.toHaveBeenCalled()
		expect(getAppByPath).not.toHaveBeenCalled()
		expect(updateDraft).toHaveBeenCalledTimes(1)
		expect(localStorage.getItem(key)).toBeNull()
	})
})
