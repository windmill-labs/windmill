import { describe, it, expect, beforeEach } from 'vitest'
import {
	migrateLegacyUserDrafts,
	__resetUserDraftLegacyMigrationForTesting
} from './userDraftLegacyMigration'

function encodeLegacy(value: unknown): string {
	return btoa(encodeURIComponent(JSON.stringify(value)))
}

function wrapped<V>(value: V): string {
	return JSON.stringify({ value })
}

beforeEach(() => {
	localStorage.clear()
	__resetUserDraftLegacyMigrationForTesting()
})

describe('migrateLegacyUserDrafts', () => {
	it('migrates a legacy app draft to the workspace-scoped key with a { value } wrapper', () => {
		const legacyApp = {
			summary: 'my app',
			value: { foo: 'bar' },
			policy: {},
			path: 'u/me/dashboard'
		}
		localStorage.setItem('app-u/me/dashboard', encodeLegacy(legacyApp))

		migrateLegacyUserDrafts('main')

		expect(localStorage.getItem('app-u/me/dashboard')).toBeNull()
		expect(localStorage.getItem('userdraft/w/main/app/u/me/dashboard')).toBe(wrapped(legacyApp))
	})

	it('migrates a legacy empty-path app draft (the `app` literal key)', () => {
		const legacyApp = { summary: '', value: {}, policy: {}, path: '' }
		localStorage.setItem('app', encodeLegacy(legacyApp))

		migrateLegacyUserDrafts('main')

		expect(localStorage.getItem('app')).toBeNull()
		expect(localStorage.getItem('userdraft/w/main/app/')).toBe(wrapped(legacyApp))
	})

	it('migrates a legacy flow draft and strips the view-state envelope', () => {
		const flow = { summary: 'f', value: { modules: [] }, path: 'u/me/myflow' }
		const legacyBundle = {
			flow,
			path: 'u/me/myflow',
			selectedId: 'settings',
			draft_triggers: [{ id: 't1' }],
			selected_trigger: null,
			loadedFromHistory: undefined
		}
		localStorage.setItem('flow-u/me/myflow', encodeLegacy(legacyBundle))

		migrateLegacyUserDrafts('main')

		expect(localStorage.getItem('flow-u/me/myflow')).toBeNull()
		// Only the inner Flow survives; the view-state envelope is dropped.
		expect(localStorage.getItem('userdraft/w/main/flow/u/me/myflow')).toBe(wrapped(flow))
	})

	it('migrates a legacy raw-app draft, defaulting the new `summary` field', () => {
		const legacy = {
			files: { 'index.tsx': 'export default () => null' },
			runnables: {},
			data: { tables: [] }
		}
		localStorage.setItem('rawapp-u/me/site', encodeLegacy(legacy))

		migrateLegacyUserDrafts('main')

		expect(localStorage.getItem('rawapp-u/me/site')).toBeNull()
		expect(localStorage.getItem('userdraft/w/main/raw_app/u/me/site')).toBe(
			wrapped({ ...legacy, summary: '' })
		)
	})

	it('preserves an existing new-format entry instead of overwriting it', () => {
		// Old and new both exist for the same item — the new one is presumed
		// fresher.
		localStorage.setItem('app-u/me/dash', encodeLegacy({ value: 'old' }))
		const existingNew = wrapped({ value: 'new' })
		localStorage.setItem('userdraft/w/main/app/u/me/dash', existingNew)

		migrateLegacyUserDrafts('main')

		expect(localStorage.getItem('app-u/me/dash')).toBeNull()
		expect(localStorage.getItem('userdraft/w/main/app/u/me/dash')).toBe(existingNew)
	})

	it('is idempotent — the second invocation is a no-op', () => {
		localStorage.setItem(
			'app-u/me/dash',
			encodeLegacy({ summary: '', value: {}, policy: {}, path: 'u/me/dash' })
		)
		migrateLegacyUserDrafts('main')
		expect(localStorage.getItem('userdraft/w/main/app/u/me/dash')).not.toBeNull()

		// Drop the migrated entry to detect any re-migration attempt.
		localStorage.removeItem('userdraft/w/main/app/u/me/dash')
		// Drop the source too, so re-running couldn't even find a source.
		// (The sentinel alone should be enough; this just clarifies the intent.)
		migrateLegacyUserDrafts('main')
		expect(localStorage.getItem('userdraft/w/main/app/u/me/dash')).toBeNull()
	})

	it('skips entirely when no workspace is available', () => {
		localStorage.setItem(
			'app-u/me/dash',
			encodeLegacy({ summary: '', value: {}, policy: {}, path: 'u/me/dash' })
		)
		migrateLegacyUserDrafts('')

		expect(localStorage.getItem('app-u/me/dash')).not.toBeNull()
	})

	it('handles malformed legacy payloads without throwing', () => {
		localStorage.setItem('app-u/me/garbled', 'not-base64!!!')
		expect(() => migrateLegacyUserDrafts('main')).not.toThrow()
		// Migration didn't migrate, didn't crash — leaves the entry alone.
		expect(localStorage.getItem('app-u/me/garbled')).toBe('not-base64!!!')
	})

	it('migrates multiple legacy entries in a single invocation', () => {
		localStorage.setItem(
			'app-u/me/a',
			encodeLegacy({ summary: '', value: {}, policy: {}, path: 'u/me/a' })
		)
		localStorage.setItem(
			'flow-u/me/b',
			encodeLegacy({ flow: { summary: '', value: { modules: [] }, path: 'u/me/b' } })
		)
		localStorage.setItem(
			'rawapp-u/me/c',
			encodeLegacy({ files: {}, runnables: {}, data: { tables: [] } })
		)

		migrateLegacyUserDrafts('main')

		expect(localStorage.getItem('userdraft/w/main/app/u/me/a')).not.toBeNull()
		expect(localStorage.getItem('userdraft/w/main/flow/u/me/b')).not.toBeNull()
		expect(localStorage.getItem('userdraft/w/main/raw_app/u/me/c')).not.toBeNull()
	})
})
