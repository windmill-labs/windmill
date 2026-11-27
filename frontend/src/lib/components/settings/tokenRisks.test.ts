import { describe, expect, it } from 'vitest'
import { tokenRisks, type TokenRiskFields } from './tokenRisks'

const now = new Date('2026-06-01T00:00:00Z')
const daysAgo = (d: number) => new Date(now.getTime() - d * 86400000).toISOString()

const base: TokenRiskFields = {
	label: 'ci',
	expiration: '2027-01-01T00:00:00Z',
	last_used_at: daysAgo(1),
	scopes: undefined,
	read_only: false,
	owner_super_admin: false,
	owner_workspace_admin: false
}

describe('tokenRisks', () => {
	it('flags each risk', () => {
		expect(tokenRisks(base, now)).toEqual([])
		expect(tokenRisks({ ...base, expiration: undefined }, now)).toEqual(['never_expires'])
		expect(tokenRisks({ ...base, last_used_at: daysAgo(91) }, now)).toEqual(['unused'])
		expect(tokenRisks({ ...base, last_used_at: daysAgo(89) }, now)).toEqual([])
		expect(
			tokenRisks({ ...base, owner_super_admin: true, owner_workspace_admin: true }, now)
		).toEqual(['superadmin'])
		expect(tokenRisks({ ...base, owner_workspace_admin: true }, now)).toEqual(['admin'])
	})

	it('does not treat a narrowed token as carrying its owner rights', () => {
		const owner = { owner_super_admin: true, owner_workspace_admin: true }
		expect(tokenRisks({ ...base, ...owner, scopes: ['jobs:read'] }, now)).toEqual([])
		expect(tokenRisks({ ...base, ...owner, read_only: true }, now)).toEqual([])
		expect(tokenRisks({ ...base, ...owner, scopes: ['if_jobs:filter_tags:prod'] }, now)).toEqual([
			'superadmin'
		])
	})

	it('leaves login sessions out', () => {
		const t = { ...base, expiration: undefined, owner_super_admin: true }
		expect(tokenRisks({ ...t, label: 'session' }, now)).toEqual([])
		expect(tokenRisks({ ...t, label: 'guest_session' }, now)).toEqual([])
	})
})
