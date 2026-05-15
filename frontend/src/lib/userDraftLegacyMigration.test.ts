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

// Read a migrated entry, strip the GC `lastWrittenAt` stamp so assertions
// can match the `{ value }` shape regardless of when the migration ran.
function storedShape(key: string): string | null {
	const raw = localStorage.getItem(key)
	if (raw == null) return null
	const parsed = JSON.parse(raw)
	delete parsed.lastWrittenAt
	return JSON.stringify(parsed)
}

beforeEach(() => {
	localStorage.clear()
	__resetUserDraftLegacyMigrationForTesting()
})

describe('migrateLegacyUserDrafts', () => {
	it('migrates a legacy app draft to the workspace-scoped key with a { value } wrapper', () => {
		// Shape mirrors what the legacy AppEditor wrote: `encodeState($appStore)`,
		// i.e. the inner App value, not the wrapping AppWithLastVersion.
		const legacyApp = {
			grid: [],
			fullscreen: false,
			theme: undefined,
			unusedInlineScripts: [],
			hiddenInlineScripts: []
		}
		localStorage.setItem('app-u/me/dashboard', encodeLegacy(legacyApp))

		migrateLegacyUserDrafts('main')

		expect(localStorage.getItem('app-u/me/dashboard')).toBeNull()
		expect(storedShape('userdraft/w/main/app/u/me/dashboard')).toBe(wrapped(legacyApp))
	})

	it('migrates a legacy empty-path app draft (the `app` literal key)', () => {
		const legacyApp = {
			grid: [],
			fullscreen: false,
			unusedInlineScripts: [],
			hiddenInlineScripts: []
		}
		localStorage.setItem('app', encodeLegacy(legacyApp))

		migrateLegacyUserDrafts('main')

		expect(localStorage.getItem('app')).toBeNull()
		expect(storedShape('userdraft/w/main/app/')).toBe(wrapped(legacyApp))
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
		expect(storedShape('userdraft/w/main/flow/u/me/myflow')).toBe(wrapped(flow))
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
		expect(storedShape('userdraft/w/main/raw_app/u/me/site')).toBe(
			wrapped({ ...legacy, summary: '' })
		)
	})

	it('preserves an existing new-format entry instead of overwriting it', () => {
		// Old and new both exist for the same item — the new one is presumed
		// fresher.
		localStorage.setItem(
			'app-u/me/dash',
			encodeLegacy({
				grid: [],
				fullscreen: false,
				unusedInlineScripts: [],
				hiddenInlineScripts: []
			})
		)
		const existingNew = wrapped({ value: 'new' })
		localStorage.setItem('userdraft/w/main/app/u/me/dash', existingNew)

		migrateLegacyUserDrafts('main')

		expect(localStorage.getItem('app-u/me/dash')).toBeNull()
		expect(localStorage.getItem('userdraft/w/main/app/u/me/dash')).toBe(existingNew)
	})

	it('is idempotent — the second invocation is a no-op', () => {
		localStorage.setItem(
			'app-u/me/dash',
			encodeLegacy({
				grid: [],
				fullscreen: false,
				unusedInlineScripts: [],
				hiddenInlineScripts: []
			})
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
			encodeLegacy({
				grid: [],
				fullscreen: false,
				unusedInlineScripts: [],
				hiddenInlineScripts: []
			})
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

	it('leaves keys whose path does not match the legacy `u|f/owner/name` shape alone', () => {
		// A future feature or neighbouring code might pick a key like
		// `app-recent` for its own purposes. The path doesn't look like a
		// Windmill item path, so the migration must skip it.
		localStorage.setItem('app-recent', 'whatever')
		localStorage.setItem('app-some_other_app', 'whatever')
		// `flow-u/me/foo` matches the shape and would be migrated, but the
		// payload also needs to look like a Windmill draft (asserted below).
		localStorage.setItem('flow-u/me/foo', encodeLegacy({ flow: { value: { modules: [] } } }))

		migrateLegacyUserDrafts('main')

		expect(localStorage.getItem('app-recent')).toBe('whatever')
		expect(localStorage.getItem('app-some_other_app')).toBe('whatever')
		expect(localStorage.getItem('userdraft/w/main/flow/u/me/foo')).not.toBeNull()
	})

	it('skips legacy-shaped keys whose payload does not look like a Windmill draft', () => {
		// `app-u/me/dash` matches LEGACY_PATH_SHAPE and decodes to valid JSON,
		// but none of the App-shape fields (grid/fullscreen/theme/
		// unusedInlineScripts/hiddenInlineScripts) are present. Treat it as
		// unrelated and leave it untouched.
		const unrelated = encodeLegacy({ random: 'data', count: 7 })
		localStorage.setItem('app-u/me/dash', unrelated)
		const unrelatedFlow = encodeLegacy({ stepsState: {} })
		localStorage.setItem('flow-u/me/bar', unrelatedFlow)

		migrateLegacyUserDrafts('main')

		expect(localStorage.getItem('app-u/me/dash')).toBe(unrelated)
		expect(localStorage.getItem('userdraft/w/main/app/u/me/dash')).toBeNull()
		expect(localStorage.getItem('flow-u/me/bar')).toBe(unrelatedFlow)
		expect(localStorage.getItem('userdraft/w/main/flow/u/me/bar')).toBeNull()
	})

	it('migrates multiple legacy entries in a single invocation', () => {
		localStorage.setItem(
			'app-u/me/a',
			encodeLegacy({
				grid: [],
				fullscreen: false,
				unusedInlineScripts: [],
				hiddenInlineScripts: []
			})
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
