import { UserService, type User } from '$lib/gen'
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
	/** May deploy at least something. */
	| 'deploy'
	/** May also deploy the kinds `check_deploy_rules` gates — everything but schedules
	 * and triggers (`kindGatedByDeployRules`), which no rule covers. */
	| 'deploy_gated_kinds'

export type SessionAccess = {
	/** The workspace these capabilities were resolved against — a session targets its
	 * own (possibly forked) workspace, which is not necessarily the navigated one. */
	workspace: string
	capabilities: ReadonlySet<SessionCapability>
}

const ALL_CAPABILITIES: SessionCapability[] = [
	'write_draft',
	'run_preview',
	'deploy',
	'deploy_gated_kinds'
]

/** Fail open, here and at every resolution failure below: blanking a toolset on a
 * transient error tells a developer mid-session that they cannot author anything,
 * which is worse and far less legible than the 403 they get by trying. Matches
 * `checkDeployPermission`, which fails open for the same reason. */
export function fullSessionAccess(workspace: string): SessionAccess {
	return { workspace, capabilities: new Set(ALL_CAPABILITIES) }
}

export function hasCapabilities(
	access: SessionAccess | undefined,
	requires: readonly SessionCapability[]
): boolean {
	if (!access) return true
	return requires.every((c) => access.capabilities.has(c))
}

export async function resolveSessionAccess(workspace: string): Promise<SessionAccess> {
	let me: User
	try {
		me = await UserService.whoami({ workspace })
	} catch {
		return fullSessionAccess(workspace)
	}

	const capabilities = new Set<SessionCapability>()

	// Per-capability precedence, NOT a role ladder: drafts.rs `require_can_write_path`
	// returns Ok on `authed.is_admin` BEFORE its operator branch, while jobs.rs
	// `run_preview_*` refuses operators first with no admin escape. `authed.is_admin` is
	// `usr.is_admin || super_admin` (auth.rs), which `whoami` reports as two fields.
	if (me.is_admin || me.is_super_admin || !me.operator) {
		capabilities.add('write_draft')
	}
	if (!me.operator) {
		capabilities.add('run_preview')
	}

	// `checkDeployPermission` is the whole gate; its verdict splits the way
	// `deployPermissionForKind` does — a direct-deployment lock refuses only the gated
	// kinds, every other refusal covers all of them.
	const deploy = await checkDeployPermission(workspace, me)
	if (deploy.ok) {
		capabilities.add('deploy')
		capabilities.add('deploy_gated_kinds')
	} else if (deploy.refusedBy === 'DisableDirectDeployment') {
		capabilities.add('deploy')
	}

	return { workspace, capabilities }
}
