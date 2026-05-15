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

/** Module-level snapshot of the last fetched items per workspace+kind. Used
 * for instant first paint (`getCachedItems`); the picker always re-fetches in
 * the background via `loadKind` so the snapshot becomes "what we last saw"
 * rather than "the source of truth". This guarantees freshness after AI
 * draft creates and after deploy without explicit invalidation. */
const lastFetched = new Map<string, WorkspaceCache>()
const inflight = new Map<string, Promise<WorkspaceItem[]>>()

const cacheKey = (workspace: string, kind: WorkspaceItemKind) => `${workspace}:${kind}`

export function getCachedItems(
	workspace: string,
	kind: WorkspaceItemKind
): WorkspaceItem[] | undefined {
	return lastFetched.get(workspace)?.[kind]
}

export async function loadKind(
	workspace: string,
	kind: WorkspaceItemKind
): Promise<WorkspaceItem[]> {
	const key = cacheKey(workspace, kind)
	const flying = inflight.get(key)
	if (flying) return flying

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
		const bucket = lastFetched.get(workspace) ?? {}
		bucket[kind] = items
		lastFetched.set(workspace, bucket)
		return items
	})()
	inflight.set(key, promise)
	try {
		return await promise
	} finally {
		inflight.delete(key)
	}
}
