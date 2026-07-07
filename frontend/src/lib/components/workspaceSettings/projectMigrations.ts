// Best-effort data table migration generation for the "project = folder" Hub
// bundle. Detects which data tables (and tables within them) a project's
// scripts/flows/raw apps reference via `datatable` assets, then generates a
// `CREATE TABLE` bundle per data table from the source workspace's live schema,
// so importing the project into another workspace can recreate those tables.
//
// Best-effort by design: the generated SQL is shown to the publisher and is
// fully editable before publishing. Low-code (non-raw) apps have no persisted
// asset list and are not scanned.

import { inferAssets } from '$lib/infer'
import type { SupportedLanguage } from '$lib/common'
import { getAllModules } from '$lib/components/flows/flowExplorer'
import { getFlowModuleAssets } from '$lib/components/assets/lib'
import {
	apiSchemaToEditorSchema,
	generateMigrationSql,
	type DatabaseSchema
} from '$lib/components/datatableSchemaSql'
import { WorkspaceService } from '$lib/gen'
import type { FetchedItem } from './projectBundle'

export interface GeneratedMigration {
	datatable_name: string
	sql: string
	enabled: boolean
}

// A datatable asset path is `datatable`, `datatable/table`, or
// `datatable/schema.table` (see the SQL asset parser). The first segment is the
// data table name; the remainder identifies a specific table (absent = whole
// data table, no table to create).
function parseDatatableAssetPath(path: string): { datatable: string; table?: string } {
	const slash = path.indexOf('/')
	if (slash === -1) return { datatable: path }
	const datatable = path.slice(0, slash)
	const table = path.slice(slash + 1).trim()
	return { datatable, table: table || undefined }
}

function addUsage(map: Map<string, Set<string>>, path: string): void {
	const { datatable, table } = parseDatatableAssetPath(path)
	if (!datatable) return
	const set = map.get(datatable) ?? new Set<string>()
	if (table) set.add(table)
	map.set(datatable, set)
}

/**
 * Scan a project's fetched items for `datatable` assets and return
 * `datatable -> set of table refs` (a table ref is `table` or `schema.table`).
 *  - scripts: re-parse the code with the asset parser (`inferAssets`)
 *  - flows: read each module's stored `assets`
 *  - raw apps: read `runnables[key].inlineScript.assets` from the app JSON
 */
export async function detectDatatableTables(
	items: FetchedItem[]
): Promise<Map<string, Set<string>>> {
	const map = new Map<string, Set<string>>()

	for (const item of items) {
		if (item.kind === 'script') {
			const res = await inferAssets(
				item.language as SupportedLanguage | undefined,
				item.content ?? ''
			)
			if (res.status === 'ok') {
				for (const a of res.assets) if (a.kind === 'datatable') addUsage(map, a.path)
			}
		} else if (item.kind === 'flow') {
			for (const mod of getAllModules(item.value?.modules ?? [], item.value?.failure_module)) {
				const assets = getFlowModuleAssets(mod)
				if (assets) for (const a of assets) if (a.kind === 'datatable') addUsage(map, a.path)
			}
		} else if (item.kind === 'raw_app') {
			let parsed: any
			try {
				parsed = JSON.parse(item.content ?? '{}')
			} catch {
				continue
			}
			const runnables = parsed?.runnables ?? {}
			for (const key of Object.keys(runnables)) {
				const assets = runnables[key]?.inlineScript?.assets
				if (Array.isArray(assets))
					for (const a of assets)
						if (a?.kind === 'datatable' && typeof a.path === 'string') addUsage(map, a.path)
			}
		}
	}
	return map
}

// Resolve a table ref (`table` or `schema.table`) to a concrete
// `{ schemaName, tableName }` present in the live schema, or undefined if the
// table can't be found (dropped since, typo, …).
function resolveTable(
	schema: DatabaseSchema,
	tableRef: string
): { schemaName: string; tableName: string } | undefined {
	const dot = tableRef.indexOf('.')
	if (dot !== -1) {
		const schemaName = tableRef.slice(0, dot)
		const tableName = tableRef.slice(dot + 1)
		if (schema[schemaName]?.[tableName]) return { schemaName, tableName }
	}
	// No schema qualifier (or the qualified lookup missed): find the table by name
	// across every schema, first match wins.
	for (const schemaName of Object.keys(schema)) {
		if (schema[schemaName][tableRef]) return { schemaName, tableName: tableRef }
	}
	return undefined
}

type ResolvedTable = { schemaName: string; tableName: string }

const tableKey = (t: ResolvedTable) => `${t.schemaName}.${t.tableName}`

// Grow the set of tables to create so it's closed under foreign keys: a used
// table's FK targets (and their FK targets, transitively) are pulled in, so the
// generated CREATE TABLEs never reference a table that isn't also created. FK
// targets that don't resolve in this schema are left out (their FK is pruned by
// pruneSchemaForTables).
function expandFkClosure(schema: DatabaseSchema, seed: ResolvedTable[]): ResolvedTable[] {
	const inSet = new Map(seed.map((t) => [tableKey(t), t]))
	const queue = [...seed]
	while (queue.length > 0) {
		const t = queue.shift()!
		const fks = schema[t.schemaName]?.[t.tableName]?.foreignKeys ?? []
		for (const fk of fks) {
			const target = resolveTable(schema, fk.targetTable ?? '')
			if (target && !inSet.has(tableKey(target))) {
				inSet.set(tableKey(target), target)
				queue.push(target)
			}
		}
	}
	return [...inSet.values()]
}

