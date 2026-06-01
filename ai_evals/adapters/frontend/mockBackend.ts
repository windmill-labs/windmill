import { randomUUID } from 'node:crypto'
import type { CompletedJob, Flow, Script } from '../../../frontend/src/lib/gen'
import type {
	DataTableTables,
	DataTableTableSchema,
	ScriptLang
} from '../../../frontend/src/lib/gen/types.gen'
import { buildScriptLintResult } from './core/script/preview'

const BENCHMARK_TIMESTAMP = '1970-01-01T00:00:00.000Z'

export interface BenchmarkWorkspaceScript {
	path: string
	summary: string
	description?: string
	language: Script['language']
	schema?: Record<string, unknown>
	content: string
}

export interface BenchmarkWorkspaceFlow {
	path: string
	summary: string
	description?: string
	schema?: Record<string, unknown>
	value: Flow['value']
}

/** One seeded datatable table: its columns (col -> compact_type) and optional canned rows. */
export interface BenchmarkDatatableTableSeed {
	columns: Record<string, string>
	rows?: Record<string, unknown>[]
}

/**
 * A seeded datatable: `datatable_name` plus a `schema -> table -> seed` map.
 * Mirrors the production shape so `list_datatables` / `get_datatable_table_schema`
 * project from the same structure.
 */
export interface BenchmarkDatatableSeed {
	datatable_name: string
	schemas: {
		[schema: string]: {
			[table: string]: BenchmarkDatatableTableSeed
		}
	}
}

export interface BenchmarkWorkspaceRunnables {
	scripts?: BenchmarkWorkspaceScript[]
	flows?: BenchmarkWorkspaceFlow[]
	datatables?: BenchmarkDatatableSeed[]
}

type BenchmarkCompletedJob = CompletedJob & { type: 'CompletedJob' }

const benchmarkWorkspaces = new Set<string>()
const benchmarkWorkspaceRunnables = new Map<string, BenchmarkWorkspaceRunnables>()
const benchmarkJobs = new Map<string, { workspace: string; job: BenchmarkCompletedJob }>()

export function resetBenchmarkMockBackend(): void {
	benchmarkWorkspaces.clear()
	benchmarkWorkspaceRunnables.clear()
	benchmarkJobs.clear()
}

export function registerBenchmarkWorkspace(workspace: string): void {
	benchmarkWorkspaces.add(workspace)
}

export function registerBenchmarkWorkspaceRunnables(
	workspace: string,
	runnables: BenchmarkWorkspaceRunnables
): void {
	benchmarkWorkspaces.add(workspace)
	benchmarkWorkspaceRunnables.set(workspace, runnables)
}

export function unregisterBenchmarkWorkspace(workspace: string): void {
	benchmarkWorkspaces.delete(workspace)
	benchmarkWorkspaceRunnables.delete(workspace)
	for (const [jobId, entry] of benchmarkJobs.entries()) {
		if (entry.workspace === workspace) {
			benchmarkJobs.delete(jobId)
		}
	}
}

export function unregisterBenchmarkWorkspaceRunnables(workspace: string): void {
	unregisterBenchmarkWorkspace(workspace)
}

export function hasBenchmarkWorkspace(workspace: string): boolean {
	return benchmarkWorkspaces.has(workspace)
}

export function listBenchmarkScripts(workspace: string): Script[] | null {
	const runnables = benchmarkWorkspaceRunnables.get(workspace)
	if (!runnables) {
		return null
	}
	return (runnables.scripts ?? []).map(buildBenchmarkScript)
}

export function listBenchmarkFlows(workspace: string): Flow[] | null {
	const runnables = benchmarkWorkspaceRunnables.get(workspace)
	if (!runnables) {
		return null
	}
	return (runnables.flows ?? []).map(buildBenchmarkFlow)
}

export function getBenchmarkScriptByPath(workspace: string, path: string): Script | null {
	const script = benchmarkWorkspaceRunnables
		.get(workspace)
		?.scripts?.find((entry) => entry.path === path)

	return script ? buildBenchmarkScript(script) : null
}

export function getBenchmarkScriptByHash(workspace: string, hash: string): Script | null {
	const script = benchmarkWorkspaceRunnables
		.get(workspace)
		?.scripts?.find((entry) => buildBenchmarkScriptHash(entry.path) === hash)

	return script ? buildBenchmarkScript(script) : null
}

export function getBenchmarkFlowByPath(workspace: string, path: string): Flow | null {
	const flow = benchmarkWorkspaceRunnables
		.get(workspace)
		?.flows?.find((entry) => entry.path === path)

	return flow ? buildBenchmarkFlow(flow) : null
}

