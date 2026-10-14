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
CREATE TABLE "Sales"."orders" (id int REFERENCES crm.customers(id));
CREATE INDEX ON sales.orders (id);
SELECT * FROM a JOIN b ON a.id = b.id;
CREATE TABLE plain (id int);
INSERT INTO pg_catalog.x VALUES ('audit.y');
END;`
		expect(suggestMigrationSchemas(sql)).toEqual(['billing', 'sales', 'crm'])
	})
})
