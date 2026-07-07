import { describe, it, expect, vi, beforeEach } from 'vitest'

// inferAssets loads WASM; stub it so script detection is deterministic and no
// wasm init runs in the test.
const inferAssetsMock = vi.fn()
vi.mock('$lib/infer', () => ({ inferAssets: (...a: any[]) => inferAssetsMock(...a) }))

// Only getDatatableFullSchema is used by the generator; stub the whole service.
const getDatatableFullSchemaMock = vi.fn()
vi.mock('$lib/gen', () => ({
	WorkspaceService: {
		getDatatableFullSchema: (...a: any[]) => getDatatableFullSchemaMock(...a)
	}
}))

import { detectDatatableTables, generateDatatableMigrations } from './projectMigrations'
import type { FetchedItem } from './projectBundle'

describe('detectDatatableTables', () => {
	beforeEach(() => inferAssetsMock.mockReset())

	it('collects datatable/table refs from scripts (re-parsed), flows and raw apps', async () => {
		inferAssetsMock.mockResolvedValue({
			status: 'ok',
			assets: [
				{ kind: 'datatable', path: 'main/customers' },
				{ kind: 'resource', path: 'u/admin/pg' } // ignored
			]
		})
		const items: FetchedItem[] = [
			{ kind: 'script', path: 'f/p/s', language: 'duckdb', content: 'select 1' },
			{
				kind: 'flow',
				path: 'f/p/fl',
				value: {
					modules: [
						{
							id: 'a',
							value: {
								type: 'rawscript',
								language: 'duckdb',
								content: '',
								assets: [{ kind: 'datatable', path: 'main/orders' }]
							}
						}
					]
				}
			},
			{
				kind: 'raw_app',
				path: 'f/p/app',
				content: JSON.stringify({
					runnables: {
						r1: { inlineScript: { assets: [{ kind: 'datatable', path: 'analytics/events' }] } }
					}
				})
			}
		]
		const usage = await detectDatatableTables(items)
		expect([...(usage.get('main') ?? [])].sort()).toEqual(['customers', 'orders'])
		expect([...(usage.get('analytics') ?? [])]).toEqual(['events'])
	})

	it('records a datatable used with no specific table', async () => {
		inferAssetsMock.mockResolvedValue({
			status: 'ok',
			assets: [{ kind: 'datatable', path: 'main' }]
		})
		const usage = await detectDatatableTables([
			{ kind: 'script', path: 'f/p/s', language: 'duckdb', content: 'x' }
		])
		expect(usage.has('main')).toBe(true)
		expect(usage.get('main')?.size).toBe(0)
	})
})

