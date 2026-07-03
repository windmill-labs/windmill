import type { Flow, ListableApp, Script } from '$lib/gen'

export type WorkspaceItemKind = 'flow' | 'script' | 'app'

export type WorkspaceItem = {
	path: string
	summary: string
	kind: WorkspaceItemKind
	raw_app?: boolean
}

/** Display label for kinds — used in the picker rows and search section headers. */
export const KIND_LABEL: Record<WorkspaceItemKind, string> = {
	flow: 'Flows',
	script: 'Scripts',
	app: 'Apps'
}

/** Lowercase variant — matches the editor breadcrumb style. */
export const KIND_LABEL_LOWER: Record<WorkspaceItemKind, string> = {
	flow: 'flows',
	script: 'scripts',
	app: 'apps'
}

/** Composite keys used for keyboard nav and `initialHighlight`. The picker and
 * its callers must agree on these — keep them all here. `'all'` is a virtual
 * kind for the cross-kind root view; only valid for `kindKey` / `dirKey`
 * (leaves always belong to a real kind). */
export const kindKey = (k: WorkspaceItemKind | 'all') => `kind:${k}`
export const dirKey = (k: WorkspaceItemKind | 'all', fullPath: string) => `dir:${k}:${fullPath}`
export const leafKeyFor = (k: WorkspaceItemKind, path: string) => `leaf:${k}:${path}`

export function editPathFor(item: WorkspaceItem): string {
	if (item.kind === 'flow') return `/flows/edit/${item.path}`
	if (item.kind === 'script') return `/scripts/edit/${item.path}`
	return item.raw_app ? `/apps_raw/edit/${item.path}` : `/apps/edit/${item.path}`
}

type WorkspaceCache = {
	flow?: WorkspaceItem[]
	script?: WorkspaceItem[]
	app?: WorkspaceItem[]
}

/** Module-level session cache. Persists across picker mounts within a single
 * page session so cached kinds render on the first frame. Pickers revalidate
 * once per mount via `loadKind(..., { revalidate: true })`, so stale entries
 * self-heal on the next open; call `invalidate()` after creating/deleting an
 * item when even a brief flash of the stale list must be avoided. */
const cache = new Map<string, WorkspaceCache>()
const inflight = new Map<string, Promise<WorkspaceItem[]>>()
/** Bumped by `invalidate()`. Each in-flight `loadKind` captures the version
 * at start and only writes back to the cache if it still matches — so a
 * deploy mid-fetch can't have its stale predecessor repopulate the cache. */
const cacheVersion = new Map<string, number>()

const cacheKey = (workspace: string, kind: WorkspaceItemKind) => `${workspace}:${kind}`

const KINDS: WorkspaceItemKind[] = ['flow', 'script', 'app']

function bumpVersion(workspace: string, kind: WorkspaceItemKind) {
	const k = cacheKey(workspace, kind)
	cacheVersion.set(k, (cacheVersion.get(k) ?? 0) + 1)
}

export function getCachedItems(
	workspace: string,
	kind: WorkspaceItemKind
): WorkspaceItem[] | undefined {
	return cache.get(workspace)?.[kind]
}

/** Drop a workspace+kind (or a whole workspace) from the cache so the next
 * picker open re-fetches. Use after creating/deleting items. Also bumps the
 * version so any in-flight `loadKind` started before the invalidate won't
 * write its (now-stale) result back to the cache. */
export function invalidate(workspace: string, kind?: WorkspaceItemKind) {
	if (!kind) {
		cache.delete(workspace)
		for (const k of KINDS) bumpVersion(workspace, k)
		return
	}
	const bucket = cache.get(workspace)
	if (bucket) delete bucket[kind]
	bumpVersion(workspace, kind)
}

export async function loadKind(
	workspace: string,
	kind: WorkspaceItemKind,
	opts?: { revalidate?: boolean }
): Promise<WorkspaceItem[]> {
	const existing = cache.get(workspace)?.[kind]
	if (existing && !opts?.revalidate) return existing
	const key = cacheKey(workspace, kind)
	// An in-flight fetch is already hitting the network, so it satisfies a
	// revalidate request too.
	const flying = inflight.get(key)
	if (flying) return flying

	const startVersion = cacheVersion.get(key) ?? 0
	const promise = (async () => {
		const { ScriptService, FlowService, AppService } = await import('$lib/gen')
		let items: WorkspaceItem[]
		if (kind === 'flow') {
			const flows = await FlowService.listFlows({
				workspace,
				includeDraftOnly: true,
				withoutDescription: true
			})
			items = flows.map((f: Flow) => ({
				path: f.path,
				summary: f.summary ?? '',
				kind: 'flow' as const
			}))
		} else if (kind === 'script') {
			const scripts = await ScriptService.listScripts({
				workspace,
				includeDraftOnly: true,
				withoutDescription: true
			})
			items = scripts.map((s: Script) => ({
				path: s.path,
				summary: s.summary ?? '',
				kind: 'script' as const
			}))
		} else {
			const apps = await AppService.listApps({
				workspace,
				includeDraftOnly: true
			})
			items = apps.map((a: ListableApp) => ({
				path: a.path,
				summary: a.summary ?? '',
				kind: 'app' as const,
				raw_app: a.raw_app ?? false
			}))
		}
		// Only commit if the cache version hasn't changed since we started —
		// otherwise we'd overwrite a deliberate `invalidate()` with stale data.
		if ((cacheVersion.get(key) ?? 0) === startVersion) {
			const bucket = cache.get(workspace) ?? {}
			bucket[kind] = items
			cache.set(workspace, bucket)
		}
		return items
	})()
	inflight.set(key, promise)
	try {
		return await promise
	} finally {
		inflight.delete(key)
	}
}
