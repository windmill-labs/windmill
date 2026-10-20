import { randomUUID } from 'node:crypto'
import confluenceFixture from '../../fixtures/frontend/global/hub/confluence.json'
import servicenowFixture from '../../fixtures/frontend/global/hub/servicenow.json'
import outreachFixture from '../../fixtures/frontend/global/hub/outreach.json'
import syncedResourceTypesFixture from '../../fixtures/frontend/global/hub/synced_resource_types.json'
import type { CompletedJob, Flow, Script } from '../../../frontend/src/lib/gen'
import type { DataMetric, ScriptLang } from '../../../frontend/src/lib/gen/types.gen'
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

export interface BenchmarkWorkspaceApp {
	path: string
	summary: string
	/** Defaults to true. Set false for a drag-and-drop app, which the chat can list
	 * and read but has no tool to edit — its value is a grid, not files. */
	rawApp?: boolean
	value: {
		files?: Record<string, string>
		runnables?: Record<string, unknown>
		data?: unknown
		policy?: unknown
		custom_path?: unknown
		[key: string]: unknown
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

/** A resource type the instance holds. */
export interface BenchmarkWorkspaceResourceType {
	name: string
	schema?: unknown
	description?: string
}

export interface BenchmarkWorkspaceJob {
	/** Stable id so a case prompt can reference a specific run (e.g. for get_run). */
	id?: string
	jobKind?: CompletedJob['job_kind']
	scriptPath?: string
	createdBy?: string
	label?: string
	success?: boolean
	logs?: string
	args?: Record<string, unknown>
	result?: unknown
}

/** One seeded datatable table: its columns (col -> compact_type) and optional rows. */
export interface BenchmarkDatatableTableSeed {
	columns: Record<string, string>
	rows?: Record<string, unknown>[]
}

/** A seeded datatable: `datatable_name` plus a `schema -> table -> seed` map. */
export interface BenchmarkDatatableSeed {
	datatable_name: string
	schemas: {
		[schema: string]: {
			[table: string]: BenchmarkDatatableTableSeed
		}
	}
	/** Whether the datatable keeps its schema as migrations. */
	migrations_enabled?: boolean
	migrations?: BenchmarkDatatableMigrationSeed[]
}

export interface BenchmarkDatatableMigrationSeed {
	timestamp: number
	name: string
	code_up: string
	code_down?: string
	applied?: boolean
}

export interface BenchmarkWorkspaceRunnables {
	scripts?: BenchmarkWorkspaceScript[]
	flows?: BenchmarkWorkspaceFlow[]
	apps?: BenchmarkWorkspaceApp[]
	variables?: BenchmarkWorkspaceVariable[]
	aiProviders?: BenchmarkWorkspaceAiProvider[]
	resources?: BenchmarkWorkspaceResource[]
	/** Unset means an instance synced with the hub. */
	resourceTypes?: BenchmarkWorkspaceResourceType[]
	datatables?: BenchmarkDatatableSeed[]
	/** DuckLake catalog names, as `list_ducklakes` reports them. */
	ducklakes?: string[]
	dataMetrics?: DataMetric[]
	jobs?: BenchmarkWorkspaceJob[]
}

type BenchmarkCompletedJob = CompletedJob & { type: 'CompletedJob' }

const benchmarkWorkspaces = new Set<string>()
const benchmarkWorkspaceRunnables = new Map<string, BenchmarkWorkspaceRunnables>()
// Keyed by `${workspace}::${jobId}` so a job is only visible from the workspace that ran it.
const benchmarkJobs = new Map<string, { workspace: string; job: BenchmarkCompletedJob }>()

function benchmarkJobKey(workspace: string, jobId: string): string {
	return `${workspace}::${jobId}`
}

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

/** Answer a real workspace's provider model listings from its fixture, which no eval run can
 * reach the providers for. Leaves its other API calls to the real backend. */
export function registerBenchmarkAiModels(
	workspace: string,
	aiProviders: BenchmarkWorkspaceAiProvider[]
): void {
	benchmarkWorkspaceRunnables.set(workspace, { aiProviders })
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

/** Resource type names of a benchmark workspace, as `ResourceService.listResourceTypeNames`
 * returns them. Null when the workspace is not a benchmark one. */
export function listBenchmarkResourceTypeNames(workspace: string): string[] | null {
	if (!benchmarkWorkspaces.has(workspace)) {
		return null
	}
	return syncedResourceTypes().map((rt) => rt.name)
}

/** Stands in for the resource type search: a type matches when a word of the query appears in
 * its name or description, ranked by how many do. */
export function queryBenchmarkResourceTypes(
	text: string,
	limit = 10
): { name: string; schema?: unknown; score: number }[] {
	const words = text
		.toLowerCase()
		.split(/[^a-z0-9]+/)
		.filter((word) => word.length >= 3)
	return syncedResourceTypes()
		.map((rt) => {
			const haystack = `${rt.name} ${rt.description ?? ''}`.toLowerCase()
			return { name: rt.name, schema: rt.schema, score: words.filter((w) => haystack.includes(w)).length }
		})
		.filter((rt) => rt.score > 0)
		.sort((a, b) => b.score - a.score)
		.slice(0, limit)
}

export function createBenchmarkCompletedJob(input: {
	workspace: string
	jobKind: CompletedJob['job_kind']
	success?: boolean
	result?: unknown
	logs?: string
	scriptPath?: string
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

	benchmarkJobs.set(benchmarkJobKey(input.workspace, jobId), {
		workspace: input.workspace,
		job
	})
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

export function runBenchmarkFlowPreview(input: {
	workspace: string
	requestBody?: { path?: string; args?: unknown }
}): string {
	const args = (input.requestBody?.args ?? {}) as Record<string, unknown>
	return createBenchmarkCompletedJob({
		workspace: input.workspace,
		jobKind: 'flowpreview',
		success: true,
		args,
		result: { path: input.requestBody?.path, args, mocked: true },
		logs: 'Mock benchmark flow preview completed successfully.'
	})
}

export function previewBenchmarkSchedule(input: {
	requestBody?: Record<string, unknown>
}): Record<string, unknown> {
	const schedule = input.requestBody?.schedule
	if (typeof schedule !== 'string' || schedule.trim().split(/\s+/).length !== 6) {
		throw new Error(
			`schedule must use a six-field cron expression, got ${JSON.stringify(schedule)}`
		)
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
		throw new Error(
			`HTTP trigger route_path must not start with /, got "${input.requestBody.route_path}"`
		)
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

// A stand-in Windmill hub. `search_hub_scripts` and a `hub/` read go out over
// relative `/api/...` fetches, which have no origin here, so without these the
// hub tools throw and no case can exercise hub reuse. Serving fixtures rather
// than the live hub also keeps assertions on script content stable as the real
// hub republishes new versions.

/** One integration lifted verbatim from the content repo — its shipped scripts, its
 * authored meta.json and its resource type. Hand-written fixtures make both routes to
 * an integration's conventions look equally cheap; a real one is the only way to tell
 * whether reading the metadata beats reading a script. */
const REAL_HUB_INTEGRATIONS = [
	confluenceFixture,
	servicenowFixture,
	outreachFixture
] as unknown as Array<{
	app: string
	meta: unknown
	resource_type: { name: string; description: string; schema: unknown }
	scripts: Array<{
		version_id: number
		app: string
		summary: string
		description: string
		terms: string
		kind: string
		language: string
		content: string
		schema: unknown
	}>
}>

const BENCHMARK_HUB_SCRIPTS = [
	...REAL_HUB_INTEGRATIONS.flatMap((integration) => integration.scripts),
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
	},
	// Baremetrics carries no annotation script, on purpose: it stands for an
	// integration whose conventions (an `apiKey` resource, bearer auth, the /v1
	// base) are only learnable by reading a script that does something else.
	{
		version_id: 8995,
		app: 'baremetrics',
		summary: 'List Sources',
		description: 'List all the sources attached to a Baremetrics account.',
		terms: 'baremetrics sources list revenue metrics',
		language: 'bunnative',
		content: `//native
type Baremetrics = {
  apiKey: string;
};
export async function main(resource: Baremetrics) {
  const response = await fetch("https://api.baremetrics.com/v1/sources", {
    method: "GET",
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
      Authorization: \`Bearer \${resource.apiKey}\`,
    },
  });
  if (!response.ok) {
    throw new Error(\`\${response.status} \${await response.text()}\`);
  }
  return await response.json();
}
`,
		schema: {
			type: 'object',
			required: ['resource'],
			properties: {
				resource: { type: 'object', format: 'resource-baremetrics' }
			}
		}
	},
	{
		version_id: 8996,
		app: 'baremetrics',
		summary: 'Create Customer',
		description: 'Create a customer on a Baremetrics source.',
		terms: 'baremetrics customer create source',
		language: 'bunnative',
		content: `//native
type Baremetrics = {
  apiKey: string;
};
export async function main(
  resource: Baremetrics,
  sourceId: string,
  body: { oid: string; name?: string; email?: string },
) {
  const response = await fetch(
    \`https://api.baremetrics.com/v1/\${sourceId}/customers\`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        Authorization: \`Bearer \${resource.apiKey}\`,
      },
      body: JSON.stringify(body),
    },
  );
  if (!response.ok) {
    throw new Error(\`\${response.status} \${await response.text()}\`);
  }
  return await response.json();
}
`,
		schema: {
			type: 'object',
			required: ['resource', 'sourceId', 'body'],
			properties: {
				resource: { type: 'object', format: 'resource-baremetrics' },
				sourceId: { type: 'string' },
				body: { type: 'object' }
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
function searchBenchmarkHubScripts(text: string, app: string | null, kind: string | null) {
	const tokens = new Set(
		text
			.toLowerCase()
			.split(/[^a-z0-9]+/)
			.filter((token) => token.length > 2)
	)
	// The real endpoint filters by app and kind before ranking, so honour both here —
	// otherwise a narrowed search silently returns other integrations' scripts, and a
	// trigger answers a search production would never have shown it to.
	return BENCHMARK_HUB_SCRIPTS.filter(
		(script) => (!app || script.app === app) && (!kind || (script.kind ?? 'script') === kind)
	)
		.map((script) => {
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
			description: script.description ?? null,
			app: script.app,
			kind: script.kind ?? 'script',
			score: 1 - index * 0.01
		}))
}

/** Listing an integration is unranked and description-bearing, matching the hub's
 * top-scripts endpoint — that asymmetry with the semantic search is the whole
 * reason the chat browses by app when no script matches the task. */
function listBenchmarkHubScriptsByApp(app: string | null, kind: string | null) {
	return BENCHMARK_HUB_SCRIPTS.filter(
		(script) => (!app || script.app === app) && (!kind || (script.kind ?? 'script') === kind)
	).map((script) => ({
		id: script.version_id,
		ask_id: script.version_id,
		version_id: script.version_id,
		summary: script.summary,
		description: script.description ?? null,
		app: script.app,
		kind: script.kind ?? 'script',
		views: 0,
		votes: 0
	}))
}

/** Display names of the integrations a case writes against, none of which the content repo
 * documents — the same majority shape as the live hub, where 18 of ~216 carry a meta.json.
 * Their conventions live in their shipped scripts, which is what these cases exercise. */
const BENCHMARK_HUB_INTEGRATION_NAMES: Record<string, string> = {
	baremetrics: 'Baremetrics',
	holded: 'Holded'
}

/** The resource type a mocked hub integration takes. */
function benchmarkHubIntegrationResourceType(app: string): BenchmarkWorkspaceResourceType | null {
	const real = REAL_HUB_INTEGRATIONS.find((integration) => integration.app === app)
	if (real) {
		return real.resource_type
	}
	const displayName = BENCHMARK_HUB_INTEGRATION_NAMES[app]
	if (!displayName) {
		return null
	}
	return {
		name: app,
		description: `${displayName} credentials`,
		schema: {
			type: 'object',
			required: ['apiKey'],
			properties: {
				apiKey: { type: 'string', description: `${displayName} API key` }
			}
		}
	}
}

/** What a hub-synced instance holds: common hub types, plus the one each mocked hub
 * integration takes, so the hub and the instance agree on them. */
function syncedResourceTypes(): BenchmarkWorkspaceResourceType[] {
	const integrations = [
		...REAL_HUB_INTEGRATIONS.map((integration) => integration.app),
		...Object.keys(BENCHMARK_HUB_INTEGRATION_NAMES)
	]
	return [
		...syncedResourceTypesFixture,
		...integrations.flatMap((app) => benchmarkHubIntegrationResourceType(app) ?? [])
	]
}

/** The hub keys a script by its version id; the app and slug segments that
 * follow are descriptive, so match on the id exactly as the real hub does. */
function getBenchmarkHubScript(path: string) {
	const versionId = Number(path.replace(/^\/api\/scripts\/hub\/get_full\/hub\//, '').split('/')[0])
	return BENCHMARK_HUB_SCRIPTS.find((script) => script.version_id === versionId)
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
		path === '/api/embeddings/query_hub_scripts' ||
		path === '/api/scripts/hub/top' ||
		path === '/api/integrations/hub/list' ||
		path.startsWith('/api/scripts/hub/get_full/') ||
		BENCHMARK_AI_MODELS_PATH.test(path)
	)
}

/** Answer the relative `/api/...` fetches no mocked service covers. */
export function handleBenchmarkApiFetch(url: string, init?: RequestInit): Response {
	const path = url.split('?')[0]
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
	if (path === '/api/embeddings/query_hub_scripts') {
		const params = new URLSearchParams(url.split('?')[1] ?? '')
		return Response.json(
			searchBenchmarkHubScripts(params.get('text') ?? '', params.get('app'), params.get('kind'))
		)
	}
	if (path === '/api/scripts/hub/top') {
		const params = new URLSearchParams(url.split('?')[1] ?? '')
		return Response.json({
			asks: listBenchmarkHubScriptsByApp(params.get('app'), params.get('kind'))
		})
	}
	if (path === '/api/integrations/hub/list') {
		const apps = [...new Set(BENCHMARK_HUB_SCRIPTS.map((script) => script.app))].sort()
		const documented = new Set(
			REAL_HUB_INTEGRATIONS.filter((integration) => integration.meta).map(
				(integration) => integration.app
			)
		)
		return Response.json(apps.map((name) => ({ name, documented: documented.has(name) })))
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
