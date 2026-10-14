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
	test('reads the scope of generated table DDL', () => {
		const sql = `BEGIN;
CREATE SCHEMA IF NOT EXISTS billing;
CREATE TABLE IF NOT EXISTS "Sales"."orders" (
    "id" uuid DEFAULT gen_random_uuid() NOT NULL,
    "amount" numeric(10, 2) CHECK (amount > 0),
    "customer" int REFERENCES crm.customers(id) ON DELETE CASCADE,
    PRIMARY KEY ("id")
);
CREATE UNIQUE INDEX orders_idx ON sales.orders USING btree (id);
ALTER TABLE sales.orders ALTER COLUMN amount TYPE numeric(12, 2);
COMMENT ON COLUMN sales.orders.amount IS 'in cents';
END;`
		expect(suggestMigrationSchemas(sql)).toEqual(['billing', 'sales', 'crm'])
		// As the database manager's query builders write it.
		expect(
			suggestMigrationSchemas(
				'BEGIN;\n\nALTER TABLE "myschema"."users" ADD COLUMN "age" INT;\nDROP TABLE "myschema"."old";\n\nEND;'
			)
		).toEqual(['myschema'])
		expect(suggestMigrationSchemas('ALTER SCHEMA "sales" RENAME TO "billing";')).toEqual([
			'sales',
			'billing'
		])
	})

	test('falls back to the whole database for anything it cannot read in full', () => {
		for (const sql of [
			// Unqualified objects resolve through the search path.
			'CREATE TABLE plain (id int);',
			'ALTER TABLE ONLY plain ADD COLUMN x int;',
			'CREATE INDEX ON plain (id);',
			'CREATE TABLE sales.t (id int REFERENCES plain(id));',
			'CREATE TABLE sales.t (status order_status);',
			'ALTER TABLE sales.t ADD COLUMN x int DEFAULT my_default();',
			'DROP INDEX plain_idx;',
			// Qualified, but where the scope is not read.
			'CREATE TABLE sales.t (status billing.order_status);',
			// Statements outside the reader.
			'CREATE TABLE sales.copy AS SELECT * FROM customers;',
			'TRUNCATE plain;',
			'GRANT USAGE ON SCHEMA sales, billing TO analyst;',
			'CREATE EXTENSION postgis WITH SCHEMA public;',
			'CREATE POLICY p ON sales.t FOR SELECT USING (true);',
			'CREATE VIEW sales.v AS SELECT o.id FROM sales.orders o;',
			// A cascade drops dependents wherever they are.
			'DROP SCHEMA billing CASCADE;',
			// Several targets, or a relation reached through another one.
			'DROP SCHEMA sales, billing;',
			'DROP TABLE sales.t, customers;',
			'ALTER TABLE sales.t ATTACH PARTITION plain FOR VALUES IN (1);',
			'ALTER TABLE sales.t INHERIT plain;',
			'CREATE TABLE sales.t OF plain_type;'
		]) {
			expect(suggestMigrationSchemas(`CREATE TABLE sales.a (id int);\n${sql}`), sql).toEqual([])
		}
	})
})
