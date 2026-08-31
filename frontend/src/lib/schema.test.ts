import { describe, it, expect } from 'vitest'
import { argsToJsonPayload } from './schema'
import type { Schema } from './common'

const schemaOf = (...names: string[]): Schema =>
	({
		$schema: undefined,
		type: 'object',
		properties: Object.fromEntries(names.map((n) => [n, { type: 'string' }])),
		required: []
	}) as Schema

describe('argsToJsonPayload', () => {
	it('spells out every schema property in schema order, unset ones as null', () => {
		// `0`, `false` and `''` are values, not gaps: only a missing arg becomes `null`.
		expect(argsToJsonPayload(schemaOf('a', 'b', 'c'), { c: 0, a: false })).toBe(
			JSON.stringify({ a: false, b: null, c: 0 }, null, '\t')
		)
	})

	it('keeps args the schema does not declare, after the declared ones', () => {
		expect(argsToJsonPayload(schemaOf('a'), { z: 9, a: 1 })).toBe(
			JSON.stringify({ a: 1, z: 9 }, null, '\t')
		)
	})

	it('falls back to the schema default for an absent arg, but not over an explicit null', () => {
		// `args` only carries defaults once a `SchemaForm` has mounted for that schema, and the
		// JSON view alone never mounts one — seeding `null` there would commit `null` over the
		// argument's default on the first keystroke.
		const schema = schemaOf('a', 'b')
		schema.properties.a.default = 'hi'
		schema.properties.b.default = 42
		expect(argsToJsonPayload(schema, {})).toBe(JSON.stringify({ a: 'hi', b: 42 }, null, '\t'))
		expect(argsToJsonPayload(schema, { a: null })).toBe(
			JSON.stringify({ a: null, b: 42 }, null, '\t')
		)
	})

	it('keeps declared args named after Object.prototype members', () => {
		// A plain `nargs[key]` read returns the inherited function for an unset `constructor`,
		// and `JSON.stringify` drops function-valued properties — the argument would vanish.
		expect(argsToJsonPayload(schemaOf('constructor', 'toString', 'ok'), {})).toBe(
			JSON.stringify({ constructor: null, toString: null, ok: null }, null, '\t')
		)
	})

	it('keeps undeclared args named after Object.prototype members', () => {
		// On a plain `{}` accumulator, `'constructor' in payload` is true before anything is
		// assigned to it.
		expect(argsToJsonPayload(undefined, { constructor: 'x', toString: 'y', ok: 1 })).toBe(
			JSON.stringify({ constructor: 'x', toString: 'y', ok: 1 }, null, '\t')
		)
	})

	it('handles a missing schema or missing args', () => {
		expect(argsToJsonPayload(undefined, undefined)).toBe('{}')
		expect(argsToJsonPayload(schemaOf('a'), undefined)).toBe(
			JSON.stringify({ a: null }, null, '\t')
		)
	})
})
