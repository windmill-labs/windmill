import { get } from 'svelte/store'
import { userWorkspaces } from '$lib/stores'
import type { ProtectionRuleset, RoleCapabilities } from '$lib/gen'
import { getWorkspaceRole } from '$lib/user'
import { roleCanAuthor, roleCanDraft } from '$lib/editRights'
import { capabilitiesIn, FULL_CAPABILITIES } from '$lib/roleCapabilities'
import { checkDeployRules } from '$lib/utils_workspace_deploy'
import {
	isRuleActiveInRulesets,
	readProtectionRulesForWorkspace
} from '$lib/workspaceProtectionRules.svelte'
import { USE_MODE_ACCESS, type SessionAccess, type SessionCapability } from '../sessionCapabilities'

const ALL_CAPABILITIES: SessionCapability[] = [
	'write_draft',
	'write_flow_draft',
	'write_app_draft',
	'write_inline_flow_code',
	'write_inline_app_code',
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
	'admin',
	'modify_workspace'
]

/** Fail open, here and at every role resolution failure below: blanking a toolset on a
 * transient error tells a developer mid-session that they cannot author anything,
 * which is worse and far less legible than the 403 they get by trying. Matches
 * `checkDeployPermission`, which fails open for the same reason. The protection rule is
 * the exception (`protects`). */
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
	const capabilities = new Set<SessionCapability>(['modify_workspace'])
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
	if (caps.flows === 'full') {
		capabilities.add('write_inline_flow_code')
	}
	if (caps.apps === 'full') {
		capabilities.add('write_inline_app_code')
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

/** Whether `rulesets` make a session read-only. `DisableDirectDeployment` holds everyone, admins
 * and bypassers included, since production changes go through its dev workspace or a fork;
 * `RestrictDeployToDeployers` alone leaves its users their drafts. Unreadable rules protect too:
 * the server holds deploys to the rule but not drafts. */
function protects(rulesets: ProtectionRuleset[] | undefined): boolean {
	return !rulesets || isRuleActiveInRulesets(rulesets, 'DisableDirectDeployment')
}

// The session UI asks this from every preview tab, the composer and the picker, and the rules
// fetch behind it is not cached, so they share one verdict per workspace for a minute.
const READ_ONLY_TTL_MS = 60_000
const readOnlyVerdicts = new Map<string, { at: number; verdict: Promise<boolean> }>()
const settledVerdicts = new Map<string, { at: number; readOnly: boolean }>()

/** The read-only verdict alone, for the UI around a session. Fails closed as the chat does; a
 * failed read is not kept, so the next ask reads the rules again. */
export function isReadOnlySessionWorkspace(workspace: string): Promise<boolean> {
	const cached = readOnlyVerdicts.get(workspace)
	if (cached && Date.now() - cached.at < READ_ONLY_TTL_MS) return cached.verdict
	const at = Date.now()
	const verdict = readProtectionRulesForWorkspace(workspace).then((rulesets) => {
		if (!rulesets) readOnlyVerdicts.delete(workspace)
		else settledVerdicts.set(workspace, { at, readOnly: protects(rulesets) })
		return protects(rulesets)
	})
	readOnlyVerdicts.set(workspace, { at, verdict })
	return verdict
}

/** The verdict read for `workspace` within the minute, without asking: what the UI stands on
 * while a fresh component waits for its own ask. */
export function settledReadOnlyVerdict(workspace: string): boolean | undefined {
	const settled = settledVerdicts.get(workspace)
	return settled && Date.now() - settled.at < READ_ONLY_TTL_MS ? settled.readOnly : undefined
}

export async function resolveSessionAccess(workspace: string): Promise<SessionAccess> {
	const [rulesets, lookup] = await Promise.all([
		// Not the memoized verdict above: the chat's toolset follows a rule change on the next send.
		readProtectionRulesForWorkspace(workspace),
		// Shares the 5-minute memo with the identity the same pre-flight resolves beside this
		// one, so a send costs one `whoami` rather than two, at the cost of a role changed
		// elsewhere landing within that window rather than on the very next message.
		// `lookup_failed` covers a rejected request AND a body too malformed to map.
		getWorkspaceRole(workspace)
	])
	if (protects(rulesets)) return USE_MODE_ACCESS
	if (lookup.kind !== 'resolved') {
		return fullSessionAccess()
	}
	const me = lookup.user

	return capabilitiesForRole({
		isAdmin: !!me.is_admin || !!me.is_super_admin,
		deployRulesPass: (await checkDeployRules(workspace, me, rulesets)).ok,
		capabilities: capabilitiesIn(
			me,
			get(userWorkspaces).find((w) => w.id === workspace)
		)
	})
}
