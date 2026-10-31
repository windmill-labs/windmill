import { describe, expect, it } from 'vitest'
import type { RoleCapabilities } from '$lib/gen'
import type { UserExt } from '$lib/stores'
import { FULL_CAPABILITIES, legacyOperatorCapabilities } from '$lib/roleCapabilities'
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

const role = (overrides: Partial<RoleCapabilities>): RoleCapabilities => ({
	...legacyOperatorCapabilities(undefined),
	...overrides
})

describe('edit rights', () => {
	it('lets a compose-only flow role author flows and agents and nothing else', () => {
		const builder = role({ flows: 'compose' })
		expect(roleCanAuthor('flow', builder)).toBe(true)
		expect(roleCanAuthor('agent', builder)).toBe(true)
		expect(roleCanDraft('agent', builder)).toBe(true)
		expect(roleCanAuthor('agent', role({ apps: 'compose' }))).toBe(false)
		expect(roleCanAuthor('script', builder)).toBe(false)
		expect(roleCanAuthor('raw_app', builder)).toBe(false)
		expect(roleCanDraft('raw_app', builder)).toBe(false)
		expect(roleCanDraft('raw_app', role({ apps: 'compose' }))).toBe(true)
	})

	// apps.rs `require_full_apps`: a composing role builds full-code apps, never low-code ones.
	it('keeps low-code apps for full app authors', () => {
		expect(roleCanAuthor('app', role({ apps: 'compose' }))).toBe(false)
		expect(roleCanAuthor('raw_app', role({ apps: 'compose' }))).toBe(true)
		const scripter = { ...FULL_CAPABILITIES, apps: 'none' as const }
		expect(roleCanAuthor('script', scripter)).toBe(true)
		expect(roleCanAuthor('app', scripter)).toBe(false)
		expect(roleCanAuthor('raw_app', scripter)).toBe(false)
	})

	// drafts.rs admits a restricted role's draft of a kind it could deploy, and no other.
	it('lets a role draft schedules and triggers until the right is withdrawn', () => {
		expect(roleCanDraft('schedule', role({}))).toBe(true)
		expect(roleCanDraft('trigger', role({}))).toBe(true)
		expect(roleCanDraft('schedule', role({ manage_schedules: false }))).toBe(false)
		expect(roleCanDraft('trigger', role({ manage_triggers: false }))).toBe(false)
	})

	it('lets any role draft resources and variables, but scripts only with code', () => {
		expect(roleCanDraft('resource', role({}))).toBe(true)
		expect(roleCanDraft('variable', role({}))).toBe(true)
		expect(roleCanDraft('script', role({}))).toBe(false)
		expect(roleCanDraft('script', role({ code: true }))).toBe(true)
	})

	// The role alone is not enough: a builder still cannot edit a flow in a folder they only read.
	it('requires write access to the item on top of the role', () => {
		const builder = role({ flows: 'compose' })
		expect(canEditItem('flow', 'f/demo/sum', {}, user({}), builder)).toBe(false)
		expect(canEditItem('flow', 'u/op/sum', {}, user({}), builder)).toBe(true)
	})
})
