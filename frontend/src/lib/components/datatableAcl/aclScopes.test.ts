import { describe, expect, it } from 'vitest'
import type { AclGrant } from '$lib/gen'
import {
	blockingSources,
	grantKey,
	groupGrants,
	revocablePrivileges,
	revokeScopeOf,
	uncoveredCreators
} from './aclScopes'

const table = (name: string) => ({ name, kind: 'TABLE' })
const by = (role: string, privileges: string[], reachable = true) => ({
	role,
	privileges,
	reachable
})
const byAdmin = (grant: Omit<AclGrant, 'sources'>): AclGrant => ({
	...grant,
	sources: [by('admin', grant.privileges)]
})

describe('grantKey', () => {
	it('tells apart a table and a function of the same name', () => {
		const row = (object: { name: string; kind: string; args?: string }) => ({
			grantee: 'analytics',
			privileges: ['SELECT'],
			objects: [object],
			sources: [by('admin', ['SELECT'])]
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
		const rows = groupGrants(grants)
		expect(rows.map((r) => [r.grantee, r.privileges, r.objects, r.future])).toEqual([
			['analytics', ['SELECT'], [table('orders'), table('salaries')], undefined],
			['operator', ['SELECT'], [table('orders')], undefined],
			['analytics', ['INSERT', 'SELECT'], [table('events')], undefined],
			['analytics', ['SELECT'], [{ name: 's', kind: 'SEQUENCE' }], undefined],
			['analytics', ['SELECT'], [], 'TABLES'],
			['analytics', ['USAGE'], [], undefined]
		])
	})

	// A revoke takes the row back from every source, so the row must name them all, with what each
	// gave.
	it('keeps every source of the grants it folds', () => {
		const grants: AclGrant[] = [
			{
				grantee: 'analytics',
				privileges: ['SELECT'],
				object: table('orders'),
				sources: [by('admin', ['SELECT'])]
			},
			{
				grantee: 'analytics',
				privileges: ['SELECT'],
				object: table('salaries'),
				sources: [by('admin', ['SELECT']), by('operator', ['SELECT'])]
			}
		]
		expect(groupGrants(grants)[0].sources).toEqual([
			by('admin', ['SELECT']),
			by('operator', ['SELECT'])
		])
	})
})

describe('revoke of a row', () => {
	const row = (future?: string) => ({
		grantee: 'analytics',
		privileges: ['SELECT'],
		objects: [],
		future,
		sources: [by('admin', ['SELECT'])]
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

	// Postgres takes a grant back only through its source: offering the revoke would promise what
	// the plan then refuses. But only the sources of what is revoked count: the catalog's CONNECT
	// on the database comes from its owner, out of reach, and must not hold back a CREATE the
	// editor granted.
	it('is held back only by a source out of reach for what it takes', () => {
		const database = {
			...row(),
			privileges: ['CONNECT', 'CREATE'],
			sources: [by('postgres', ['CONNECT'], false), by('admin', ['CREATE'])]
		}
		const revocable = revocablePrivileges(database, { kind: 'database' })
		expect(blockingSources(database, revocable)).toEqual([])
		expect(blockingSources(database, ['CONNECT'])).toEqual(['postgres'])
		const partly = {
			...row('TABLES'),
			sources: [by('admin', ['SELECT']), by('postgres', ['SELECT'], false)]
		}
		expect(blockingSources(partly, ['SELECT'])).toEqual(['postgres'])
	})

	// Whether a grant can be taken back depends on its object, so a row folding several objects is
	// only revocable if each of its grants is.
	it('is held back by a source out of reach on any of the objects it folds', () => {
		const grants: AclGrant[] = [
			{
				grantee: 'analytics',
				privileges: ['SELECT'],
				object: table('orders'),
				sources: [by('admin', ['SELECT'])]
			},
			{
				grantee: 'analytics',
				privileges: ['SELECT'],
				object: table('salaries'),
				sources: [by('admin', ['SELECT'], false)]
			}
		]
		const [folded] = groupGrants(grants)
		expect(folded.objects).toHaveLength(2)
		expect(blockingSources(folded, ['SELECT'])).toEqual(['admin'])
		// Folding reads the grants, never rewrites them.
		expect(grants[0].sources[0].reachable).toBe(true)
	})
})

describe('uncoveredCreators', () => {
	// A default privilege binds only the creating roles it was granted for: a role added since is
	// left out until the grant is made again.
	it('names the roles a created-later row leaves out', () => {
		const future = {
			grantee: 'analytics',
			privileges: ['SELECT'],
			objects: [],
			future: 'TABLES',
			sources: [by('admin', ['SELECT']), by('analytics', ['SELECT'])]
		}
		expect(uncoveredCreators(future, ['admin', 'analytics', 'late'])).toEqual(['late'])
		expect(uncoveredCreators({ ...future, future: undefined }, ['late'])).toEqual([])
		// Set by a role outside the catalog, it was never meant to cover the catalog's roles.
		expect(
			uncoveredCreators({ ...future, sources: [by('postgres', ['SELECT'], false)] }, [
				'admin',
				'late'
			])
		).toEqual([])
	})
})
