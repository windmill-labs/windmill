import { describe, expect, it } from 'vitest'
import {
	conformArgsToSchema,
	enforceDisabledDefaults,
	redactFileArgs,
	redactSecretArgs,
	stripSecretArgs
} from './job_args'

describe('conformArgsToSchema', () => {
	it('drops what the schema does not declare, prototype names included', () => {
		const { args, droppedKeys } = conformArgsToSchema(
			JSON.parse('{"keep":1,"force":true,"constructor":"x","toString":"y","__proto__":{"p":1}}'),
			{ properties: { keep: { type: 'number' } } }
		)
		expect(args).toEqual({ keep: 1 })
		expect(Object.getPrototypeOf(args)).toBe(Object.prototype)
		expect(droppedKeys.sort()).toEqual(['__proto__', 'constructor', 'force', 'toString'])
	})

	it('keeps a declared __proto__ instead of losing it to the setter', () => {
		const { args, droppedKeys } = conformArgsToSchema(
			JSON.parse('{"__proto__":"legit","keep":1}'),
			JSON.parse('{"properties":{"__proto__":{"type":"string"},"keep":{"type":"number"}}}')
		)
		expect(Object.hasOwn(args, '__proto__')).toBe(true)
		expect(args['__proto__']).toBe('legit')
		expect(droppedKeys).toEqual([])
	})

	// $state.snapshot deep-copies a plain object and hands back a null-prototype one by
	// identity, so a form editing it in place would write into the persisted transcript.
	it('returns a plain object even when the schema declares nothing', () => {
		const { args } = conformArgsToSchema(JSON.parse('{"a":1}'), undefined)
		expect(Object.getPrototypeOf(args)).toBe(Object.prototype)
	})

	// The mounted nested form prunes its own extras, but ArgInput renders only the first
	// 50 array items, so past that nothing else would ever drop them.
	it('drops undeclared arguments nested inside declared ones', () => {
		const { args, droppedKeys } = conformArgsToSchema(
			{ cfg: { batch: 10, evil: true }, rows: [{ mode: 'safe' }, { mode: 'safe', evil: true }] },
			{
				properties: {
					cfg: { properties: { batch: { type: 'number' } } },
					rows: { items: { properties: { mode: { type: 'string' } } } }
				}
			}
		)
		expect(args).toEqual({ cfg: { batch: 10 }, rows: [{ mode: 'safe' }, { mode: 'safe' }] })
		expect(droppedKeys).toEqual(['cfg.evil', 'rows[1].evil'])
	})

	// ArgInput writes the tag itself and reads it back to pick the branch that opens, so
	// dropping it would reopen the form on the wrong variant.
	it('keeps the oneOf tag and every branch key', () => {
		const { args, droppedKeys } = conformArgsToSchema(
			{ either: { kind: 'b', level: 2, evil: true } },
			{
				properties: {
					either: {
						oneOf: [
							{ title: 'a', properties: { name: { type: 'string' } } },
							{ title: 'b', properties: { level: { type: 'number' } } }
						]
					}
				}
			}
		)
		expect(args).toEqual({ either: { kind: 'b', level: 2 } })
		expect(droppedKeys).toEqual(['either.evil'])
	})

	// A value shaped unlike its schema matches no level below, so every filter walked
	// past it and the form rendered nothing over an argument the run still carried.
	it('drops a value whose shape contradicts the declared one', () => {
		const schema = {
			properties: {
				rows: { type: 'array', items: { properties: { token: { type: 'string' } } } },
				cfg: { type: 'object', properties: { token: { type: 'string' } } },
				free: { type: 'object' }
			}
		}
		expect(
			conformArgsToSchema({ rows: { token: '$var:u/ada/prod' }, cfg: [{ token: 'x' }] }, schema)
		).toMatchObject({ args: {}, droppedKeys: ['rows', 'cfg'] })
		// One level down too. A free-form object still passes unread: it declares no
		// structure for the value to contradict, so its contents were never filtered.
		expect(
			conformArgsToSchema({ rows: [{ token: 'a' }, ['sneaky']], free: { anything: 1 } }, schema)
		).toMatchObject({
			args: { rows: [{ token: 'a' }], free: { anything: 1 } },
			droppedKeys: ['rows[1]']
		})
	})
})