describe('generateDatatableMigrations', () => {
	beforeEach(() => getDatatableFullSchemaMock.mockReset())

	const schema = {
		public: {
			customers: {
				name: 'customers',
				columns: [
					{ name: 'id', datatype: 'integer', primary_key: true, nullable: false },
					{ name: 'email', datatype: 'text', nullable: true }
				],
				foreign_keys: []
			},
			orders: {
				name: 'orders',
				columns: [
					{ name: 'id', datatype: 'integer', primary_key: true, nullable: false },
					{ name: 'customer_id', datatype: 'integer', nullable: false }
				],
				foreign_keys: [
					{
						target_table: 'public.customers',
						columns: [{ source_column: 'customer_id', target_column: 'id' }],
						on_delete: 'NO ACTION',
						on_update: 'NO ACTION'
					}
				]
			}
		}
	}

	it('creates referenced tables in FK-dependency order in one transaction, enabled', async () => {
		getDatatableFullSchemaMock.mockResolvedValue(schema)
		const usage = new Map([['main', new Set(['orders', 'customers'])]])
		const migrations = await generateDatatableMigrations('ws', usage)
		expect(migrations).toHaveLength(1)
		const m = migrations[0]
		expect(m.datatable_name).toBe('main')
		expect(m.enabled).toBe(true)
		expect(m.sql.startsWith('BEGIN;')).toBe(true)
		expect(m.sql.trimEnd().endsWith('COMMIT;')).toBe(true)
		// customers (FK target) must be created before orders (FK source).
		expect(m.sql.indexOf('"public"."customers"')).toBeLessThan(m.sql.indexOf('"public"."orders"'))
		// A single wrapping transaction, not one per table.
		expect(m.sql.match(/BEGIN;/g)?.length).toBe(1)
		// Idempotent: won't abort if a pulled-in parent already exists in the target.
		expect(m.sql).toContain('CREATE TABLE IF NOT EXISTS "public"."customers"')
		// Down migration drops in reverse order: orders (child) before customers (parent).
		expect(m.sql_down).toContain('DROP TABLE IF EXISTS "public"."orders";')
		expect(m.sql_down).toContain('DROP TABLE IF EXISTS "public"."customers";')
		expect(m.sql_down.indexOf('"public"."orders"')).toBeLessThan(
			m.sql_down.indexOf('"public"."customers"')
		)
	})

	it('accepts schema-qualified table refs', async () => {
		getDatatableFullSchemaMock.mockResolvedValue(schema)
		const usage = new Map([['main', new Set(['public.customers'])]])
		const migrations = await generateDatatableMigrations('ws', usage)
		expect(migrations[0].enabled).toBe(true)
		expect(migrations[0].sql).toContain('"public"."customers"')
	})

	it('transitively pulls in FK-referenced tables not directly used', async () => {
		getDatatableFullSchemaMock.mockResolvedValue(schema)
		// Only `orders` is referenced; `customers` (its FK target) must still be
		// created, and before `orders`.
		const usage = new Map([['main', new Set(['orders'])]])
		const migrations = await generateDatatableMigrations('ws', usage)
		const m = migrations[0]
		expect(m.enabled).toBe(true)
		expect(m.sql).toContain('"public"."customers"')
		expect(m.sql).toContain('"public"."orders"')
		expect(m.sql.indexOf('"public"."customers"')).toBeLessThan(m.sql.indexOf('"public"."orders"'))
	})

	it('drops a foreign key whose target is not in the schema', async () => {
		// `orders` references a `warehouses` table that no longer exists in the
		// schema: the FK must be pruned so the migration still runs.
		const schemaWithDanglingFk = {
			public: {
				orders: {
					name: 'orders',
					columns: [{ name: 'id', datatype: 'integer', primary_key: true, nullable: false }],
					foreign_keys: [
						{
							target_table: 'public.warehouses',
							columns: [{ source_column: 'id', target_column: 'id' }],
							on_delete: 'NO ACTION',
							on_update: 'NO ACTION'
						}
					]
				}
			}
		}
		getDatatableFullSchemaMock.mockResolvedValue(schemaWithDanglingFk)
		const usage = new Map([['main', new Set(['orders'])]])
		const migrations = await generateDatatableMigrations('ws', usage)
		expect(migrations[0].enabled).toBe(true)
		expect(migrations[0].sql).toContain('"public"."orders"')
		expect(migrations[0].sql).not.toContain('warehouses')
	})

	it('emits a disabled comment entry when a referenced table is not found', async () => {
		getDatatableFullSchemaMock.mockResolvedValue(schema)
		const usage = new Map([['main', new Set(['nonexistent'])]])
		const migrations = await generateDatatableMigrations('ws', usage)
		expect(migrations).toHaveLength(1)
		expect(migrations[0].enabled).toBe(false)
		expect(migrations[0].sql).toContain('-- Table "nonexistent" is referenced but was not found')
		expect(migrations[0].sql).not.toContain('BEGIN;')
	})

	it('keeps found tables and comments the missing ones in one migration', async () => {
		getDatatableFullSchemaMock.mockResolvedValue(schema)
		const usage = new Map([['main', new Set(['customers', 'ghost'])]])
		const migrations = await generateDatatableMigrations('ws', usage)
		expect(migrations[0].enabled).toBe(true)
		expect(migrations[0].sql).toContain('"public"."customers"')
		expect(migrations[0].sql).toContain('-- Table "ghost" is referenced but was not found')
		// Comments precede the runnable transaction.
		expect(migrations[0].sql.indexOf('-- Table "ghost"')).toBeLessThan(
			migrations[0].sql.indexOf('BEGIN;')
		)
	})

	it('comments a data table used with no specific table', async () => {
		getDatatableFullSchemaMock.mockResolvedValue(schema)
		const usage = new Map([['main', new Set<string>()]])
		const migrations = await generateDatatableMigrations('ws', usage)
		expect(migrations[0].enabled).toBe(false)
		expect(migrations[0].sql).toContain('no specific table was referenced')
	})
})
