// SvelteKit-specific utilities
// This file should only be imported in SvelteKit apps as it depends on $app/environment

import * as runed from 'runed/kit'
import type z from 'zod'

// The original from runed has a weird behavior with dedup reads causing duplicate effect runs
// (Every field has to be derived to avoid it : https://runed.dev/docs/utilities/use-search-params)
export function useSearchParams<S extends z.ZodType>(
	schema: S,
	options?: runed.SearchParamsOptions
): runed.ReturnUseSearchParams<S> {
	let params = runed.useSearchParams(schema, options)
	let keys = Object.keys((schema as any).shape ?? {})
	let obj = { ...params }
	for (const key of keys) {
		// Somehow using $derived does not trigger reactivity sometimes ...
		// (e.g: filters.arg in RunsPage.svelte updates in the URL but does not trigger reactivity)
		let derivedVal = $state(params[key])
		Object.defineProperty(obj, key, {
			get: () => {
				if (typeof derivedVal === 'string') return decodeURIComponent(derivedVal)
				return derivedVal
			},
			set: (v) => {
				const val = typeof v === 'string' ? encodeURIComponent(v) : v
				params[key] = val
				derivedVal = val
			},
			enumerable: true,
			configurable: true
		})
	}
	return obj
}
