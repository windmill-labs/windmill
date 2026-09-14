import { get } from 'svelte/store'
import { userStore } from '$lib/stores'
import { getWorkspaceRole } from '$lib/user'

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

/**
 * The viewer's identity in `workspace`. Username and groups are per workspace,
 * and a session chat can operate on a workspace other than the one being
 * browsed, so `userStore` only answers when it describes that same workspace.
 * A failed lookup yields no username: every `u/` server is then hidden rather
 * than judged against another workspace's identity.
 */
export async function mcpViewer(workspace: string): Promise<McpViewer> {
	const u = get(userStore)
	if (u?.workspace_id === workspace) return { username: u.username, pgroups: u.pgroups ?? [] }
	const role = await getWorkspaceRole(workspace)
	if (role.kind === 'resolved') {
		return { username: role.user.username, pgroups: role.user.pgroups ?? [] }
	}
	return { username: undefined, pgroups: [] }
}

export function listableMcpResource(
	r: { path: string; extra_perms?: Record<string, unknown> },
	viewer: McpViewer
): boolean {
	return isOwnOrSharedMcpPath(r.path, r.extra_perms, viewer)
}
