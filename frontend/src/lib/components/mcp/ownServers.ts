import { get } from 'svelte/store'
import { userStore } from '$lib/stores'

/** The principals an `extra_perms` entry can name to grant this user access. */
export type McpViewer = { username: string | undefined; pgroups: string[] }

/**
 * Whether the chat lists this MCP server. A server under another user's `u/`
 * prefix carries that person's credentials, and an admin's database role lets
 * the resource listing return it. Acting through it would call the provider
 * as them, so the chat only offers it when its owner shared it explicitly.
 */
export function isOwnOrSharedMcpPath(
	path: string,
	extraPerms: Record<string, unknown> | undefined,
	viewer: McpViewer
): boolean {
	if (!path.startsWith('u/')) return true
	if (viewer.username !== undefined && path.startsWith(`u/${viewer.username}/`)) return true
	if (!extraPerms) return false
	return (
		(viewer.username !== undefined && `u/${viewer.username}` in extraPerms) ||
		viewer.pgroups.some((g) => g in extraPerms)
	)
}

export function listableMcpResource(r: {
	path: string
	extra_perms?: Record<string, unknown>
}): boolean {
	const user = get(userStore)
	return isOwnOrSharedMcpPath(r.path, r.extra_perms, {
		username: user?.username,
		pgroups: user?.pgroups ?? []
	})
}