export function createBenchmarkCompletedJob(input: {
	workspace: string
	jobKind: CompletedJob['job_kind']
	success?: boolean
	result?: unknown
	logs?: string
	scriptPath?: string
	scriptHash?: string
	args?: Record<string, unknown>
}): string {
	const jobId = `benchmark-job-${randomUUID()}`
	const now = new Date().toISOString()
	const job: BenchmarkCompletedJob = {
		type: 'CompletedJob',
		id: jobId,
		workspace_id: input.workspace,
		created_by: 'ai-evals',
		created_at: now,
		started_at: now,
		completed_at: now,
		duration_ms: 0,
		success: input.success ?? true,
		script_path: input.scriptPath,
		script_hash: input.scriptHash,
		args: input.args,
		result: input.result,
		logs: input.logs,
		canceled: false,
		job_kind: input.jobKind,
		permissioned_as: 'u/ai-evals',
		is_flow_step: false,
		is_skipped: false,
		email: 'ai-evals@local',
		visible_to_owner: true,
		tag: 'benchmark'
	}

	benchmarkJobs.set(jobId, { workspace: input.workspace, job })
	return jobId
}

export function getBenchmarkCompletedJob(
	workspace: string,
	jobId: string
): BenchmarkCompletedJob | null {
	const entry = benchmarkJobs.get(jobId)
	if (!entry || entry.workspace !== workspace) {
		return null
	}
	return structuredClone(entry.job)
}

// ============= Datatables (canned SQL, no engine) =============

/**
 * Project the seeded datatables down to the `list_datatable_tables` response:
 * `datatable_name` + `schema -> table_names`, with no column detail.
 * Returns `null` for a non-benchmark workspace so callers can fall through to
 * the real backend; an empty seed yields `[]`.
 */
export function listBenchmarkDatatables(workspace: string): DataTableTables[] | null {
	const runnables = benchmarkWorkspaceRunnables.get(workspace)
	if (!runnables) {
		return null
	}
	return (runnables.datatables ?? []).map((datatable) => ({
		datatable_name: datatable.datatable_name,
		schemas: Object.fromEntries(
			Object.entries(datatable.schemas).map(([schema, tables]) => [schema, Object.keys(tables)])
		)
	}))
}

export function getBenchmarkDatatableSchema(input: {
	workspace: string
	datatableName: string
	schemaName: string
	tableName: string
}): DataTableTableSchema {
	const runnables = benchmarkWorkspaceRunnables.get(input.workspace)
	const datatable = (runnables?.datatables ?? []).find(
		(entry) => entry.datatable_name === input.datatableName
	)
	if (!datatable) {
		// Message MUST match the production `isDatatableNotConfiguredError` regex
		// (/datatable\s+\S+\s+not found/i in datatableTools.ts) so the
		// get_datatable_table_schema not-configured mapping is actually exercised.
		throw new Error(`datatable "${input.datatableName}" not found`)
	}
	const table = datatable.schemas?.[input.schemaName]?.[input.tableName]
	if (!table) {
		throw new Error(
			`table "${input.schemaName}.${input.tableName}" not found in datatable "${input.datatableName}"`
		)
	}
	return {
		datatable_name: input.datatableName,
		schema_name: input.schemaName,
		table_name: input.tableName,
		columns: table.columns
	}
}

/**
 * Execute SQL against a seeded datatable with NO real engine: a SELECT returns
 * the canned rows of the referenced (or first) seeded table; any other
 * statement (CREATE/INSERT/UPDATE/DELETE/...) returns `[]` success. Creates a
 * benchmark completed job and returns its id, like `runBenchmarkScriptPreview`.
 */
export function runBenchmarkDatatableSql(input: {
	workspace: string
	datatableName: string
	sql: string
}): string {
	const rows = /^\s*select/i.test(input.sql)
		? resolveBenchmarkDatatableSelectRows(input.workspace, input.datatableName, input.sql)
		: []
	return createBenchmarkCompletedJob({
		workspace: input.workspace,
		jobKind: 'preview',
		success: true,
		args: { database: `datatable://${input.datatableName}` },
		result: rows
	})
}

/**
 * Best-effort row resolution for a canned SELECT: match the table named in the
 * first FROM clause, else fall back to the first seeded table. Row fidelity is
 * intentionally loose — SELECT cases judge behavior (queried + reported back),
 * not exact cell values (see datatable-evals-plan.md).
 */
function resolveBenchmarkDatatableSelectRows(
	workspace: string,
	datatableName: string,
	sql: string
): Record<string, unknown>[] {
	const runnables = benchmarkWorkspaceRunnables.get(workspace)
	const datatable = (runnables?.datatables ?? []).find(
		(entry) => entry.datatable_name === datatableName
	)
	if (!datatable) {
		return []
	}
	const tables = Object.values(datatable.schemas).flatMap((schemaTables) =>
		Object.entries(schemaTables).map(([table, seed]) => ({ table, seed }))
	)
	if (tables.length === 0) {
		return []
	}
	const referenced = sql.match(/\bfrom\s+([a-zA-Z_][\w.]*)/i)?.[1]?.split('.').pop()?.toLowerCase()
	const matched = referenced
		? tables.find((entry) => entry.table.toLowerCase() === referenced)
		: undefined
	return (matched ?? tables[0]).seed.rows ?? []
}

/**
 * Mirror `JobService.getCompletedJobResultMaybe` for benchmark workspaces — the
 * shape `pollJobResult` consumes. The job is created synchronously before
 * polling, so it is always present and completed.
 */
