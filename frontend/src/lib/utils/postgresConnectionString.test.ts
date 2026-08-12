import { describe, expect, it } from 'vitest'
import {
	composePostgresConnectionString,
	parsePostgresConnectionString
} from './postgresConnectionString'

// Two callers depend on this producing the same resource value from the same string:
// the resource form's "From connection string", and the data table wizard.
describe('parsePostgresConnectionString', () => {
	it('reads every part of a full URI', () => {
		expect(
			parsePostgresConnectionString('postgres://u:p@db.example.com:6543/mydb?sslmode=require')
		).toEqual({
			user: 'u',
			password: 'p',
			host: 'db.example.com',
			port: 6543,
			dbname: 'mydb',
			sslmode: 'require'
		})
	})

	it('leaves optional parts undefined rather than empty', () => {
		expect(parsePostgresConnectionString('postgresql://u@host/')).toEqual({
			user: 'u',
			password: undefined,
			host: 'host',
			port: undefined,
			dbname: undefined,
			sslmode: undefined
		})
	})

	it('returns undefined for anything that is not a postgres URI', () => {
		expect(parsePostgresConnectionString('mysql://u:p@host/db')).toBeUndefined()
		expect(parsePostgresConnectionString('')).toBeUndefined()
	})
})

// The wizard offers the same connection as a string or as fields and switches between them
// by composing and reparsing. A password holding a character the URI reserves is the case
// that breaks silently: it comes back wrong rather than failing to parse.
describe('composePostgresConnectionString', () => {
	it('leaves an explicit prefer out of the string, since libpq assumes it', () => {
		const parts = { user: 'u', host: 'h', port: undefined, dbname: 'db', sslmode: 'prefer' }
		const composed = composePostgresConnectionString(parts)
		expect(composed).not.toContain('sslmode')
		// So the value cannot survive the trip, and a caller merging the parse back over its
		// own state has to skip the undefined rather than let it overwrite the choice.
		expect(parsePostgresConnectionString(composed)?.sslmode).toBeUndefined()
	})

	it('round-trips through parse', () => {
		const parts = {
			user: 'u@corp',
			password: 'p@ss/w:rd',
			host: 'db.example.com',
			port: 6543,
			dbname: 'mydb',
			sslmode: 'require'
		}
		expect(parsePostgresConnectionString(composePostgresConnectionString(parts))).toEqual(parts)
	})
})
