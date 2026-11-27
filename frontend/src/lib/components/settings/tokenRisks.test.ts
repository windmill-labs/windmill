import { describe, expect, it } from 'vitest'
import { tokenRisks, type TokenRiskFields } from './tokenRisks'

const now = new Date('2026-06-01T00:00:00Z')
const daysAgo = (d: number) => new Date(now.getTime() - d * 86400000).toISOString()

const base: TokenRiskFields = {
	label: 'ci',
	expiration: '2027-01-01T00:00:00Z',
	last_used_at: daysAgo(1),
	scopes: undefined,
	workspace_id: 'prod',
	read_only: false,
	super_admin: false,
	workspace_admin: false
}

describe('tokenRisks', () => {
	it('flags each risk', () => {
		expect(tokenRisks(base, now)).toEqual([])
		expect(tokenRisks({ ...base, expiration: undefined }, now)).toEqual(['never_expires'])
		expect(tokenRisks({ ...base, workspace_id: undefined }, now)).toEqual(['all_workspaces'])
		expect(tokenRisks({ ...base, last_used_at: daysAgo(91) }, now)).toEqual(['unused'])
		expect(tokenRisks({ ...base, last_used_at: daysAgo(89) }, now)).toEqual([])
		expect(tokenRisks({ ...base, super_admin: true, workspace_admin: true }, now)).toEqual([
			'superadmin'
		])
		expect(tokenRisks({ ...base, workspace_admin: true }, now)).toEqual(['admin'])
	})

	it('does not treat a narrowed token as carrying its rights', () => {
		const rights = { super_admin: true, workspace_admin: true }
		expect(tokenRisks({ ...base, ...rights, scopes: ['jobs:read'] }, now)).toEqual([])
		expect(tokenRisks({ ...base, ...rights, read_only: true }, now)).toEqual([])
		expect(tokenRisks({ ...base, ...rights, scopes: ['if_jobs:filter_tags:prod'] }, now)).toEqual([
			'superadmin'
		])
	})

	it('leaves tokens the server manages out', () => {
		const t = { ...base, expiration: undefined, workspace_id: undefined, super_admin: true }
		for (const label of [
			'session',
			'guest_session',
			'ephemeral-script',
			'mcp-oauth-x',
			'impersonation:a'
		])
			expect(tokenRisks({ ...t, label }, now)).toEqual([])
		expect(tokenRisks({ ...t, label: 'cli-login:bob' }, now)).toEqual([
			'superadmin',
			'all_workspaces',
			'never_expires'
		])
	})
})
