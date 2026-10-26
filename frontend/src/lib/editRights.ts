import type { RoleCapabilities } from '$lib/gen'
import type { UserExt } from '$lib/stores'
import { canWrite } from '$lib/utils'
import { isRestricted } from '$lib/roleCapabilities'

/**
 * The frontend's one copy of "may this user edit this item". The server decides — drafts.rs
 * `require_can_write_path`, the item handlers' role refusals, `gate_operator_writes`, and
 * row-level security — and this mirrors it so a control the server would refuse is never
 * offered. Check edits through `canEditItem` (or `useEditRights` in a component), never by
 * pairing `canWrite` with a role test of your own.
 */

/** Kinds whose role half differs. Any other kind is code, authored under `code`. */
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

/**
 * Whether a role with `caps` lets its member author items of `kind` (create, edit, deploy or
 * delete one): the role half of every check against the item handlers. Saving a draft is
 * `roleCanDraft`'s rule, and which items they may write is `canEditItem`'s other half.
 * - code (scripts, pipelines): `code`;
 * - flows, agents and full-code apps: `compose` or above (`check_operator_can_build`). An
 *   agent's tools are checked as a composed flow's steps are, under the flows right;
 * - low-code apps: `full` apps only (apps.rs `require_full_apps`);
 * - schedules and triggers: the matching manage right;
 * - resources, variables and groups: always, their permissions alone decide.
 */
export function roleCanAuthor(kind: EditKind, caps: RoleCapabilities): boolean {
	switch (kind) {
		case 'flow':
		case 'agent':
			return caps.flows !== 'none'
		case 'raw_app':
			return caps.apps !== 'none'
		case 'app':
			return caps.apps === 'full'
		case 'schedule':
			return caps.manage_schedules
		case 'trigger':
			return caps.manage_triggers
		case 'resource':
		case 'variable':
		case 'group':
			return true
		default:
			return caps.code
	}
}

/**
 * Whether the role may save drafts of `kind`, which every session editor and the chat's draft
 * tools write. Mirrors drafts.rs `operator_can_draft`: a restricted role drafts only the kinds it
 * could deploy, and resources and variables always. An unrestricted role drafts every kind.
 */
export function roleCanDraft(kind: EditKind, caps: RoleCapabilities): boolean {
	if (!isRestricted(caps)) return true
	return kind === 'group' ? false : roleCanAuthor(kind, caps)
}

/** Whether the role builds anything: code, flows, apps, schedules or triggers. Resources and
 * variables do not count, since every role edits them. */
export function buildsAnything(caps: RoleCapabilities): boolean {
	return (['script', 'flow', 'raw_app', 'schedule', 'trigger'] as const).some((kind) =>
		roleCanDraft(kind, caps)
	)
}

/** `roleCanDraft` for the item at `path`, with its own permissions as in `canEditItem`. */
export function canDraftItem(
	kind: EditKind,
	path: string,
	extraPerms: Record<string, boolean> | undefined,
	user: UserExt | undefined,
	caps: RoleCapabilities
): boolean {
	return roleCanDraft(kind, caps) && canWrite(path, extraPerms ?? {}, user)
}

/** Whether `user` may edit the item at `path`: their role authors `kind`, and the item's own
 * permissions (`canWrite`: ownership, folder write, `extra_perms`) let them write it. */
export function canEditItem(
	kind: EditKind,
	path: string,
	extraPerms: Record<string, boolean> | undefined,
	user: UserExt | undefined,
	caps: RoleCapabilities
): boolean {
	return roleCanAuthor(kind, caps) && canWrite(path, extraPerms ?? {}, user)
}
