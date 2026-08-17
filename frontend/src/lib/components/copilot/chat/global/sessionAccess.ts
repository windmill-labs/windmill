import { UserService, type User } from '$lib/gen'
import { checkDeployPermission } from '$lib/utils_workspace_deploy'
import {
	canUserBypassRuleKindInRulesets,
	fetchProtectionRulesForWorkspace
} from '$lib/workspaceProtectionRules.svelte'

/**
 * What a user may do in ONE workspace, as the AI session toolset needs to know it.
 *
 * These are permission facts, never relevance judgements — a capability is absent
 * only when the backend would refuse the call. The client is not the enforcement
 * point (the token is); this exists so the model is never handed a tool whose every
 * invocation would 401.
 *
 * The rules below are NOT a role ladder: the backend's precedence between "admin"
 * and "operator" differs per capability. Collapsing them into one ordering would get
 * `write_draft` wrong for a superadmin whose workspace role is operator.
 */
export type SessionCapability = 'write_draft' | 'run_preview' | 'deploy'

export type SessionAccess = {
	/** The workspace these capabilities were resolved against — a session targets its
	 * own (possibly forked) workspace, which is not necessarily the navigated one. */
	workspace: string
	capabilities: ReadonlySet<SessionCapability>
}

const ALL_CAPABILITIES: SessionCapability[] = ['write_draft', 'run_preview', 'deploy']

/** Benefit of the doubt: an unresolvable role must not blank the toolset. Mirrors the
 * fail-open contract of `checkDeployPermission`, which is the same kind of advisory
 * client-side mirror and defers to the server for the actual refusal. */
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

/**
 * `ApiAuthed.is_admin` is `usr.is_admin || super_admin` (windmill-api-auth/src/auth.rs),
 * while `whoami` reports the two separately — so every rule below that mirrors an
 * `authed.is_admin` check must OR them back together.
 */
function isAuthedAdmin(me: User): boolean {
	return !!me.is_admin || !!me.is_super_admin
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
	// drafts.
	if (isAuthedAdmin(me) || !me.operator) {
		capabilities.add('write_draft')
	}

	// windmill-api/src/jobs.rs `run_preview_script` / `run_preview_flow_job`: the operator
	// check comes first and has no admin escape — the opposite precedence to drafts.
	if (!me.operator) {
		capabilities.add('run_preview')
	}

	// Mirrors the backend's `check_deploy_rules`, which gates on BOTH protection rules.
	// `checkDeployPermission` carries only the operator and `RestrictDeployToDeployers`
	// halves, so `DisableDirectDeployment` is checked on top. An admin bypasses every
	// rule, which is why the rulesets are fetched for everyone else only.
	const authedAdmin = isAuthedAdmin(me)
	const rulesets = authedAdmin ? undefined : await fetchProtectionRulesForWorkspace(workspace)
	const deployAllowed =
		(await checkDeployPermission(workspace, { ...me, is_admin: authedAdmin })).ok &&
		(authedAdmin ||
			canUserBypassRuleKindInRulesets(rulesets ?? [], 'DisableDirectDeployment', {
				is_admin: false,
				username: me.username,
				groups: me.groups ?? []
			}))
	if (deployAllowed) {
		capabilities.add('deploy')
	}

	return { workspace, capabilities }
}
