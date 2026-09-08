/**
 * A job's arguments prepared for a run form, its readers, and a result view. Coercing must
 * not lose what the caller meant to send, so it is exact and shallow; stripping and
 * redacting only blank a field, so they go to any depth and err towards visiting too much.
 */
import { deepEqual } from 'fast-equals'

const isLockedProp = (prop: any) => !!prop?.disabled && 'default' in prop

/**
 * A field the schema disables is not the caller's to set: the run sends the schema's
 * default whatever it holds. Top-level only, like every filter here. Returns the keys it
 * overwrote; notifying is the caller's job.
 */
export function enforceDisabledDefaults(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined
): { args: Record<string, any>; resetKeys: string[] } {
	// Null prototype: assigning a declared `__proto__` into a plain `{}` reaches the
	// inherited setter and the default vanishes. Always copied — callers bind the result to
	// a form that edits in place, so returning the input would write through to theirs.
	const result: Record<string, any> = Object.assign(Object.create(null), args)
	if (!schema?.properties) return { args: { ...result }, resetKeys: [] }
	const resetKeys: string[] = []
	for (const [key, prop] of Object.entries<any>(schema.properties)) {
		if (!isLockedProp(prop)) continue
		// Never supplied is not overwritten, and compared by value: a default can be an
		// object, where identity would report every correct run as overridden.
		if (result[key] !== undefined && !deepEqual(result[key], prop.default)) resetKeys.push(key)
		result[key] = prop.default
	}
	return { args: { ...result }, resetKeys }
}

/** How a form says what {@link enforceDisabledDefaults} overwrote, shared by the two that
 * run it so the wording cannot drift apart. */
export const resetKeysToast = (resetKeys: string[]): string =>
	`Disabled field${resetKeys.length > 1 ? 's' : ''} ${resetKeys
		.map((k) => `'${k}'`)
		.join(', ')} reset to default value${resetKeys.length > 1 ? 's' : ''}`

/** Types `setInputCat` routes to a widget bound to a scalar. */
const SCALAR_TYPES = new Set(['string', 'number', 'integer', 'boolean'])

/**
 * Declares an array though its `type` says `object`. A mismatch here throws rather than
 * reading wrong: `MultiSelect` maps over the value as it renders, so anything else takes
 * the form down, Cancel with it — a reference included, since it draws before resolving.
 */
const declaresDynMultiselect = (prop: any) =>
	typeof prop?.format === 'string' && prop.format.startsWith('dynmultiselect-')

/** Whether a primitive already is what its declared scalar type asks for. */
const fitsScalarType = (value: any, type: string): boolean =>
	type === 'integer' ? typeof value === 'number' : typeof value === type

/**
 * Resolved at run time, so the declared type describes what the job receives and never the
 * string standing in for it. `ArgInput.validateInput` blesses these ahead of every type check.
 */
const REFERENCE_PREFIXES = ['$var:', '$res:', '$jsonvar:']
const isReference = (value: any): boolean =>
	typeof value === 'string' && REFERENCE_PREFIXES.some((prefix) => value.startsWith(prefix))

/** No plain reading in the declared type; distinct from a value that reads as `undefined`. */
const UNCOERCIBLE = Symbol('uncoercible')

/**
 * The value a scalar widget would stand for, or {@link UNCOERCIBLE}. Only conversions with
 * one plain reading: a number input shows `"7"` as 7 and a toggle shows any non-empty
 * string as on, so guessing past this would put a value on screen that nobody wrote.
 */
function coerceScalar(value: any, type: string): any {
	if (typeof value === 'object') return UNCOERCIBLE
	if (type === 'string') {
		return typeof value === 'number' || typeof value === 'boolean' ? String(value) : UNCOERCIBLE
	}
	if (typeof value !== 'string') return UNCOERCIBLE
	const trimmed = value.trim()
	if (type === 'number' || type === 'integer') {
		if (trimmed === '') return UNCOERCIBLE
		const parsed = Number(trimmed)
		return Number.isFinite(parsed) ? parsed : UNCOERCIBLE
	}
	if (type === 'boolean') {
		if (trimmed.toLowerCase() === 'true') return true
		if (trimmed.toLowerCase() === 'false') return false
	}
	return UNCOERCIBLE
}

