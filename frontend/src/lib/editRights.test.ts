import { describe, expect, it } from 'vitest'
import type { UserExt } from '$lib/stores'
import { canEditItem, roleCanAuthor, roleCanDraft } from './editRights'

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

	// drafts.rs admits an admin before its operator branch; the item handlers do not.
	it('lets an admin who is also an operator save drafts, but not author code', () => {
		const adminOperator = user({ is_admin: true })
		expect(roleCanDraft('script', adminOperator, undefined)).toBe(true)
		expect(roleCanAuthor('script', adminOperator, undefined)).toBe(false)
	})

	// drafts.rs refuses an operator every draft but a builder flow, rights to write directly or not.
	it('refuses an operator drafts of what they may write directly', () => {
		expect(roleCanAuthor('trigger', user({}), undefined)).toBe(true)
		expect(roleCanDraft('trigger', user({}), undefined)).toBe(false)
		expect(roleCanDraft('resource', user({}), undefined)).toBe(false)
	})

	// The role alone is not enough: a builder still cannot edit a flow in a folder they only read.
	it('requires write access to the item on top of the role', () => {
		const builder = { builder_flows: true }
		expect(canEditItem('flow', 'f/demo/sum', {}, user({}), builder)).toBe(false)
		expect(canEditItem('flow', 'u/op/sum', {}, user({}), builder)).toBe(true)
	})
})
