import { describe, expect, it, vi } from 'vitest'

// The module reaches the API for the username policy and the workspace list; the name
// helper touches neither.
vi.mock('./gen', () => ({ SettingService: {}, UserService: {}, WorkspaceService: {} }))
vi.mock('./stores', () => ({ usersWorkspaceStore: { set: () => {} } }))
vi.mock('./storeUtils', () => ({ switchWorkspace: () => {} }))
vi.mock('./cloud', () => ({ isCloudHosted: () => false }))

import { defaultWorkspaceName, usernameFromName } from './workspaceCreation'

describe('defaultWorkspaceName', () => {
	it('names the workspace after the person, not the address', () => {
		expect(defaultWorkspaceName(undefined, 'bob@example.com')).toBe("Bob's workspace")
		expect(defaultWorkspaceName(undefined, 'ada.lovelace@example.com')).toBe(
			"Ada Lovelace's workspace"
		)
		expect(defaultWorkspaceName(undefined, 'jean-luc_picard+wm@example.com')).toBe(
			"Jean Luc Picard Wm's workspace"
		)
	})

	it('prefers the name the login provider gave', () => {
		expect(defaultWorkspaceName('Ruben', 'r.k@example.com')).toBe("Ruben's workspace")
		// Blank is not a name: fall back rather than produce "'s workspace".
		expect(defaultWorkspaceName('   ', 'bob@example.com')).toBe("Bob's workspace")
	})

	it('falls back rather than offering a name the backend refuses', () => {
		// Over the 50-char cap the field would be prefilled with something rejected on submit.
		expect(defaultWorkspaceName('Bartholomew Maximilian Featherstonehaugh III', undefined)).toBe(
			'My workspace'
		)
		// Nothing to derive from at all.
		expect(defaultWorkspaceName(undefined, undefined)).toBe('My workspace')
		expect(defaultWorkspaceName(undefined, '@example.com')).toBe('My workspace')
	})
})

describe('usernameFromName', () => {
	// The `proper_username` constraint is `^[\w-]+$`, so a suggestion outside it is posted and
	// then refused by the database, with the form showing nothing that explains why.
	it('keeps only what the username constraint accepts', () => {
		expect(usernameFromName("O'Connor")).toBe('oconnor')
		expect(usernameFromName('alice+demo')).toBe('alicedemo')
		expect(usernameFromName('Jean-Luc')).toBe('jean-luc')
		expect(usernameFromName('ada.lovelace')).toBe('adalovelace')
	})

	it('answers undefined when nothing usable is left', () => {
		// The caller opens the full form instead of prefilling something unusable.
		expect(usernameFromName('++')).toBeUndefined()
		expect(usernameFromName('')).toBeUndefined()
	})
})