/**
 * Drop every argument the schema does not declare, naming them. The schema is what every run
 * surface builds its fields from, so a value under a name it never declares has no widget
 * anywhere: sending one is sending what nobody could see or edit before the run.
 */
export function dropUndeclaredArgs(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined
): { args: Record<string, any>; undeclaredKeys: string[] } {
	const properties = schema?.properties ?? {}
	// hasOwn, not `in`: every object inherits `constructor` and `toString`, so `in` would
	// hand an inherited declaration to an argument the schema never named.
	const kept: Record<string, any> = Object.create(null)
	const undeclaredKeys: string[] = []
	for (const [key, value] of Object.entries(args ?? {})) {
		if (Object.hasOwn(properties, key)) kept[key] = value
		else undeclaredKeys.push(key)
	}
	return { args: { ...kept }, undeclaredKeys }
}

/**
 * Make arguments say what the run form will show, then apply {@link enforceDisabledDefaults}.
 * A scalar widget renders its own reading of a wrong-typed value and never writes it back, so
 * an untouched form would submit what it never displayed; a value with no reading is cleared.
 * Top-level only: descending means resolving `oneOf`, where being wrong rewrites user input.
 */
export function coerceArgsToSchema(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined
): {
	args: Record<string, any>
	resetKeys: string[]
	clearedKeys: string[]
	undeclaredKeys: string[]
} {
	const properties = schema?.properties ?? {}
	const clearedKeys: string[] = []
	const { args: declared, undeclaredKeys } = dropUndeclaredArgs(args, schema)
	const kept: Record<string, any> = Object.create(null)
	for (const [key, value] of Object.entries(declared)) {
		// Declared, but a declaration can still be nothing, and reading `.type` off it throws.
		const prop = properties[key]
		if (
			prop === undefined ||
			value == null ||
			(isReference(value) && !declaresDynMultiselect(prop))
		) {
			kept[key] = value
			continue
		}
		if (declaresDynMultiselect(prop)) {
			if (Array.isArray(value)) kept[key] = value
			else clearedKeys.push(key)
			continue
		}
		if (!SCALAR_TYPES.has(prop.type) || fitsScalarType(value, prop.type)) {
			kept[key] = value
			continue
		}
		const coerced = coerceScalar(value, prop.type)
		if (coerced === UNCOERCIBLE) clearedKeys.push(key)
		else kept[key] = coerced
	}
	const { args: result, resetKeys } = enforceDisabledDefaults({ ...kept }, schema)
	return { args: result, resetKeys, clearedKeys, undeclaredKeys }
}

/**
 * Every bag of `properties` a declaration can show a value's keys through, including every
 * `oneOf` branch rather than the selected one: a secret under a variant nobody opened
 * leaves the form just the same.
 */
function declarationBags(prop: any): Record<string, any>[] {
	const bags: Record<string, any>[] = []
	if (prop?.properties) bags.push(prop.properties)
	if (Array.isArray(prop?.oneOf))
		for (const branch of prop.oneOf) if (branch?.properties) bags.push(branch.properties)
	return bags
}

/**
 * Apply `visit` to every value whose declaration matches `isLeaf`, at any depth; returning
 * `undefined` removes it. Recursive because the form is, so a level left unvisited is one a
 * secret can sit at. Descends on the value's shape, never on the declaration's keys: one
 * carrying both `items` and `properties` must not route a shape down the other's branch.
 */
