import { BROWSER } from 'esm-env'

/**
 * A module the app shell reaches only on a click (a drawer, a modal, the AI engine), imported
 * dynamically so it stays out of the shell's static imports: those all load and evaluate before
 * the first page renders.
 *
 * Every instance is also loaded once the browser is idle after the first paint (see
 * `prefetchLazyModulesWhenIdle`), so it is in memory by the time the user reaches for it and
 * `current` renders it in the same frame as the click.
 */
export class LazyModule<T> {
	current: T | undefined = $state(undefined)
	#loader: () => Promise<T>
	#loading: Promise<T> | undefined

	constructor(loader: () => Promise<T>) {
		this.#loader = loader
		if (BROWSER) enqueue(this)
	}

	load(): Promise<T> {
		this.#loading ??= this.#loader().then(
			(m) => (this.current = m),
			(e) => {
				// A failed chunk fetch is retried by the next call rather than cached.
				this.#loading = undefined
				throw e
			}
		)
		return this.#loading
	}
}

export function lazyModule<T>(loader: () => Promise<T>): LazyModule<T> {
	return new LazyModule(loader)
}

const queue: LazyModule<unknown>[] = []
let started = false
let draining = false

function enqueue(m: LazyModule<unknown>) {
	queue.push(m)
	if (started) drain()
}

function whenIdle(fn: () => void) {
	if ('requestIdleCallback' in window) window.requestIdleCallback(fn, { timeout: 2000 })
	else setTimeout(fn, 50)
}

// One module per idle callback, so warming never holds the main thread for long.
function drain() {
	if (draining) return
	draining = true
	whenIdle(function next() {
		const m = queue.shift()
		if (!m) {
			draining = false
			return
		}
		m.load().catch(() => {})
		whenIdle(next)
	})
}

/** Start warming every lazy module, after the next frame has painted. Called once the shell
 *  has mounted; earlier, the imports would compete with the page's own chunks. */
export function prefetchLazyModulesWhenIdle(): void {
	if (!BROWSER || started) return
	started = true
	requestAnimationFrame(() => setTimeout(drain))
}
