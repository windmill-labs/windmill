import { describe, expect, it } from 'vitest'
import {
	coerceArgsToSchema,
	enforceDisabledDefaults,
	redactFileArgs,
	redactSecretArgs,
	stripSecretArgs
} from './job_args'

describe('coerceArgsToSchema', () => {
	// A scalar widget renders its own reading of a wrong-typed value and never writes that
	// reading back, so an untouched form submits something it never displayed: a number
	// input paints `"12"` as a filled-looking 12, and a toggle shows `"false"` as on.
	it('converts a value its widget would read, so the form shows what runs', () => {
		const schema = {
			properties: {
				count: { type: 'number' },
				flag: { type: 'boolean' },
				label: { type: 'string' },
				name: { type: 'string' }
			}
		}
		const { args, clearedKeys } = coerceArgsToSchema(
			{ count: '12', flag: 'false', label: 3, name: 'ada' },
			schema
		)
		expect(args).toEqual({ count: 12, flag: false, label: '3', name: 'ada' })
		expect(clearedKeys).toEqual([])
	})

	// Cleared, not carried: the widget shows nothing for these, so nothing is what an
	// untouched form should send.
	it('empties a value with no reading in its declared type', () => {
		const schema = {
			properties: {
				count: { type: 'number' },
				flag: { type: 'boolean' },
				label: { type: 'string' }
			}
		}
		const { args, clearedKeys } = coerceArgsToSchema(
			{ count: 'abc', flag: 'maybe', label: { a: 1 } },
			schema
		)
		expect(args).toEqual({})
		expect(clearedKeys.sort()).toEqual(['count', 'flag', 'label'])
	})

	// The worker takes the arguments its own signature names, so a **kwargs script and one
	// whose stored schema is stale or absent accept what no property declares. Removing
	// these made such a script unrunnable through the form.
	it('carries arguments the schema does not declare', () => {
		const kept = coerceArgsToSchema({ a: 'keep', b: 2, constructor: 'x' }, {
			properties: { a: { type: 'string' } }
		} as any)
		expect(kept.args).toEqual({ a: 'keep', b: 2, constructor: 'x' })
		expect(coerceArgsToSchema({ a: 1 }, undefined).args).toEqual({ a: 1 })
	})

	// Resolved by the job, so the declared type describes what it receives and never the
	// string standing in for it. `Number('$var:…')` is NaN, so coercing would destroy it.
	it('leaves a variable or resource reference in any slot', () => {
		const schema = {
			properties: {
				size: { type: 'number' },
				on: { type: 'boolean' },
				db: { type: 'object', format: 'resource-postgresql' }
			}
		}
		const { args, clearedKeys } = coerceArgsToSchema(
			{ size: '$var:u/admin/size', on: '$var:u/admin/on', db: '$res:u/admin/pg' },
			schema
		)
		expect(args).toEqual({
			size: '$var:u/admin/size',
			on: '$var:u/admin/on',
			db: '$res:u/admin/pg'
		})
		expect(clearedKeys).toEqual([])
	})

	// Not merely unreadable: `MultiSelect` maps over the value as it renders, so anything
	// else throws and takes the whole card down, Cancel with it. A reference is no
	// exception — the widget draws before anything resolves — so this slot is the one
	// place the reference rule above does not hold.
	it('empties a non-array in a dyn-multiselect slot, reference included', () => {
		const schema = { properties: { tags: { type: 'object', format: 'dynmultiselect-list' } } }
		expect(coerceArgsToSchema({ tags: ['a'] }, schema).args).toEqual({ tags: ['a'] })
		for (const bad of [{ a: 1 }, '$var:u/admin/watchlist']) {
			const { args, clearedKeys } = coerceArgsToSchema({ tags: bad }, schema)
			expect(args).toEqual({})
			expect(clearedKeys).toEqual(['tags'])
		}
	})

	// Below the top the form has the same limitations as everywhere else in the product,
	// and descending means resolving `oneOf` branches — where being wrong rewrites what the
	// user typed into the branch they did open.
	it('leaves nested and container values to the widget that renders them', () => {
		const schema = {
			properties: {
				obj: { type: 'object', properties: { known: { type: 'string' } } },
				rows: { type: 'array', items: { type: 'object' } }
			}
		}
		const { args, clearedKeys } = coerceArgsToSchema(
			{ obj: { known: 1, extra: 'b' }, rows: { id: 'x' } },
			schema
		)
		expect(args).toEqual({ obj: { known: 1, extra: 'b' }, rows: { id: 'x' } })
		expect(clearedKeys).toEqual([])
	})

	// Both sides parsed, never written as literals: `__proto__:` in an object literal is
	// the prototype setter, so a literal declares nothing to coerce in the first place.
	it('keeps a declared __proto__ instead of losing it to the setter', () => {
		const { args } = coerceArgsToSchema(
			JSON.parse('{"__proto__":"legit","keep":1}'),
			JSON.parse('{"properties":{"__proto__":{"type":"string"},"keep":{"type":"number"}}}')
		)
		expect(Object.hasOwn(args, '__proto__')).toBe(true)
		expect(args['__proto__']).toBe('legit')
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

	it('strips every one of them, keeping a variable reference', () => {
		expect(stripSecretArgs(args, schema)).toEqual({
			// Naming a workspace variable is how a secret is meant to reach a job, and the
			// value never leaves it — so the reference is the caller's to send.
			obj: { inner: '$var:u/ada/prod', keep: 1 },
			list: [{ name: 'a' }, {}],
			either: { kind: 'a' }
		})
	})

	// `$jsonvar:` paths are minted from what the user typed into the form, so a caller naming
	// one is naming a secret it was never shown. A bare `$var:` names nothing.
	it('keeps only a reference that names a workspace variable', () => {
		const one = { properties: { token: { type: 'string', password: true } } }
		const kept = (v: unknown) => stripSecretArgs({ token: v }, one).token
		expect(kept('$var:f/team/api_token')).toBe('$var:f/team/api_token')
		expect(kept('$jsonvar:u/ada/secret_arg/abc123')).toBeUndefined()
		expect(kept('$var:')).toBeUndefined()
		expect(kept('hunter2')).toBeUndefined()
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

	// A container shaped unlike its declaration is kept, so the walk has to reach in
	// through the half the declaration does carry — descending on the value's shape alone
	// left the secret sitting there for the persisted card and the model to read.
	it('strips through a container shaped unlike its declaration', () => {
		const declaresArray = {
			properties: {
				rows: { type: 'array', items: { properties: { token: { password: true } } } }
			}
		}
		expect(stripSecretArgs({ rows: { token: 'hunter2' } }, declaresArray)).toEqual({ rows: {} })

		const declaresObject = {
			properties: { cfg: { type: 'object', properties: { token: { password: true } } } }
		}
		expect(stripSecretArgs({ cfg: [{ token: 'hunter2' }] }, declaresObject)).toEqual({ cfg: [{}] })
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
