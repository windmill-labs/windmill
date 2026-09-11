import { describe, expect, it } from 'vitest'
import type { AclGrant } from '$lib/gen'
import {
	grantKey,
	groupGrants,
	revocablePrivileges,
	revokeScopeOf,
	unreachableSources
} from './aclScopes'

const table = (name: string) => ({ name, kind: 'TABLE' })
const from = (...roles: string[]) => roles.map((role) => ({ role, reachable: true }))
const byAdmin = (grant: Omit<AclGrant, 'sources'>): AclGrant => ({
	...grant,
	sources: from('admin')
})

describe('grantKey', () => {
	it('tells apart a table and a function of the same name', () => {
		const row = (object: { name: string; kind: string; args?: string }) => ({
			grantee: 'analytics',
			privileges: ['SELECT'],
			objects: [object],
			sources: from('admin')
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
		].map(byAdmin)
		const sources = from('admin')
		expect(groupGrants(grants)).toEqual([
			{
				grantee: 'analytics',
				privileges: ['SELECT'],
				objects: [table('orders'), table('salaries')],
				future: undefined,
				sources
			},
			{
				grantee: 'operator',
				privileges: ['SELECT'],
				objects: [table('orders')],
				future: undefined,
				sources
			},
			{
				grantee: 'analytics',
				privileges: ['INSERT', 'SELECT'],
				objects: [table('events')],
				future: undefined,
				sources
			},
			{
				grantee: 'analytics',
				privileges: ['SELECT'],
				objects: [{ name: 's', kind: 'SEQUENCE' }],
				future: undefined,
				sources
			},
			{ grantee: 'analytics', privileges: ['SELECT'], objects: [], future: 'TABLES', sources },
			{ grantee: 'analytics', privileges: ['USAGE'], objects: [], future: undefined, sources }
		])
	})

	// A revoke takes the row back from every source, so the row must name them all.
	it('keeps every source of the grants it folds', () => {
		const grants: AclGrant[] = [
			{
				grantee: 'analytics',
				privileges: ['SELECT'],
				object: table('orders'),
				sources: from('admin')
			},
			{
				grantee: 'analytics',
				privileges: ['SELECT'],
				object: table('salaries'),
				sources: from('admin', 'operator')
			}
		]
		expect(groupGrants(grants)[0].sources).toEqual(from('admin', 'operator'))
	})
})

describe('revoke of a row', () => {
	const row = (future?: string) => ({
		grantee: 'analytics',
		privileges: ['SELECT'],
		objects: [],
		future,
		sources: from('admin')
	})

	it('takes back only what the editor may revoke on the database', () => {
		const database = { ...row(), privileges: ['CONNECT', 'CREATE'] }
		expect(revocablePrivileges(database, { kind: 'database' })).toEqual(['CREATE'])
		expect(revocablePrivileges(database, { kind: 'schema', schema: 'public' })).toEqual([
			'CONNECT',
			'CREATE'
		])
	})

	it('maps default privileges to their scope, and refuses the ones it has none for', () => {
		expect(revokeScopeOf(row())).toBe('target')
		expect(revokeScopeOf(row('TABLES'))).toBe('future_tables')
		expect(revokeScopeOf(row('TYPES'))).toBeUndefined()
		expect(revokeScopeOf({ ...row(), objects: [{ name: 'mood', kind: 'TYPE' }] })).toBeUndefined()
	})

	// Whether a grant can be taken back depends on its object, so a row folding several objects is
	// only revocable if each of its grants is.
	it('offers none for a folded row with a source out of reach on any of its objects', () => {
		const grants: AclGrant[] = [
			{
				grantee: 'analytics',
				privileges: ['SELECT'],
				object: table('orders'),
				sources: from('admin')
			},
			{
				grantee: 'analytics',
				privileges: ['SELECT'],
				object: table('salaries'),
				sources: [{ role: 'admin', reachable: false }]
			}
		]
		const [folded] = groupGrants(grants)
		expect(folded.objects).toHaveLength(2)
		expect(revokeScopeOf(folded)).toBeUndefined()
		// Folding reads the grants, never rewrites them.
		expect(grants[0].sources[0].reachable).toBe(true)
	})

	// Postgres takes a grant back only through its source: offering the revoke would promise what
	// the plan then refuses.
	it('offers none for a row with a source out of reach', () => {
		const partly = {
			...row('TABLES'),
			sources: [...from('admin'), { role: 'postgres', reachable: false }]
		}
		expect(revokeScopeOf(partly)).toBeUndefined()
		expect(unreachableSources(partly)).toEqual(['postgres'])
		expect(unreachableSources(row('TABLES'))).toEqual([])
	})
})
