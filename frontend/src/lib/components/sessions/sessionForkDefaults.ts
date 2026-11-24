import type { CreateWorkspaceFork } from '$lib/gen'
import { forkDatatableDbName } from '$lib/utils/forkCreation'

export type DatatableCopy = 'schema_only' | 'schema_and_data'

/** What a staged fork is created with besides its id, name and base. */
export type StagedForkSettings = {
	color?: string
	/** The data tables the fork copies, by name; every other one is shared with the base. */
	datatables?: Record<string, DatatableCopy>
	shared_ducklakes?: string[]
	shared_storages?: string[]
	/** A persistent dev workspace of the base, whose id carries no `wm-fork-` prefix. */
	is_dev_workspace?: boolean
	dev_workspace_label?: CreateWorkspaceFork['dev_workspace_label']
	lock_prod_deploy?: boolean
	lock_prod_forking?: boolean
	copy_members?: boolean
}

type ForkDefaults = { base?: string; settings: StagedForkSettings }

const KEY_PREFIX = 'wm-session-fork-defaults:'

/** The settings the last fork staged in this workspace family was saved with. Data table,
 *  lake and storage names are the family's own, so each family keeps its own. */
export function loadForkDefaults(rootId: string | undefined): ForkDefaults | undefined {
	if (!rootId) return undefined
	try {
		const raw = localStorage.getItem(KEY_PREFIX + rootId)
		return raw ? (JSON.parse(raw) as ForkDefaults) : undefined
	} catch {
		return undefined
	}
}

export function saveForkDefaults(rootId: string | undefined, defaults: ForkDefaults): void {
	if (!rootId) return
	try {
		localStorage.setItem(KEY_PREFIX + rootId, JSON.stringify(defaults))
	} catch {}
}

/** What the base holds when the fork is created, which saved settings may no longer match. */
export type ForkBase = {
	/** Each list is undefined when it could not be read: the saved names are sent as they are. */
	datatables?: { name: string; database_workspace_id?: string }[]
	ducklakes?: string[]
	storages?: string[]
	/** The base's `RequireIsolatedForks` rule binds this user: nothing may stay shared with it. */
	requireIsolation: boolean
}

/** The create-fork request for a staged fork. Saved names the base no longer has are dropped (the
 *  server refuses to share an unknown lake or storage), and under isolation every base-owned table
 *  is copied and nothing is shared, as the fork form does. */
export function stagedForkRequest(
	fork: { id: string; name: string; settings?: StagedForkSettings },
	base: ForkBase
): CreateWorkspaceFork {
	const s = fork.settings ?? {}
	const tables = base.datatables ?? Object.keys(s.datatables ?? {}).map((name) => ({ name }))
	const copies = tables.flatMap((dt: { name: string; database_workspace_id?: string }) => {
		const chosen = s.datatables?.[dt.name]
		// A table whose database another workspace owns is not the base's data; the rule leaves it.
		const behavior =
			chosen ?? (base.requireIsolation && !dt.database_workspace_id ? 'schema_only' : undefined)
		return behavior
			? [
					{
						name: dt.name,
						new_dbname: forkDatatableDbName(fork.id, dt.name),
						fork_behavior: behavior
					}
				]
			: []
	})
	return {
		id: fork.id,
		name: fork.name,
		color: s.color,
		forked_datatables: copies,
		shared_ducklakes: base.requireIsolation
			? []
			: (s.shared_ducklakes ?? []).filter((n) => base.ducklakes?.includes(n) ?? true),
		shared_storages: base.requireIsolation
			? []
			: (s.shared_storages ?? []).filter((n) => base.storages?.includes(n) ?? true),
		is_dev_workspace: !!s.is_dev_workspace,
		dev_workspace_label: s.is_dev_workspace ? s.dev_workspace_label : undefined,
		lock_prod_deploy: !!s.is_dev_workspace && !!s.lock_prod_deploy,
		lock_prod_forking: !!s.is_dev_workspace && !!s.lock_prod_forking,
		copy_members: !!s.copy_members
	}
}

/** A fork a session creates at its first send. */
export type StagedFork = {
	// Existing workspace to fork from.
	parent_workspace_id: string
	// Prefixed id the fork will take, e.g. `wm-fork-foo`.
	id: string
	name: string
	settings?: StagedForkSettings
}
