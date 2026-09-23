import { getWorkspaceRole } from '$lib/user'
import { checkDeployRules } from '$lib/utils_workspace_deploy'

/**
 * What a user may do in ONE workspace, as the AI session toolset needs to know it.
 * Best-effort, not a boundary — the token is the enforcement point, so this narrows
 * what the model is offered and guarantees nothing.
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

/** Pure, so the profiles this can produce are enumerable rather than listed by hand.
 * `isAdmin` is one field because the server's `authed.is_admin` is
 * `usr.is_admin || super_admin` (auth.rs), which `whoami` reports as two. */
export function capabilitiesForRole(role: {
	isAdmin: boolean
	operator: boolean
	deployRulesPass: boolean
}): SessionAccess {
	const capabilities = new Set<SessionCapability>()
	// Per-capability precedence, NOT a role ladder: drafts.rs `require_can_write_path`
	// returns Ok on `authed.is_admin` BEFORE its operator branch, while jobs.rs
	// `run_preview_*` refuses operators first with no admin escape.
	if (role.isAdmin || !role.operator) {
		capabilities.add('write_draft')
	}
	if (!role.operator) {
		capabilities.add('run_preview')
	}
	// No operator term: folders.rs `create_folder`, the one kind this gates, has no
	// operator check, and its RLS insert passes because the handler makes the creator an
	// owner. The rules alone also cover admins, who bypass them inside that check.
	if (role.deployRulesPass) {
		capabilities.add('deploy')
	}
	return capabilities
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

	return capabilitiesForRole({
		isAdmin: !!me.is_admin || !!me.is_super_admin,
		operator: !!me.operator,
		deployRulesPass: (await checkDeployRules(workspace, me)).ok
	})
}
