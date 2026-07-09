import { randomUUID } from 'node:crypto'
import type {
	AppWithLastVersion,
	CompletedJob,
	Flow,
	Job,
	ListableApp,
	Script
} from '../../../frontend/src/lib/gen'
import type {
	DataTableTables,
	DataTableTableSchema,
	GetDraftForUserResponse,
	GetOwnDraftResponse,
	ListDraftsResponse,
	ScriptLang,
	UpdateDraftResponse,
	UserDraftItemKind
} from '../../../frontend/src/lib/gen/types.gen'
import { buildScriptLintResult } from './core/script/preview'
import { applyDatatableSql, type BenchmarkDatatableSeed } from './datatableSqlEngine'

export type { BenchmarkDatatableSeed, BenchmarkDatatableTableSeed } from './datatableSqlEngine'

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

export interface BenchmarkWorkspaceApp {
	path: string
	summary: string
	value: {
		files: Record<string, string>
		runnables: Record<string, unknown>
		data?: unknown
		policy?: unknown
		custom_path?: unknown
	}
}

export interface BenchmarkWorkspaceJob {
	/** Stable id so a case prompt can reference a specific run (e.g. for get_job_logs). */
	id?: string
	jobKind?: CompletedJob['job_kind']
	scriptPath?: string
	createdBy?: string
	label?: string
	success?: boolean
	logs?: string
}

export interface BenchmarkWorkspaceRunnables {
	scripts?: BenchmarkWorkspaceScript[]
	flows?: BenchmarkWorkspaceFlow[]
	apps?: BenchmarkWorkspaceApp[]
	datatables?: BenchmarkDatatableSeed[]
	jobs?: BenchmarkWorkspaceJob[]
}

type BenchmarkCompletedJob = CompletedJob & { type: 'CompletedJob' }

const benchmarkWorkspaces = new Set<string>()
const benchmarkWorkspaceRunnables = new Map<string, BenchmarkWorkspaceRunnables>()
// Keyed by `${workspace}::${jobId}` so concurrent attempts (or distinct cases)
// can seed the same fixed job id without clobbering each other's entry.
const benchmarkJobs = new Map<string, { workspace: string; job: BenchmarkCompletedJob }>()

function benchmarkJobKey(workspace: string, jobId: string): string {
	return `${workspace}::${jobId}`
}

export function resetBenchmarkMockBackend(): void {
	benchmarkWorkspaces.clear()
	benchmarkWorkspaceRunnables.clear()
	benchmarkJobs.clear()
	benchmarkDrafts.clear()
}

// Stand-in for FolderService.createFolder so the global create_folder tool runs in
// memory instead of mutating the real backend. Folders aren't otherwise modelled
// (no folder-listing in evals), so this just echoes the created name.
export function createBenchmarkFolder(_workspace: string, name: string): string {
	return name
}

export function registerBenchmarkWorkspace(workspace: string): void {
	benchmarkWorkspaces.add(workspace)
}

export function registerBenchmarkWorkspaceRunnables(
	workspace: string,
	runnables: BenchmarkWorkspaceRunnables
): void {
	benchmarkWorkspaces.add(workspace)
	// Fresh case: drop any drafts left from a prior run on this workspace id.
	clearBenchmarkDrafts(workspace)
	// Datatables are mutated in place by exec_datatable_sql (a write must be visible
	// to later reads), so store an isolated deep copy — never mutate the caller's seed.
	benchmarkWorkspaceRunnables.set(workspace, {
		...runnables,
		datatables: runnables.datatables ? structuredClone(runnables.datatables) : undefined
	})
	// Seed any fixture jobs so list_runs / get_job_logs have data to return.
	for (const seed of runnables.jobs ?? []) {
		createBenchmarkCompletedJob({
			workspace,
			id: seed.id,
			jobKind: seed.jobKind ?? 'script',
			success: seed.success,
			scriptPath: seed.scriptPath,
			createdBy: seed.createdBy,
			label: seed.label,
			logs: seed.logs
		})
	}
}

