import { afterEach, beforeEach, describe, expect, it } from 'bun:test'
import {
	getBenchmarkCompletedJobResultMaybe,
	getBenchmarkDatatableSchema,
	listBenchmarkDatatables,
	registerBenchmarkWorkspaceRunnables,
	resetBenchmarkMockBackend,
	runBenchmarkDatatableSql,
	type BenchmarkWorkspaceRunnables
} from './mockBackend'

const WORKSPACE = 'benchmark-datatable-ws'

// Mirrors the production `isDatatableNotConfiguredError` regex in
// datatableTools.ts. The schema mock's "not configured" message MUST match it,
// otherwise the not-configured mapping in get_datatable_table_schema is silently
// untested.
const NOT_CONFIGURED_RE = /datatable\s+\S+\s+not found/i

const SEED: BenchmarkWorkspaceRunnables = {
	datatables: [
		{
			datatable_name: 'main',
			schemas: {
				public: {
					orders: {
						columns: { id: 'int', total: 'numeric' },
						rows: [
							{ id: 1, total: 10 },
							{ id: 2, total: 20 }
						]
					},
					customers: {
						columns: { id: 'int', name: 'text' },
						rows: [{ id: 1, name: 'alice' }]
					}
				}
			}
		}
	]
}

beforeEach(() => resetBenchmarkMockBackend())
afterEach(() => resetBenchmarkMockBackend())

describe('listBenchmarkDatatables', () => {
	it('returns null for a non-benchmark workspace (caller falls through to real backend)', () => {
		expect(listBenchmarkDatatables('unregistered')).toBeNull()
	})

	it('returns [] for a registered workspace with no datatables seed', () => {
		registerBenchmarkWorkspaceRunnables(WORKSPACE, {})
		expect(listBenchmarkDatatables(WORKSPACE)).toEqual([])
	})

	it('projects seeded datatables to schema -> table names only (no columns)', () => {
		registerBenchmarkWorkspaceRunnables(WORKSPACE, SEED)
		expect(listBenchmarkDatatables(WORKSPACE)).toEqual([
			{ datatable_name: 'main', schemas: { public: ['orders', 'customers'] } }
		])
	})
})

describe('getBenchmarkDatatableSchema', () => {
	beforeEach(() => registerBenchmarkWorkspaceRunnables(WORKSPACE, SEED))

	it('returns the columns for a seeded table', () => {
		expect(
			getBenchmarkDatatableSchema({
				workspace: WORKSPACE,
				datatableName: 'main',
				schemaName: 'public',
				tableName: 'orders'
			})
		).toEqual({
			datatable_name: 'main',
			schema_name: 'public',
			table_name: 'orders',
			columns: { id: 'int', total: 'numeric' }
		})
	})

	it('throws a not-configured error matching the production regex for an unknown datatable', () => {
		let error: Error | undefined
		try {
			getBenchmarkDatatableSchema({
				workspace: WORKSPACE,
				datatableName: 'ghost',
				schemaName: 'public',
				tableName: 'orders'
			})
		} catch (e) {
			error = e as Error
		}
		expect(error).toBeDefined()
		expect(error!.message).toMatch(NOT_CONFIGURED_RE)
	})

	it('throws a table-not-found error that does NOT match the datatable-not-configured regex', () => {
		// The datatable IS configured; only the table is missing. Production maps
		// this to a generic "error getting schema", not the blocking message.
		let error: Error | undefined
		try {
			getBenchmarkDatatableSchema({
				workspace: WORKSPACE,
				datatableName: 'main',
				schemaName: 'public',
				tableName: 'ghost'
			})
		} catch (e) {
			error = e as Error
		}
		expect(error).toBeDefined()
		expect(error!.message).not.toMatch(NOT_CONFIGURED_RE)
	})
})

describe('runBenchmarkDatatableSql + getBenchmarkCompletedJobResultMaybe', () => {
	beforeEach(() => registerBenchmarkWorkspaceRunnables(WORKSPACE, SEED))

	function exec(sql: string): { success: boolean; completed: boolean; result: unknown } {
		const jobId = runBenchmarkDatatableSql({ workspace: WORKSPACE, datatableName: 'main', sql })
		return getBenchmarkCompletedJobResultMaybe({ workspace: WORKSPACE, id: jobId })
	}

	it('returns the canned rows of the table named in a SELECT FROM clause', () => {
		expect(exec('SELECT * FROM customers')).toEqual({
			success: true,
			completed: true,
			result: [{ id: 1, name: 'alice' }]
		})
	})

	it('falls back to the first seeded table when the SELECT references no known table', () => {
		expect(exec('select 1').result).toEqual([
			{ id: 1, total: 10 },
			{ id: 2, total: 20 }
		])
	})

	it('returns [] success for DDL and DML statements', () => {
		expect(exec('CREATE TABLE foo (id int)').result).toEqual([])
		expect(exec('INSERT INTO orders VALUES (3, 30)').result).toEqual([])
		expect(exec('update orders set total = 0').result).toEqual([])
	})

	it('throws for an unknown job id', () => {
		expect(() =>
			getBenchmarkCompletedJobResultMaybe({ workspace: WORKSPACE, id: 'does-not-exist' })
		).toThrow()
	})
})
