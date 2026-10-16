import { get } from 'svelte/store'
import { userWorkspaces } from '$lib/stores'
import type { RoleCapabilities } from '$lib/gen'
import { getWorkspaceRole } from '$lib/user'
import { roleCanAuthor, roleCanDraft } from '$lib/editRights'
import { capabilitiesIn, FULL_CAPABILITIES } from '$lib/roleCapabilities'
import { checkDeployRules } from '$lib/utils_workspace_deploy'
import type { SessionAccess, SessionCapability } from '../sessionCapabilities'

const ALL_CAPABILITIES: SessionCapability[] = [
	'write_draft',
	'write_flow_draft',
	'write_app_draft',
	'write_schedule_draft',
	'write_trigger_draft',
	'write_resource_draft',
	'write_variable_draft',
	'run_preview',
	'run_flow_preview',
	'deploy',
	'manage_code',
	'manage_flows',
	'manage_apps',
	'manage_schedules',
	'manage_triggers',
	'admin'
]

/** Fail open, here and at every resolution failure below: blanking a toolset on a
 * transient error tells a developer mid-session that they cannot author anything,
 * which is worse and far less legible than the 403 they get by trying. Matches
 * `checkDeployPermission`, which fails open for the same reason. */
export function fullSessionAccess(): SessionAccess {
	return new Set(ALL_CAPABILITIES)
}

/** Pure, so the profiles this can produce are enumerable rather than listed by hand.
 * `isAdmin` is one field because the server's `authed.is_admin` is
 * `usr.is_admin || super_admin` (auth.rs), which `whoami` reports as two. An admin holds every
 * role capability, as `caller_capabilities` gives them. */
export function capabilitiesForRole(role: {
	isAdmin: boolean
	deployRulesPass: boolean
	capabilities: RoleCapabilities
}): SessionAccess {
	const capabilities = new Set<SessionCapability>()
	const caps = role.isAdmin ? FULL_CAPABILITIES : role.capabilities
	if (roleCanDraft('script', caps)) {
		capabilities.add('write_draft')
	}
	if (roleCanDraft('flow', caps)) {
		capabilities.add('write_flow_draft')
	}
	if (roleCanDraft('raw_app', caps)) {
		capabilities.add('write_app_draft')
	}
	if (roleCanDraft('schedule', caps)) {
		capabilities.add('write_schedule_draft')
	}
	if (roleCanDraft('trigger', caps)) {
		capabilities.add('write_trigger_draft')
	}
	if (roleCanDraft('resource', caps)) {
		capabilities.add('write_resource_draft')
	}
	if (roleCanDraft('variable', caps)) {
		capabilities.add('write_variable_draft')
	}
	if (roleCanAuthor('script', caps)) {
		capabilities.add('run_preview')
		capabilities.add('manage_code')
	}
	if (roleCanAuthor('flow', caps)) {
		capabilities.add('run_flow_preview')
		capabilities.add('manage_flows')
	}
	if (roleCanAuthor('raw_app', caps)) {
		capabilities.add('manage_apps')
	}
	if (roleCanAuthor('schedule', caps)) {
		capabilities.add('manage_schedules')
	}
	if (roleCanAuthor('trigger', caps)) {
		capabilities.add('manage_triggers')
	}
	// No role term: the rules are their own gate, and the handlers that also refuse a role
	// say so through `manage_code`. Admins bypass the rules inside the check.
	if (role.deployRulesPass) {
		capabilities.add('deploy')
	}
	if (role.isAdmin) {
		capabilities.add('admin')
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
		deployRulesPass: (await checkDeployRules(workspace, me)).ok,
		capabilities: capabilitiesIn(
			me,
			get(userWorkspaces).find((w) => w.id === workspace)
		)
	})
}
