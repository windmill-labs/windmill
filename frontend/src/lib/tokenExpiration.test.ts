import { describe, expect, it } from 'vitest'
import { MAX_TOKEN_EXPIRATION_DAYS_BOUND, parseMaxTokenExpirationDays } from './tokenExpiration'

// Each expectation mirrors how `cap_token_expiration` (windmill-api-users) reads the same stored
// value. A mismatch makes the token form announce a ceiling the server does not apply, or hide
// one it does.
describe('parseMaxTokenExpirationDays', () => {
	it.each([
		[7, 7],
		// jsonb keeps `7.0` as written; the server reads an integral float as whole days.
		[7.0, 7],
		['7', 7],
		[' 30 ', 30],
		['+7', 7],
		[MAX_TOKEN_EXPIRATION_DAYS_BOUND, MAX_TOKEN_EXPIRATION_DAYS_BOUND]
	])('reads %j as a ceiling of %j days', (stored, days) => {
		expect(parseMaxTokenExpirationDays(stored)).toBe(days)
	})

	it.each([
		7.5,
		0,
		-3,
		// `str::parse::<i64>` refuses these, however JavaScript's `Number` reads them.
		'7.0',
		'1e1',
		'0x7',
		'',
		MAX_TOKEN_EXPIRATION_DAYS_BOUND + 1,
		'99999999999999999999',
		null,
		true
	])('reads %j as no ceiling', (stored) => {
		expect(parseMaxTokenExpirationDays(stored)).toBeUndefined()
	})
})