export function unregisterBenchmarkWorkspace(workspace: string): void {
	benchmarkWorkspaces.delete(workspace)
	benchmarkWorkspaceRunnables.delete(workspace)
	clearBenchmarkDrafts(workspace)
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

export function listBenchmarkApps(workspace: string): ListableApp[] | null {
	const runnables = benchmarkWorkspaceRunnables.get(workspace)
	if (!runnables) {
		return null
	}
	return (runnables.apps ?? []).map(buildBenchmarkListableApp)
}

export function getBenchmarkAppByPath(workspace: string, path: string): AppWithLastVersion | null {
	const app = benchmarkWorkspaceRunnables
		.get(workspace)
		?.apps?.find((entry) => entry.path === path)

	return app ? buildBenchmarkApp(app) : null
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
	id?: string
	createdBy?: string
	label?: string
}): string {
	const jobId = input.id ?? `benchmark-job-${randomUUID()}`
	const now = new Date().toISOString()
	const job: BenchmarkCompletedJob = {
		type: 'CompletedJob',
		id: jobId,
		workspace_id: input.workspace,
		created_by: input.createdBy ?? 'ai-evals',
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
		tag: 'benchmark',
		labels: input.label ? [input.label] : undefined
	}

	benchmarkJobs.set(benchmarkJobKey(input.workspace, jobId), { workspace: input.workspace, job })
	return jobId
}

export function getBenchmarkCompletedJob(
	workspace: string,
	jobId: string
): BenchmarkCompletedJob | null {
	const entry = benchmarkJobs.get(benchmarkJobKey(workspace, jobId))
	if (!entry) {
		return null
	}
	return structuredClone(entry.job)
}

/**
 * List seeded/recorded jobs for a benchmark workspace, most recent first —
 * the shape `JobService.listJobs` returns. Returns `null` for a non-benchmark
 * workspace so the caller can fall through to the real backend. Server-side
 * filters (path/creator/status/limit) are intentionally not applied: global
 * eval cases assert on the recorded `list_runs` tool call, not on filtering.
 */
export function listBenchmarkJobs(workspace: string): Job[] | null {
	if (!hasBenchmarkWorkspace(workspace)) {
		return null
	}
	return [...benchmarkJobs.values()]
		.filter((entry) => entry.workspace === workspace)
		.map((entry) => structuredClone(entry.job) as Job)
		.sort((a, b) => (b.created_at ?? '').localeCompare(a.created_at ?? ''))
}

/**
 * Mirror `JobService.getJobLogs` (response is the raw log string). Throws a
 * "not found" error for an unknown id, matching the backend 404.
 */
export function getBenchmarkJobLogs(workspace: string, jobId: string): string {
	const job = getBenchmarkCompletedJob(workspace, jobId)
	if (!job) {
		throw new Error(`Job Logs not found for "${jobId}"`)
	}
	return job.logs ?? ''
}

// ============= Drafts (per-user, DB-backed in production) =============

/**
 * In-memory stand-in for the per-user draft backend (`DraftService`). The global
 * AI chat now persists and reads drafts through the backend DB instead of an
 * in-tab `UserDraft` cell, so the eval mocks the draft endpoints it exercises
 * (`updateDraft` / `getOwnDraft` / `getDraftForUser` / `listDrafts`) and keeps the
 * saved values here, keyed by workspace + draft kind + storage path. Mirrors the
 * semantics of the production unit test's mock in
 * `frontend/src/lib/components/copilot/chat/global/core.test.ts`.
 */
const benchmarkDrafts = new Map<
	string,
	{ workspace: string; kind: UserDraftItemKind; path: string; value: unknown }
>()

// Fixed timestamp so artifacts stay deterministic. No eval simulates a
// concurrent writer, so every save is accepted and the conflict branch is
// never taken — the syncer just records this as its `last_sync` baseline.
const BENCHMARK_DRAFT_TIMESTAMP = '1970-01-01T00:00:00.000Z'

function benchmarkDraftKey(workspace: string, kind: string, path: string): string {
	return `${workspace}::${kind}::${path}`
}

export function clearBenchmarkDrafts(workspace: string): void {
	for (const [key, entry] of benchmarkDrafts.entries()) {
		if (entry.workspace === workspace) {
			benchmarkDrafts.delete(key)
		}
	}
}

/**
 * Seed a draft straight into the store — used by the eval's live-editor draft
 * fixtures, which model "the user already has this draft open/saved". Writing it
 * here (instead of through `UserDraft.save`) keeps it a backend draft row with no
 * shadowing in-tab cell, so a model edit that persists to the backend is what the
 * output read-back captures — not the stale seed.
 */
export function seedBenchmarkDraft(
	workspace: string,
	kind: UserDraftItemKind,
	path: string,
	value: unknown
): void {
	benchmarkDrafts.set(benchmarkDraftKey(workspace, kind, path), {
		workspace,
		kind,
		path,
		value
	})
}

