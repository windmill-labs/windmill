import { describe, expect, it } from 'vitest'
import type { AclGrant } from '$lib/gen'
import { grantKey, groupGrants, revocablePrivileges, revokeScopeOf } from './aclScopes'

const table = (name: string) => ({ name, kind: 'TABLE' })

describe('grantKey', () => {
	it('tells apart a table and a function of the same name', () => {
		const row = (object: { name: string; kind: string; args?: string }) => ({
			grantee: 'analytics',
			privileges: ['SELECT'],
			objects: [object]
		})
		expect(grantKey(row(table('orders')))).not.toBe(
			grantKey(row({ name: 'orders', kind: 'FUNCTION', args: '' }))
		)
	})
})

describe('groupGrants', () => {
	// A row's revoke names every object in it, so a row must only hold what one revoke may take.
	it('folds the same privileges on objects of one kind, and nothing else', () => {
		const grants: AclGrant[] = [
			{ grantee: 'analytics', privileges: ['SELECT'], object: table('orders') },
			{ grantee: 'analytics', privileges: ['SELECT'], object: table('salaries') },
			{ grantee: 'operator', privileges: ['SELECT'], object: table('orders') },
			{ grantee: 'analytics', privileges: ['INSERT', 'SELECT'], object: table('events') },
			{ grantee: 'analytics', privileges: ['SELECT'], object: { name: 's', kind: 'SEQUENCE' } },
			{ grantee: 'analytics', privileges: ['SELECT'], future: 'TABLES' },
			{ grantee: 'analytics', privileges: ['USAGE'] }
		]
		expect(groupGrants(grants)).toEqual([
			{
				grantee: 'analytics',
				privileges: ['SELECT'],
				objects: [table('orders'), table('salaries')],
				future: undefined
			},
			{
				grantee: 'operator',
				privileges: ['SELECT'],
				objects: [table('orders')],
				future: undefined
			},
			{
				grantee: 'analytics',
				privileges: ['INSERT', 'SELECT'],
				objects: [table('events')],
				future: undefined
			},
			{
				grantee: 'analytics',
				privileges: ['SELECT'],
				objects: [{ name: 's', kind: 'SEQUENCE' }],
				future: undefined
			},
			{ grantee: 'analytics', privileges: ['SELECT'], objects: [], future: 'TABLES' },
			{ grantee: 'analytics', privileges: ['USAGE'], objects: [], future: undefined }
		])
	})
})

describe('revoke of a row', () => {
	it('takes back only what the editor may revoke on the database', () => {
		const row = { grantee: 'analytics', privileges: ['CONNECT', 'CREATE'], objects: [] }
		expect(revocablePrivileges(row, { kind: 'database' })).toEqual(['CREATE'])
		expect(revocablePrivileges(row, { kind: 'schema', schema: 'public' })).toEqual([
			'CONNECT',
			'CREATE'
		])
	})

	it('maps default privileges to their scope, and refuses the ones it has none for', () => {
		const row = (future?: string) => ({
			grantee: 'analytics',
			privileges: ['SELECT'],
			objects: [],
			future
		})
		expect(revokeScopeOf(row())).toBe('target')
		expect(revokeScopeOf(row('TABLES'))).toBe('future_tables')
		expect(revokeScopeOf(row('TYPES'))).toBeUndefined()
		expect(revokeScopeOf({ ...row(), objects: [{ name: 'mood', kind: 'TYPE' }] })).toBeUndefined()
	})
})
