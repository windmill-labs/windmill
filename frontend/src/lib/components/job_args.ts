/**
 * A job's arguments as something other than the job sees them: prepared for a run form,
 * for the readers of one, and — at the bottom of the file — for a result view.
 *
 * The form filters split by what a mistake costs: conforming must not drop what the user
 * meant to send, so it stays exact and shallow, while stripping and redacting only blank
 * a field, so they go to any depth and err towards visiting too much.
 */
import { deepEqual } from 'fast-equals'

const isLockedProp = (prop: any) => !!prop?.disabled && 'default' in prop

/**
 * A field the schema disables is not the caller's to set: whatever it holds, the run
 * sends the schema's default. Top-level only, like every other filter here — see
 * {@link conformArgsToSchema}. Returns the keys it overwrote so the caller can say so;
 * notifying is the caller's job.
 */
export function enforceDisabledDefaults(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined
): { args: Record<string, any>; resetKeys: string[] } {
	// Accumulated on a null prototype: assigning a declared `__proto__` into a plain `{}`
	// reaches the inherited setter instead, and the default vanishes rather than landing.
	// Copied even with nothing to enforce, since callers hand the result to a form that
	// edits it in place and one branch returning the input would write through to theirs.
	const result: Record<string, any> = Object.assign(Object.create(null), args)
	if (!schema?.properties) return { args: { ...result }, resetKeys: [] }
	const resetKeys: string[] = []
	for (const [key, prop] of Object.entries<any>(schema.properties)) {
		if (!isLockedProp(prop)) continue
		// An argument never supplied was not overwritten: the field shows the default
		// either way, and a caller told otherwise would try to correct what it never sent.
		// By value, since a default can be an object or an array: identity would report
		// every run of such a field as overridden, the caller that got it right included.
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

/**
 * Keys removed from a caller's arguments, split by cause. Kept apart because the two
 * read as opposites to whoever is told: an undeclared argument is one to stop sending,
 * an unshowable one is an argument the script really has, sent in a shape no field fits.
 */
export type DroppedKeys = { undeclared: string[]; unshowable: string[] }

/**
 * Conform caller-supplied arguments to what a run form can show, then apply
 * {@link enforceDisabledDefaults}. An argument the schema does not declare — including
 * every argument of a script whose schema declares none — would otherwise be approved
 * without ever being seen, and one in a shape its field cannot bind renders as an empty
 * box over a value only the job gets.
 *
 * Top-level only, deliberately. `SchemaForm` prunes undeclared keys at every level it
 * mounts, and below the top the form has the same limitations everywhere else in the
 * product: a nested mismatch renders the way it renders on the script run page. Matching
 * that is the point — a filter precise enough to descend has to resolve `oneOf` branches
 * and merged declarations, and getting that wrong deletes what the user typed.
 */
export function conformArgsToSchema(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined
): { args: Record<string, any>; resetKeys: string[]; dropped: DroppedKeys } {
	const dropped: DroppedKeys = { undeclared: [], unshowable: [] }
	const properties = schema?.properties ?? {}
	// hasOwn, not `in`: every object inherits `constructor` and `toString`, so `in` would
	// wave through arguments no schema declares.
	const kept: Record<string, any> = Object.create(null)
	for (const [key, value] of Object.entries(args ?? {})) {
		if (!Object.hasOwn(properties, key)) dropped.undeclared.push(key)
		else if (!fitsDeclaredShape(value, properties[key])) dropped.unshowable.push(key)
		else kept[key] = value
	}
	const { args: result, resetKeys } = enforceDisabledDefaults({ ...kept }, schema)
	return { args: result, resetKeys, dropped }
}

/** Types `setInputCat` routes to a widget bound to a scalar. */
const SCALAR_TYPES = new Set(['string', 'number', 'integer', 'boolean'])

/**
 * Declares an array even though its `type` says `object`, and the only slot where a
 * mismatch is worse than unreadable: `MultiSelect` maps over the value as it renders,
 * so anything else throws and takes the whole form down — Cancel with it.
 */
const declaresDynMultiselect = (prop: any) =>
	typeof prop?.format === 'string' && prop.format.startsWith('dynmultiselect-')

/**
 * Whether a primitive fits a declared scalar type. Each scalar widget binds one JS type and
 * shows nothing else: a string in a number input renders blank, and `ArgInput.validateInput`
 * range-checks only an actual number, so the field passes as filled. The user would approve
 * an empty box over a value only the job ever sees.
 */
const fitsScalarType = (value: any, type: string): boolean =>
	type === 'integer' ? typeof value === 'number' : typeof value === type

/**
 * Whether `prop` declares a slot the form can show `value` in. Only the mismatches
 * `ArgInput` itself passes over: a scalar slot renders a wrong-typed value as an empty
 * box, with no error and Run still enabled, so the user approves nothing over a value
 * only the job gets. Where `ArgInput` already says something — "Expected an array, got
 * object instead" over a list, a nested form rewriting a stray array into its own shape —
 * the value is left to it, and this form reads like every other one in the product.
 */
function fitsDeclaredShape(value: any, prop: any): boolean {
	if (value == null) return true
	if (declaresDynMultiselect(prop)) return Array.isArray(value)
	if (!SCALAR_TYPES.has(prop?.type)) return true
	return typeof value !== 'object' && fitsScalarType(value, prop.type)
}

/**
 * Every bag of `properties` a declaration can show a value's keys through: both, when it
 * carries `properties` and `oneOf`, and every branch rather than the selected one. A value
 * can hold a key belonging to a variant nobody opened, and a secret sitting there leaves
 * the form just the same.
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
 * `undefined` removes it. Recursive because the form is: `ArgInput` mounts a nested
 * `SchemaForm` for an object property, for each element of an object-typed array, and for
 * a `oneOf` branch, so a level left unvisited is one a secret can sit at.
 *
 * Descends on the shape of the value, never on which keys the declaration happens to
 * carry: an array is read through `items` and an object through `properties`/`oneOf`, so a
 * declaration holding both cannot route one shape down the other's branch. Deliberately
 * permissive — it only ever removes or replaces what it matches, so reaching a level the
 * form would not have opened costs an emptied field, never a lost argument.
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

const isSecretProp = (prop: any) => !!prop?.password

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
function mapArgLeaves(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined,
	isLeaf: (prop: any) => boolean,
	visit: (value: unknown, prop: any, path: string) => unknown
): Record<string, any> {
	return mapLeaves(args ?? {}, { properties: schema?.properties ?? {} }, isLeaf, visit, '')
}

/**
 * Drop every password-typed argument, so a caller cannot propose a secret on the user's
 * behalf: the field falls back to whatever the script itself declares, as on any other
 * run form, and the user fills in the rest. Appends the path of
 * each one removed, so the caller can be told the field was emptied rather than left to
 * read the absence as the user having deleted it.
 */
export function stripSecretArgs(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined,
	strippedKeys?: string[]
): Record<string, any> {
	return mapArgLeaves(args, schema, isSecretProp, (value, _prop, path) => {
		if (value !== undefined) strippedKeys?.push(path)
		return undefined
	})
}

/**
 * Drop every file argument, so a caller cannot propose file bytes on the user's behalf:
 * the field opens empty and the user attaches the file. Bytes a form is prefilled with
 * are bytes the stored transcript carries, unbounded, for a value no caller can produce.
 * Reports what it removed for the same reason {@link stripSecretArgs} does.
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
