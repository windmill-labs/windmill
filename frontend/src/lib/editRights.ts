import type { OperatorSettings } from '$lib/gen'
import type { UserExt } from '$lib/stores'
import { canWrite } from '$lib/utils'

/**
 * The frontend's one copy of "may this user edit this item". The server decides — drafts.rs
 * `require_can_write_path`, the item handlers' operator refusals, `gate_operator_writes`, and
 * row-level security — and this mirrors it so a control the server would refuse is never
 * offered. Check edits through `canEditItem` (or `useEditRights` in a component), never by
 * pairing `canWrite` with an operator test of your own.
 */

/** Kinds whose role half differs. Any other kind is code an operator never authors. */
export type EditKind =
	| 'script'
	| 'flow'
	| 'app'
	| 'raw_app'
	| 'agent'
	| 'pipeline'
	| 'schedule'
	| 'trigger'
	| 'resource'
	| 'variable'
	| 'group'
	| (string & {})

type OperatorRights =
	| Pick<NonNullable<OperatorSettings>, 'builder_flows' | 'manage_schedules' | 'manage_triggers'>
	| null
	| undefined

/**
 * Whether a user's role lets them author items of `kind` — create one, edit it, save its
 * drafts — in a workspace with these `operator_settings`. The role half of every edit check;
 * which items they may write is `canEditItem`'s other half. For an operator:
 * - code (scripts, apps, agents, pipelines): never;
 * - flows: where the workspace grants `builder_flows` (`check_operator_can_build_flows`);
 * - schedules and triggers: unless the workspace set `manage_*` to false — a withdrawable
 *   right, so an unset key grants it (docs/operator-write-rights.md);
 * - resources, variables and groups: always, their permissions alone decide.
 */
export function roleCanAuthor(
	kind: EditKind,
	user: Pick<UserExt, 'operator'> | undefined,
	operatorSettings: OperatorRights
): boolean {
	if (!user?.operator) return true
	switch (kind) {
		case 'flow':
			return operatorSettings?.builder_flows === true
		case 'schedule':
			return operatorSettings?.manage_schedules !== false
		case 'trigger':
			return operatorSettings?.manage_triggers !== false
		case 'resource':
		case 'variable':
		case 'group':
			return true
		default:
			return false
	}
}

/** Whether `user` may edit the item at `path`: their role authors `kind`, and the item's own
 * permissions (`canWrite`: ownership, folder write, `extra_perms`) let them write it. */
export function canEditItem(
	kind: EditKind,
	path: string,
	extraPerms: Record<string, boolean> | undefined,
	user: UserExt | undefined,
	operatorSettings: OperatorRights
): boolean {
	return roleCanAuthor(kind, user, operatorSettings) && canWrite(path, extraPerms ?? {}, user)
}
