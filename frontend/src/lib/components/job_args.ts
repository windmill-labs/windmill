import { deepEqual } from 'fast-equals'

const isLockedProp = (prop: any) => !!prop?.disabled && 'default' in prop

/**
 * A field the schema disables is not the caller's to set: whatever it holds, the run
 * sends the schema's default. Only a field's own `disabled` counts — `SchemaForm`
 * propagates a parent's downward, so one locked by inheritance alone keeps its value.
 * It still renders that value in its locked input, so it is unnamed here, not unseen.
 * Returns the paths it overwrote so the caller can say so; notifying is the caller's job.
 */
export function enforceDisabledDefaults(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined
): { args: Record<string, any>; resetKeys: string[] } {
	// Copied even with nothing to enforce: callers hand the result to a form that edits
	// it in place, and one branch returning the input would write through to their copy.
	if (!schema?.properties) return { args: { ...args }, resetKeys: [] }
	const resetKeys: string[] = []
	const result = mapMatchingArgs(
		args,
		schema.properties,
		isLockedProp,
		(value, prop, path) => {
			// An argument never supplied was not overwritten: the field shows the default
			// either way, and a caller told otherwise would try to correct what it never sent.
			// By value, since a default can be an object or an array: identity would report
			// every run of such a field as overridden, the caller that got it right included.
			if (value !== undefined && !deepEqual(value, prop.default)) resetKeys.push(path)
			return prop.default
		},
		true
	)
	return { args: result, resetKeys }
}

/**
 * Paths removed from a caller's arguments, split by cause. Kept apart because the two
 * read as opposites to whoever is told: an undeclared argument is one to stop sending,
 * an unshowable one is an argument the script really has, sent in a shape no field fits.
 */
export type DroppedPaths = { undeclared: string[]; unshowable: string[] }

/**
 * Conform caller-supplied arguments to what a run form can actually show: drop what the
 * schema does not declare at any level, then apply {@link enforceDisabledDefaults}. An
 * argument with no field — including every argument of a script whose schema declares
 * none — would otherwise be approved without ever being seen. A mounted nested form
 * prunes its own extras, but `ArgInput` renders only the first 50 items of an array.
 */
export function conformArgsToSchema(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined
): { args: Record<string, any>; resetKeys: string[]; dropped: DroppedPaths } {
	const dropped: DroppedPaths = { undeclared: [], unshowable: [] }
	const known = dropUndeclaredArgs(args ?? {}, schema?.properties ?? {}, dropped)
	const { args: result, resetKeys } = enforceDisabledDefaults(known, schema)
	return { args: result, resetKeys, dropped }
}

/** The tag naming the selected `oneOf` branch. No branch has to declare it, but
 * `ArgInput` writes it into the value and reads it back to pick the branch that opens. */
const ONE_OF_TAG_KEYS = ['kind', 'label']

/**
 * Rebuild `holder` with only the arguments `properties` declares, appending the dotted
 * path of each one removed. Recurses down the levels the form nests, so an undeclared
 * argument cannot ride along inside a declared one.
 */
function dropUndeclaredArgs(
	holder: Record<string, any>,
	properties: Record<string, any>,
	dropped: DroppedPaths,
	path = '',
	alsoAllowed?: string[]
): Record<string, any> {
	// hasOwn, not `in`: every object inherits `constructor` and `toString`, so `in` would
	// wave through arguments no schema declares. Accumulated on a null prototype for the
	// same reason from the other side: assigning a declared `__proto__` into a plain `{}`
	// reaches the inherited setter instead, and the argument vanishes unreported.
	const kept: Record<string, any> = Object.create(null)
	for (const [key, value] of Object.entries(holder)) {
		const keyPath = path ? `${path}.${key}` : key
		if (!Object.hasOwn(properties, key)) {
			if (alsoAllowed?.includes(key)) kept[key] = value
			else dropped.undeclared.push(keyPath)
			continue
		}
		const nested = dropUndeclaredNested(value, properties[key], dropped, keyPath)
		if (nested === DROP) dropped.unshowable.push(keyPath)
		else kept[key] = nested
	}
	// Spread rather than the accumulator itself: $state.snapshot returns a null-prototype
	// object by identity, and a form editing it in place would write into the stored copy.
	return { ...kept }
}

