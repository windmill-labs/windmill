import { randomUUID } from 'node:crypto'
import type { CompletedJob, Flow, Script } from '../../../frontend/src/lib/gen'
import type { ScriptLang } from '../../../frontend/src/lib/gen/types.gen'
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

export interface BenchmarkWorkspaceRunnables {
	scripts?: BenchmarkWorkspaceScript[]
	flows?: BenchmarkWorkspaceFlow[]
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
