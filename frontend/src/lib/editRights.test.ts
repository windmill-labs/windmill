import { describe, expect, it } from 'vitest'
import type { UserExt } from '$lib/stores'
import { canEditItem, roleCanAuthor } from './editRights'

function user(overrides: Partial<UserExt>): UserExt {
	return {
		email: 'op@windmill.dev',
		username: 'op',
		name: 'op',
		is_admin: false,
		is_super_admin: false,
		created_at: '',
		groups: [],
		pgroups: [],
		folders: [],
		folders_owners: [],
		folders_read: [],
		operator: true,
		...overrides
	} as UserExt
}

describe('edit rights', () => {
	it('lets a builder operator author flows and nothing else', () => {
		const builder = { builder_flows: true }
		expect(roleCanAuthor('flow', user({}), builder)).toBe(true)
		expect(roleCanAuthor('script', user({}), builder)).toBe(false)
		expect(roleCanAuthor('flow', user({}), undefined)).toBe(false)
	})

	// The role alone is not enough: a builder still cannot edit a flow in a folder they only read.
	it('requires write access to the item on top of the role', () => {
		const builder = { builder_flows: true }
		expect(canEditItem('flow', 'f/demo/sum', {}, user({}), builder)).toBe(false)
		expect(canEditItem('flow', 'u/op/sum', {}, user({}), builder)).toBe(true)
	})
})
