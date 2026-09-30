import { describe, it, expect } from 'vitest'
import { argsForSchema } from './pendingRun'
import type { Schema } from '$lib/common'

const schemaOf = (...names: string[]): Schema =>
	({
		properties: Object.fromEntries(names.map((n) => [n, { type: 'string' }]))
	}) as unknown as Schema

describe('argsForSchema', () => {
	// The model writes its arguments against the item it has been editing, which can carry
	// parameters the deployed version does not. Dropping those leaves a form the reader can
	// still fill, where passing them whole would send arguments it never declared.
	it('keeps only the arguments the deployed schema declares', () => {
		expect(argsForSchema({ path: '/x', retries: 3 }, schemaOf('path', 'verbose'))).toEqual({
			path: '/x'
		})
	})

	// A falsy value is still an answer the model gave; filtering on the value rather than the
	// key would silently drop `false` and `0`.
	it('keeps falsy values', () => {
		expect(argsForSchema({ verbose: false, count: 0 }, schemaOf('verbose', 'count'))).toEqual({
			verbose: false,
			count: 0
		})
	})

	// Seeding wholesale against a schema that has not loaded would be a guess, and the form
	// would raise the "AI generated" badge over arguments nobody can check.
	it('seeds nothing when there is no schema to narrow against', () => {
		expect(argsForSchema({ path: '/x' }, undefined)).toEqual({})
	})
})
