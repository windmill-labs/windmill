/**
 * What is already true in the destination, read from the destination itself.
 *
 * The wizard used to remember what a run had done — a note in `sessionStorage` saying "this
 * run created workspace X" — which a reload could outlive but a stale entry could also
 * outlive the workspace it named. Everything here is asked of the instance instead, so there
 * is nothing to go stale and nothing to clear: the plan in the URL says what should exist,
 * and these functions say what does.
 *
 * The one thing the instance cannot answer is *which tables a migration was supposed to
 * create*. That is inferred from the SQL the project ships (`expectedTables`), because the
 * export states only what to run, never what running it should produce.
 */

import { ResourceService, ScriptService, FlowService, AppService, WorkspaceService } from '$lib/gen'
import type {
	ProjectExport,
	ProjectMigration
} from '$lib/components/workspaceSettings/projectBundle'

/**
 * The tables a migration creates, as `schema.table`, read off its `CREATE TABLE` statements.
 *
 * Inference, not a contract: the export ships SQL and nothing else, so this is the only way
 * to check a migration's work without a record of it having run. It deliberately reads only
 * the shape this project's generator emits (`datatableSchemaSql.ts` always writes the
 * schema-qualified, quoted form) — anything hand-edited into a different shape simply reads
 * as no expected tables, which makes the caller fall back to "cannot tell" rather than to a
 * confident wrong answer.
 */
export function expectedTables(sql: string): string[] {
	const out: string[] = []
	const re = /CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?"([^"]+)"\s*\.\s*"([^"]+)"/gi
	let m: RegExpExecArray | null
	while ((m = re.exec(sql)) !== null) out.push(`${m[1]}.${m[2]}`)
	return [...new Set(out)]
}

/** Every path the export will write, under the folder the import is targeting. */
export function expectedPaths(exportData: ProjectExport, folder: string): string[] {
	const from = exportData.project.slug
	const rewrite = (p: string) => (folder === from ? p : p.replace(`f/${from}/`, `f/${folder}/`))
	return [...exportData.scripts, ...exportData.flows, ...exportData.apps, ...exportData.resources]
		.map((i: any) => String(i.path))
		.map(rewrite)
}

export interface WorkspaceState {
	/** The user is a member of a workspace with this id. */
	exists: boolean
	/** …and it is one they own, so a run of theirs is what made it. */
	ours: boolean
}

/**
 * Whether the destination workspace is there, and whether it is the caller's.
 *
 * `listWorkspaces` answers both in one call: it returns only workspaces the caller is a
 * member of, each carrying the `owner` email set at creation. Ownership is what makes
 * skipping a create safe — an id that exists but belongs to someone else is not a workspace
 * this run made, and importing into it would be importing into a stranger's.
 */
export async function probeWorkspace(
	id: string,
	email: string | undefined
): Promise<WorkspaceState> {
	try {
		const mine = await WorkspaceService.listWorkspaces()
		const found = mine.find((w) => w.id === id)
		if (!found) return { exists: false, ours: false }
		return { exists: true, ours: !!email && found.owner === email }
	} catch {
		// Cannot tell. Reported as absent so the caller creates rather than adopts: a failed
		// create is a clear error, adopting the wrong workspace is a silent one.
		return { exists: false, ours: false }
	}
}

/**
 * Which of the paths the import would write are already there.
 *
 * Scoped by `pathStart` to the import's own folder, so this is four small reads rather than a
 * workspace scan. Presence is not provenance — importing into an existing workspace that
 * already held a path reads the same as having imported it — so callers use this to decide
 * what is left to do, never to claim credit for what is there.
 */
export async function probeImportedPaths(workspace: string, folder: string): Promise<Set<string>> {
	const pathStart = `f/${folder}/`
	const paths = new Set<string>()
	const collect = (rows: unknown) => {
		for (const r of (rows as { path?: string }[] | undefined) ?? []) if (r.path) paths.add(r.path)
	}
	const calls = [
		ScriptService.listScripts({ workspace, pathStart }).then(collect),
		FlowService.listFlows({ workspace, pathStart }).then(collect),
		AppService.listApps({ workspace, pathStart }).then(collect),
		ResourceService.listResource({ workspace, pathStart }).then(collect)
	]
	// One kind failing should narrow the answer, not lose the other three: a missing path
	// only ever means "still to do", which is the safe direction.
	await Promise.allSettled(calls)
	return paths
}

/**
 * Whether a migration's tables are in the data table it targets.
 *
 * The ground truth for "did this run", and the only one that covers both paths
 * `applyOneMigration` takes: it records a migration when the data table has migrations
 * enabled, and otherwise runs the SQL once as a job that nothing remembers. The tables
 * outlive both.
 *
 * `undefined` means the question could not be answered — the schema was unreadable, or the
 * SQL named no tables this can recognise. Distinct from `false`, because "not there" invites
 * a caller to run the migration and "cannot tell" does not.
 */
export async function probeMigrationApplied(
	workspace: string,
	migration: ProjectMigration
): Promise<boolean | undefined> {
	const wanted = expectedTables(migration.sql ?? '')
	if (wanted.length === 0) return undefined
	try {
		const schema = (await WorkspaceService.getDatatableFullSchema({
			workspace,
			requestBody: { source: `datatable://${migration.datatable_name}` }
		})) as Record<string, Record<string, unknown>>
		const present = new Set<string>()
		for (const [schemaName, tables] of Object.entries(schema ?? {})) {
			for (const table of Object.keys(tables ?? {})) present.add(`${schemaName}.${table}`)
		}
		return wanted.every((t) => present.has(t))
	} catch {
		return undefined
	}
}
