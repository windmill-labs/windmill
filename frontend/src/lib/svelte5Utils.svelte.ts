// https://github.com/sveltejs/svelte/issues/14600

import { untrack } from 'svelte'
import { deepEqual } from 'fast-equals'
import { type StateStore } from './utils'
import { resource, type ResourceReturn } from 'runed'
import * as runed from 'runed/kit'
import type z from 'zod'

export function withProps<Component, Props>(component: Component, props: Props) {
	const ret = $state({
		component,
		props
	})
	return ret
}

export function createState<T>(initialValue: T): T {
	let s = $state(initialValue)
	return s
}

export function stateSnapshot<T>(state: T) {
	return $state.snapshot(state)
}
export function refreshStateStore<T>(store: StateStore<T>): void {
	store.val = $state.snapshot(store.val) as any
}

export type UsePromiseResult<T> = (
	| { status: 'loading'; value?: undefined; error?: undefined }
	| { status: 'error'; error: any; value?: undefined }
	| { status: 'ok'; value: T; error?: undefined }
	| { status: 'idle'; value?: undefined; error?: undefined }
) & {
	refresh: () => void
	clear: () => void
}

export type UsePromiseOptions = {
	loadInit?: boolean
	clearValueOnRefresh?: boolean
}

/**
 * @deprecated Use `resource` from `runed` instead
 */
export function usePromise<T>(
	createPromise: () => Promise<T>,
	{ loadInit = true, clearValueOnRefresh = true }: UsePromiseOptions = {}
): UsePromiseResult<T> {
	const ret: any = $state({
		status: 'idle',
		__promise: undefined,
		refresh: () => {
			untrack(() => {
				let promise = createPromise()
				ret.__promise = promise
				ret.status = 'loading'
				if (clearValueOnRefresh) ret.value = undefined
				ret.error = undefined

				promise
					.then((value) => {
						if (ret.__promise !== promise) return
						ret.value = value
						ret.status = 'ok'
					})
					.catch((error) => {
						if (ret.__promise !== promise) return
						ret.error = error
						ret.status = 'error'
					})
			})
		},
		clear: () => {
			ret.status = loadInit ? 'loading' : 'idle'
			ret.value = undefined
			ret.error = undefined
			ret.__promise = undefined
		}
	})
	if (loadInit) ret.refresh()

	return ret
}

/**
 * Generic change tracker class that monitors changes in state using deep equality comparison
 * and provides a counter to trigger Svelte 5 reactivity. Similar to the pattern used in
 * FlowGraphV2.svelte's onModulesChange2 function.
 */
export class ChangeTracker<T> {
	counter = $state(0)
	#lastState: T | undefined

	constructor(initialValue?: T) {
		this.#lastState = initialValue ? initialValue : undefined
	}

	/**
	 * Check if the value has changed and update the counter to trigger reactivity
	 * @param value - The current value to check for changes
	 * @returns true if the value changed, false otherwise
	 */
	track(value: T): boolean {
		if (!deepEqual(value, this.#lastState)) {
			this.#lastState = value
			this.counter++
			return true
		}
		return false
	}
}

/**
 * This allows using async resources that only fetch missing data based on a set of keys.
 * It maintains a Record of the fetched data and only calls the fetcher for keys that
 * are not already present in the map.
 * The fetcher takes a record of keys to allow fetching multiple items in a single call.
 */
export class MapResource<U, T> {
	private _cached: Record<string, T> = {}
	private _fetcherResource: ResourceReturn<Record<string, T>, unknown, false>

	constructor(
		getValues: () => Record<string, U>,
		fetcher: (toFetch: Record<string, U>) => Promise<Record<string, T>>
	) {
		this._fetcherResource = resource(getValues, async (values) => {
			let obj = { ...this._cached }

			// Delete keys that are no longer present.
			for (const key of Object.keys(obj)) {
				if (!(key in values)) {
					delete obj[key]
				}
			}

			// Determine which keys are missing and need to be fetched
			let toFetch: Record<string, U> = {}
			for (const [key, value] of Object.entries(values)) {
				if (!obj[key]) {
					toFetch[key] = value
				}
			}

			// Fetch missing data and update the map
			if (Object.keys(toFetch).length > 0) {
				let fetchedData = await fetcher(toFetch)
				for (const key of Object.keys(toFetch)) {
					let value = fetchedData[key]
					obj[key] = value
				}
			}

			this._cached = obj

			return { ...obj }
		})
	}

	get current(): Record<string, T> | undefined {
		return this._fetcherResource.current
	}

	get loading(): boolean {
		return this._fetcherResource.loading
	}

	get error(): Error | undefined {
		return this._fetcherResource.error
	}
}

export class ChangeOnDeepInequality<T> {
	private _cached: T | undefined = $state()

	constructor(compute: () => T) {
		$effect.pre(() => {
			const newVal = compute()
			if (!deepEqual(newVal, this._cached)) {
				this._cached = newVal
			}
		})
	}

	get value(): T {
		return this._cached!
	}
}

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
			get: () => derivedVal,
			set: (v) => {
				params[key] = v
				derivedVal = v
			},
			enumerable: true,
			configurable: true
		})
	}
	return obj
}
