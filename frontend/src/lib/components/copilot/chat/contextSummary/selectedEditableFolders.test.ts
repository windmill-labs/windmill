import { beforeEach, describe, expect, it, vi } from 'vitest'

const { session } = vi.hoisted(() => ({
	session: { email: 'alice@windmill.dev' } as { email?: string }
}))

vi.mock('$lib/stores', () => ({
	userStore: { subscribe: (run: (value: unknown) => void) => (run({ ...session }), () => {}) }
}))

import {
	getAgentContextSelection,
	isEditableFolderSelected,
	setAllEditableFoldersSelected,
	setEditableFolderSelected
} from './selectedEditableFolders'

describe('selected editable folders', () => {
	beforeEach(() => {
		localStorage.clear()
		session.email = 'alice@windmill.dev'
	})

	it('selects new scopes by default in all mode', () => {
		expect(isEditableFolderSelected('ws', 'folder:finance')).toBe(true)
		setEditableFolderSelected('ws', 'folder:finance', false)
		expect(isEditableFolderSelected('ws', 'folder:finance')).toBe(false)
		expect(isEditableFolderSelected('ws', 'folder:new_team')).toBe(true)
	})

	it('keeps future scopes deselected after selecting none', () => {
		setAllEditableFoldersSelected('ws', false)
		setEditableFolderSelected('ws', 'folder:finance', true)
		expect(isEditableFolderSelected('ws', 'folder:finance')).toBe(true)
		expect(isEditableFolderSelected('ws', 'folder:new_team')).toBe(false)
	})

	it('migrates the prototype boolean map and remains scoped by account', () => {
		localStorage.setItem(
			'wm_ai_editable_folders',
			JSON.stringify({ 'ws:alice@windmill.dev': { personal: false, 'folder:finance': true } })
		)
		expect(getAgentContextSelection('ws')).toEqual({
			version: 2,
			baseline: 'selected',
			overrides: ['personal']
		})
		session.email = 'bob@windmill.dev'
		expect(isEditableFolderSelected('ws', 'personal')).toBe(true)
	})
})