export function getBenchmarkCompletedJobResultMaybe(input: {
	workspace: string
	id: string
}): { success: boolean; completed: boolean; result: unknown } {
	const job = getBenchmarkCompletedJob(input.workspace, input.id)
	if (!job) {
		throw new Error(`Job "${input.id}" not found in benchmark workspace`)
	}
	return { success: job.success, completed: true, result: job.result }
}

export function runBenchmarkScriptPreview(input: {
	workspace: string
	requestBody: {
		content?: string
		language?: ScriptLang | 'bunnative'
		args?: Record<string, unknown>
		path?: string
	}
}): string {
	const content = input.requestBody.content ?? ''
	const language = input.requestBody.language ?? 'bun'
	const lintResult = buildScriptLintResult(content, language)
	const success = lintResult.errorCount === 0

	return createBenchmarkCompletedJob({
		workspace: input.workspace,
		jobKind: 'preview',
		success,
		scriptPath: input.requestBody.path,
		args: input.requestBody.args,
		result: success
			? {
					path: input.requestBody.path,
					args: input.requestBody.args ?? {},
					validated: true
				}
			: {
					path: input.requestBody.path,
					args: input.requestBody.args ?? {},
					errorCount: lintResult.errorCount,
					errors: lintResult.errors.map((entry) => ({
						line: entry.startLineNumber,
						message: entry.message
					}))
				}
	})
}

export function runBenchmarkFlowByPath(input: {
	workspace: string
	path: string
	args?: Record<string, unknown>
}): string {
	const flow = getBenchmarkFlowByPath(input.workspace, input.path)
	return createBenchmarkCompletedJob({
		workspace: input.workspace,
		jobKind: 'flowpreview',
		success: flow !== null,
		args: input.args,
		result:
			flow !== null
				? {
						path: input.path,
						args: input.args ?? {},
						mocked: true
					}
				: {
						error: `Flow "${input.path}" not found in benchmark workspace`
					},
		logs:
			flow !== null
				? 'Mock benchmark flow run completed successfully.'
				: `Flow "${input.path}" not found in benchmark workspace.`
	})
}

export function previewBenchmarkSchedule(input: {
	requestBody?: Record<string, unknown>
}): Record<string, unknown> {
	const schedule = input.requestBody?.schedule
	if (typeof schedule !== 'string' || schedule.trim().split(/\s+/).length !== 6) {
		throw new Error(`schedule must use a six-field cron expression, got ${JSON.stringify(schedule)}`)
	}

	return {
		next_runs: ['1970-01-02T00:00:00.000Z']
	}
}

export function createBenchmarkSchedule(input: {
	workspace: string
	requestBody: Record<string, unknown>
}): Record<string, unknown> {
	assertBenchmarkWorkspacePath('schedule', input.requestBody.path)
	assertBenchmarkWorkspacePath('target', input.requestBody.script_path)
	return {
		path: input.requestBody.path,
		target_path: input.requestBody.script_path,
		is_flow: input.requestBody.is_flow,
		mocked: true
	}
}

export function createBenchmarkHttpTrigger(input: {
	workspace: string
	requestBody: Record<string, unknown>
}): Record<string, unknown> {
	assertBenchmarkWorkspacePath('trigger', input.requestBody.path)
	assertBenchmarkWorkspacePath('target', input.requestBody.script_path)
	if (
		typeof input.requestBody.route_path === 'string' &&
		input.requestBody.route_path.startsWith('/')
	) {
		throw new Error(`HTTP trigger route_path must not start with /, got "${input.requestBody.route_path}"`)
	}
	return {
		path: input.requestBody.path,
		target_path: input.requestBody.script_path,
		route_path: input.requestBody.route_path,
		is_flow: input.requestBody.is_flow,
		mocked: true
	}
}

function assertBenchmarkWorkspacePath(label: string, value: unknown): void {
	if (typeof value !== 'string' || (!value.startsWith('f/') && !value.startsWith('u/'))) {
		throw new Error(`${label} path must start with f/ or u/, got ${JSON.stringify(value)}`)
	}
}

function buildBenchmarkScriptHash(path: string): string {
	return `benchmark:${path}`
}

function buildBenchmarkScript(script: BenchmarkWorkspaceScript): Script {
	return {
		workspace_id: 'benchmark',
		hash: buildBenchmarkScriptHash(script.path),
		path: script.path,
		parent_hashes: [],
		summary: script.summary,
		description: script.description ?? '',
		content: script.content,
		created_by: 'benchmark',
		created_at: BENCHMARK_TIMESTAMP,
		archived: false,
		schema: script.schema ?? {},
		deleted: false,
		is_template: false,
		extra_perms: {},
		language: script.language,
		kind: 'script',
		starred: false,
		has_preprocessor: false,
		modules: null
	}
}

function buildBenchmarkFlow(flow: BenchmarkWorkspaceFlow): Flow {
	return {
		path: flow.path,
		summary: flow.summary,
		description: flow.description ?? '',
		value: flow.value,
		schema: flow.schema ?? {},
		edited_by: 'benchmark',
		edited_at: BENCHMARK_TIMESTAMP,
		archived: false,
		extra_perms: {}
	} as Flow
}
