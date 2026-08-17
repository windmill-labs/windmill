import type { WorkspaceItemDiff } from '$lib/gen'

// Editor URL for a workspace-item diff, scoped to a given workspace. Returns
// undefined for kinds we don't have a dedicated editor for (resource,
// variable, schedule, triggers, …).
export function editUrlFor(d: WorkspaceItemDiff, workspaceId: string): string | undefined {
	const ws = encodeURIComponent(workspaceId)
	const path = d.path
	if (d.kind === 'flow') return `/flows/edit/${path}?workspace=${ws}`
	if (d.kind === 'script') return `/scripts/edit/${path}?workspace=${ws}`
	if (d.kind === 'app') return `/apps/edit/${path}?workspace=${ws}`
	if (d.kind === 'raw_app') return `/apps_raw/edit/${path}?workspace=${ws}`
	return undefined
}
