import type { OperatorSettings } from '$lib/gen'
import type { UserExt } from '$lib/stores'
import { roleCanAuthor, type EditKind } from '$lib/editRights'
import { capabilitiesForRole } from './global/sessionAccess'
import type { SessionAccess } from './sessionCapabilities'

/** Where a workspace admin changes what operators may do. Named in the prompt and the
 * permissions section alike, so the two send the user to the same place. */
export const OPERATOR_SETTINGS_LOCATION = 'Workspace settings → Users → Operator settings'

type PermissionRow = {
	kind: EditKind
	label: string
	/** The draft capability the kind's write tool declares in `requires`, plus the schedule or
	 * trigger write gate where the kind has one. */
	assistant: (access: SessionAccess) => boolean
}

const ROWS: PermissionRow[] = [
	{ kind: 'script', label: 'Scripts', assistant: (a) => a.has('write_draft') },
	{ kind: 'flow', label: 'Flows', assistant: (a) => a.has('write_flow_draft') },
	{ kind: 'raw_app', label: 'Apps', assistant: (a) => a.has('write_app_draft') },
	{
		kind: 'schedule',
		label: 'Schedules',
		assistant: (a) => a.has('write_draft') && a.has('manage_schedules')
	},
	{
		kind: 'trigger',
		label: 'Triggers',
		assistant: (a) => a.has('write_draft') && a.has('manage_triggers')
	},
	{ kind: 'resource', label: 'Resources', assistant: (a) => a.has('write_draft') },
	{ kind: 'variable', label: 'Variables', assistant: (a) => a.has('write_draft') }
]

export type RoleName = 'admin' | 'developer' | 'operator'

export function roleName(
	user: Pick<UserExt, 'operator' | 'is_admin' | 'is_super_admin'>
): RoleName {
	if (user.is_admin || user.is_super_admin) return 'admin'
	return user.operator ? 'operator' : 'developer'
}

/**
 * Per item kind, whether the user's role lets them create and edit it themselves, and whether
 * the chat can for them. Both act with the user's rights, but the user's column follows the
 * item handlers (`roleCanAuthor`) and the chat's the draft endpoint every write tool goes
 * through (`capabilitiesForRole`), which refuses operators more than the handlers do.
 */
export function permissionRows(
	user: Pick<UserExt, 'operator' | 'is_admin' | 'is_super_admin'>,
	operatorSettings: OperatorSettings | null | undefined
): { label: string; user: boolean; assistant: boolean }[] {
	const access = capabilitiesForRole({
		isAdmin: !!user.is_admin || !!user.is_super_admin,
		operator: !!user.operator,
		// Deploy rules gate deploying, not the drafts these rows are about.
		deployRulesPass: true,
		operatorSettings
	})
	return ROWS.map((row) => ({
		label: row.label,
		user: roleCanAuthor(row.kind, user, operatorSettings),
		assistant: row.assistant(access)
	}))
}
