import { describe, expect, it } from 'vitest'
import { buildDbm, parseDbm } from './dbManagerDrawerModel.svelte'
import { schemaCacheKey } from './dbSchemaCache'
import { datatableReference, type DbInput } from './dbTypes'

describe('dbm role segment', () => {
	it('round-trips a role, with and without a table', () => {
		for (const dbm of ['datatable~main~.orders~role=p4_analytics', 'datatable~main~role=p4-op']) {
			expect(buildDbm(parseDbm(dbm)!)).toBe(dbm)
		}
		expect(parseDbm('datatable~main~sales.orders~role=analyst')).toMatchObject({
			path: 'main',
			schema: 'sales',
			table: 'orders',
			role: 'analyst'
		})
	})

	it('reads a link without a role as the default role', () => {
		const parsed = parseDbm('datatable~main~.orders')!
		expect(parsed.role).toBeUndefined()
		expect(parsed).toMatchObject({ schema: 'public', table: 'orders' })
		expect(buildDbm(parsed)).toBe('datatable~main~.orders')
	})

	it('keeps an invalid role as written, so the connection refuses it', () => {
		expect(parseDbm('datatable~main~role=a;b')?.role).toBe('a;b')
		// A dot does not turn it into a schema.table selection read as the default role.
		expect(parseDbm('datatable~main~role=bad.name')).toMatchObject({
			role: 'bad.name',
			schema: undefined,
			table: undefined
		})
	})
})

describe('connecting as a role', () => {
	const input = (role?: string): DbInput => ({
		type: 'database',
		resourceType: 'postgresql',
		resourcePath: 'datatable://main',
		role
	})

	// What `getDatabaseArg` builds every DB manager connection from.
	it('appends the role to the data table reference', () => {
		expect(datatableReference('main', 'p4_analytics')).toBe('datatable://main?role=p4_analytics')
		expect(datatableReference('main', undefined)).toBe('datatable://main')
	})

	it('never appends a role to a name containing ?', () => {
		// The server reads such a whole reference as the stored name first, so `?role=` would
		// be taken as part of the name or refused instead of picking the role.
		expect(datatableReference('legacy?x', undefined)).toBe('datatable://legacy?x')
		expect(() => datatableReference('sales?role=analytics', 'admin')).toThrow(/'\?' in its name/)
	})

	it('refuses a role name the server would not accept', () => {
		expect(() => datatableReference('main', 'a&role=admin')).toThrow(/Invalid data table role/)
		expect(() => datatableReference('main', '')).toThrow(/Invalid data table role/)
	})

	it('keys the schema cache by role', () => {
		expect(schemaCacheKey('ws', input('a'))).not.toBe(schemaCacheKey('ws', input('b')))
		expect(schemaCacheKey('ws', input('a'))).not.toBe(schemaCacheKey('ws', input()))
	})
})
