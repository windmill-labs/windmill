import { isCloudHosted } from '$lib/cloud'

import { SettingService } from '$lib/gen'
import { superadmin } from '$lib/stores'
import { resource } from 'runed'
import { derived } from 'svelte/store'

export let isCustomInstanceDbEnabled = derived(
	[superadmin],
	([superadmin_]) => superadmin_ && !isCloudHosted()
)

// Postgres caps identifiers at 63 bytes; the backend rejects longer db names.
const MAX_INSTANCE_DB_NAME_LEN = 63

// Builds a default instance database name scoped to the workspace (e.g. `dt_myworkspace`),
// appending `_1`, `_2`... until an unused name is found. Workspace ids may contain hyphens,
// which are not valid in unquoted postgres identifiers, so they are replaced with underscores.
// The result is truncated to keep it within the postgres identifier length limit.
export function getUnusedInstanceDbName(
	prefix: string,
	workspaceId: string,
	usedNames: Iterable<string>
): string {
	const used = new Set(usedNames)
	const base = `${prefix}_${workspaceId.toLowerCase().replace(/-/g, '_')}`.slice(
		0,
		MAX_INSTANCE_DB_NAME_LEN
	)
	if (!used.has(base)) return base
	let i = 1
	let candidate: string
	do {
		const suffix = `_${i}`
		candidate = base.slice(0, MAX_INSTANCE_DB_NAME_LEN - suffix.length) + suffix
		i++
	} while (used.has(candidate))
	return candidate
}

/**
 * What to call the two substrates Windmill administers. They are one concept to a workspace admin
 * — a database Windmill makes and manages — so each is only qualified while the other is also on
 * offer; alone, either is just "Managed instance".
 */
export function managedInstanceLabels(instanceAvailable: boolean, externalAvailable: boolean) {
	const both = instanceAvailable && externalAvailable
	return {
		instance: both ? 'Managed instance (Internal)' : 'Managed instance',
		external: both ? 'Managed instance (External)' : 'Managed instance'
	}
}

/**
 * What the closed select shows for a managed kind. The list needs the whole name to tell the
 * two substrates apart, but once one is picked the row is narrow and the qualifier alone
 * carries the distinction.
 */
export function shortManagedInstanceLabel(text: string): string {
	return text.startsWith('Managed instance (')
		? text.replace('Managed instance (', 'Managed (')
		: text
}

/**
 * Whether each Postgres Windmill manages can take a new data table, and the external cluster's
 * databases. Both answer only to a superadmin, and only a superadmin creates on either, so for
 * anyone else neither is available and nothing is fetched.
 */
export function useManagedInstances(isSuperadmin: () => boolean) {
	const externalStatus = resource([isSuperadmin], ([su]) =>
		su ? SettingService.getExternalInstancePgStatus() : Promise.resolve(undefined)
	)
	const externalDbs = resource([isSuperadmin], ([su]) =>
		su ? SettingService.listExternalInstancePgDatabases() : Promise.resolve({})
	)
	// Absent means on.
	const instancePgDisabled = resource([isSuperadmin], ([su]) =>
		su
			? SettingService.getGlobal({ key: 'instance_pg_disabled' }).catch(() => undefined)
			: Promise.resolve(undefined)
	)
	return {
		externalDbs,
		get externalAvailable() {
			return isSuperadmin() && externalStatus.current?.configured === true
		},
		get instanceAvailable() {
			return isSuperadmin() && !isCloudHosted() && !instancePgDisabled.current
		},
		refresh() {
			externalStatus.refetch()
			externalDbs.refetch()
			instancePgDisabled.refetch()
		}
	}
}
