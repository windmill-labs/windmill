import type { OperatorSettings, RoleCapabilities, RoleVisibility } from '$lib/gen'
import type { UserExt, UserWorkspace } from '$lib/stores'
import { FULL_CAPABILITIES, VISIBILITY_KEYS } from '$lib/workspaceRoles'

/**
 * What a member's workspace role lets them author, manage and see: the frontend's copy of
 * `windmill_common::roles`, see `docs/workspace-roles.md`. Gate on these, never on
 * `UserExt.operator`, which only says "this role cannot write code" and so lets through a role
 * that may code but not, say, build apps.
 */

export type VisibilityKey = keyof RoleVisibility

export { FULL_CAPABILITIES }

const VISIBILITY = VISIBILITY_KEYS.map((v) => v.key)

/**
 * The operator role as a server that predates roles describes it, through `operator_settings`:
 * builder rights are `compose`, the manage rights are granted unless withdrawn, and a page is
 * visible only when its flag is `true` (absent settings hide every page, as they always did).
 */
export function legacyOperatorCapabilities(
	settings: OperatorSettings | null | undefined
): RoleCapabilities {
	const visibility = Object.fromEntries(
		VISIBILITY.map((k) => [k, settings?.[k] === true])
	) as RoleVisibility
	return {
		code: false,
		flows: settings?.builder_flows === true ? 'compose' : 'none',
		apps: settings?.builder_apps === true ? 'compose' : 'none',
		manage_schedules: settings?.manage_schedules !== false,
		manage_triggers: settings?.manage_triggers !== false,
		fork: false,
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

/** Whether the role is gated at all, as `RoleCapabilities::is_restricted`: visibility and `fork`
 * are left out, since neither must cost a full author what restricted roles are refused. Gate
 * forking on `caps.fork` itself. */
export function isRestricted(caps: RoleCapabilities): boolean {
	return (
		!caps.code ||
		caps.flows !== 'full' ||
		caps.apps !== 'full' ||
		!caps.manage_schedules ||
		!caps.manage_triggers
	)
}

/** The role with nothing left to build and the same pages in view: what a session in Use mode
 * acts as. */
export function withoutBuilding(caps: RoleCapabilities): RoleCapabilities {
	return {
		...caps,
		code: false,
		flows: 'none',
		apps: 'none',
		manage_schedules: false,
		manage_triggers: false,
		fork: false
	}
}

/** Whether the role hides any page. */
export function hidesPages(caps: RoleCapabilities): boolean {
	return VISIBILITY.some((k) => !caps.visibility[k])
}
