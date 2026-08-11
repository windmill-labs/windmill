/**
 * Reading a data table's provenance for display.
 *
 * `origin` is only recorded by the setup wizard, so anything created before it --
 * or by editing the config directly -- has none. Those fall back to what the
 * config alone can say, and the panel, which loads the resource itself, can
 * recognise a Supabase host from its value.
 */

import type { DataTableOrigin } from '$lib/gen'

export type DataTableProvider = 'supabase' | 'instance' | 'resource'

type DatabaseConfig = { resource_type: 'postgresql' | 'instance'; resource_path?: string }

export function dataTableProvider(
	database: DatabaseConfig,
	origin: DataTableOrigin | undefined
): DataTableProvider {
	if (origin?.provider === 'supabase') return 'supabase'
	return database.resource_type === 'instance' ? 'instance' : 'resource'
}

/** What the row prints under "Database". */
export function dataTableSubtitle(
	database: DatabaseConfig,
	origin: DataTableOrigin | undefined
): string {
	if (origin?.provider === 'supabase' && origin.project_name)
		return `Supabase · ${origin.project_name}`
	if (database.resource_type === 'instance')
		return `Windmill database · ${database.resource_path ?? ''}`
	return database.resource_path ?? ''
}

/**
 * Both host shapes a Supabase database answers on: the direct one, and any
 * Supavisor pooler. Used to label a resource that predates `origin`, and to warn
 * that a plain postgres resource is in fact a Supabase project.
 */
export function isSupabaseHost(host: string | undefined): boolean {
	if (!host) return false
	return /\.supabase\.co$/.test(host) || /\.pooler\.supabase\.com$/.test(host)
}

/** The project ref, when the host spells it out. */
export function supabaseRefFromHost(host: string | undefined): string | undefined {
	if (!host) return undefined
	const direct = host.match(/^db\.([a-z0-9]+)\.supabase\.co$/)
	if (direct) return direct[1]
	return undefined
}

/** Two data tables on one database share `_wm_migrations`, so identity is host + dbname. */
export function databaseIdentity(value: any): string | undefined {
	const host = value?.host
	const dbname = value?.dbname
	if (!host) return undefined
	return `${String(host).toLowerCase()}/${dbname ?? ''}`
}
