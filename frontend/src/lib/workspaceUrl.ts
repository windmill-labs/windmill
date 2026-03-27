import { derived, get } from 'svelte/store'
import { workspaceStore } from './stores'
import { base } from './base'

const GLOBAL_PATH_PREFIXES = [
	'/user/',
	'/oauth/',
	'/approve/',
	'/public/',
	'/a/',
	'/api/',
	'/view_graph',
	'/kitchen_sink',
	'/gh_success',
	'/test_dev',
	'/dev'
]

/** Extract workspace ID from a URL pathname matching `/w/<workspace>/...` */
export function extractWorkspaceFromPath(pathname: string): string | undefined {
	const stripped = base && pathname.startsWith(base) ? pathname.slice(base.length) : pathname
	const match = stripped.match(/^\/w\/([^/]+)/)
	return match?.[1]
}

/** Strip `/w/<workspace>` prefix from a pathname, returning the rest */
export function stripWsPrefix(pathname: string): string {
	const stripped = base && pathname.startsWith(base) ? pathname.slice(base.length) : pathname
	const match = stripped.match(/^\/w\/[^/]+(\/.*)?$/)
	if (match) {
		return match[1] || '/'
	}
	return stripped
}

/** Returns true if the path is global (not workspace-scoped) */
export function isGlobalPath(path: string): boolean {
	if (path.startsWith('?') || path.startsWith('#')) return true
	const stripped = base && path.startsWith(base) ? path.slice(base.length) : path
	// Already has /w/ prefix
	if (stripped.match(/^\/w\/[^/]+/)) return true
	return GLOBAL_PATH_PREFIXES.some((prefix) => stripped.startsWith(prefix))
}

/** Prepend `/w/<workspace>` to a workspace-scoped path */
export function toWorkspacePath(path: string, workspace?: string): string {
	if (isGlobalPath(path)) return path
	const ws = workspace ?? get(workspaceStore)
	if (!ws) return path
	return `/w/${ws}${path}`
}

/** Reactive store: `base + '/w/' + workspace` for use in `<a href>` attributes */
export const wsBase = derived(workspaceStore, ($ws) => {
	if (!$ws) return base
	return `${base}/w/${$ws}`
})

/** Imperative getter: returns `base + '/w/' + workspace` synchronously */
export function getWsBase(workspace?: string): string {
	const ws = workspace ?? get(workspaceStore)
	if (!ws) return base
	return `${base}/w/${ws}`
}