// A copy of the schema restricted to `tables`, with each table's foreign keys
// filtered to targets that are also in `tables`. generateMigrationSql emits every
// FK it finds on a table, so pruning here keeps a stray FK (to a table outside the
// migration) from making the generated SQL fail.
function pruneSchemaForTables(schema: DatabaseSchema, tables: ResolvedTable[]): DatabaseSchema {
	const inSet = new Set(tables.map(tableKey))
	const pruned: DatabaseSchema = {}
	for (const t of tables) {
		const orig = schema[t.schemaName]?.[t.tableName]
		if (!orig) continue
		;(pruned[t.schemaName] ??= {})[t.tableName] = {
			...orig,
			foreignKeys: (orig.foreignKeys ?? []).filter((fk) => {
				const target = resolveTable(schema, fk.targetTable ?? '')
				return target != null && inSet.has(tableKey(target))
			})
		}
	}
	return pruned
}

// Order tables so a table is created after the in-set tables it references via a
// foreign key. Falls back to input order on a cycle so generation never hangs.
function orderByFkDependency(
	schema: DatabaseSchema,
	tables: { schemaName: string; tableName: string }[]
): { schemaName: string; tableName: string }[] {
	const inSet = new Set(tables.map((t) => t.tableName))
	const deps = new Map<string, Set<string>>()
	for (const t of tables) {
		const fks = schema[t.schemaName]?.[t.tableName]?.foreignKeys ?? []
		const targets = new Set<string>()
		for (const fk of fks) {
			const target = (fk.targetTable ?? '').split('.').pop() ?? ''
			if (target && target !== t.tableName && inSet.has(target)) targets.add(target)
		}
		deps.set(t.tableName, targets)
	}
	const ordered: { schemaName: string; tableName: string }[] = []
	const done = new Set<string>()
	const visiting = new Set<string>()
	const byName = new Map(tables.map((t) => [t.tableName, t]))
	const visit = (name: string) => {
		if (done.has(name) || visiting.has(name)) return
		visiting.add(name)
		for (const dep of deps.get(name) ?? []) visit(dep)
		visiting.delete(name)
		done.add(name)
		const t = byName.get(name)
		if (t) ordered.push(t)
	}
	for (const t of tables) visit(t.tableName)
	return ordered
}

// Strip the per-table `BEGIN;`/`COMMIT;` wrapper that generateMigrationSql adds,
// so several tables can share one transaction.
function unwrapTransaction(sql: string): string {
	return sql
		.replace(/^\s*BEGIN;\s*\n?/, '')
		.replace(/\n?\s*COMMIT;\s*$/, '')
		.trim()
}

/**
 * Generate one best-effort migration per used data table. A migration whose
 * tables were resolved from the live schema is a single transaction creating
 * every referenced table plus the tables they depend on via foreign key (in
 * FK-dependency order) and is enabled by default. A data table used with no
 * resolvable table (referenced as a whole, dropped table, or schema fetch
 * failure) yields an empty, disabled entry so the publisher still sees it and
 * can fill it in.
 */
export async function generateDatatableMigrations(
	workspace: string,
	usage: Map<string, Set<string>>
): Promise<GeneratedMigration[]> {
	const out: GeneratedMigration[] = []
	for (const [datatable, tableRefs] of usage) {
		const disabled: GeneratedMigration = { datatable_name: datatable, sql: '', enabled: false }
		let schema: DatabaseSchema
		try {
			const api = await WorkspaceService.getDatatableFullSchema({
				workspace,
				requestBody: { source: `datatable://${datatable}` }
			})
			schema = apiSchemaToEditorSchema(api)
		} catch {
			out.push(disabled)
			continue
		}
		const resolved = [...tableRefs]
			.map((ref) => resolveTable(schema, ref))
			.filter((t): t is ResolvedTable => t != null)
		// Pull in the tables the referenced ones depend on via FK, then generate
		// against a schema whose FKs are restricted to this set, so the migration
		// creates everything it references and never emits a dangling FK.
		const closure = expandFkClosure(schema, resolved)
		const ordered = orderByFkDependency(schema, closure)
		const prunedSchema = pruneSchemaForTables(schema, ordered)
		const statements = ordered
			.map((t) =>
				unwrapTransaction(
					generateMigrationSql(
						{ schemaName: t.schemaName, tableName: t.tableName, kind: 'added' },
						prunedSchema
					)
				)
			)
			.filter((s) => s.length > 0)
		if (statements.length === 0) {
			out.push(disabled)
			continue
		}
		out.push({
			datatable_name: datatable,
			sql: `BEGIN;\n${statements.join('\n\n')}\nCOMMIT;`,
			enabled: true
		})
	}
	return out.sort((a, b) => a.datatable_name.localeCompare(b.datatable_name))
}
