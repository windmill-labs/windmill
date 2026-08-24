import { type User, UserService } from '$lib/gen'
import type { UserExt } from './stores.js'

async function fetchUserExt(workspace: string): Promise<UserExt> {
	return mapUserToUserExt(await UserService.whoami({ workspace }), workspace)
}

export async function getUserExt(workspace: string): Promise<UserExt | undefined> {
	try {
		return await fetchUserExt(workspace)
	} catch (error) {
		return undefined
	}
}

// A role gate must be able to tell "not this role" from "we don't know": a lookup that
// failed is no evidence about the user, so callers need to fall back rather than infer.
export type RoleLookup = { kind: 'resolved'; user: UserExt } | { kind: 'lookup_failed' }

// `whoami` for a workspace other than the one being browsed. Memoized because a
// per-workspace role gate can sit on a hot path. Only successes are kept, so a transient
// error doesn't stick, and they expire so a role edited elsewhere lands without a reload.
const WORKSPACE_ROLE_TTL_MS = 5 * 60_000
const workspaceRoleCache = new Map<string, { at: number; lookup: Promise<RoleLookup> }>()

export function getWorkspaceRole(workspace: string): Promise<RoleLookup> {
	const cached = workspaceRoleCache.get(workspace)
	if (cached && Date.now() - cached.at < WORKSPACE_ROLE_TTL_MS) return cached.lookup
	const lookup: Promise<RoleLookup> = fetchUserExt(workspace).then(
		(user) => ({ kind: 'resolved', user }),
		() => {
			// Evict only our own entry: a slow failure must not drop the retry that replaced it.
			if (workspaceRoleCache.get(workspace)?.lookup === lookup) workspaceRoleCache.delete(workspace)
			return { kind: 'lookup_failed' }
		}
	)
	workspaceRoleCache.set(workspace, { at: Date.now(), lookup })
	return lookup
}

/** Roles are per-identity, so anything that changes who is logged in must drop this. */
export function clearWorkspaceRoleCache(): void {
	workspaceRoleCache.clear()
}

function mapUserToUserExt(user: User, workspace: string): UserExt {
	const ext: UserExt = {
		...user,
		workspace_id: workspace,
		groups: user.groups!,
		pgroups: user.groups!.map((x) => `g/${x}`)
	}
	if (ext.is_service_account && sessionStorage.getItem('pre_impersonation_token')) {
		ext.impersonating_email = sessionStorage.getItem('pre_impersonation_email') ?? undefined
	}
	return ext
}
