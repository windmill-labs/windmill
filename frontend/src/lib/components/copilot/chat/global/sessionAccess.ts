import { getWorkspaceRole } from '$lib/user'
import { checkDeployPermission } from '$lib/utils_workspace_deploy'

/**
 * What a user may do in ONE workspace, as the AI session toolset needs to know it.
 *
 * Permission facts, never relevance judgements: a capability is absent only when the
 * backend would refuse the call. Best-effort, not a boundary — the token is the
 * enforcement point, so this narrows what the model is offered and guarantees nothing.
 */
export type SessionCapability =
	| 'write_draft'
	| 'run_preview'
	/** May deploy the kinds `check_deploy_rules` gates. There is no capability for the
	 * rest — schedules and triggers reach no rule (`kindGatedByDeployRules`), so every
	 * user may deploy those and a capability for them would always be present. */
	| 'deploy'

export type SessionAccess = ReadonlySet<SessionCapability>

const ALL_CAPABILITIES: SessionCapability[] = ['write_draft', 'run_preview', 'deploy']

/** Fail open, here and at every resolution failure below: blanking a toolset on a
 * transient error tells a developer mid-session that they cannot author anything,
 * which is worse and far less legible than the 403 they get by trying. Matches
 * `checkDeployPermission`, which fails open for the same reason. */
export function fullSessionAccess(): SessionAccess {
	return new Set(ALL_CAPABILITIES)
}

export async function resolveSessionAccess(workspace: string): Promise<SessionAccess> {
	// Shares the 5-minute memo with the identity the same pre-flight resolves beside this
	// one, so a send costs one `whoami` rather than two, at the cost of a role changed
	// elsewhere landing within that window rather than on the very next message.
	// `lookup_failed` covers a rejected request AND a body too malformed to map.
	const lookup = await getWorkspaceRole(workspace)
	if (lookup.kind !== 'resolved') {
		return fullSessionAccess()
	}
	const me = lookup.user

	const capabilities = new Set<SessionCapability>()
	// The server's `authed.is_admin` is `usr.is_admin || super_admin` (auth.rs), which
	// `whoami` reports as two fields. Two of the three rules below turn on it.
	const isAdmin = me.is_admin || me.is_super_admin

	// Per-capability precedence, NOT a role ladder: drafts.rs `require_can_write_path`
	// returns Ok on `authed.is_admin` BEFORE its operator branch, while jobs.rs
	// `run_preview_*` refuses operators first with no admin escape.
	if (isAdmin || !me.operator) {
		capabilities.add('write_draft')
	}
	if (!me.operator) {
		capabilities.add('run_preview')
	}

	// Protection rules come from the shared preflight, but its operator refusal is not
	// mirrored: it is stricter than the server, and the one kind this gates — folders.rs
	// `create_folder` — has no operator check and bypasses its rules on `authed.is_admin`,
	// like the draft path above. Without the admin term this withholds a folder the server
	// creates, and the prompt tells a superadmin they cannot create one.
	if (isAdmin || (await checkDeployPermission(workspace, me)).ok) {
		capabilities.add('deploy')
	}

	return capabilities
}
