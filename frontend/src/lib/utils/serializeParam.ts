// A leaf on purpose: navigation.ts (loaded by nearly every page) must not pull in zod
// through svelte5UtilsKit, and svelte5UtilsKit must not pull in $app/* through
// navigation.ts, since the sharedUtils library type-checks it outside SvelteKit.

/** Serialize a value to a URL search param string. Primitives are written as-is; anything else is JSON. */
export function serializeParam(value: unknown): string {
	if (typeof value === 'string') return value
	if (typeof value === 'number') return String(value)
	if (typeof value === 'boolean') return String(value)
	return JSON.stringify(value)
}
