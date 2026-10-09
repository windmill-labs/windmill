import { getContext, setContext } from 'svelte'
import { SvelteMap } from 'svelte/reactivity'
import { ScriptService, type RunnableActivity } from '$lib/gen'

export type ActivityKind = 'script' | 'flow'

const CONTEXT_KEY = 'homeActivity'
// The server caps one call at this many paths.
const MAX_PATHS_PER_CALL = 500

const EMPTY: RunnableActivity = { recent_runs: [], triggers: [] }

/**
 * Latest runs and attached triggers of the rows the home list renders. Rows
 * `request` their path as they mount; the requests of one tick are sent as a
 * single batched call, so a page of rows costs one round trip, not one per row.
 */
export class HomeActivity {
	#workspace: () => string | undefined
	#data = new SvelteMap<string, RunnableActivity>()
	#requested = new Set<string>()
	#pending = new Map<string, { kind: ActivityKind; path: string }>()
	#flushScheduled = false
	#generation = 0
	// Read by `request`, so a row's requesting effect re-runs after a reset.
	#version = $state(0)

	constructor(workspace: () => string | undefined) {
		this.#workspace = workspace
	}

	request(kind: ActivityKind, path: string) {
		void this.#version
		const key = activityKey(kind, path)
		if (this.#requested.has(key)) return
		this.#requested.add(key)
		this.#pending.set(key, { kind, path })
		if (!this.#flushScheduled) {
			this.#flushScheduled = true
			queueMicrotask(() => this.#flush())
		}
	}

	/** `undefined` until loaded; a loaded path with no activity reads as empty. */
	get(kind: ActivityKind, path: string): RunnableActivity | undefined {
		return this.#data.get(activityKey(kind, path))
	}

	/** Drop everything loaded, e.g. on workspace change or after a list reload. */
	reset() {
		this.#generation++
		this.#data.clear()
		this.#requested.clear()
		this.#pending.clear()
		this.#version++
	}

	async #flush() {
		this.#flushScheduled = false
		const workspace = this.#workspace()
		const batch = [...this.#pending.values()]
		this.#pending.clear()
		if (!workspace || batch.length === 0) return
		const generation = this.#generation
		for (let i = 0; i < batch.length; i += MAX_PATHS_PER_CALL) {
			const chunk = batch.slice(i, i + MAX_PATHS_PER_CALL)
			try {
				const res = await ScriptService.getRunnablesActivity({
					workspace,
					requestBody: {
						scripts: chunk.filter((c) => c.kind === 'script').map((c) => c.path),
						flows: chunk.filter((c) => c.kind === 'flow').map((c) => c.path)
					}
				})
				if (generation !== this.#generation) return
				for (const c of chunk) {
					const map = c.kind === 'flow' ? res.flows : res.scripts
					this.#data.set(activityKey(c.kind, c.path), map[c.path] ?? EMPTY)
				}
			} catch (e) {
				// The indicators are decorative: leave the rows without them, and let
				// a later request retry.
				console.error('Could not load runnables activity', e)
				for (const c of chunk) this.#requested.delete(activityKey(c.kind, c.path))
			}
		}
	}
}

function activityKey(kind: ActivityKind, path: string): string {
	return `${kind}:${path}`
}

export function setHomeActivity(activity: HomeActivity) {
	setContext(CONTEXT_KEY, activity)
}

export function getHomeActivity(): HomeActivity | undefined {
	return getContext<HomeActivity | undefined>(CONTEXT_KEY)
}
