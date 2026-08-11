import { describe, expect, it } from 'vitest'
import { parsePostgresConnectionString } from './postgresConnectionString'

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
