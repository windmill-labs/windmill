import { describe, it, expect, beforeEach } from 'vitest'
import {
	purgeLegacyUserDrafts,
	__resetUserDraftLegacyMigrationForTesting
} from './userDraftLegacyMigration'

function encodeLegacy(value: unknown): string {
	return btoa(encodeURIComponent(JSON.stringify(value)))
}

const legacyApp = {
	grid: [],
	fullscreen: false,
	theme: undefined,
	unusedInlineScripts: [],
	hiddenInlineScripts: []
}

beforeEach(() => {
	localStorage.clear()
	__resetUserDraftLegacyMigrationForTesting()
})

describe('purgeLegacyUserDrafts', () => {
	it('drops a recognised legacy app draft without re-creating it under any key', () => {
		localStorage.setItem('app-u/me/dashboard', encodeLegacy(legacyApp))

		purgeLegacyUserDrafts()

		expect(localStorage.getItem('app-u/me/dashboard')).toBeNull()
		// The workspace-blind key is gone, NOT promoted to a guessed workspace.
		expect(localStorage.getItem('userdraft/w/main/app/u/me/dashboard')).toBeNull()
	})

	it('drops the empty-path legacy keys (`app` / `flow` / `rawapp` literals)', () => {
		localStorage.setItem('app', encodeLegacy(legacyApp))
		localStorage.setItem('flow', encodeLegacy({ flow: { summary: '', value: { modules: [] } } }))
		localStorage.setItem('rawapp', encodeLegacy({ files: {}, runnables: {}, data: {} }))

		purgeLegacyUserDrafts()

		expect(localStorage.getItem('app')).toBeNull()
		expect(localStorage.getItem('flow')).toBeNull()
		expect(localStorage.getItem('rawapp')).toBeNull()
	})

	it('drops recognised legacy flow and raw-app drafts', () => {
		localStorage.setItem(
			'flow-u/me/myflow',
			encodeLegacy({ flow: { summary: 'f', value: { modules: [] } }, selectedId: 'settings' })
		)
		localStorage.setItem(
			'rawapp-u/me/site',
			encodeLegacy({ files: { 'index.tsx': 'x' }, runnables: {}, data: {} })
		)

		purgeLegacyUserDrafts()

		expect(localStorage.getItem('flow-u/me/myflow')).toBeNull()
		expect(localStorage.getItem('rawapp-u/me/site')).toBeNull()
	})

	it('leaves the workspace-scoped interim keys untouched (migrateUserDraftsToDb owns those)', () => {
		const interim = JSON.stringify({ value: { modules: [] } })
		localStorage.setItem('userdraft/w/main/flow/u/me/keep', interim)

		purgeLegacyUserDrafts()

		expect(localStorage.getItem('userdraft/w/main/flow/u/me/keep')).toBe(interim)
	})

	it('leaves keys whose path does not match the legacy `u|f/owner/name` shape alone', () => {
		// `app-recent` / `app-some_other_app` look like the legacy prefix but the
		// suffix isn't a Windmill item path — a future feature might own them.
		localStorage.setItem('app-recent', 'whatever')
		localStorage.setItem('app-some_other_app', 'whatever')

		purgeLegacyUserDrafts()

		expect(localStorage.getItem('app-recent')).toBe('whatever')
		expect(localStorage.getItem('app-some_other_app')).toBe('whatever')
	})

	it('leaves legacy-shaped keys whose payload does not look like a Windmill draft', () => {
		// Matches LEGACY_PATH_SHAPE and decodes to valid JSON, but carries none of
		// the App/flow draft fields — treat as unrelated, do not delete.
		const unrelatedApp = encodeLegacy({ random: 'data', count: 7 })
		localStorage.setItem('app-u/me/dash', unrelatedApp)
		const unrelatedFlow = encodeLegacy({ stepsState: {} })
		localStorage.setItem('flow-u/me/bar', unrelatedFlow)

		purgeLegacyUserDrafts()

		expect(localStorage.getItem('app-u/me/dash')).toBe(unrelatedApp)
		expect(localStorage.getItem('flow-u/me/bar')).toBe(unrelatedFlow)
	})

	it('leaves a malformed (non-base64) legacy payload alone and does not throw', () => {
		localStorage.setItem('app-u/me/garbled', 'not-base64!!!')

		expect(() => purgeLegacyUserDrafts()).not.toThrow()
		expect(localStorage.getItem('app-u/me/garbled')).toBe('not-base64!!!')
	})

	it('is idempotent — once the sentinel is set, a later legacy key survives', () => {
		localStorage.setItem('app-u/me/a', encodeLegacy(legacyApp))
		purgeLegacyUserDrafts()
		expect(localStorage.getItem('app-u/me/a')).toBeNull()

		// A key written after the first run is NOT swept (the sentinel short-circuits).
		localStorage.setItem('app-u/me/b', encodeLegacy(legacyApp))
		purgeLegacyUserDrafts()
		expect(localStorage.getItem('app-u/me/b')).not.toBeNull()
	})

	it('purges multiple legacy entries in a single invocation', () => {
		localStorage.setItem('app-u/me/a', encodeLegacy(legacyApp))
		localStorage.setItem(
			'flow-u/me/b',
			encodeLegacy({ flow: { summary: '', value: { modules: [] } } })
		)
		localStorage.setItem('rawapp-u/me/c', encodeLegacy({ files: {}, runnables: {}, data: {} }))

		purgeLegacyUserDrafts()

		expect(localStorage.getItem('app-u/me/a')).toBeNull()
		expect(localStorage.getItem('flow-u/me/b')).toBeNull()
		expect(localStorage.getItem('rawapp-u/me/c')).toBeNull()
	})
})
