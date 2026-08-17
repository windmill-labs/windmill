import { describe, expect, it } from 'vitest'
import { buildDataTableWhitelist, isDatatableTableAllowed } from './dataTableRefUtils'

describe('datatable whitelist helpers', () => {
	it('allows every datatable table when no refs are configured', () => {
		const whitelist = buildDataTableWhitelist([])

		expect(isDatatableTableAllowed(whitelist, 'main', 'public', 'users')).toBe(true)
		expect(isDatatableTableAllowed(whitelist, 'analytics', 'events', 'clicks')).toBe(true)
	})

	it('treats datatable-level refs as all tables in that datatable', () => {
		const whitelist = buildDataTableWhitelist([{ datatable: 'main' }])

		expect(isDatatableTableAllowed(whitelist, 'main', 'public', 'users')).toBe(true)
		expect(isDatatableTableAllowed(whitelist, 'main', 'analytics', 'events')).toBe(true)
		expect(isDatatableTableAllowed(whitelist, 'other', 'public', 'users')).toBe(false)
	})

	it('allows only explicitly listed tables for table-level refs', () => {
		const whitelist = buildDataTableWhitelist([
			{ datatable: 'main', schema: 'public', table: 'users' },
			{ datatable: 'main', schema: 'analytics', table: 'events' }
		])

		expect(isDatatableTableAllowed(whitelist, 'main', 'public', 'users')).toBe(true)
		expect(isDatatableTableAllowed(whitelist, 'main', 'analytics', 'events')).toBe(true)
		expect(isDatatableTableAllowed(whitelist, 'main', 'public', 'orders')).toBe(false)
	})

	it('does not treat an explicit empty schema as public', () => {
		const whitelist = buildDataTableWhitelist([{ datatable: 'main', schema: '', table: 'users' }])

		expect(isDatatableTableAllowed(whitelist, 'main', '', 'users')).toBe(true)
		expect(isDatatableTableAllowed(whitelist, 'main', 'public', 'users')).toBe(false)
	})
})
