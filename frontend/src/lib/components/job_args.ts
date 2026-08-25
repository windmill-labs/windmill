import { deepEqual } from 'fast-equals'

const isLockedProp = (prop: any) => !!prop?.disabled && 'default' in prop

/**
 * A field the schema disables is not the caller's to set: whatever it holds, the run
 * sends the schema's default. Only a field's own `disabled` counts — `SchemaForm`
 * propagates a parent's downward, so one locked by inheritance alone keeps its value.
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
	const result = mapMatchingArgs(args, schema.properties, isLockedProp, (value, prop, path) => {
		// An argument never supplied was not overwritten: the field shows the default
		// either way, and a caller told otherwise would try to correct what it never sent.
		if (value !== undefined && value !== prop.default) resetKeys.push(path)
		return prop.default
	})
	return { args: result, resetKeys }
}

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
): { args: Record<string, any>; resetKeys: string[]; droppedKeys: string[] } {
	const droppedKeys: string[] = []
	const known = dropUndeclaredArgs(args ?? {}, schema?.properties ?? {}, droppedKeys)
	const { args: result, resetKeys } = enforceDisabledDefaults(known, schema)
	return { args: result, resetKeys, droppedKeys }
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
	droppedKeys: string[],
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
			else droppedKeys.push(keyPath)
			continue
		}
		kept[key] = dropUndeclaredNested(value, properties[key], droppedKeys, keyPath)
	}
	// Spread rather than the accumulator itself: $state.snapshot returns a null-prototype
	// object by identity, and a form editing it in place would write into the stored copy.
	return { ...kept }
}

function dropUndeclaredNested(value: any, prop: any, droppedKeys: string[], path: string): any {
	if (value == null || typeof value !== 'object') return value
	if (prop?.properties && !Array.isArray(value)) {
		return dropUndeclaredArgs(value, prop.properties, droppedKeys, path)
	}
	if (prop?.items?.properties && Array.isArray(value)) {
		return value.map((item: any, i: number) =>
			item != null && typeof item === 'object' && !Array.isArray(item)
				? dropUndeclaredArgs(item, prop.items.properties, droppedKeys, `${path}[${i}]`)
				: item
		)
	}
	if (Array.isArray(prop?.oneOf) && !Array.isArray(value)) {
		// The union of every branch, not the one the tag names: which branch is selected is
		// runtime state, and pruning by a stale tag would delete what the user typed.
		const union: Record<string, any> = {}
		for (const branch of prop.oneOf) Object.assign(union, branch?.properties ?? {})
		return dropUndeclaredArgs(value, union, droppedKeys, path, ONE_OF_TAG_KEYS)
	}
	return value
}

/**
 * Rebuild `holder` with `visit` applied to every argument whose schema property matches
 * `isLeaf`, at any depth; returning `undefined` removes that argument.
 *
 * Recursive because the form is: `ArgInput` mounts a nested `SchemaForm` for an object
 * property, for each element of an object-typed array, and for a `oneOf` branch. A
 * matching field mounts the same way down any of them, so a level left unvisited here is
 * one the model can prefill.
 */
function mapMatchingArgs(
	holder: any,
	properties: Record<string, any>,
	isLeaf: (prop: any) => boolean,
	visit: (value: unknown, prop: any, path: string) => unknown,
	path = '',
	inOneOf = false
): any {
	if (holder == null || typeof holder !== 'object' || Array.isArray(holder)) return holder
	const result = { ...holder }
	for (const [key, prop] of Object.entries<any>(properties)) {
		const keyPath = path ? `${path}.${key}` : key
		// A matching object is a leaf, not a level: a password object is stored whole as a
		// single $jsonvar: reference, and a file is one opaque base64 string.
		if (isLeaf(prop)) {
			// Every oneOf branch is visited, so an absent argument under one belongs to a
			// variant that was not selected: a visitor that writes would add it to the run.
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
			result[key] = mapMatchingArgs(result[key], prop.properties, isLeaf, visit, keyPath, inOneOf)
		} else if (prop?.items?.properties && Array.isArray(result[key])) {
			result[key] = result[key].map((item: any, i: number) =>
				mapMatchingArgs(item, prop.items.properties, isLeaf, visit, `${keyPath}[${i}]`, inOneOf)
			)
		} else if (Array.isArray(prop?.oneOf)) {
			// Every branch, not the one the value's tag names: which branch is selected is
			// runtime state, and a missing or stale tag must not decide whether an argument
			// is visited.
			for (const branch of prop.oneOf) {
				if (branch?.properties) {
					result[key] = mapMatchingArgs(
						result[key],
						branch.properties,
						isLeaf,
						visit,
						keyPath,
						true
					)
				}
			}
		}
	}
	return result
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
 * Drop every password-typed argument, so a caller cannot propose a secret on the user's
 * behalf: password fields open empty and the user fills them in.
 */
export function stripSecretArgs(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined
): Record<string, any> {
	const properties = schema?.properties
	if (!properties) return args
	return mapMatchingArgs(args, properties, isSecretProp, () => undefined)
}

/**
 * Drop every file argument, so a caller cannot propose file bytes on the user's behalf:
 * the field opens empty and the user attaches the file. Bytes a form is prefilled with
 * are bytes the stored transcript carries, unbounded, for a value no caller can produce.
 */
export function stripFileArgs(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined
): Record<string, any> {
	const properties = schema?.properties
	if (!properties) return args
	return mapMatchingArgs(args, properties, isFileProp, () => undefined)
}

/**
 * Replace every password-typed argument with a fixed marker, for text that leaves the
 * form. A reference is enough to run a job on something the reader cannot see.
 */
export function redactSecretArgs(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined
): Record<string, any> {
	const properties = schema?.properties
	if (!properties) return args
	return mapMatchingArgs(args, properties, isSecretProp, (value) =>
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
	const properties = schema?.properties
	if (!properties) return args
	const mark = (value: unknown) => (typeof value === 'string' ? fileMarker(value) : value)
	return mapMatchingArgs(args, properties, isFileProp, (value) =>
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