/** Stands in for a value the caller must remove rather than keep. */
const DROP = Symbol('drop')

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
 * Declares a structure to filter against, rather than a free-form object. An empty
 * `properties` is what the parsers emit for a bare `dict`/`object` annotation, and
 * `ArgInput` gives it a JSON editor holding whatever the user types — so reading it as
 * structure would report every key the editor accepts as an argument nobody declared.
 */
const declaresProperties = (prop: any) =>
	prop?.properties != null && Object.keys(prop.properties).length > 0

/**
 * Whether `prop` declares a slot the form can show `value` in. A value that fits nowhere
 * is one the user would approve unseen: it matches no level below, so every filter falls
 * straight through it, and `ArgInput` binds it to a widget that renders nothing — an
 * object in a list slot, or in a scalar input.
 */
function fitsDeclaredShape(value: any, prop: any): boolean {
	if (value == null) return true
	if (declaresDynMultiselect(prop)) return Array.isArray(value)
	if (typeof value !== 'object') return true
	if (SCALAR_TYPES.has(prop?.type)) return false
	const isArray = Array.isArray(value)
	// Declared nested structure, never the declared `type`: a dyn-multiselect argument is
	// `type: 'object'` holding an array, so reading `type` would drop what the user picked.
	const declaresArray = prop?.items != null
	const declaresObject = declaresProperties(prop) || Array.isArray(prop?.oneOf)
	return !(isArray ? declaresObject && !declaresArray : declaresArray && !declaresObject)
}

function dropUndeclaredNested(value: any, prop: any, dropped: DroppedPaths, path: string): any {
	if (value == null) return value
	if (!fitsDeclaredShape(value, prop)) return DROP
	if (typeof value !== 'object') return value
	const isArray = Array.isArray(value)
	if (declaresProperties(prop) && !isArray) {
		return dropUndeclaredArgs(value, prop.properties, dropped, path)
	}
	// Every declared element shape, not just an object one: the guards above are what drop
	// an object in a scalar slot, so an element reaches them only by recursing here.
	if (prop?.items && isArray) {
		const kept: any[] = []
		value.forEach((item: any, i: number) => {
			const itemPath = `${path}[${i}]`
			const nested = dropUndeclaredNested(item, prop.items, dropped, itemPath)
			if (nested === DROP) dropped.unshowable.push(itemPath)
			else kept.push(nested)
		})
		return kept
	}
	if (Array.isArray(prop?.oneOf) && !isArray) {
		// The union of every branch, not the one the tag names: which branch is selected is
		// runtime state, and pruning by a stale tag would delete what the user typed.
		const union: Record<string, any> = {}
		for (const branch of prop.oneOf) {
			for (const [key, declared] of Object.entries(branch?.properties ?? {})) {
				// Two branches can declare one key with shapes that exclude each other, so the
				// declaration the value fits wins the union: a last-writer-wins merge validates
				// what the user filled in against a branch they never opened, and drops it.
				const held = union[key]
				if (
					held === undefined ||
					(!fitsDeclaredShape(value[key], held) && fitsDeclaredShape(value[key], declared))
				)
					union[key] = declared
			}
		}
		return dropUndeclaredArgs(value, union, dropped, path, ONE_OF_TAG_KEYS)
	}
	return value
}

/** The `oneOf` branch the form shows: `ArgInput` opens the one the value's tag names,
 * and the first branch when no tag names any. */
function selectedOneOfBranch(value: any, oneOf: any[]): any {
	const tag = ONE_OF_TAG_KEYS.map((key) => value?.[key]).find((v) => typeof v === 'string')
	return oneOf.find((branch) => tag != null && branch?.title === tag) ?? oneOf[0]
}

/**
 * Rebuild `holder` with `visit` applied to every argument whose schema property matches
 * `isLeaf`, at any depth; returning `undefined` removes that argument.
 *
 * Recursive because the form is: `ArgInput` mounts a nested `SchemaForm` for an object
 * property, for each element of an object-typed array, and for a `oneOf` branch. A
 * matching field mounts the same way down any of them, so a level left unvisited here is
 * one the model can prefill.
 *
 * Set `selectedBranchOnly` for a visitor whose result comes from the property it is
 * handed, so that a `oneOf` yields the branch the form shows rather than all of them.
 */
