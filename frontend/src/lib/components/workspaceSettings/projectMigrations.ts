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
	/** Up migration: creates the tables. */
	sql: string
	/** Down migration: drops the created tables. Best-effort, generated once and
	 *  editable by the publisher (not re-derived from `sql`). */
	sql_down: string
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

function addDatatableTable(map: Map<string, Set<string>>, datatable: string, table?: string): void {
	if (!datatable) return
	const set = map.get(datatable) ?? new Set<string>()
	if (table) set.add(table)
	map.set(datatable, set)
}

function addUsage(map: Map<string, Set<string>>, path: string): void {
	const { datatable, table } = parseDatatableAssetPath(path)
	addDatatableTable(map, datatable, table)
}

// Low-code apps declare data table usage explicitly on the DB-table component:
// a `oneOf` `type` config with `selected === 'datatable'` holding the data table
// (as `datatable://<name>`) and the table. Walk the app value for those instead
// of parsing assets (low-code inline scripts don't carry a persisted asset list).
function collectAppDatatableConfigs(node: any, map: Map<string, Set<string>>): void {
	if (node == null || typeof node !== 'object') return
	if (Array.isArray(node)) {
		for (const v of node) collectAppDatatableConfigs(v, map)
		return
	}
	if (node.selected === 'datatable' && node.configuration?.datatable) {
		const dtRaw = node.configuration.datatable.datatable?.value
		const tableRaw = node.configuration.datatable.table?.value
		if (typeof dtRaw === 'string' && dtRaw.trim() !== '') {
			const datatable = dtRaw
				.trim()
				.replace(/^\$res:/, '')
				.replace(/^datatable:\/\//, '')
			const table =
				typeof tableRaw === 'string' && tableRaw.trim() !== '' ? tableRaw.trim() : undefined
			addDatatableTable(map, datatable, table)
		}
	}
	for (const k of Object.keys(node)) collectAppDatatableConfigs(node[k], map)
}

/**
 * Scan a project's fetched items for `datatable` assets and return
 * `datatable -> set of table refs` (a table ref is `table` or `schema.table`).
 *  - scripts: re-parse the code with the asset parser (`inferAssets`)
 *  - flows: read each module's stored `assets`
 *  - raw apps: read `runnables[key].inlineScript.assets` from the app JSON
 *  - low-code apps: read the DB-table component's explicit datatable/table config
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
		} else if (item.kind === 'app') {
			collectAppDatatableConfigs(item.value, map)
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
	// No schema qualifier (or the qualified lookup missed, e.g. a stale schema name):
	// find the bare table name across every schema, first match wins.
	const bareName = dot !== -1 ? tableRef.slice(dot + 1) : tableRef
	for (const schemaName of Object.keys(schema)) {
		if (schema[schemaName][bareName]) return { schemaName, tableName: bareName }
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
// foreign key. Keyed by schema-qualified name (like the rest of the pipeline) so
// two same-named tables in different schemas aren't collapsed. Falls back to input
// order on a cycle so generation never hangs.
function orderByFkDependency(schema: DatabaseSchema, tables: ResolvedTable[]): ResolvedTable[] {
	const inSet = new Set(tables.map(tableKey))
	const deps = new Map<string, Set<string>>()
	for (const t of tables) {
		const fks = schema[t.schemaName]?.[t.tableName]?.foreignKeys ?? []
		const targets = new Set<string>()
		for (const fk of fks) {
			const target = resolveTable(schema, fk.targetTable ?? '')
			if (target && tableKey(target) !== tableKey(t) && inSet.has(tableKey(target))) {
				targets.add(tableKey(target))
			}
		}
		deps.set(tableKey(t), targets)
	}
	const ordered: ResolvedTable[] = []
	const done = new Set<string>()
	const visiting = new Set<string>()
	const byKey = new Map(tables.map((t) => [tableKey(t), t]))
	const visit = (key: string) => {
		if (done.has(key) || visiting.has(key)) return
		visiting.add(key)
		for (const dep of deps.get(key) ?? []) visit(dep)
		visiting.delete(key)
		done.add(key)
		const t = byKey.get(key)
		if (t) ordered.push(t)
	}
	for (const t of tables) visit(tableKey(t))
	return ordered
}

// Pull a readable one-line message out of an API error for embedding in a SQL
// comment (collapse whitespace so it can't break out of the `--` line).
function errorText(e: any): string {
	const body = e?.body
	const raw =
		typeof body === 'string' && body.trim()
			? body
			: body && typeof body === 'object'
				? (body.error?.message ?? body.message ?? JSON.stringify(body))
				: (e?.message ?? String(e))
	return String(raw).replace(/\s+/g, ' ').trim()
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
 * Generate one best-effort migration per used data table. Resolved tables (plus
 * the tables they depend on via foreign key, in FK-dependency order) become a
 * single CREATE TABLE transaction, enabled by default. Anything that couldn't be
 * auto-generated — a table not found in the schema, a data table referenced as a
 * whole, or a schema that couldn't be loaded — is written as a `--` SQL comment
 * describing the problem, so the publisher sees what's missing instead of a blank
 * entry. A migration with no runnable statements (only comments) is left disabled.
 */
export async function generateDatatableMigrations(
	workspace: string,
	usage: Map<string, Set<string>>
): Promise<GeneratedMigration[]> {
	const out: GeneratedMigration[] = []
	for (const [datatable, tableRefs] of usage) {
		let schema: DatabaseSchema
		try {
			const api = await WorkspaceService.getDatatableFullSchema({
				workspace,
				requestBody: { source: `datatable://${datatable}` }
			})
			schema = apiSchemaToEditorSchema(api)
		} catch (e) {
			// Couldn't reach the schema at all: leave a commented stub explaining why,
			// so the publisher can fill it in rather than seeing a silent blank.
			out.push({
				datatable_name: datatable,
				sql:
					`-- Could not load the schema of data table "${datatable}": ${errorText(e)}\n` +
					`-- Add the CREATE TABLE statement(s) for the tables this project uses.`,
				sql_down: '',
				enabled: false
			})
			continue
		}
		// Resolve the referenced tables; record a comment for each one we can't find
		// so a partial migration still explains what's missing.
		const resolved: ResolvedTable[] = []
		const comments: string[] = []
		for (const ref of tableRefs) {
			const t = resolveTable(schema, ref)
			if (t) resolved.push(t)
			else
				comments.push(
					`-- Table "${ref}" is referenced but was not found in data table "${datatable}"; add its CREATE TABLE manually.`
				)
		}
		if (tableRefs.size === 0) {
			comments.push(
				`-- Data table "${datatable}" is used but no specific table was referenced; nothing to generate automatically.`
			)
		}
		// Pull in the tables the referenced ones depend on via FK, then generate
		// against a schema whose FKs are restricted to this set, so the migration
		// creates everything it references and never emits a dangling FK.
		const closure = expandFkClosure(schema, resolved)
		const ordered = orderByFkDependency(schema, closure)
		const prunedSchema = pruneSchemaForTables(schema, ordered)
		const statements = ordered
			.map((t) =>
				unwrapTransaction(
					// IF NOT EXISTS: FK closure pulls in shared parent tables (e.g. a
					// referenced `orders` drags in `customers`) that often already exist in
					// the target, so a plain CREATE would abort the whole transaction. The
					// caveat — an existing differently-shaped table is silently left as-is —
					// is acceptable for a best-effort, editable migration.
					generateMigrationSql(
						{ schemaName: t.schemaName, tableName: t.tableName, kind: 'added' },
						prunedSchema,
						{ ifNotExists: true }
					)
				)
			)
			.filter((s) => s.length > 0)
		// Comments (the errors) go on top; the CREATE TABLE transaction, if any,
		// follows. Enabled only when there's something to run.
		const parts: string[] = []
		if (comments.length > 0) parts.push(comments.join('\n'))
		if (statements.length > 0) parts.push(`BEGIN;\n${statements.join('\n\n')}\nCOMMIT;`)
		// Best-effort down migration: the DROP TABLE statements are commented out
		// because the FK closure pulls in shared parent tables that may have
		// pre-existed in the target (dropping them would lose data the project never
		// created). The publisher uncomments the tables this migration should drop.
		const drops = [...ordered]
			.reverse()
			.map((t) => `-- DROP TABLE IF EXISTS "${t.schemaName}"."${t.tableName}";`)
		const sqlDown =
			drops.length > 0
				? `-- Rollback: uncomment the tables this migration should drop (leave shared\n` +
					`-- tables that already existed in the workspace commented out).\nBEGIN;\n${drops.join('\n')}\nCOMMIT;`
				: ''
		out.push({
			datatable_name: datatable,
			sql: parts.join('\n\n'),
			sql_down: sqlDown,
			enabled: statements.length > 0
		})
	}
	return out.sort((a, b) => a.datatable_name.localeCompare(b.datatable_name))
}
