import { describe, test, expect } from 'vitest'
import {
	parseMigrationSchemas,
	suggestMigrationSchemas,
	withMigrationSchemas
} from './datatableMigrationSchemas'
import { withMigrationRole, parseMigrationRole } from './datatableMigrationRole'

describe('parseMigrationSchemas', () => {
	test('reads the spellings the server accepts, lowercased and deduplicated', () => {
		expect(parseMigrationSchemas('-- role a\n-- Schemas: Sales, billing,sales;\nBEGIN;')).toEqual({
			kind: 'schemas',
			schemas: ['sales', 'billing']
		})
		expect(parseMigrationSchemas('BEGIN;\n-- schemas sales')).toEqual({ kind: 'none' })
		expect(parseMigrationSchemas('-- schemas: changes to orders\nBEGIN;')).toEqual({
			kind: 'malformed',
			line: '-- schemas: changes to orders'
		})
	})

	test('rewriting the scope keeps the role annotation readable', () => {
		const sql = withMigrationSchemas(withMigrationRole('BEGIN;\nEND;', 'analyst'), ['sales'])
		expect(sql).toBe('-- schemas sales\n-- role analyst\nBEGIN;\nEND;')
		expect(parseMigrationRole(sql)).toEqual({ kind: 'role', role: 'analyst' })
		expect(withMigrationSchemas(sql, [])).toBe('-- role analyst\nBEGIN;\nEND;')
	})
})

describe('suggestMigrationSchemas', () => {
	test('collects the schemas of qualified objects the SQL defines or writes', () => {
		const sql = `BEGIN;
CREATE SCHEMA IF NOT EXISTS billing;
CREATE TABLE IF NOT EXISTS "Sales"."orders" (id int REFERENCES crm.customers(id) ON UPDATE CASCADE);
CREATE INDEX orders_idx ON sales.orders (id);
SELECT * FROM sales.orders JOIN crm.customers USING (id);
INSERT INTO pg_catalog.x VALUES ('audit.y');
END;`
		expect(suggestMigrationSchemas(sql)).toEqual(['billing', 'sales', 'crm'])
		expect(suggestMigrationSchemas('ALTER SCHEMA "sales" RENAME TO "billing";')).toEqual([
			'sales',
			'billing'
		])
	})

	test('suggests nothing once an unqualified object is touched: its schema is not in the SQL', () => {
		for (const unqualified of [
			'CREATE TABLE plain (id int);',
			'ALTER TABLE ONLY plain ADD COLUMN x int;',
			'CREATE INDEX ON plain (id);',
			'CREATE TABLE sales.t (id int REFERENCES plain(id));',
			// Qualified, but in a position whose schema is not read as part of the scope.
			'CREATE TABLE sales.t (status billing.order_status);',
			'CREATE VIEW sales.v AS SELECT o.id FROM sales.orders o;'
		]) {
			expect(suggestMigrationSchemas(`CREATE TABLE sales.a (id int);\n${unqualified}`)).toEqual([])
		}
	})
})
