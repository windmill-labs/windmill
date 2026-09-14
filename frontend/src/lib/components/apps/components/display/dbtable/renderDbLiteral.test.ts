import { describe, expect, it } from 'vitest'
import { renderDbEqualityFilter, renderDbLiteral } from './utils'

describe('renderDbLiteral', () => {
	it('doubles single quotes on every dialect', () => {
		expect(renderDbLiteral("O'Brien", 'postgresql')).toBe("'O''Brien'")
		expect(renderDbLiteral("O'Brien", 'mysql')).toBe("'O''Brien'")
	})

	it('doubles backslashes only where the dialect treats them as escapes', () => {
		expect(renderDbLiteral('C:\\dir\\', 'postgresql')).toBe("'C:\\dir\\'")
		expect(renderDbLiteral('C:\\dir\\', 'mysql')).toBe("'C:\\\\dir\\\\'")
		expect(renderDbLiteral('C:\\dir\\', 'snowflake')).toBe("'C:\\\\dir\\\\'")
	})

	it('marks SQL Server strings as Unicode constants', () => {
		expect(renderDbLiteral("Zoë's", 'ms_sql_server')).toBe("N'Zoë''s'")
	})

	it('renders numbers and booleans without quotes', () => {
		expect(renderDbLiteral(42, 'postgresql')).toBe('42')
		expect(renderDbLiteral(true, 'postgresql')).toBe('TRUE')
		expect(renderDbLiteral(true, 'ms_sql_server')).toBe('1')
	})

	it('has no literal for values that cannot be compared safely', () => {
		expect(renderDbLiteral(null, 'postgresql')).toBeUndefined()
		expect(renderDbLiteral({ a: 1 }, 'postgresql')).toBeUndefined()
		expect(renderDbLiteral(NaN, 'postgresql')).toBeUndefined()
	})
})

describe('renderDbEqualityFilter', () => {
	it('quotes the identifier per dialect', () => {
		expect(renderDbEqualityFilter('user id', 'x', 'postgresql')).toBe(`"user id" = 'x'`)
		expect(renderDbEqualityFilter('user id', 'x', 'ms_sql_server')).toBe(`[user id] = N'x'`)
		expect(renderDbEqualityFilter('user id', 'x', 'mysql')).toBe("`user id` = 'x'")
		expect(renderDbEqualityFilter('user id', null, 'postgresql')).toBeUndefined()
	})

	it('doubles a delimiter embedded in the identifier', () => {
		expect(renderDbEqualityFilter('a"b', 1, 'postgresql')).toBe(`"a""b" = 1`)
		expect(renderDbEqualityFilter('a"b', 1, 'snowflake')).toBe(`"a""b" = 1`)
		expect(renderDbEqualityFilter('a"b', 1, 'duckdb')).toBe(`"a""b" = 1`)
		expect(renderDbEqualityFilter('a]b', 1, 'ms_sql_server')).toBe(`[a]]b] = 1`)
		expect(renderDbEqualityFilter('a`b', 1, 'mysql')).toBe('`a``b` = 1')
	})
})
