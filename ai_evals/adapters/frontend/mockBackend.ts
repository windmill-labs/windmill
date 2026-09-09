import { randomUUID } from 'node:crypto'
import type {
	AppWithLastVersion,
	CompletedJob,
	Flow,
	Job,
	ListableApp,
	ListableResource,
	ListableVariable,
	Resource,
	Script
} from '../../../frontend/src/lib/gen'
import type {
	DataTableTables,
	DataTableTableSchema,
	EndpointTool,
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

export interface BenchmarkWorkspaceVariable {
	path: string
	value: string
	is_secret: boolean
	description?: string
	labels?: string[]
	ws_specific?: boolean
}

/** An AI provider resource of the benchmark workspace, as an AI agent step would reference it.
 * `models` stands in for the provider's model listing, which no eval run can reach. */
export interface BenchmarkWorkspaceAiProvider {
	path: string
	/** Resource type, which for AI resources is the provider kind (`anthropic`, `openai`, ...). */
	kind: string
	/** What this resource's `/ai/proxy/models` listing returns. */
	models?: string[]
	/** Set to point the resource at a gateway rather than the provider's own API. */
	base_url?: string
	/** Models the workspace AI settings selected for this provider. */
	configuredModels?: string[]
	/** Marks this provider's first configured model as the workspace default. */
	isDefault?: boolean
}

/** A plain (non-AI) resource of the benchmark workspace, for cases about referencing a
 * credential — passing one as a run argument, say. `value` is what `get_resource` returns. */
export interface BenchmarkWorkspaceResource {
	path: string
	resource_type: string
	value?: Record<string, unknown>
	description?: string
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
	variables?: BenchmarkWorkspaceVariable[]
	aiProviders?: BenchmarkWorkspaceAiProvider[]
	resources?: BenchmarkWorkspaceResource[]
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

function buildBenchmarkVariable(
	workspace: string,
	seed: BenchmarkWorkspaceVariable,
	decryptSecret: boolean
): ListableVariable {
	return {
		workspace_id: workspace,
		path: seed.path,
		// Mirror `get_variable`: a secret's value is withheld unless decryption was
		// asked for, so a reader genuinely cannot see it.
		value: seed.is_secret && !decryptSecret ? undefined : seed.value,
		is_secret: seed.is_secret,
		description: seed.description,
		labels: seed.labels,
		ws_specific: seed.ws_specific ?? false,
		extra_perms: {},
		edited_at: BENCHMARK_TIMESTAMP
	}
}

export function listBenchmarkVariables(workspace: string): ListableVariable[] | null {
	const runnables = benchmarkWorkspaceRunnables.get(workspace)
	if (!runnables) {
		return null
	}
	// The list route never decrypts.
	return (runnables.variables ?? []).map((seed) => buildBenchmarkVariable(workspace, seed, false))
}

/** AI provider resources of a benchmark workspace, shaped like `ResourceService.listResource`
 * rows (which carry no value). Null when the workspace is not a benchmark one. */
export function listBenchmarkAiProviderResources(workspace: string): ListableResource[] | null {
	const runnables = benchmarkWorkspaceRunnables.get(workspace)
	if (!runnables) {
		return null
	}
	return (runnables.aiProviders ?? []).map((seed) => ({
		workspace_id: workspace,
		path: seed.path,
		resource_type: seed.kind,
		value: null,
		is_oauth: false,
		is_linked: false,
		is_refreshed: false,
		extra_perms: {},
		edited_at: BENCHMARK_TIMESTAMP
	}))
}

/** Plain seeded resources of a benchmark workspace, shaped like `ResourceService.listResource`
 * rows. Null when the workspace is not a benchmark one. */
export function listBenchmarkPlainResources(workspace: string): ListableResource[] | null {
	const runnables = benchmarkWorkspaceRunnables.get(workspace)
	if (!runnables) {
		return null
	}
	return (runnables.resources ?? []).map((seed) => ({
		workspace_id: workspace,
		path: seed.path,
		resource_type: seed.resource_type,
		description: seed.description,
		value: null,
		is_oauth: false,
		is_linked: false,
		is_refreshed: false,
		extra_perms: {},
		edited_at: BENCHMARK_TIMESTAMP
	}))
}

/** A seeded resource with its value, as `ResourceService.getResource` returns it. Covers both
 * seed kinds, so it agrees with `existsResource` and `listResource` — both of those report AI
 * providers too, and a case that lists resources and then reads one by path would otherwise get
 * a row it cannot fetch. */
export function getBenchmarkResource(workspace: string, path: string): Resource | null {
	const runnables = benchmarkWorkspaceRunnables.get(workspace)
	const seed = runnables?.resources?.find((entry) => entry.path === path)
	if (seed) {
		return {
			workspace_id: workspace,
			path: seed.path,
			resource_type: seed.resource_type,
			description: seed.description,
			value: seed.value ?? {},
			is_oauth: false,
			extra_perms: {}
		} as Resource
	}
	const provider = runnables?.aiProviders?.find((entry) => entry.path === path)
	if (!provider) {
		return null
	}
	return {
		workspace_id: workspace,
		path: provider.path,
		resource_type: provider.kind,
		value: getBenchmarkResourceValue(workspace, path) ?? {},
		is_oauth: false,
		extra_perms: {}
	} as Resource
}

/** The value of a seeded resource. For an AI provider only the endpoint fields are modelled — a
 * key is never needed, because no eval run calls the provider through this resource. */
export function getBenchmarkResourceValue(
	workspace: string,
	path: string
): Record<string, unknown> | null {
	const runnables = benchmarkWorkspaceRunnables.get(workspace)
	const plain = runnables?.resources?.find((entry) => entry.path === path)
	if (plain) {
		return plain.value ?? {}
	}
	const seed = runnables?.aiProviders?.find((entry) => entry.path === path)
	if (!seed) {
		return null
	}
	return seed.base_url ? { base_url: seed.base_url } : {}
}

/** The AI settings of a benchmark workspace, as `WorkspaceService.getCopilotInfo` returns them. */
export function getBenchmarkAiConfig(workspace: string): Record<string, unknown> | null {
	const seeds = benchmarkWorkspaceRunnables.get(workspace)?.aiProviders
	if (!seeds) {
		return null
	}
	const providers: Record<string, unknown> = {}
	let defaultModel: { model: string; provider: string } | undefined
	for (const seed of seeds) {
		const models = seed.configuredModels ?? seed.models ?? []
		providers[seed.kind] = { resource_path: seed.path, models }
		if (seed.isDefault && models[0]) {
			defaultModel = { model: models[0], provider: seed.kind }
		}
	}
	return { providers, ...(defaultModel ? { default_model: defaultModel } : {}) }
}

export function getBenchmarkVariableByPath(
	workspace: string,
	path: string,
	decryptSecret = true
): ListableVariable | null {
	const seed = benchmarkWorkspaceRunnables
		.get(workspace)
		?.variables?.find((entry) => entry.path === path)

	return seed ? buildBenchmarkVariable(workspace, seed, decryptSecret) : null
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
	{ workspace: string; kind: UserDraftItemKind; path: string; value: unknown; createdAt: string }
>()

// Counter-based timestamps: deterministic run-to-run (same event order → same
// values) but MONOTONIC per update, because production bumps a draft row's
// created_at on every upsert and the diff snapshot cache keys patch reuse on
// it — a fixed timestamp would serve stale patches after an edit. No eval
// simulates a concurrent writer, so every save is accepted and the conflict
// branch is never taken.
let benchmarkDraftClock = 0
function nextBenchmarkDraftTimestamp(): string {
	benchmarkDraftClock += 1
	return new Date(benchmarkDraftClock * 1000).toISOString()
}

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
		value,
		createdAt: nextBenchmarkDraftTimestamp()
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
	const createdAt = nextBenchmarkDraftTimestamp()
	if (value == null) {
		benchmarkDrafts.delete(key)
	} else {
		benchmarkDrafts.set(key, {
			workspace: input.workspace,
			kind: input.kind,
			path: input.path,
			value,
			createdAt
		})
	}
	return { status: 'saved', current_timestamp: createdAt }
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
	return { value: entry.value, created_at: entry.createdAt }
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
	return { value: entry.value, created_at: entry.createdAt }
}

/** Whether a deployed benchmark item exists for a draft row's kind+path —
 * drives `draft_only`, which production computes against the deployed tables. */
function benchmarkDeployedExists(workspace: string, kind: UserDraftItemKind, path: string): boolean {
	if (kind === 'script') return Boolean(getBenchmarkScriptByPath(workspace, path))
	if (kind === 'flow') return Boolean(getBenchmarkFlowByPath(workspace, path))
	if (kind === 'app' || kind === 'raw_app') return Boolean(getBenchmarkAppByPath(workspace, path))
	if (kind === 'variable') return Boolean(getBenchmarkVariableByPath(workspace, path))
	// The remaining drawer kinds (resources/schedules/triggers) have no deployed
	// benchmark stores today.
	return false
}

/** Mirror `DraftService.listDrafts`: metadata rows (no value) for a workspace. */
export function listBenchmarkDrafts(workspace: string): ListDraftsResponse {
	return [...benchmarkDrafts.values()]
		.filter((entry) => entry.workspace === workspace)
		.map((entry) => ({
			kind: entry.kind,
			path: entry.path,
			summary: (entry.value as { summary?: string } | null)?.summary,
			draft_only: !benchmarkDeployedExists(workspace, entry.kind, entry.path),
			legacy_draft: false,
			created_at: entry.createdAt
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

export function runBenchmarkScriptByPath(input: {
	workspace: string
	path: string
	args?: Record<string, unknown>
}): string {
	const script = getBenchmarkScriptByPath(input.workspace, input.path)
	return createBenchmarkCompletedJob({
		workspace: input.workspace,
		jobKind: 'script',
		success: script !== null,
		scriptPath: input.path,
		args: input.args,
		result:
			script !== null
				? {
						path: input.path,
						args: input.args ?? {},
						mocked: true
					}
				: {
						error: `Script "${input.path}" not found in benchmark workspace`
					},
		logs:
			script !== null
				? 'Mock benchmark script run completed successfully.'
				: `Script "${input.path}" not found in benchmark workspace.`
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

// ============= API endpoint catalog (McpService.listMcpTools + raw fetch) =============
// The global chat's API catalog tools list endpoints via McpService and execute
// them with a plain relative fetch('/api/...'), which has no meaning in the
// vitest environment. A representative slice of the real catalog is served here,
// and `handleBenchmarkApiFetch` answers the executed calls.

const BENCHMARK_MCP_TOOLS: EndpointTool[] = [
	{
		name: 'listWorkers',
		description: 'List workers',
		instructions: 'List all workers with their last ping and job counts.',
		path: '/workers/list',
		method: 'GET',
		query_params_schema: {
			type: 'object',
			properties: { page: { type: 'integer' }, per_page: { type: 'integer' } }
		}
	},
	{
		name: 'listQueue',
		description: 'List queued jobs',
		instructions: '',
		path: '/w/{workspace}/jobs/queue/list',
		method: 'GET',
		path_params_schema: {
			type: 'object',
			properties: { workspace: { type: 'string' } },
			required: ['workspace']
		}
	},
	{
		name: 'getJob',
		description: 'get job',
		instructions: '',
		path: '/w/{workspace}/jobs_u/get/{id}',
		method: 'GET',
		path_params_schema: {
			type: 'object',
			properties: { workspace: { type: 'string' }, id: { type: 'string', format: 'uuid' } },
			required: ['workspace', 'id']
		},
		query_params_schema: {
			type: 'object',
			properties: {
				no_logs: { type: 'boolean' },
				no_code: { type: 'boolean' },
				approval_token: { type: 'string' }
			},
			required: []
		}
	},
	{
		name: 'runScriptByPath',
		description: 'Run the deployed version of a script by path',
		instructions: '',
		path: '/w/{workspace}/jobs/run/p/{path}',
		method: 'POST',
		path_params_schema: {
			type: 'object',
			properties: { workspace: { type: 'string' }, path: { type: 'string' } },
			required: ['workspace', 'path']
		},
		body_schema: { type: 'object', properties: {} }
	},
	{
		name: 'runFlowByPath',
		description: 'Run the deployed version of a flow by path',
		instructions: '',
		path: '/w/{workspace}/jobs/run/f/{path}',
		method: 'POST',
		path_params_schema: {
			type: 'object',
			properties: { workspace: { type: 'string' }, path: { type: 'string' } },
			required: ['workspace', 'path']
		},
		body_schema: { type: 'object', properties: {} }
	},
	// Draft-covered endpoints, present so steering cases exercise the guard the
	// way production does (hidden from search, refused at call time).
	{
		name: 'getScriptByPath',
		description: 'Get a script by path',
		instructions: '',
		path: '/w/{workspace}/scripts/get/p/{path}',
		method: 'GET'
	},
	{
		name: 'createFlow',
		description: 'Create a flow',
		instructions: '',
		path: '/w/{workspace}/flows/create',
		method: 'POST'
	},
	{
		name: 'deleteSchedule',
		description: 'Delete a schedule',
		instructions: '',
		path: '/w/{workspace}/schedules/delete/{path}',
		method: 'DELETE'
	},
	{
		name: 'getVariable',
		description: 'Get a variable',
		instructions: '',
		path: '/w/{workspace}/variables/get/{path}',
		method: 'GET'
	}
]

export function listBenchmarkMcpTools(): EndpointTool[] {
	return BENCHMARK_MCP_TOOLS
}

/** A stand-in Windmill hub. `search_hub_scripts` and a `hub/` read go out over
 * relative `/api/...` fetches, which have no origin here, so without these the
 * hub tools throw and no case can exercise hub reuse. Serving fixtures rather
 * than the live hub also keeps assertions on script content stable as the real
 * hub republishes new versions. */
const BENCHMARK_HUB_SCRIPTS = [
	{
		version_id: 22235,
		app: 'holded',
		summary: 'Send Document',
		terms: 'holded invoice document send email mail',
		language: 'bun',
		content: `//native
type Holded = {
  apiKey: string;
};
/**
 * Send Document
 * Send a specific document by email.
 */
export async function main(
  auth: Holded,
  docType: string,
  documentId: string,
  body: {
    mailTemplateId?: string;
    emails: string;
    subject?: string;
    message?: string;
    docIds?: string;
  },
) {
  const url = new URL(
    \`https://api.holded.com/api/invoicing/v1/documents/\${docType}/\${documentId}/send\`,
  );

  const response = await fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      key: auth.apiKey,
    },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    const text = await response.text();
    throw new Error(\`\${response.status} \${text}\`);
  }
  return await response.json();
}
`,
		schema: {
			type: 'object',
			required: ['auth', 'docType', 'documentId', 'body'],
			properties: {
				auth: { type: 'object', format: 'resource-holded' },
				docType: { type: 'string' },
				documentId: { type: 'string' },
				body: { type: 'object' }
			}
		}
	},
	{
		version_id: 28294,
		app: 'discord',
		summary: 'Send a message to Discord using Webhook',
		terms: 'discord webhook message send chat channel',
		language: 'bunnative',
		content: `//native

type DiscordWebhook = {
  webhook_url: string;
};
export async function main(discord_webhook: DiscordWebhook, message: string) {
  const response = await fetch(\`\${discord_webhook.webhook_url}?wait=true\`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ content: message }),
  });
  if (!response.ok) {
    throw new Error(\`\${response.status} \${await response.text()}\`);
  }
  return await response.json();
}
`,
		schema: {
			type: 'object',
			required: ['discord_webhook', 'message'],
			properties: {
				discord_webhook: { type: 'object', format: 'resource-discord_webhook' },
				message: { type: 'string' }
			}
		}
	}
]

/** Naive whole-word overlap — enough to rank a handful of fixtures for a natural
 * query without pulling an embedding model into the benchmark. Every frontend eval
 * shares this handler, so the bar to match is deliberately high: naming the
 * integration, or overlapping on three meaningful words. A looser bar answers
 * "send a Slack message" with the Discord fixture, handing an unrelated case a
 * plausible-looking wrong integration. */
function searchBenchmarkHubScripts(text: string) {
	const tokens = new Set(
		text
			.toLowerCase()
			.split(/[^a-z0-9]+/)
			.filter((token) => token.length > 2)
	)
	return BENCHMARK_HUB_SCRIPTS.map((script) => {
		const words = new Set(
			`${script.app} ${script.summary} ${script.terms}`.toLowerCase().split(/[^a-z0-9]+/)
		)
		const score = [...tokens].filter((token) => words.has(token)).length
		return { script, score, namesApp: tokens.has(script.app) }
	})
		.filter((entry) => entry.namesApp || entry.score >= 3)
		.sort((a, b) => b.score - a.score)
		.map(({ script }, index) => ({
			ask_id: script.version_id,
			id: script.version_id,
			version_id: script.version_id,
			summary: script.summary,
			app: script.app,
			kind: 'script',
			score: 1 - index * 0.01
		}))
}

/** The hub keys a script by its version id; the app and slug segments that
 * follow are descriptive, so match on the id exactly as the real hub does. */
function getBenchmarkHubScript(path: string) {
	const versionId = Number(path.replace(/^\/api\/scripts\/hub\/get_full\/hub\//, '').split('/')[0])
	return BENCHMARK_HUB_SCRIPTS.find((script) => script.version_id === versionId)
}

const BENCHMARK_WORKERS = [
	{
		worker: 'wk-benchmark-1',
		worker_instance: 'benchmark-host',
		last_ping: 2,
		started_at: BENCHMARK_TIMESTAMP,
		jobs_executed: 42,
		custom_tags: null,
		worker_group: 'default',
		wm_version: 'benchmark'
	},
	{
		worker: 'wk-benchmark-2',
		worker_instance: 'benchmark-host',
		last_ping: 5,
		started_at: BENCHMARK_TIMESTAMP,
		jobs_executed: 17,
		custom_tags: null,
		worker_group: 'default',
		wm_version: 'benchmark'
	}
]

const BENCHMARK_JOB_GET_PATH = /^\/api\/w\/([^/]+)\/jobs_u\/get\/([^/]+)$/
const BENCHMARK_RUN_BY_PATH = /^\/api\/w\/([^/]+)\/jobs\/run\/(p|f)\/([^/]+)$/

/** `executeEndpoint` sends a JSON string; anything else means no args were supplied. */
function parseBenchmarkRequestBody(
	body: BodyInit | null | undefined
): Record<string, unknown> | undefined {
	if (typeof body !== 'string') {
		return undefined
	}
	try {
		const parsed = JSON.parse(body)
		return typeof parsed === 'object' && parsed !== null
			? (parsed as Record<string, unknown>)
			: undefined
	} catch {
		return undefined
	}
}

/** True when `handleBenchmarkApiFetch` has an answer for this `/api/...` url.
 * Any other relative fetch must keep its normal (non-benchmark) behavior —
 * intercepting it with a synthetic 404 sends the model into retry loops. */
// Not anchored: the frontend builds this URL from location.origin, so it arrives absolute. The
// workspace id is greedy because an eval workspace is a temp directory path, slashes and all.
const BENCHMARK_AI_MODELS_PATH = /\/api\/w\/(.+)\/ai\/proxy\/models$/

export function hasBenchmarkApiHandler(url: string): boolean {
	const path = url.split('?')[0]
	return (
		path === '/api/workers/list' ||
		BENCHMARK_JOB_GET_PATH.test(path) ||
		BENCHMARK_RUN_BY_PATH.test(path) ||
		/^\/api\/w\/[^/]+\/jobs\/queue\/list$/.test(path) ||
		path === '/api/embeddings/query_hub_scripts' ||
		path.startsWith('/api/scripts/hub/get_full/') ||
		BENCHMARK_AI_MODELS_PATH.test(path)
	)
}

/** Answer a relative `/api/...` fetch — from the API catalog executor, or from the
 * chat's hub tools. */
export function handleBenchmarkApiFetch(url: string, init?: RequestInit): Response {
	const path = url.split('?')[0]
	if (path === '/api/workers/list') {
		return Response.json(BENCHMARK_WORKERS)
	}
	// The provider's own model listing, which grounds an AI agent step's model id. Keyed by the
	// resource the caller names, so two seeded providers can serve different models.
	const aiModels = BENCHMARK_AI_MODELS_PATH.exec(path)
	if (aiModels) {
		const headers = new Headers(init?.headers)
		const resourcePath = headers.get('X-Resource-Path') ?? ''
		const seed = benchmarkWorkspaceRunnables
			.get(decodeURIComponent(aiModels[1]))
			?.aiProviders?.find((entry) => entry.path === resourcePath)
		return Response.json({ data: (seed?.models ?? []).map((id) => ({ id })) })
	}
	if (/^\/api\/w\/[^/]+\/jobs\/queue\/list$/.test(path)) {
		return Response.json([])
	}
	const jobGet = BENCHMARK_JOB_GET_PATH.exec(path)
	if (jobGet) {
		const id = decodeURIComponent(jobGet[2])
		const job = getBenchmarkCompletedJob(decodeURIComponent(jobGet[1]), id)
		if (!job) {
			return Response.json({ error: `Job not found for "${id}"` }, { status: 404 })
		}
		// The real endpoint lets a caller drop the bulky fields. Ignoring that here would
		// size the model's context off a payload it explicitly asked to shrink.
		const query = new URLSearchParams(url.split('?')[1] ?? '')
		if (query.get('no_logs') === 'true') {
			delete job.logs
		}
		if (query.get('no_code') === 'true') {
			delete job.raw_code
		}
		return Response.json(job)
	}
	const runByPath = BENCHMARK_RUN_BY_PATH.exec(path)
	if (runByPath) {
		const workspace = decodeURIComponent(runByPath[1])
		const runnablePath = decodeURIComponent(runByPath[3])
		const args = parseBenchmarkRequestBody(init?.body)
		// The real endpoint answers with the bare job id as text, not JSON.
		return new Response(
			runByPath[2] === 'f'
				? runBenchmarkFlowByPath({ workspace, path: runnablePath, args })
				: runBenchmarkScriptByPath({ workspace, path: runnablePath, args })
		)
	}
	if (path === '/api/embeddings/query_hub_scripts') {
		const text = new URLSearchParams(url.split('?')[1] ?? '').get('text') ?? ''
		return Response.json(searchBenchmarkHubScripts(text))
	}
	if (path.startsWith('/api/scripts/hub/get_full/')) {
		const script = getBenchmarkHubScript(path)
		if (!script) {
			return Response.json({ error: 'hub script not found' }, { status: 404 })
		}
		return Response.json({
			content: script.content,
			language: script.language,
			schema: script.schema,
			summary: script.summary
		})
	}
	return Response.json({ error: `no benchmark handler for ${path}` }, { status: 404 })
}
