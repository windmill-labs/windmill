import { describe, expect, it } from 'bun:test'
import { applyDatatableSql, type BenchmarkDatatableSeed } from './datatableSqlEngine'

function makeDatatable(): BenchmarkDatatableSeed {
	return {
		datatable_name: 'main',
		schemas: {
			public: {
				orders: {
					columns: { id: 'int4', customer_id: 'int4', total: 'numeric', status: 'text' },
					rows: [
						{ id: 1, customer_id: 1, total: 42.5, status: 'shipped' },
						{ id: 2, customer_id: 2, total: 19.99, status: 'pending' },
						{ id: 3, customer_id: 1, total: 88, status: 'shipped' }
					]
				},
				customers: {
					columns: { id: 'int4', name: 'text' },
					rows: [{ id: 1, name: 'Alice' }]
				}
			}
		}
	}
}

describe('SELECT', () => {
	it('returns the referenced table rows', () => {
		const dt = makeDatatable()
		expect(applyDatatableSql(dt, 'SELECT id, name FROM customers').rows).toEqual([
			{ id: 1, name: 'Alice' }
		])
	})

	it('falls back to the first table when no known table is referenced', () => {
		const dt = makeDatatable()
		expect(applyDatatableSql(dt, 'select 1').rows).toHaveLength(3)
	})

	it('resolves a schema-qualified table', () => {
		const dt = makeDatatable()
		expect(applyDatatableSql(dt, 'SELECT * FROM public.customers').rows).toEqual([
			{ id: 1, name: 'Alice' }
		])
	})
})

describe('CREATE TABLE', () => {
	it('adds a table with parsed columns, skipping table constraints and FK clauses', () => {
		const dt = makeDatatable()
		const result = applyDatatableSql(
			dt,
			'CREATE TABLE public.refunds (\n  order_id int4 NOT NULL REFERENCES public.orders(id),\n  amount numeric(10,2),\n  PRIMARY KEY (order_id)\n)'
		)
		expect(result.rows).toEqual([])
		expect(dt.schemas.public.refunds).toEqual({
			columns: { order_id: 'int4', amount: 'numeric(10,2)' },
			rows: []
		})
	})

	it('defaults an unqualified table to the public schema', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, 'CREATE TABLE notes (id int4, body text)')
		expect(dt.schemas.public.notes.columns).toEqual({ id: 'int4', body: 'text' })
	})

	it('is a no-op for an existing table with IF NOT EXISTS', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, 'CREATE TABLE IF NOT EXISTS public.orders (x int4)')
		expect(Object.keys(dt.schemas.public.orders.columns)).toContain('status')
	})
})

describe('DROP TABLE', () => {
	it('removes the table', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, 'DROP TABLE IF EXISTS public.customers')
		expect(dt.schemas.public.customers).toBeUndefined()
	})
})

describe('INSERT', () => {
	it('appends a row using an explicit column list', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, "INSERT INTO customers (id, name) VALUES (2, 'Bob')")
		expect(dt.schemas.public.customers.rows).toContainEqual({ id: 2, name: 'Bob' })
	})

	it('infers columns from the table when none are given, and appends multiple tuples', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, "INSERT INTO customers VALUES (2, 'Bob'), (3, 'Carol')")
		expect(dt.schemas.public.customers.rows).toHaveLength(3)
	})

	it('returns the inserted rows when RETURNING is present', () => {
		const dt = makeDatatable()
		const result = applyDatatableSql(
			dt,
			"INSERT INTO customers (id, name) VALUES (2, 'Bob') RETURNING *"
		)
		expect(result.rows).toEqual([{ id: 2, name: 'Bob' }])
	})
})

describe('UPDATE', () => {
	it('updates only the rows matching an equality WHERE', () => {
		const dt = makeDatatable()
		const result = applyDatatableSql(
			dt,
			"UPDATE public.orders SET status = 'shipped' WHERE id = 2"
		)
		expect(result.rows).toEqual([])
		expect(dt.schemas.public.orders.rows?.find((r) => r.id === 2)?.status).toBe('shipped')
		expect(dt.schemas.public.orders.rows?.find((r) => r.id === 1)?.status).toBe('shipped')
	})

	it('strips a Postgres cast in the WHERE value', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, "UPDATE orders SET status = 'done' WHERE id = 2::int4")
		expect(dt.schemas.public.orders.rows?.find((r) => r.id === 2)?.status).toBe('done')
	})

	it('matches multiple AND predicates including a numeric literal', () => {
		const dt = makeDatatable()
		applyDatatableSql(
			dt,
			"UPDATE orders SET status = 'done' WHERE customer_id = 2 AND total = 19.99"
		)
		expect(dt.schemas.public.orders.rows?.find((r) => r.id === 2)?.status).toBe('done')
		expect(dt.schemas.public.orders.rows?.find((r) => r.id === 1)?.status).toBe('shipped')
	})

	it('updates every row when there is no WHERE', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, "UPDATE orders SET status = 'archived'")
		expect(dt.schemas.public.orders.rows?.every((r) => r.status === 'archived')).toBe(true)
	})

	it('returns the affected rows when RETURNING is present', () => {
		const dt = makeDatatable()
		const result = applyDatatableSql(
			dt,
			"UPDATE orders SET status = 'shipped' WHERE id = 2 RETURNING *"
		)
		expect(result.rows).toHaveLength(1)
		expect(result.rows[0]).toMatchObject({ id: 2, status: 'shipped' })
	})

	it('affects no rows when the WHERE clause cannot be parsed', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, "UPDATE orders SET status = 'x' WHERE total > 20")
		expect(dt.schemas.public.orders.rows?.some((r) => r.status === 'x')).toBe(false)
	})
})

