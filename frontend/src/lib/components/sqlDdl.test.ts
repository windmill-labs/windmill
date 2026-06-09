import { describe, it, expect } from 'vitest'
import { containsDdlStatement } from './sqlDdl'

describe('containsDdlStatement', () => {
	it('detects common DDL verbs regardless of case and leading whitespace', () => {
		expect(containsDdlStatement(['CREATE TABLE foo (id int)'])).toBe(true)
		expect(containsDdlStatement(['  alter table foo add column bar text'])).toBe(true)
		expect(containsDdlStatement(['drop table foo'])).toBe(true)
		expect(containsDdlStatement(['TRUNCATE foo'])).toBe(true)
		expect(containsDdlStatement(['comment on table foo is $$x$$'])).toBe(true)
	})

	it('does not flag pure read/DML statements', () => {
		expect(containsDdlStatement(['SELECT * FROM foo'])).toBe(false)
		expect(containsDdlStatement(['insert into foo values (1)'])).toBe(false)
		expect(containsDdlStatement(['update foo set bar = 1'])).toBe(false)
		expect(containsDdlStatement(['delete from foo'])).toBe(false)
		expect(containsDdlStatement(['with t as (select 1) select * from t'])).toBe(false)
	})

	it('flags a batch when any statement is DDL', () => {
		expect(containsDdlStatement(['SELECT * FROM foo', 'DROP TABLE foo'])).toBe(true)
	})

	it('handles empty input', () => {
		expect(containsDdlStatement([])).toBe(false)
		expect(containsDdlStatement([''])).toBe(false)
	})

	it('only matches the leading verb, not DDL keywords appearing later', () => {
		expect(containsDdlStatement(["SELECT 'create table' AS note"])).toBe(false)
	})
})
