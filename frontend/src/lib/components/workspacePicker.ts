import type { Flow, ListableApp, Script } from '$lib/gen'

export type WorkspaceItemKind = 'flow' | 'script' | 'app'

export type WorkspaceItem = {
	path: string
	summary: string
	kind: WorkspaceItemKind
	raw_app?: boolean
}

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

const cache = new Map<string, WorkspaceCache>()
const inflight = new Map<string, Promise<WorkspaceItem[]>>()

const cacheKey = (workspace: string, kind: WorkspaceItemKind) => `${workspace}:${kind}`

export function getCachedItems(
	workspace: string,
	kind: WorkspaceItemKind
): WorkspaceItem[] | undefined {
	return cache.get(workspace)?.[kind]
}

export async function loadKind(
	workspace: string,
	kind: WorkspaceItemKind
): Promise<WorkspaceItem[]> {
	const existing = cache.get(workspace)?.[kind]
	if (existing) return existing
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
		const bucket = cache.get(workspace) ?? {}
		bucket[kind] = items
		cache.set(workspace, bucket)
		return items
	})()
	inflight.set(key, promise)
	try {
		return await promise
	} finally {
		inflight.delete(key)
	}
}