/** Mirror `DraftService.updateDraft`: a `null`/omitted value deletes the row. */
export function updateBenchmarkDraft(input: {
	workspace: string
	kind: UserDraftItemKind
	path: string
	requestBody?: { value?: unknown }
}): UpdateDraftResponse {
	const key = benchmarkDraftKey(input.workspace, input.kind, input.path)
	const value = input.requestBody?.value
	if (value == null) {
		benchmarkDrafts.delete(key)
	} else {
		benchmarkDrafts.set(key, {
			workspace: input.workspace,
			kind: input.kind,
			path: input.path,
			value
		})
	}
	return { status: 'saved', current_timestamp: BENCHMARK_DRAFT_TIMESTAMP }
}

/** Mirror `DraftService.getDraftForUser`: 404-shaped throw when absent so the
 * adapter's narrowed catch treats it as "no draft" instead of re-throwing. */
export function getBenchmarkDraftForUser(input: {
	workspace: string
	kind: UserDraftItemKind
	path: string
}): GetDraftForUserResponse {
	const entry = benchmarkDrafts.get(benchmarkDraftKey(input.workspace, input.kind, input.path))
	if (!entry) {
		throw Object.assign(new Error(`no draft for "${input.path}"`), { status: 404 })
	}
	return { value: entry.value, created_at: BENCHMARK_DRAFT_TIMESTAMP }
}

/** Mirror `DraftService.getOwnDraft`: `null` (200) when absent — unlike
 * `getDraftForUser`, absence is not an error on this route. */
export function getBenchmarkOwnDraft(input: {
	workspace: string
	kind: UserDraftItemKind
	path: string
}): GetOwnDraftResponse {
	const entry = benchmarkDrafts.get(benchmarkDraftKey(input.workspace, input.kind, input.path))
	if (!entry) {
		return null
	}
	return { value: entry.value, created_at: BENCHMARK_DRAFT_TIMESTAMP }
}

/** Mirror `DraftService.listDrafts`: metadata rows (no value) for a workspace. */
export function listBenchmarkDrafts(workspace: string): ListDraftsResponse {
	return [...benchmarkDrafts.values()]
		.filter((entry) => entry.workspace === workspace)
		.map((entry) => ({
			kind: entry.kind,
			path: entry.path,
			summary: (entry.value as { summary?: string } | null)?.summary,
			draft_only: true,
			legacy_draft: false,
			created_at: BENCHMARK_DRAFT_TIMESTAMP
		}))
}

// ============= Datatables (best-effort in-memory SQL) =============

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
 * Execute SQL against a seeded datatable through the best-effort in-memory engine
 * (`applyDatatableSql`). Writes (CREATE/INSERT/UPDATE/DELETE/DROP) mutate the
 * stored datatable in place so a later list/schema/SELECT reflects them; SELECT
 * (and RETURNING) yield rows, other statements yield `[]`. Creates a benchmark
 * completed job and returns its id, like `runBenchmarkScriptPreview`.
 */
export function runBenchmarkDatatableSql(input: {
	workspace: string
	datatableName: string
	sql: string
}): string {
	const runnables = benchmarkWorkspaceRunnables.get(input.workspace)
	const datatable = (runnables?.datatables ?? []).find(
		(entry) => entry.datatable_name === input.datatableName
	)
	const rows = datatable ? applyDatatableSql(datatable, input.sql).rows : []
	return createBenchmarkCompletedJob({
		workspace: input.workspace,
		jobKind: 'preview',
		success: true,
		args: { database: `datatable://${input.datatableName}` },
		result: rows
	})
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

function buildBenchmarkListableApp(app: BenchmarkWorkspaceApp): ListableApp {
	return {
		id: 0,
		workspace_id: 'benchmark',
		path: app.path,
		summary: app.summary,
		version: 1,
		extra_perms: {},
		edited_at: BENCHMARK_TIMESTAMP,
		execution_mode: 'viewer',
		raw_app: true
	}
}

function buildBenchmarkApp(app: BenchmarkWorkspaceApp): AppWithLastVersion {
	return {
		id: 0,
		workspace_id: 'benchmark',
		path: app.path,
		summary: app.summary,
		versions: [1],
		created_by: 'benchmark',
		created_at: BENCHMARK_TIMESTAMP,
		value: app.value,
		policy: (app.value.policy ?? {}) as AppWithLastVersion['policy'],
		execution_mode: 'viewer',
		extra_perms: {},
		custom_path: app.value.custom_path as string | undefined,
		raw_app: true
	}
}
