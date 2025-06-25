// https://github.com/sveltejs/svelte/issues/14600

import { untrack } from 'svelte'
import type { StateStore } from './utils'

export function withProps<Component, Props>(component: Component, props: Props) {
	const ret = $state({
		component,
		props
	})
	return ret
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
) & {
	refresh: () => void
}

export type UsePromiseOptions = {
	loadInit?: boolean
}

export function usePromise<T>(
	createPromise: () => Promise<T>,
	{ loadInit = true }: UsePromiseOptions = {}
): UsePromiseResult<T> {
	const ret: any = $state({
		status: 'loading',
		__promise: undefined,
		refresh: () => {
			let promise = createPromise()
			ret.__promise = promise
			ret.status = 'loading'
			ret.value = undefined
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
		}
	})
	if (loadInit) ret.refresh()

	return ret
}

export type UsePaginatedResult<T> = {
	items: T[]
	status: 'loading' | 'error' | 'ok'
	currentPage: number
	loadMore: () => void
}

export function usePaginated<T>(query: (page: number) => Promise<{ items: T[] }>) {
	let s: UsePaginatedResult<T> = $state({
		items: [],
		status: 'loading',
		currentPage: 1,
		loadMore: () => {
			s.currentPage++
			s.status = 'loading'
			promise.refresh()
		}
	})

	const promise = usePromise(() => query(s.currentPage))
	$effect(() => {
		if (promise.status === 'ok') {
			untrack(() => {
				s.status = promise.status
				s.items = [...s.items, ...promise.value.items]
			})
		}
	})
	return s
}
