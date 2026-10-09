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
	| Pick<
			NonNullable<OperatorSettings>,
			'builder_flows' | 'builder_apps' | 'manage_schedules' | 'manage_triggers'
	  >
	| null
	| undefined

/**
 * Whether a user's role lets them author items of `kind` — create, edit, deploy or delete one —
 * in a workspace with these `operator_settings`: the role half of every check against the item
 * handlers. Saving a draft is `roleCanDraft`'s rule, and which items they may write is
 * `canEditItem`'s other half. For an operator:
 * - code (scripts, low-code apps, agents, pipelines): never;
 * - flows and full-code apps: where the workspace grants `builder_flows` / `builder_apps`
 *   (`check_operator_can_build`);
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
		case 'raw_app':
			return builderGrants(kind, operatorSettings)
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

/**
 * Whether the user may save drafts of `kind` — what every session editor writes. Mirrors drafts.rs
 * `require_can_write_path`, which differs from the item handlers both ways: it admits an admin
 * before its operator branch, and it refuses every operator draft but a flow or full-code app
 * under its builder right — schedules, triggers, resources and variables included, though an operator may
 * write those directly.
 */
export function roleCanDraft(
	kind: EditKind,
	user: Pick<UserExt, 'operator' | 'is_admin' | 'is_super_admin'> | undefined,
	operatorSettings: OperatorRights
): boolean {
	if (user?.is_admin || user?.is_super_admin || !user?.operator) return true
	return builderGrants(kind, operatorSettings)
}

function builderGrants(kind: EditKind, operatorSettings: OperatorRights): boolean {
	return (
		(kind === 'flow' && operatorSettings?.builder_flows === true) ||
		(kind === 'raw_app' && operatorSettings?.builder_apps === true)
	)
}

/** `roleCanDraft` for the item at `path`, with its own permissions as in `canEditItem`. */
export function canDraftItem(
	kind: EditKind,
	path: string,
	extraPerms: Record<string, boolean> | undefined,
	user: UserExt | undefined,
	operatorSettings: OperatorRights
): boolean {
	return roleCanDraft(kind, user, operatorSettings) && canWrite(path, extraPerms ?? {}, user)
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