function mapLeaves(
	value: any,
	prop: any,
	isLeaf: (prop: any) => boolean,
	visit: (value: unknown, prop: any, path: string) => unknown,
	path: string
): any {
	if (value == null || typeof value !== 'object') return value
	// A container shaped unlike its declaration is kept rather than dropped, since the widget
	// is the one that reports it — so the walk has to reach in through whichever half the
	// declaration does carry, or a secret under one leaves the form verbatim.
	if (Array.isArray(value))
		return value.map((item, i) =>
			mapLeaves(item, prop?.items ?? prop, isLeaf, visit, `${path}[${i}]`)
		)
	const bags = declarationBags(prop)
	if (bags.length === 0)
		return prop?.items ? mapLeaves(value, prop.items, isLeaf, visit, path) : value
	// Null prototype, and keyed off the value rather than the declaration: a key is only
	// ever rewritten where it already exists, so no branch of a `oneOf` can add one.
	const result: Record<string, any> = Object.assign(Object.create(null), value)
	for (const key of Object.keys(result)) {
		const declared = bags.filter((bag) => Object.hasOwn(bag, key)).map((bag) => bag[key])
		const keyPath = path ? `${path}.${key}` : key
		// A matching object is a leaf, not a level: a password object is stored whole as a
		// single $jsonvar: reference, and a file is one opaque base64 string.
		const leaf = declared.find(isLeaf)
		if (leaf) {
			const mapped = visit(result[key], leaf, keyPath)
			if (mapped === undefined) delete result[key]
			else result[key] = mapped
			continue
		}
		for (const declaration of declared)
			result[key] = mapLeaves(result[key], declaration, isLeaf, visit, keyPath)
	}
	return { ...result }
}

export const isSecretProp = (prop: any) => !!prop?.password

const isFileProp = (prop: any) =>
	prop?.contentEncoding === 'base64' || prop?.items?.contentEncoding === 'base64'

function fileMarker(base64: string): string {
	const bytes = Math.floor((base64.length * 3) / 4)
	return bytes < 1024 * 1024
		? `<file: ${Math.max(1, Math.round(bytes / 1024))} KB>`
		: `<file: ${(bytes / 1024 / 1024).toFixed(1)} MB>`
}

/** {@link mapLeaves} over a whole argument object, against a schema that may declare
 * nothing to match. Copies either way, for the reason {@link enforceDisabledDefaults}
 * copies. */
export function mapArgLeaves(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined,
	isLeaf: (prop: any) => boolean,
	visit: (value: unknown, prop: any, path: string) => unknown
): Record<string, any> {
	return mapLeaves(args ?? {}, { properties: schema?.properties ?? {} }, isLeaf, visit, '')
}

/**
 * Drop every file argument, so a caller cannot propose file bytes on the user's behalf:
 * the field opens empty and the user attaches the file. Bytes a form is prefilled with
 * are bytes the stored transcript carries, unbounded, for a value no caller can produce.
 * Reports the path of each one removed, or the caller reads the absence as the user having
 * deleted the value.
 */
export function stripFileArgs(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined,
	strippedKeys?: string[]
): Record<string, any> {
	return mapArgLeaves(args, schema, isFileProp, (value, _prop, path) => {
		if (value !== undefined) strippedKeys?.push(path)
		return undefined
	})
}

/**
 * Replace every password-typed argument with a fixed marker, for text that leaves the
 * form. A reference is enough to run a job on something the reader cannot see.
 */
export function redactSecretArgs(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined
): Record<string, any> {
	return mapArgLeaves(args, schema, isSecretProp, (value) =>
		value == null ? undefined : '<hidden>'
	)
}

/**
 * Replace every file argument with a marker naming its size. The base64 belongs in the
 * job request and nowhere else: rendered it is unreadable, persisted it is unbounded, and
 * a file small enough to survive truncation reaches the model whole.
 */
export function redactFileArgs(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined
): Record<string, any> {
	const mark = (value: unknown) => (typeof value === 'string' ? fileMarker(value) : value)
	return mapArgLeaves(args, schema, isFileProp, (value) =>
		Array.isArray(value) ? value.map(mark) : mark(value)
	)
}

export function isWindmillTooBigObject(obj: any): boolean {
	return (
		typeof obj === 'object' &&
		deepEqual(Object.keys(obj), ['reason']) &&
		obj['reason'] == 'WINDMILL_TOO_BIG'
	)
}
