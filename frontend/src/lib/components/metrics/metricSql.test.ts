import { describe, expect, it } from 'vitest'
import { attachAliasFor, attachAliasTaken, composeMetricSql, pickAttachAlias } from './metricSql'
import type { DataMetric } from '$lib/gen'

const revenue: DataMetric = {
	script_path: 'f/metrics/orders',
	table_path: 'sales/main.orders',
	kind: 'measure',
	name: 'revenue',
	expr: 'sum(amount)',
	filter: 'not is_refund'
}
const region: DataMetric = {
	script_path: 'f/metrics/orders',
	table_path: 'sales/main.orders',
	kind: 'dimension',
	name: 'region',
	expr: 'region'
}

describe('attachAliasFor', () => {
	it('ignores ATTACH-like text inside a string or dollar-quoted literal', () => {
		// The `$$…$$` body is data, not a statement.
		expect(
			attachAliasFor("SELECT $$ATTACH 'ducklake://sales' AS ghost;$$;", 'sales')
		).toBeUndefined()
		expect(attachAliasTaken("SELECT $$ATTACH 'x.db' AS dl;$$;", 'dl')).toBe(false)
		// A real ATTACH alongside literal noise is still found.
		const code = "SELECT '--not an attach';\nATTACH 'ducklake://sales' AS dl;"
		expect(attachAliasFor(code, 'sales')).toBe('dl')
	})

	it('ignores an ATTACH that is commented out, in any of the comment styles', () => {
		for (const comment of ['--', '//']) {
			const code = `${comment} ATTACH 'ducklake://sales' AS old;\nATTACH 'ducklake://sales' AS dl;`
			expect(attachAliasFor(code, 'sales')).toBe('dl')
		}
		expect(attachAliasFor("/* ATTACH 'ducklake://sales' AS old; */", 'sales')).toBeUndefined()
	})

	it('resolves the default-lake shorthand, which refers to the lake named main', () => {
		expect(attachAliasFor("ATTACH 'ducklake' AS dl;", 'main')).toBe('dl')
		expect(attachAliasFor("ATTACH 'ducklake://main' AS dl;", 'main')).toBe('dl')
		// The shorthand must not be read as an attachment of some other lake.
		expect(attachAliasFor("ATTACH 'ducklake' AS dl;", 'sales')).toBeUndefined()
	})

	it('matches the lake name case-sensitively, unlike the SQL keywords', () => {
		// `sales` and `Sales` are distinct DuckLake configs.
		expect(attachAliasFor("attach 'ducklake://sales' as dl;", 'sales')).toBe('dl')
		expect(attachAliasFor("ATTACH 'ducklake://sales' AS dl;", 'Sales')).toBeUndefined()
		expect(attachAliasFor("ATTACH 'ducklake://Sales' AS dl;", 'sales')).toBeUndefined()
	})

	it('does not treat mid-line // (integer division) as a comment', () => {
		// `5 // 2` is division; the following ATTACH must still be found.
		const code = "SELECT 5 // 2;\nATTACH 'ducklake://sales' AS dl;"
		expect(attachAliasFor(code, 'sales')).toBe('dl')
		// A leading `// materialize` annotation is still a comment.
		expect(attachAliasFor("// materialize x\nATTACH 'ducklake://sales' AS dl;", 'sales')).toBe('dl')
	})

	it('reads a doubled-quote alias as one identifier', () => {
		expect(attachAliasFor(`ATTACH 'ducklake://sales' AS "a""b";`, 'sales')).toBe('a"b')
	})

	it('does not let a backslash-ending literal hide a following ATTACH', () => {
		// `'C:\'` is a complete SQL literal (quotes escape by doubling, not `\`); the
		// ATTACH after it must still be found, and its alias seen as taken.
		const code = "SELECT 'C:\\';\nATTACH 'ducklake://sales' AS dl;"
		expect(attachAliasFor(code, 'sales')).toBe('dl')
		expect(attachAliasTaken(code, 'dl')).toBe(true)
		// A doubled quote embeds a literal quote without ending the string.
		expect(attachAliasFor("SELECT 'a''b';\nATTACH 'ducklake://sales' AS dl;", 'sales')).toBe('dl')
	})

	it('recognizes the optional ATTACH DATABASE keyword', () => {
		expect(attachAliasFor("ATTACH DATABASE 'ducklake://sales' AS dl;", 'sales')).toBe('dl')
		expect(attachAliasTaken("ATTACH DATABASE 'x.db' AS dl;", 'dl')).toBe(true)
	})

	it('reads a quoted alias whole, including one containing a space', () => {
		expect(attachAliasFor(`ATTACH 'ducklake://sales' AS "my dl";`, 'sales')).toBe('my dl')
		expect(attachAliasFor(`ATTACH 'ducklake://sales' AS dl;`, 'sales')).toBe('dl')
	})
})