describe('enforceDisabledDefaults', () => {
	const schema = {
		properties: {
			top: { type: 'string', default: 'fixed', disabled: true },
			cfg: { properties: { force: { type: 'boolean', default: false, disabled: true } } },
			list: {
				items: { properties: { mode: { type: 'string', default: 'safe', disabled: true } } }
			},
			either: {
				oneOf: [
					{ title: 'a', properties: { level: { type: 'number', default: 1, disabled: true } } },
					{ title: 'b', properties: { rate: { type: 'number', default: 5, disabled: true } } }
				]
			},
			free: { type: 'string' }
		}
	}

	it('resets a disabled field at every level the form nests', () => {
		const { args, resetKeys } = enforceDisabledDefaults(
			{
				top: 'tampered',
				cfg: { force: true },
				list: [{ mode: 'destructive' }],
				either: { kind: 'a', level: 99 },
				free: 'kept'
			},
			schema
		)
		expect(args).toEqual({
			top: 'fixed',
			cfg: { force: false },
			list: [{ mode: 'safe' }],
			either: { kind: 'a', level: 1 },
			free: 'kept'
		})
		expect(resetKeys).toEqual(['top', 'cfg.force', 'list[0].mode', 'either.level'])
	})

	it('reports only the arguments it actually overwrote', () => {
		const { args, resetKeys } = enforceDisabledDefaults({ free: 'kept' }, schema)
		// The default still runs; the caller supplied nothing to overwrite, and one told
		// otherwise would try to correct an argument it never sent.
		expect(args.top).toBe('fixed')
		expect(resetKeys).toEqual([])
	})

	// Every branch is visited because the tag is runtime state, so writing an absent
	// default would hand the run an argument from the variant nobody selected.
	it('leaves the unselected oneOf branch out of the run', () => {
		const { args } = enforceDisabledDefaults({ either: { kind: 'a', level: 99 } }, schema)
		expect(args.either).toEqual({ kind: 'a', level: 1 })
	})
})

describe('secret args at every level the form nests', () => {
	const schema = {
		properties: {
			top: { type: 'string', password: true },
			obj: { properties: { inner: { type: 'string', password: true } } },
			list: { items: { properties: { secret: { type: 'string', password: true } } } },
			either: {
				oneOf: [
					{ title: 'a', properties: { key: { type: 'string', password: true } } },
					{ title: 'b', properties: { other: { type: 'string', password: true } } }
				]
			}
		}
	}
	const args = {
		top: 'hunter2',
		obj: { inner: '$var:u/ada/prod', keep: 1 },
		list: [{ secret: 'one', name: 'a' }, { secret: 'two' }],
		// Tagged as branch 'a', but 'b' is stripped too: the tag is runtime state.
		either: { kind: 'a', key: 'k', other: 'o' }
	}

	it('strips every one of them', () => {
		expect(stripSecretArgs(args, schema)).toEqual({
			obj: { keep: 1 },
			list: [{ name: 'a' }, {}],
			either: { kind: 'a' }
		})
	})

	it('redacts every one of them', () => {
		const redacted = JSON.stringify(redactSecretArgs(args, schema))
		for (const secret of ['hunter2', 'prod', 'one', 'two', '"k"', '"o"']) {
			expect(redacted).not.toContain(secret)
		}
		expect(redacted).toContain('<hidden>')
		expect(redacted).toContain('"name":"a"')
	})

	it('leaves no key behind for a level the args never carried', () => {
		expect(Object.keys(stripSecretArgs({ top: 'x' }, schema))).toEqual([])
	})
})

describe('redactFileArgs', () => {
	const schema = {
		properties: {
			doc: { type: 'string', contentEncoding: 'base64' },
			pics: { type: 'array', items: { type: 'string', contentEncoding: 'base64' } },
			wrap: { properties: { inner: { type: 'string', contentEncoding: 'base64' } } },
			note: { type: 'string' }
		}
	}

	it('replaces the bytes with a size marker at every level, and keeps the rest', () => {
		const oneMeg = 'A'.repeat(1024 * 1024 * 2)
		const redacted = redactFileArgs(
			{ doc: oneMeg, pics: ['B'.repeat(4096)], wrap: { inner: 'C'.repeat(2048) }, note: 'hi' },
			schema
		)
		expect(redacted).toEqual({
			doc: '<file: 1.5 MB>',
			pics: ['<file: 3 KB>'],
			wrap: { inner: '<file: 2 KB>' },
			note: 'hi'
		})
	})
})
