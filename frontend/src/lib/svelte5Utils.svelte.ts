// https://github.com/sveltejs/svelte/issues/14600

import { untrack } from 'svelte'
import { deepEqual } from 'fast-equals'
import type { StateStore } from './utils'
import { resource, watch, type ResourceReturn } from 'runed'

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

export function useReducedMotion(): { val: boolean } {
	if (typeof window === 'undefined') return { val: false }

	const query = window.matchMedia('(prefers-reduced-motion: reduce)')
	let s = $state(query.matches)
	$effect(() => {
		const handler = (event: MediaQueryListEvent) => {
			s = event.matches
		}
		query.addEventListener('change', handler)
		return () => query.removeEventListener('change', handler)
	})
	return {
		get val() {
			return s
		}
	}
}

// Prevents flickering when data is unloaded (undefined) then reloaded quickly
// But still becomes undefined if data is not reloaded within the timeout
// so the user has feedback that the data is not available anymore.
export class StaleWhileLoading<T> {
	private _current: T | undefined = $state()
	private _currentTimeout: ReturnType<typeof setTimeout> | undefined
	constructor(getter: () => T, timeout = 400) {
		watch(getter, (value) => {
			if (this._currentTimeout) clearTimeout(this._currentTimeout)
			if (value === undefined) {
				this._currentTimeout = setTimeout(() => (this._current = undefined), timeout)
			} else {
				this._current = value
			}
		})
	}
	get current(): T | undefined {
		return this._current
	}
}

export interface UseInfiniteQueryOptions<TData, TPageParam> {
	queryFn: (pageParam: TPageParam) => Promise<TData>
	initialPageParam: TPageParam
	getNextPageParam: (lastPage: TData, allPages: TData[]) => TPageParam | undefined
}

export interface UseInfiniteQueryReturn<TData> {
	current: TData[]
	isLoading: boolean
	isFetchingNextPage: boolean
	hasNextPage: boolean
	error: Error | undefined
	fetchNextPage: () => Promise<void>
	reset: () => void
}

/**
 * A Svelte 5 hook that detects when the user has scrolled to the bottom of an element
 *
 * @param selector - CSS selector for the element to monitor (if not provided, monitors window)
 * @param threshold - Distance from bottom in pixels to trigger (default: 0)
 * @returns Object with `current` getter that returns true when at bottom
 *
 * @example
 * // Monitor a specific element
 * const bottomDetector = useScrollToBottom('.my-scrollable-div', 100)
 *
 * // Monitor the window
 * const windowBottomDetector = useScrollToBottom()
 *
 * $effect(() => {
 *   if (bottomDetector.current) {
 *     console.log('User scrolled to bottom!')
 *   }
 * })
 */
export function useScrollToBottom(selector: string, threshold: number = 0): { current: boolean } {
	if (typeof window === 'undefined') return { current: false }

	let current = $state(false)

	$effect(() => {
		const el = document.querySelector(selector) as HTMLElement
		if (!el) {
			console.warn(`useScrollToBottom: Element with selector "${selector}" not found`)
			return
		}
		let element: HTMLElement = el

		function checkIfAtBottom() {
			const scrollTop = element.scrollTop
			const scrollHeight = element.scrollHeight
			const clientHeight = element.clientHeight

			current = scrollTop + clientHeight >= scrollHeight - threshold
		}

		// Check immediately
		checkIfAtBottom()

		// Add scroll listener
		element.addEventListener('scroll', checkIfAtBottom, { passive: true })
		if (element instanceof Window) {
			window.addEventListener('resize', checkIfAtBottom, { passive: true })
		}

		return () => {
			element.removeEventListener('scroll', checkIfAtBottom)
			if (element instanceof Window) {
				window.removeEventListener('resize', checkIfAtBottom)
			}
		}
	})

	return {
		get current() {
			return current
		}
	}
}

/**
 * A Svelte 5 hook for infinite query/pagination functionality
 *
 * @example
 * const query = useInfiniteQuery({
 *   queryFn: async (page) => fetchItems(page),
 *   initialPageParam: 0,
 *   getNextPageParam: (lastPage, allPages) =>
 *     lastPage.length > 0 ? allPages.length : undefined
 * })
 *
 * // Access data
 * query.current // All pages of data
 * query.isLoading // Initial loading state
 * query.isFetchingNextPage // Loading next page
 * query.hasNextPage // Whether more pages exist
 *
 * // Fetch next page
 * await query.fetchNextPage()
 */
export function useInfiniteQuery<TData, TPageParam = number>(
	options: UseInfiniteQueryOptions<TData, TPageParam>
): UseInfiniteQueryReturn<TData> {
	const { queryFn, initialPageParam, getNextPageParam } = options

	let pages = $state<TData[]>([])
	let isLoading = $state(true)
	let isFetchingNextPage = $state(false)
	let error = $state<Error | undefined>(undefined)
	let nextPageParam = $state<TPageParam | undefined>(initialPageParam)
	let currentPromise: Promise<void> | undefined

	async function fetchPage(pageParam: TPageParam): Promise<void> {
		try {
			const data = await queryFn(pageParam)
			pages = [...pages, data]
			nextPageParam = getNextPageParam(data, pages)
		} catch (err) {
			error = err instanceof Error ? err : new Error(String(err))
			throw err
		}
	}

	async function fetchNextPage(): Promise<void> {
		if (!nextPageParam || isFetchingNextPage) {
			return
		}

		const pageToFetch = nextPageParam
		isFetchingNextPage = true
		error = undefined

		const promise = fetchPage(pageToFetch).finally(() => {
			if (currentPromise === promise) {
				isFetchingNextPage = false
			}
		})

		currentPromise = promise
		return promise
	}

	function reset(): void {
		pages = []
		isLoading = true
		isFetchingNextPage = false
		error = undefined
		nextPageParam = initialPageParam
		currentPromise = undefined

		// Automatically fetch first page on reset
		untrack(() => {
			fetchPage(initialPageParam).finally(() => {
				isLoading = false
			})
		})
	}

	// Initial fetch
	untrack(() => {
		fetchPage(initialPageParam).finally(() => {
			isLoading = false
		})
	})

	return {
		get current() {
			return pages
		},
		get isLoading() {
			return isLoading
		},
		get isFetchingNextPage() {
			return isFetchingNextPage
		},
		get hasNextPage() {
			return nextPageParam !== undefined
		},
		get error() {
			return error
		},
		fetchNextPage,
		reset
	}
}
