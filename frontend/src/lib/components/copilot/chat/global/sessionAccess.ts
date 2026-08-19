import { UserService, type User } from '$lib/gen'
import { checkDeployPermission } from '$lib/utils_workspace_deploy'

/**
 * What a user may do in ONE workspace, as the AI session toolset needs to know it.
 *
 * These are permission facts, never relevance judgements — a capability is absent
 * only when the backend would refuse the call. Best-effort, not a boundary: the token
 * is the enforcement point, and every failure resolves OPEN — a failed `whoami` yields
 * every capability, a failed rules fetch reads as no rule active. So this narrows what
 * the model is offered; it does not guarantee it is never offered a tool that would 401.
 *
 * The rules below are NOT a role ladder: the backend's precedence between "admin"
 * and "operator" differs per capability. Collapsing them into one ordering would get
 * `write_draft` wrong for a superadmin whose workspace role is operator.
 */
export type SessionCapability =
	| 'write_draft'
	| 'run_preview'
	/** May deploy at least something: no operator or deployers-only refusal. */
	| 'deploy'
	/** May also deploy the kinds `check_deploy_rules` gates — everything except
	 * schedules and triggers, which no rule covers (`kindGatedByDeployRules`). */
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

/** Benefit of the doubt: an unresolvable role must not blank the toolset, since a
 * transient failure would otherwise tell a developer mid-session that they cannot
 * author anything — a worse and far less legible outcome than the 403 they get by
 * trying. Mirrors the fail-open contract of `checkDeployPermission`. */
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

	// windmill-api/src/drafts.rs `require_can_write_path`: `authed.is_admin` returns Ok
	// BEFORE the operator branch, so an admin who is also an operator may still save
	// drafts. That field is `usr.is_admin || super_admin` (windmill-api-auth/src/auth.rs)
	// while `whoami` reports the two separately, hence the OR.
	if (me.is_admin || me.is_super_admin || !me.operator) {
		capabilities.add('write_draft')
	}

	// windmill-api/src/jobs.rs `run_preview_script` / `run_preview_flow_job`: the operator
	// check comes first and has no admin escape — the opposite precedence to drafts.
	if (!me.operator) {
		capabilities.add('run_preview')
	}

	// `checkDeployPermission` carries every term of the backend's `check_deploy_rules`,
	// superadmin included, so it is the whole answer. Splitting its verdict in two mirrors
	// `deployPermissionForKind`: a direct-deployment lock stops the kinds that reach
	// `check_deploy_rules` and nothing else, so schedules and triggers stay deployable —
	// the same narrowing the Compare page applies. An operator or deployers-only refusal
	// covers every kind, so it takes both.
	const deploy = await checkDeployPermission(workspace, me)
	if (deploy.ok) {
		capabilities.add('deploy')
		capabilities.add('deploy_gated_kinds')
	} else if (deploy.refusedBy === 'DisableDirectDeployment') {
		capabilities.add('deploy')
	}

	return { workspace, capabilities }
}
