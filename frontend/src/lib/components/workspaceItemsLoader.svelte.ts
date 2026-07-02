import { untrack } from 'svelte'
import {
	getCachedItems,
	loadKind,
	type WorkspaceItem,
	type WorkspaceItemKind
} from './workspacePicker'

/**
 * Shared loader for workspace items in drill pickers. Owns the
 * `loaded` / `loadingKind` state, the stale-while-revalidate `ensureLoaded`
 * coroutine, the `kind:` / `dir:` scope-segment decoder, and the
 * "load every kind on first search" filter callback.
 *
 * Both `WorkspaceItemDrillPicker` and `ChatContextPicker` mount a
 * `DrillPicker` over a workspace tree built from these maps. They each
 * keep their own scope-walking policy (chat collapses an optional
 * `'workspace'` wrapper segment; workspace handles single-kind mode at
 * the top), but the kind decoding and lazy fetch live here.
 *
 * Both getters are read inside the returned closures so changing
 * workspace or kinds after mount Just Works.
 */
function itemsEqual(a: WorkspaceItem[] | undefined, b: WorkspaceItem[]): boolean {
	if (!a || a.length !== b.length) return false
	return a.every(
		(item, i) =>
			item.path === b[i].path &&
			item.summary === b[i].summary &&
			item.kind === b[i].kind &&
			item.raw_app === b[i].raw_app
	)
}

export function useWorkspaceItemsLoader(
	workspace: () => string | undefined,
	kinds: () => readonly WorkspaceItemKind[]
) {
	// Seed from the module-level cache so kinds already fetched in this
	// session render on the first frame.
	let loaded = $state<Partial<Record<WorkspaceItemKind, WorkspaceItem[]>>>(
		(() => {
			const ws = untrack(workspace)
			if (!ws) return {}
			const out: Partial<Record<WorkspaceItemKind, WorkspaceItem[]>> = {}
			for (const k of untrack(kinds)) {
				const cached = getCachedItems(ws, k)
				if (cached) out[k] = cached
			}
			return out
		})()
	)
	let loadingKind = $state<Partial<Record<WorkspaceItemKind, boolean>>>({})
	// Workspace+kind pairs this loader instance has refreshed — at most one
	// background re-fetch per kind per mount. Marked on success only, so a
	// failed fetch is retried by the next call instead of stranding stale data.
	const revalidated = new Set<string>()

	async function ensureLoaded(kind: WorkspaceItemKind) {
		const ws = workspace()
		if (!ws) return
		const revalidateKey = `${ws}:${kind}`
		// `loaded[kind]` read inside `untrack` so callers wiring this into
		// a reactive context (DrillPicker's onFilterChange effect) don't
		// subscribe to a signal `ensureLoaded` itself writes — that would
		// re-fire the effect on every assignment and busy-loop.
		const hasCached = !!untrack(() => loaded[kind])
		if (hasCached && revalidated.has(revalidateKey)) return
		if (!hasCached) loadingKind[kind] = true
		try {
			const items = await loadKind(ws, kind, { revalidate: true })
			revalidated.add(revalidateKey)
			// Keep the reference stable when nothing changed so an open picker's
			// derived tree isn't rebuilt under the user on every revalidation.
			if (
				!itemsEqual(
					untrack(() => loaded[kind]),
					items
				)
			)
				loaded[kind] = items
		} catch (e) {
			// Callers fire-and-forget; surface the failure without an
			// unhandled rejection. Cached items (if any) keep rendering.
			console.error(`Failed to load workspace ${kind}s`, e)
		} finally {
			loadingKind[kind] = false
		}
	}

	function ensureAll() {
		for (const k of kinds()) ensureLoaded(k)
	}

	/** Decode one scope segment and trigger loads for the kind(s) it refers to.
	 * Accepts:
	 *  - `kind:<k>` (or `kind:all` — loads everything)
	 *  - `dir:<k>:<path>` (the single-kind layout where there's no `kind:`
	 *    wrapper at the top of the path)
	 * Unknown segments and kinds outside the current `kinds()` set are
	 * ignored — the caller has already filtered scope chains it cares about.
	 */
	function ensureForScopeSegment(segment: string) {
		const ks = kinds()
		const triggerKind = (k: string) => {
			if (k === 'all') return ensureAll()
			if ((ks as readonly string[]).includes(k)) ensureLoaded(k as WorkspaceItemKind)
		}
		if (segment.startsWith('kind:')) {
			triggerKind(segment.slice(5))
			return
		}
		if (segment.startsWith('dir:')) {
			const rest = segment.slice(4)
			const colon = rest.indexOf(':')
			if (colon > 0) triggerKind(rest.slice(0, colon))
		}
	}

	/** Global search → load every kind so results appear across the tree.
	 * Skip on the empty filter so a bare mount doesn't cold-load anything. */
	function onFilterChange(filter: string) {
		if (filter.trim() === '') return
		ensureAll()
	}

	return {
		get loaded() {
			return loaded
		},
		get loadingKind() {
			return loadingKind
		},
		ensureLoaded,
		ensureAll,
		ensureForScopeSegment,
		onFilterChange
	}
}
