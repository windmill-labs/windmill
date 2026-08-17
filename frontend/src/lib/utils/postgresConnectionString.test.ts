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

	// Verified against psql: `postgres://role:p%40ss@host/db` authenticates as `p@ss`, and an
	// unencoded `@` puts the rest of the password in libpq's host too. Reading these any other
	// way would make the same string mean something here that it means nowhere else.
	it('decodes percent escapes in credentials, as libpq does', () => {
		expect(parsePostgresConnectionString('postgres://u:p%40ss@host/db')?.password).toBe('p@ss')
		expect(parsePostgresConnectionString('postgres://u%40corp:p@host/db')?.user).toBe('u@corp')
	})
})

// The wizard offers the same connection as a string or as fields and switches between them
// by composing and reparsing. A password holding a character the URI reserves is the case
// that breaks silently: it comes back wrong rather than failing to parse.
describe('composePostgresConnectionString', () => {
	// `prefer` is libpq's default, so it is the one a composer is tempted to leave out -- and
	// the one that silently becomes `require` when the wizard reparses the string and falls
	// back to its own default. It is a weaker TLS setting chosen on purpose; it has to survive.
	it('keeps an explicit prefer through the round trip', () => {
		const parts = { user: 'u', host: 'h', port: undefined, dbname: 'db', sslmode: 'prefer' }
		const composed = composePostgresConnectionString(parts)
		expect(composed).toContain('sslmode=prefer')
		expect(parsePostgresConnectionString(composed)?.sslmode).toBe('prefer')
	})

	// The wizard composes this from fields, so a database name holding a character the URI
	// reserves has to survive the toggle. `?` is the one that truncates silently: the parser
	// reads everything after it as the query string.
	it('round-trips a database name holding reserved characters', () => {
		const parts = { user: 'u', host: 'h', dbname: 'sales?archive', sslmode: 'require' }
		expect(parsePostgresConnectionString(composePostgresConnectionString(parts))?.dbname).toBe(
			'sales?archive'
		)
	})

	// A literal IPv6 address is all colons, so the URI brackets it and the resource stores it
	// bare. Both halves have to agree or the wizard's own toggle produces a string it rejects.
	it('brackets an IPv6 host and reads it back bare', () => {
		const composed = composePostgresConnectionString({
			user: 'u',
			host: '2001:db8::1',
			port: 5432,
			dbname: 'db'
		})
		expect(composed).toContain('@[2001:db8::1]:5432/')
		expect(parsePostgresConnectionString(composed)?.host).toBe('2001:db8::1')
		expect(parsePostgresConnectionString('postgres://u:p@[2001:db8::1]/db')?.host).toBe(
			'2001:db8::1'
		)
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
