import { deepEqual } from 'fast-equals'

/**
 * A field the schema disables is not the caller's to set: whatever it holds, the run
 * sends the schema's default. Returns the keys it actually overwrote so the caller can
 * say so — notifying is the caller's job, this stays pure.
 *
 * Top-level properties only; a disabled field nested in an object is not normalized.
 */
export function enforceDisabledDefaults(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined
): { args: Record<string, any>; resetKeys: string[] } {
	if (!schema?.properties) return { args, resetKeys: [] }
	const result = { ...args }
	const resetKeys: string[] = []
	for (const [key, prop] of Object.entries(schema.properties) as [string, any][]) {
		if (!prop?.disabled || !('default' in prop)) continue
		if (result[key] !== prop.default) resetKeys.push(key)
		result[key] = prop.default
	}
	return { args: result, resetKeys }
}

/**
 * Conform caller-supplied arguments to what a run form can actually show: drop what the
 * schema does not declare, then apply {@link enforceDisabledDefaults}. An argument with
 * no field — including every argument of a script whose schema declares none — would
 * otherwise be approved without ever being seen.
 */
export function conformArgsToSchema(
	args: Record<string, any>,
	schema: { properties?: Record<string, any> } | undefined
): { args: Record<string, any>; resetKeys: string[]; droppedKeys: string[] } {
	const properties = schema?.properties ?? {}
	const known: Record<string, any> = {}
	const droppedKeys: string[] = []
	for (const [key, value] of Object.entries(args ?? {})) {
		// hasOwn, not `in`: every object inherits `constructor`, `toString` and
		// `__proto__`, so `in` would wave through arguments no schema declares —
		// and assigning `__proto__` would mutate the accumulator instead of it.
		if (Object.hasOwn(properties, key)) {
			known[key] = value
		} else {
			droppedKeys.push(key)
		}
	}
	const { args: result, resetKeys } = enforceDisabledDefaults(known, schema)
	return { args: result, resetKeys, droppedKeys }
}

/**
 * Rebuild `holder` with `visit` applied to every password-typed argument the schema
 * declares, at any depth; returning `undefined` removes that argument.
 *
 * Recursive because the form is: `ArgInput` renders a nested `SchemaForm` for any object
 * property declaring properties of its own. Password props inside `items` or a `oneOf`
 * branch mount through a different path and are not reached — a known limit.
 */
function mapSecretArgs(
	holder: any,
	properties: Record<string, any>,
	visit: (value: unknown) => unknown
): any {
	if (holder == null || typeof holder !== 'object' || Array.isArray(holder)) return holder
	const result = { ...holder }
	for (const [key, prop] of Object.entries<any>(properties)) {
		// A password-typed object is a leaf, not a level: it is stored whole as one
		// $jsonvar: reference rather than field by field.
		if (prop?.password) {
			const mapped = visit(result[key])
			if (mapped === undefined) delete result[key]
			else result[key] = mapped
		} else if (prop?.properties) {
			result[key] = mapSecretArgs(result[key], prop.properties, visit)
		}
	}
	return result
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
	return mapSecretArgs(args, properties, () => undefined)
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
	return mapSecretArgs(args, properties, (value) => (value == null ? undefined : '<hidden>'))
}

export function isWindmillTooBigObject(obj: any): boolean {
	return (
		typeof obj === 'object' &&
		deepEqual(Object.keys(obj), ['reason']) &&
		obj['reason'] == 'WINDMILL_TOO_BIG'
	)
}
