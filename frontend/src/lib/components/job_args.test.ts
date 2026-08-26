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
		const { args, dropped } = conformArgsToSchema(
			JSON.parse('{"keep":1,"force":true,"constructor":"x","toString":"y","__proto__":{"p":1}}'),
			{ properties: { keep: { type: 'number' } } }
		)
		expect(args).toEqual({ keep: 1 })
		expect(Object.getPrototypeOf(args)).toBe(Object.prototype)
		expect(dropped.undeclared.sort()).toEqual(['__proto__', 'constructor', 'force', 'toString'])
	})

	it('keeps a declared __proto__ instead of losing it to the setter', () => {
		const { args, dropped } = conformArgsToSchema(
			JSON.parse('{"__proto__":"legit","keep":1}'),
			JSON.parse('{"properties":{"__proto__":{"type":"string"},"keep":{"type":"number"}}}')
		)
		expect(Object.hasOwn(args, '__proto__')).toBe(true)
		expect(args['__proto__']).toBe('legit')
		expect(dropped.undeclared).toEqual([])
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
		const { args, dropped } = conformArgsToSchema(
			{ cfg: { batch: 10, evil: true }, rows: [{ mode: 'safe' }, { mode: 'safe', evil: true }] },
			{
				properties: {
					cfg: { properties: { batch: { type: 'number' } } },
					rows: { items: { properties: { mode: { type: 'string' } } } }
				}
			}
		)
		expect(args).toEqual({ cfg: { batch: 10 }, rows: [{ mode: 'safe' }, { mode: 'safe' }] })
		expect(dropped.undeclared).toEqual(['cfg.evil', 'rows[1].evil'])
	})

	// ArgInput writes the tag itself and reads it back to pick the branch that opens, so
	// dropping it would reopen the form on the wrong variant.
	it('keeps the oneOf tag and every branch key', () => {
		const { args, dropped } = conformArgsToSchema(
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
		expect(dropped.undeclared).toEqual(['either.evil'])
	})

	// Merging the branches last-writer-wins validated the value against a branch the user
	// never opened, so submitting deleted what they had just filled in — and blamed the
	// script for it. Both orders, because either branch can be the one that loses.
	it('shape-checks a colliding oneOf key against the branch it fits', () => {
		const obj = {
			title: 'obj',
			properties: { src: { properties: { bucket: { type: 'string' } } } }
		}
		const list = { title: 'list', properties: { src: { items: { type: 'string' } } } }
		for (const branches of [
			[obj, list],
			[list, obj]
		]) {
			const schema = { properties: { either: { oneOf: branches } } }
			expect(
				conformArgsToSchema({ either: { kind: 'obj', src: { bucket: 'b' } } }, schema)
			).toEqual({
				args: { either: { kind: 'obj', src: { bucket: 'b' } } },
				resetKeys: [],
				dropped: { undeclared: [], unshowable: [] }
			})
			expect(conformArgsToSchema({ either: { kind: 'list', src: ['a'] } }, schema).args).toEqual({
				either: { kind: 'list', src: ['a'] }
			})
		}
		// Fitting no branch is still unshowable: the collision widens what the form can
		// show, it does not stop dropping what it cannot.
		expect(
			conformArgsToSchema(
				{ either: { kind: 'a', src: { evil: 1 } } },
				{
					properties: {
						either: {
							oneOf: [
								{ title: 'a', properties: { src: { type: 'string' } } },
								{ title: 'b', properties: { src: { type: 'number' } } }
							]
						}
					}
				}
			)
		).toMatchObject({ args: { either: { kind: 'a' } }, dropped: { unshowable: ['either.src'] } })
	})

	// Both declarations fit an object, so resolving the key to one of them filtered the
	// value against the branch the user did not open and emptied it. Both orders, because
	// only the branch that lost the tie was affected.
	it('merges what each oneOf branch declares under a key they share', () => {
		const left = { title: 'left', properties: { cfg: { properties: { l: { type: 'string' } } } } }
		const right = { title: 'right', properties: { cfg: { properties: { r: { type: 'string' } } } } }
		for (const branches of [
			[left, right],
			[right, left]
		]) {
			const schema = { properties: { either: { oneOf: branches } } }
			for (const [kind, cfg] of [
				['left', { l: 'L' }],
				['right', { r: 'R' }]
			] as const) {
				expect(conformArgsToSchema({ either: { kind, cfg } }, schema)).toEqual({
					args: { either: { kind, cfg } },
					resetKeys: [],
					dropped: { undeclared: [], unshowable: [] }
				})
			}
			// Widened to the union of both, not to anything: a key neither declares still goes.
			expect(
				conformArgsToSchema({ either: { kind: 'left', cfg: { evil: 1 } } }, schema).dropped
					.undeclared
			).toEqual(['either.cfg.evil'])
		}
	})

	// The union was accumulated on a plain object, so a branch key named `toString` read as
	// one an earlier branch had already declared and never made it in.
	it('keeps a oneOf branch key named like an Object.prototype member', () => {
		const { args, dropped } = conformArgsToSchema(
			{ either: { kind: 'a', toString: 'ts' } },
			{
				properties: {
					either: { oneOf: [{ title: 'a', properties: { toString: { type: 'string' } } }] }
				}
			}
		)
		expect(args).toEqual({ either: { kind: 'a', toString: 'ts' } })
		expect(dropped.undeclared).toEqual([])
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
		).toMatchObject({ args: {}, dropped: { undeclared: [], unshowable: ['rows', 'cfg'] } })
		// One level down too. A free-form object still passes unread: it declares no
		// structure for the value to contradict, so its contents were never filtered.
		expect(
			conformArgsToSchema({ rows: [{ token: 'a' }, ['sneaky']], free: { anything: 1 } }, schema)
		).toMatchObject({
			args: { rows: [{ token: 'a' }], free: { anything: 1 } },
			dropped: { undeclared: [], unshowable: ['rows[1]'] }
		})
	})

	// What the parsers emit for a bare `dict`/`object` annotation, and `ArgInput` gives it a
	// JSON editor: reading the empty declaration set as structure reported every key the
	// user typed as one the script has no field for, then ran it with an empty object.
	it('leaves a free-form object declared with empty properties alone', () => {
		const freeForm = { type: 'object', properties: {} }
		expect(
			conformArgsToSchema({ cfg: { env: 'prod', retries: 2 } }, { properties: { cfg: freeForm } })
		).toEqual({
			args: { cfg: { env: 'prod', retries: 2 } },
			resetKeys: [],
			dropped: { undeclared: [], unshowable: [] }
		})
		// Nested and per element, since the same declaration reaches both.
		expect(
			conformArgsToSchema(
				{ outer: { cfg: { env: 'prod' } }, rows: [{ a: 1 }] },
				{
					properties: {
						outer: { type: 'object', properties: { cfg: freeForm } },
						rows: { type: 'array', items: freeForm }
					}
				}
			).args
		).toEqual({ outer: { cfg: { env: 'prod' } }, rows: [{ a: 1 }] })
	})

	// The guard reads declared structure, never the declared `type`: a dyn-multiselect is
	// `type: 'object'` holding an array, and reading `type` dropped what the user picked.
	it('keeps a dyn-multiselect array and drops an object in a scalar slot', () => {
		const schema = {
			properties: {
				tenants: { type: 'object', format: 'dynmultiselect-list_tenants' },
				name: { type: 'string' }
			}
		}
		expect(
			conformArgsToSchema(
				{ tenants: ['acme', 'globex'], name: { evil: '$var:u/ada/prod' } },
				schema
			)
		).toMatchObject({
			args: { tenants: ['acme', 'globex'] },
			dropped: { undeclared: [], unshowable: ['name'] }
		})
	})

	// A list element is bound to a widget the same way a top-level argument is, so an
	// object in a scalar slot renders as [object Object] and the run carries it verbatim.
	it('filters array elements whose schema declares no properties', () => {
		expect(
			conformArgsToSchema(
				{ tags: ['ok', { token: '$var:u/ada/prod' }] },
				{ properties: { tags: { type: 'array', items: { type: 'string' } } } }
			)
		).toMatchObject({ args: { tags: ['ok'] }, dropped: { unshowable: ['tags[1]'] } })
		expect(
			conformArgsToSchema(
				{ rows: [{ n: 1, evil: 2 }] },
				{
					properties: {
						rows: { items: { oneOf: [{ title: 'a', properties: { n: { type: 'number' } } }] } }
					}
				}
			)
		).toMatchObject({ args: { rows: [{ n: 1 }] }, dropped: { undeclared: ['rows[0].evil'] } })
	})

	// MultiSelect maps over the value while rendering, so a non-array here throws and
	// takes the whole form with it — the user cannot even cancel what they were shown.
	it('drops a non-array in a dyn-multiselect slot', () => {
		const schema = {
			properties: { tenants: { type: 'object', format: 'dynmultiselect-list_tenants' } }
		}
		expect(conformArgsToSchema({ tenants: { evil: 1 } }, schema)).toMatchObject({
			args: {},
			dropped: { unshowable: ['tenants'] }
		})
		expect(conformArgsToSchema({ tenants: 'acme' }, schema)).toMatchObject({
			args: {},
			dropped: { unshowable: ['tenants'] }
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

	// By value: a locked object default never matches by identity, so every run of such
	// a field reported a reset, and the caller was corrected for getting it right.
	it('reports no reset for an object default the caller already matched', () => {
		const objSchema = {
			properties: { opts: { type: 'object', disabled: true, default: { dry_run: true } } }
		}
		expect(enforceDisabledDefaults({ opts: { dry_run: true } }, objSchema).resetKeys).toEqual([])
		expect(enforceDisabledDefaults({ opts: { dry_run: false } }, objSchema).resetKeys).toEqual([
			'opts'
		])
	})

	// The tag picks the branch, as it does in ArgInput: a default read off the branch the
	// form never showed both overwrites the value and reports a reset nobody made.
	it('takes a disabled default two branches share from the one the form shows', () => {
		const locked = (def: string) => ({ type: 'string', default: def, disabled: true })
		const collide = {
			properties: {
				either: {
					oneOf: [
						{ title: 'a', properties: { mode: locked('alpha') } },
						{ title: 'b', properties: { mode: locked('beta') } }
					]
				}
			}
		}
		expect(enforceDisabledDefaults({ either: { kind: 'a', mode: 'alpha' } }, collide)).toEqual({
			args: { either: { kind: 'a', mode: 'alpha' } },
			resetKeys: []
		})
		expect(enforceDisabledDefaults({ either: { kind: 'b', mode: 'x' } }, collide)).toEqual({
			args: { either: { kind: 'b', mode: 'beta' } },
			resetKeys: ['either.mode']
		})
		// Untagged opens the first branch, so that is the default it must enforce.
		expect(enforceDisabledDefaults({ either: { mode: 'x' } }, collide).args).toEqual({
			either: { mode: 'alpha' }
		})
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
