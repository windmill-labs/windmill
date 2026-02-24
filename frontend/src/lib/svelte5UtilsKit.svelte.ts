// SvelteKit-specific utilities
// This file should only be imported in SvelteKit apps as it depends on $app/environment

import { z } from 'zod'

export type SearchParamsResult<S extends z.ZodType> =
	S extends z.ZodObject<infer Shape>
		? { -readonly [K in keyof Shape]: z.infer<Shape[K]> } & Record<string, unknown>
		: z.infer<S> & Record<string, unknown>

/** Serialize a value to a URL search param string. Primitives are written as-is; anything else is JSON. */
function serializeParam(value: unknown): string {
	if (typeof value === 'string') return value
	if (typeof value === 'number') return String(value)
	if (typeof value === 'boolean') return String(value)
	return JSON.stringify(value)
}

/** Parse a raw string from the URL back to a typed value guided by the zod field schema. */
function deserializeParam(raw: string, fieldSchema: z.ZodType): unknown {
	// Unwrap nullable / optional / default wrappers to get the inner type
	let inner: z.ZodType = fieldSchema
	while (
		inner instanceof (z as any).ZodNullable ||
		inner instanceof (z as any).ZodOptional ||
		inner instanceof (z as any).ZodDefault
	) {
		inner = (inner as any)._def.innerType ?? (inner as any)._def.type
	}

	if (inner instanceof (z as any).ZodNumber) {
		const n = Number(raw)
		return isNaN(n) ? null : n
	}
	if (inner instanceof (z as any).ZodBoolean) {
		if (raw === 'true') return true
		if (raw === 'false') return false
		return null
	}
	// ZodString, ZodEnum, ZodLiteral, ZodAny, or unknown → try as plain string first,
	// fall back to JSON parse for non-string schemas
	if (
		inner instanceof (z as any).ZodString ||
		inner instanceof (z as any).ZodEnum ||
		inner instanceof (z as any).ZodLiteral
	) {
		return raw
	}
	// For any other type (object, array, …) try JSON
	try {
		return JSON.parse(raw)
	} catch {
		return raw
	}
}

/**
 * Returns a reactive object whose properties are synced to the browser's URL search params.
 * Reading a property returns the current value from the URL (reactive).
 * Writing a property updates the URL via history.replaceState (no navigation).
 *
 * Primitive types (string / number / boolean) are serialized as plain text.
 * Everything else is JSON-serialized.
 */
export function useSearchParams<S extends z.ZodType>(schema: S): SearchParamsResult<S> {
	const shape: Record<string, z.ZodType> = (schema as any).shape ?? {}
	const keys = Object.keys(shape)

	// Reactive snapshot of search params - one $state cell per key
	const values: Record<string, unknown> = $state(
		Object.fromEntries(
			keys.map((k) => {
				const raw = new URLSearchParams(window.location.search).get(k)
				const parsed = raw != null ? deserializeParam(raw, shape[k]) : null
				return [k, parsed]
			})
		)
	)

	function syncFromUrl() {
		const sp = new URLSearchParams(window.location.search)
		for (const k of keys) {
			const raw = sp.get(k)
			const parsed = raw != null ? deserializeParam(raw, shape[k]) : null
			;(values as any)[k] = parsed
		}
	}

	// Keep in sync when the user navigates back/forward
	$effect(() => {
		window.addEventListener('popstate', syncFromUrl)
		return () => window.removeEventListener('popstate', syncFromUrl)
	})

	// Build the proxy object: reads come from $state, writes go to the URL + $state
	const proxy: Record<string, unknown> = {}
	for (const key of keys) {
		Object.defineProperty(proxy, key, {
			get() {
				return (values as any)[key]
			},
			set(v: unknown) {
				;(values as any)[key] = v
				const sp = new URLSearchParams(window.location.search)
				if (v == null) {
					sp.delete(key)
				} else {
					sp.set(key, serializeParam(v))
				}
				const newUrl = sp.toString()
					? `${window.location.pathname}?${sp}`
					: window.location.pathname
				history.replaceState(history.state, '', newUrl)
			},
			enumerable: true,
			configurable: true
		})
	}

	return proxy as SearchParamsResult<S>
}
