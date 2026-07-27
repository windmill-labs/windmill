import { describe, expect, it } from 'vitest'
import { endsWithUnterminatedStatement, splitSqlStatements, stripSqlComments } from './sqlDdl'

describe('endsWithUnterminatedStatement', () => {
	it('reports a missing terminator on the final statement', () => {
		expect(endsWithUnterminatedStatement('SELECT 1;')).toBe(false)
		expect(endsWithUnterminatedStatement('SELECT 1')).toBe(true)
		expect(endsWithUnterminatedStatement('')).toBe(false)
	})

	it('is not fooled by a trailing comment after the terminator', () => {
		expect(endsWithUnterminatedStatement('SELECT 1;\n-- note')).toBe(false)
		expect(endsWithUnterminatedStatement('SELECT 1\n-- note')).toBe(true)
	})

	it('treats terminators and comment markers inside quoted runs as data', () => {
		expect(endsWithUnterminatedStatement("SELECT ';--literal'")).toBe(true)
		expect(endsWithUnterminatedStatement("SELECT ';--literal';")).toBe(false)
		// Dollar-quoted bodies protect their contents just as strings do.
		expect(endsWithUnterminatedStatement('SELECT $$;--literal$$')).toBe(true)
		expect(endsWithUnterminatedStatement('SELECT $$;--literal$$;')).toBe(false)
		expect(endsWithUnterminatedStatement('SELECT $tag$;--x$tag$')).toBe(true)
	})
})

describe('stripSqlComments', () => {
	it('removes comments without touching quoted runs', () => {
		expect(stripSqlComments('SELECT 1 -- note').trim()).toBe('SELECT 1')
		expect(stripSqlComments("SELECT '--not a comment'").trim()).toBe("SELECT '--not a comment'")
		expect(stripSqlComments('SELECT $$--not a comment$$').trim()).toBe('SELECT $$--not a comment$$')
	})

	it('only treats // as a comment when asked, and only at a line start', () => {
		// `//` is division mid-line: never stripped, and not even when asked.
		expect(stripSqlComments('SELECT 1 // note').trim()).toBe('SELECT 1 // note')
		expect(stripSqlComments('SELECT 5 // 2', true).trim()).toBe('SELECT 5 // 2')
		// A `//` that opens a line is a Windmill annotation header.
		expect(stripSqlComments('// materialize x\nSELECT 1', true).trim()).toBe('SELECT 1')
	})
})

describe('splitSqlStatements', () => {
	it('keeps splitting behaviour intact through the shared scan', () => {
		expect(splitSqlStatements('SELECT 1; SELECT 2;')).toEqual(['SELECT 1', 'SELECT 2'])
		expect(splitSqlStatements("SELECT ';'; SELECT 2")).toEqual(["SELECT ';'", 'SELECT 2'])
		expect(splitSqlStatements('SELECT $$;$$;')).toEqual(['SELECT $$;$$'])
	})

	it('escapes quotes per dialect (backslashEscapes)', () => {
		// Standard SQL (false): `\` is data, quotes escape by doubling. A `\`-ending
		// literal is complete and a doubled quote embeds one, so neither splits.
		expect(splitSqlStatements("SELECT 'C:\\'; SELECT 2", false)).toEqual([
			"SELECT 'C:\\'",
			'SELECT 2'
		])
		expect(splitSqlStatements("SELECT 'a''; b'", false)).toEqual(["SELECT 'a''; b'"])
		// Even in standard mode a Postgres E-string backslash-escapes, so its `;` is data.
		expect(splitSqlStatements("SELECT E'x: \\'; still data' AS v; SELECT 2", false)).toEqual([
			"SELECT E'x: \\'; still data' AS v",
			'SELECT 2'
		])
		// Backslash dialects (default, e.g. MySQL): an ordinary `\'` is an escaped
		// quote, so the embedded `;` is data and the literal does not split.
		expect(splitSqlStatements("SELECT 'it\\'s; still data' AS v; SELECT 2")).toEqual([
			"SELECT 'it\\'s; still data' AS v",
			'SELECT 2'
		])
	})
})
