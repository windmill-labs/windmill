// A leaf on purpose. navigation.ts (loaded by nearly every page) must not pull in zod
// through svelte5UtilsKit, and svelte5UtilsKit must not import navigation.ts: the
// sharedUtils declaration build type-checks it via utils.ts -> stores.ts ->
// dbManagerDrawerModel, where $app/* does not resolve.

/** Serialize a value to a URL search param string. Primitives are written as-is; anything else is JSON. */
export function serializeParam(value: unknown): string {
	if (typeof value === 'string') return value
	if (typeof value === 'number') return String(value)
	if (typeof value === 'boolean') return String(value)
	return JSON.stringify(value)
}
