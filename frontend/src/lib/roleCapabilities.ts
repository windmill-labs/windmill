import type { OperatorSettings, RoleCapabilities, RoleVisibility } from '$lib/gen'
import type { UserExt, UserWorkspace } from '$lib/stores'

/**
 * What a member's workspace role lets them author, manage and see: the frontend's copy of
 * `windmill_common::roles`, see `docs/workspace-roles.md`. Gate on these, never on
 * `UserExt.operator`, which only says "this role cannot write code" and so lets through a role
 * that may code but not, say, build apps.
 */

export type VisibilityKey = keyof RoleVisibility

const ALL_VISIBLE: RoleVisibility = {
	runs: true,
	schedules: true,
	resources: true,
	variables: true,
	assets: true,
	triggers: true,
	audit_logs: true,
	groups: true,
	folders: true,
	workers: true
}

export const FULL_CAPABILITIES: RoleCapabilities = {
	code: true,
	flows: 'full',
	apps: 'full',
	manage_schedules: true,
	manage_triggers: true,
	visibility: ALL_VISIBLE
}

/**
 * The operator role as a server that predates roles describes it, through `operator_settings`:
 * builder rights are `compose`, the manage rights are granted unless withdrawn, and a page is
 * visible only when its flag is `true` (absent settings hide every page, as they always did).
 */
export function legacyOperatorCapabilities(
	settings: OperatorSettings | null | undefined
): RoleCapabilities {
	const visibility = Object.fromEntries(
		(Object.keys(ALL_VISIBLE) as VisibilityKey[]).map((k) => [k, settings?.[k] === true])
	) as RoleVisibility
	return {
		code: false,
		flows: settings?.builder_flows === true ? 'compose' : 'none',
		apps: settings?.builder_apps === true ? 'compose' : 'none',
		manage_schedules: settings?.manage_schedules !== false,
		manage_triggers: settings?.manage_triggers !== false,
		visibility
	}
}

export type CapabilitiesUser = Partial<Pick<UserExt, 'is_admin' | 'is_super_admin' | 'operator'>>
export type CapabilitiesWorkspace = Pick<UserWorkspace, 'capabilities' | 'operator_settings'>

/**
 * The capabilities of `user` in `workspace` (its entry of `$userWorkspaces`). An admin holds
 * everything. The server's `capabilities` win; an older server sends none, and the role is then
 * derived from `operator` and `operator_settings` the way it read them. Without a user, a
 * non-null `operator_settings` is what marks an operator, as the server sends it to them only.
 */
export function capabilitiesIn(
	user: CapabilitiesUser | undefined,
	workspace: CapabilitiesWorkspace | undefined
): RoleCapabilities {
	if (user?.is_admin || user?.is_super_admin) return FULL_CAPABILITIES
	if (workspace?.capabilities) return workspace.capabilities
	const operator = user?.operator ?? workspace?.operator_settings != null
	return operator ? legacyOperatorCapabilities(workspace?.operator_settings) : FULL_CAPABILITIES
}

/** Whether the role is gated at all, as `RoleCapabilities::is_restricted`. */
export function isRestricted(caps: RoleCapabilities): boolean {
	return (
		!caps.code ||
		caps.flows !== 'full' ||
		caps.apps !== 'full' ||
		!caps.manage_schedules ||
		!caps.manage_triggers ||
		(Object.keys(ALL_VISIBLE) as VisibilityKey[]).some((k) => !caps.visibility[k])
	)
}