describe('DELETE', () => {
	it('removes only the matching rows', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, 'DELETE FROM orders WHERE id = 2')
		expect(dt.schemas.public.orders.rows?.map((r) => r.id)).toEqual([1, 3])
	})

	it('returns the removed rows when RETURNING is present', () => {
		const dt = makeDatatable()
		const result = applyDatatableSql(dt, 'DELETE FROM orders WHERE id = 2 RETURNING *')
		expect(result.rows).toEqual([{ id: 2, customer_id: 2, total: 19.99, status: 'pending' }])
	})
})

describe('writes are reflected by later reads', () => {
	it('UPDATE then SELECT sees the new value (the verify-loop fix)', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, "UPDATE orders SET status = 'shipped' WHERE id = 2")
		const seen = applyDatatableSql(dt, 'SELECT * FROM orders').rows
		expect(seen.find((r) => r.id === 2)?.status).toBe('shipped')
	})

	it('INSERT then SELECT sees the new row', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, "INSERT INTO customers (id, name) VALUES (9, 'Zed')")
		const seen = applyDatatableSql(dt, 'SELECT * FROM customers').rows
		expect(seen).toContainEqual({ id: 9, name: 'Zed' })
	})

	it('CREATE then SELECT on the new table returns its (empty) rows', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, 'CREATE TABLE public.refunds (order_id int4, amount numeric)')
		expect(applyDatatableSql(dt, 'SELECT * FROM refunds').rows).toEqual([])
	})
})

describe('system-catalog queries reflect the current tables/columns', () => {
	it('lists current tables (including a freshly created one) via information_schema.tables', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, 'CREATE TABLE public.refunds (order_id int4)')
		const rows = applyDatatableSql(
			dt,
			"SELECT table_name FROM information_schema.tables WHERE table_name = 'refunds'"
		).rows
		expect(rows.map((r) => r.table_name)).toContain('refunds')
	})

	it('does not list a dropped table', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, 'DROP TABLE public.customers')
		const rows = applyDatatableSql(dt, 'SELECT table_name FROM information_schema.tables').rows
		expect(rows.map((r) => r.table_name)).not.toContain('customers')
	})

	it('reports columns via information_schema.columns', () => {
		const dt = makeDatatable()
		const rows = applyDatatableSql(
			dt,
			"SELECT column_name FROM information_schema.columns WHERE table_name = 'orders'"
		).rows
		expect(rows.map((r) => r.column_name)).toContain('status')
	})
})

describe('parser robustness (string/paren-aware splitting)', () => {
	it('does not treat the word "returning" inside a string value as a RETURNING clause', () => {
		const dt = makeDatatable()
		const result = applyDatatableSql(
			dt,
			"INSERT INTO customers (id, name) VALUES (5, 'is returning soon')"
		)
		expect(result.rows).toEqual([])
		expect(dt.schemas.public.customers.rows).toContainEqual({ id: 5, name: 'is returning soon' })
	})

	it('does not split on the word "where" inside a SET string value', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, "UPDATE orders SET status = 'ship where ordered' WHERE id = 2")
		expect(dt.schemas.public.orders.rows?.find((r) => r.id === 2)?.status).toBe('ship where ordered')
		expect(dt.schemas.public.orders.rows?.find((r) => r.id === 1)?.status).toBe('shipped')
	})

	it('keeps INSERT tuples intact when a value contains a function call', () => {
		const dt = makeDatatable()
		applyDatatableSql(dt, "INSERT INTO customers (id, name) VALUES (6, coalesce(NULL, 'x'))")
		expect(dt.schemas.public.customers.rows).toHaveLength(2)
		expect(dt.schemas.public.customers.rows?.[1]).toMatchObject({ id: 6 })
	})

	it('CREATE TABLE ignores a trailing semicolon-separated statement', () => {
		const dt = makeDatatable()
		applyDatatableSql(
			dt,
			'CREATE TABLE public.refunds (id int4, amount numeric); INSERT INTO refunds VALUES (1, 5)'
		)
		expect(dt.schemas.public.refunds.columns).toEqual({ id: 'int4', amount: 'numeric' })
		expect(dt.schemas.public.refunds.rows).toEqual([])
	})
})

describe('unparseable statements are a safe no-op', () => {
	it('returns [] and does not throw', () => {
		const dt = makeDatatable()
		expect(applyDatatableSql(dt, 'VACUUM ANALYZE').rows).toEqual([])
		expect(applyDatatableSql(dt, 'GRANT SELECT ON orders TO someone').rows).toEqual([])
	})
})
