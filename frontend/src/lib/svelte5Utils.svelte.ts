// https://github.com/sveltejs/svelte/issues/14600

import { untrack } from 'svelte'
import { deepEqual } from 'fast-equals'
import { type StateStore } from './utils'

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
