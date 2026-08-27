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
		const schema = { properties: { a: { type: 'string' } } }
		const { args, dropped } = conformArgsToSchema(
			{ a: 'keep', b: 'drop', constructor: 'drop' },
			schema
		)
		expect(args).toEqual({ a: 'keep' })
		expect(dropped.undeclared.sort()).toEqual(['b', 'constructor'])
	})

	// Both sides parsed, never written as literals: `__proto__:` in an object literal is
	// the prototype setter, so a literal declares nothing to keep in the first place.
	it('keeps a declared __proto__ instead of losing it to the setter', () => {
		const { args, dropped } = conformArgsToSchema(
			JSON.parse('{"__proto__":"legit","keep":1}'),
			JSON.parse('{"properties":{"__proto__":{"type":"string"},"keep":{"type":"number"}}}')
		)
		expect(Object.hasOwn(args, '__proto__')).toBe(true)
		expect(args['__proto__']).toBe('legit')
		expect(dropped.undeclared).toEqual([])
	})

	it('returns a plain object even when the schema declares nothing', () => {
		const { args, dropped } = conformArgsToSchema({ a: 1 }, undefined)
		expect(args).toEqual({})
		expect(dropped.undeclared).toEqual(['a'])
	})

	// The form has the same limitations here as everywhere else in the product: below the
	// top level, `SchemaForm` prunes what it mounts and a mismatch renders as it renders on
	// the run page. A filter precise enough to descend has to resolve `oneOf` branches, and
	// getting that wrong deletes what the user typed into the branch they did open.
	it('leaves nested arguments alone, declared or not', () => {
		const schema = {
			properties: {
				obj: { type: 'object', properties: { known: { type: 'string' } } },
				either: {
					type: 'object',
					oneOf: [
						{ title: 'Structured', properties: { name: { type: 'string' } } },
						{ title: 'Freeform', properties: {} }
					]
				}
			}
		}
		const { args, dropped } = conformArgsToSchema(
			{ obj: { known: 'a', extra: 'b' }, either: { label: 'Freeform', anything: 1 } },
			schema
		)
		expect(args).toEqual({
			obj: { known: 'a', extra: 'b' },
			either: { label: 'Freeform', anything: 1 }
		})
		expect(dropped).toEqual({ undeclared: [], unshowable: [] })
	})

	// Each scalar widget binds one JS type and shows nothing else: a string in a number
	// input renders blank and still passes validation, so the user would approve an empty
	// box over a value only the job sees. A boolean is worse — `"false"` renders checked.
	it('drops a primitive that contradicts its declared scalar type', () => {
		const schema = {
			properties: {
				count: { type: 'number' },
				flag: { type: 'boolean' },
				name: { type: 'string' }
			}
		}
		const { args, dropped } = conformArgsToSchema(
			{ count: '12', flag: 'false', name: 'ada' },
			schema
		)
		expect(args).toEqual({ name: 'ada' })
		expect(dropped.unshowable.sort()).toEqual(['count', 'flag'])
	})

	// Not merely unreadable: `MultiSelect` maps over the value as it renders, so anything
	// else throws and takes the whole card down, Cancel with it.
	it('drops a non-array in a dyn-multiselect slot', () => {
		const schema = {
			properties: { tags: { type: 'object', format: 'dynmultiselect-list' } }
		}
		expect(conformArgsToSchema({ tags: ['a'] }, schema).args).toEqual({ tags: ['a'] })
		const { args, dropped } = conformArgsToSchema({ tags: { a: 1 } }, schema)
		expect(args).toEqual({})
		expect(dropped.unshowable).toEqual(['tags'])
	})
})

describe('enforceDisabledDefaults', () => {
	const schema = {
		properties: {
			locked: { type: 'string', disabled: true, default: 'fixed' },
			open: { type: 'string' }
		}
	}

	it('overwrites a disabled field and reports only what it changed', () => {
		expect(enforceDisabledDefaults({ locked: 'mine', open: 'ok' }, schema)).toEqual({
			args: { locked: 'fixed', open: 'ok' },
			resetKeys: ['locked']
		})
		// Never supplied is not overwritten: the field shows the default either way, and a
		// caller told otherwise would try to correct what it never sent.
		expect(enforceDisabledDefaults({ open: 'ok' }, schema)).toEqual({
			args: { locked: 'fixed', open: 'ok' },
			resetKeys: []
		})
	})

	it('reports no reset for an object default the caller already matched', () => {
		const objSchema = {
			properties: { conf: { type: 'object', disabled: true, default: { a: 1 } } }
		}
		expect(enforceDisabledDefaults({ conf: { a: 1 } }, objSchema).resetKeys).toEqual([])
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

	it('strips a secret under a oneOf branch of an array element', () => {
		const oneOfItems = {
			properties: {
				steps: {
					type: 'array',
					items: {
						oneOf: [{ title: 'push', properties: { token: { type: 'string', password: true } } }]
					}
				}
			}
		}
		const stripped: string[] = []
		expect(
			stripSecretArgs({ steps: [{ token: 'hunter2', name: 'a' }] }, oneOfItems, stripped)
		).toEqual({ steps: [{ name: 'a' }] })
		expect(stripped).toEqual(['steps[0].token'])
	})

	// Descending on which keys the declaration carries rather than on the shape of the
	// value routed this into `properties`, which cannot hold an array — so the elements
	// were never visited and the secret reached the persisted card verbatim.
	it('strips through a declaration carrying both items and properties', () => {
		const both = {
			properties: {
				creds: {
					type: 'array',
					items: { properties: { token: { type: 'string', password: true } } },
					properties: { token: { type: 'string', password: true } }
				}
			}
		}
		expect(stripSecretArgs({ creds: [{ token: 'hunter2' }] }, both)).toEqual({ creds: [{}] })
	})

	// The caller binds the result to a form that edits in place, so a schema declaring
	// nothing must not hand back the object it was given.
	it('copies even when the schema declares nothing to strip', () => {
		const args = { top: 'x' }
		expect(stripSecretArgs(args, undefined)).not.toBe(args)
		expect(stripSecretArgs(args, undefined)).toEqual(args)
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