describe('pickAttachAlias', () => {
	it('skips aliases already attached, whatever they are attached to', () => {
		expect(pickAttachAlias('SELECT 1;', 'sales')).toBe('dl')
		// `dl` taken by another lake: fall back to the lake's own name.
		expect(pickAttachAlias("ATTACH 'ducklake://other' AS dl;", 'sales')).toBe('sales')
		// Both taken, and the second by a non-DuckLake attachment, which occupies
		// the alias just as effectively.
		expect(
			pickAttachAlias("ATTACH 'ducklake://other' AS dl;\nATTACH 'foo.db' AS sales;", 'sales')
		).toBe('dl2')
	})

	it('does not treat a longer alias as a match for a shorter one', () => {
		expect(attachAliasTaken("ATTACH 'x.db' AS dl2;", 'dl')).toBe(false)
	})
})

describe('attachAliasFor with tricky sources', () => {
	it('still finds a real attachment when a literal contains comment markers', () => {
		const code = "SELECT '--not a comment';\nATTACH 'ducklake://sales' AS dl;"
		expect(attachAliasFor(code, 'sales')).toBe('dl')
	})
})

describe('composeMetricSql', () => {
	it('qualifies the table with the alias the script already attached the lake under', () => {
		// DuckDB names the catalog after the ATTACH alias, so qualifying with the
		// lake name would not resolve against a script that attached it as `dl`.
		const code = "ATTACH 'ducklake://sales' AS dl;\nSELECT 1;"
		const sql = composeMetricSql({
			tablePath: 'sales/main.orders',
			measures: [revenue],
			dimensions: [],
			existingAlias: attachAliasFor(code, 'sales')
		})
		expect(sql).toContain('FROM "dl"."main"."orders"')
		expect(sql).not.toContain('ATTACH')
	})

	it('attaches the lake under the conventional dl alias when the script has none', () => {
		const sql = composeMetricSql({
			tablePath: 'sales/main.orders',
			measures: [revenue],
			dimensions: [],
			existingAlias: attachAliasFor('SELECT 1;', 'sales')
		})
		expect(sql).toContain(`ATTACH 'ducklake://sales' AS "dl";`)
		expect(sql).toContain('FROM "dl"."main"."orders"')
	})

	it('falls back to the lake name when the script already binds dl elsewhere', () => {
		const code = "ATTACH 'ducklake://other' AS dl;"
		const sql = composeMetricSql({
			tablePath: 'sales/main.orders',
			measures: [revenue],
			dimensions: [],
			existingAlias: attachAliasFor(code, 'sales'),
			attachAs: attachAliasTaken(code, 'dl') ? 'sales' : 'dl'
		})
		expect(sql).toContain(`ATTACH 'ducklake://sales' AS "sales";`)
		expect(sql).toContain('FROM "sales"."main"."orders"')
	})

	it('quotes every generated identifier, so reserved words are safe', () => {
		const reservedMeasure: DataMetric = {
			...revenue,
			name: 'select',
			expr: 'count(*)',
			filter: undefined
		}
		const reservedDim: DataMetric = { ...region, name: 'group', expr: 'region' }
		const sql = composeMetricSql({
			// `order` is a reserved word, and so is the `select`/`group` naming below.
			tablePath: 'sales/main.order',
			measures: [reservedMeasure],
			dimensions: [reservedDim],
			existingAlias: 'dl'
		})
		expect(sql).toContain('AS "select"')
		expect(sql).toContain('AS "group"')
		expect(sql).toContain('FROM "dl"."main"."order"')
	})

	it('renders a measure predicate as FILTER so measures can share one GROUP BY', () => {
		const sql = composeMetricSql({
			tablePath: 'sales/main.orders',
			measures: [revenue],
			dimensions: [region],
			existingAlias: 'dl'
		})
		expect(sql).toContain('sum(amount) FILTER (WHERE not is_refund) AS "revenue"')
		expect(sql).toContain('GROUP BY 1')
	})
})
