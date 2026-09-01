/**
 * What is already true in the destination, read from the destination itself.
 *
 * Nothing here is remembered between runs, and nothing may be: a note saying "this run
 * created workspace X" outlives the reload it was written for, but it also outlives the
 * workspace it names, and the two are indistinguishable when it is read back. The plan in
 * the URL says what should exist; these functions ask the instance what does.
 *
 * The one thing the instance cannot answer is *which tables a migration was supposed to
 * create*. That is inferred from the SQL the project ships (`expectedTables`), because the
 * export states only what to run, never what running it should produce.
 */

import { ResourceService, ScriptService, FlowService, AppService, WorkspaceService } from '$lib/gen'
import type { ProjectMigration } from '$lib/components/workspaceSettings/projectBundle'
import {
	presenceKey,
	type ImportedKind
} from '$lib/components/workspaceSettings/projectInstall'
import { listAllWorkspaceTriggers } from '$lib/components/triggers/workspaceTriggersList'

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

const PROBE_PAGE_SIZE = 100
/**
 * A stop, not a limit on what may be imported: 100 pages is 10,000 items in one folder, far
 * past any project, and a paginating endpoint that never returns a short page would otherwise
 * loop forever. Hitting it under-reports, which only ever means "still to do".
 */
const MAX_PROBE_PAGES = 100

/**
 * Which of the items the import would write are already there, as `presenceKey` keys.
 *
 * Scoped by `pathStart` to the import's own folder, so the four path-bearing kinds are four
 * small reads rather than a workspace scan. Presence is not provenance — importing into an
 * existing workspace that already held a path reads the same as having imported it — so
 * callers use this to decide what is left to do, never to claim credit for what is there.
 *
 * Triggers are asked for separately and only when the project ships some: they have no
 * prefix-filtered list endpoint, so answering for them means one call per trigger kind, and a
 * project without triggers should not pay for that.
 */
export async function probeImportedPaths(
	workspace: string,
	folder: string,
	opts?: { triggers?: boolean; hasEeLicense?: boolean }
): Promise<Set<string>> {
	const pathStart = `f/${folder}/`
	const found = new Set<string>()
	/**
	 * Every page, not the first one. These endpoints paginate and default to 30 rows, so a
	 * single call answers for a small project and quietly under-reports a large one — leaving
	 * everything past the first page to be created again, and rejected as already existing.
	 */
	const collectAll = async (
		kind: ImportedKind,
		list: (page: number) => Promise<unknown>
	): Promise<void> => {
		for (let page = 1; page <= MAX_PROBE_PAGES; page++) {
			const rows = ((await list(page)) as { path?: string }[] | undefined) ?? []
			for (const r of rows) if (r.path) found.add(presenceKey(kind, r.path))
			if (rows.length < PROBE_PAGE_SIZE) return
		}
	}
	const calls: Promise<unknown>[] = [
		collectAll('script', (page) =>
			ScriptService.listScripts({ workspace, pathStart, page, perPage: PROBE_PAGE_SIZE })
		),
		collectAll('flow', (page) =>
			FlowService.listFlows({ workspace, pathStart, page, perPage: PROBE_PAGE_SIZE })
		),
		collectAll('app', (page) =>
			AppService.listApps({ workspace, pathStart, page, perPage: PROBE_PAGE_SIZE })
		),
		collectAll('resource', (page) =>
			ResourceService.listResource({ workspace, pathStart, page, perPage: PROBE_PAGE_SIZE })
		)
	]
	if (opts?.triggers) {
		// `failedKinds` is deliberately ignored: a kind that could not be listed leaves its
		// triggers out of the set, and a missing key only ever means "still to do".
		calls.push(
			listAllWorkspaceTriggers(workspace, {
				includeEeOnly: opts.hasEeLicense === true
			}).then(({ triggers }) => {
				for (const t of triggers) {
					if (t.path?.startsWith(pathStart)) found.add(presenceKey(`trigger:${t.kind}`, t.path))
				}
			})
		)
	}
	// One kind failing should narrow the answer, not lose the others: a missing key only ever
	// means "still to do", which is the safe direction.
	await Promise.allSettled(calls)
	return found
}

/**
 * Whether every table a data table's migrations create is in it.
 *
 * The ground truth for "did this run", and the only one that covers both paths
 * `applyOneMigration` takes: it records a migration when the data table has migrations
 * enabled, and otherwise runs the SQL once as a job that nothing remembers. The tables
 * outlive both.
 *
 * Every migration for one data table at once: they all target the same schema, so asking
 * per migration would introspect the same database N times for one answer.
 *
 * `undefined` means the question could not be answered — the schema was unreadable, or the
 * SQL named no tables this can recognise. Distinct from `false`, because "not there" invites
 * a caller to run the migration and "cannot tell" does not.
 */
export async function probeMigrationsApplied(
	workspace: string,
	datatableName: string,
	migrations: ProjectMigration[]
): Promise<boolean | undefined> {
	const wanted = [...new Set(migrations.flatMap((m) => expectedTables(m.sql ?? '')))]
	if (wanted.length === 0) return undefined
	try {
		const schema = (await WorkspaceService.getDatatableFullSchema({
			workspace,
			requestBody: { source: `datatable://${datatableName}` }
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