function mapMatchingArgs(
	holder: any,
	properties: Record<string, any>,
	isLeaf: (prop: any) => boolean,
	visit: (value: unknown, prop: any, path: string) => unknown,
	selectedBranchOnly = false,
	path = '',
	inOneOf = false
): any {
	if (holder == null || typeof holder !== 'object' || Array.isArray(holder)) return holder
	// Null prototype for the same reason as dropUndeclaredArgs: writing a declared
	// `__proto__` into a plain `{}` reaches the inherited setter, so the default a
	// disabled field must enforce would vanish instead of overwriting.
	const result: Record<string, any> = Object.assign(Object.create(null), holder)
	for (const [key, prop] of Object.entries<any>(properties)) {
		const keyPath = path ? `${path}.${key}` : key
		// A matching object is a leaf, not a level: a password object is stored whole as a
		// single $jsonvar: reference, and a file is one opaque base64 string.
		if (isLeaf(prop)) {
			// Reached only where every branch is visited, so an absent argument under one
			// belongs to a variant nobody selected: writing it would add it to the run.
			if (inOneOf && !Object.hasOwn(result, key)) continue
			const mapped = visit(result[key], prop, keyPath)
			if (mapped === undefined) delete result[key]
			else result[key] = mapped
			continue
		}
		// Absent means the form never carried this level; recursing would rebuild it and
		// leave the key behind holding undefined.
		if (!Object.hasOwn(result, key)) continue
		if (prop?.properties) {
			result[key] = mapMatchingArgs(
				result[key],
				prop.properties,
				isLeaf,
				visit,
				selectedBranchOnly,
				keyPath,
				inOneOf
			)
		} else if (prop?.items?.properties && Array.isArray(result[key])) {
			result[key] = result[key].map((item: any, i: number) =>
				mapMatchingArgs(
					item,
					prop.items.properties,
					isLeaf,
					visit,
					selectedBranchOnly,
					`${keyPath}[${i}]`,
					inOneOf
				)
			)
		} else if (Array.isArray(prop?.oneOf)) {
			// One branch or all of them, and the difference is the visitor: a value can carry
			// a key from a variant nobody selected, so stripping has to reach every branch,
			// while a default read off an unselected branch is one the form never showed.
			const branches = selectedBranchOnly
				? [selectedOneOfBranch(result[key], prop.oneOf)]
				: prop.oneOf
			for (const branch of branches) {
				if (branch?.properties) {
					result[key] = mapMatchingArgs(
						result[key],
						branch.properties,
						isLeaf,
						visit,
						selectedBranchOnly,
						keyPath,
						!selectedBranchOnly
					)
				}
			}
		}
	}
	// Spread rather than the accumulator itself, so callers get a plain object back.
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

/**
 * {@link mapMatchingArgs} against a schema that may declare nothing to match. Copied even
 * then, for the reason {@link enforceDisabledDefaults} copies: callers hand the result to
 * a form that edits it in place, and one branch returning the input would write every
 * keystroke through to their own copy.
 */
function mapMatchingOrCopy(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined,
	isLeaf: (prop: any) => boolean,
	visit: (value: unknown, prop: any, path: string) => unknown
): Record<string, any> {
	const properties = schema?.properties
	if (!properties) return { ...args }
	return mapMatchingArgs(args, properties, isLeaf, visit)
}

/**
 * Drop every password-typed argument, so a caller cannot propose a secret on the user's
 * behalf: password fields open empty and the user fills them in. Appends the path of
 * each one removed, so the caller can be told the field was emptied rather than left to
 * read the absence as the user having deleted it.
 */
export function stripSecretArgs(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined,
	strippedKeys?: string[]
): Record<string, any> {
	return mapMatchingOrCopy(args, schema, isSecretProp, (value, _prop, path) => {
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
	return mapMatchingOrCopy(args, schema, isFileProp, (value, _prop, path) => {
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
	return mapMatchingOrCopy(args, schema, isSecretProp, (value) =>
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
	return mapMatchingOrCopy(args, schema, isFileProp, (value) =>
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
