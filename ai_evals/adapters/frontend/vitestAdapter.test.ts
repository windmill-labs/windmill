import { expect, it, vi } from 'vitest'
// @ts-ignore - Node.js fs/promises
import { mkdir, readFile, writeFile } from 'fs/promises'
// @ts-ignore - Node.js path
import { dirname, resolve } from 'path'
// @ts-ignore - Node.js url
import { fileURLToPath } from 'url'
import { handleBenchmarkApiFetch, hasBenchmarkApiHandler } from './mockBackend'
import { resolveWindmillBackendSettings } from '../../core/windmillBackendSettings'
import { WindmillBackendClient } from './windmillBackend'
import { realWorkspaceRoute } from './core/global/realWorkspace'

const FRONTEND_DIR = fileURLToPath(new URL('../../../frontend/', import.meta.url))

// Some tools reach the backend by relative fetch('/api/...'), which has no meaning
// in the vitest environment — serve the ones the benchmark handles.
// Every other relative fetch keeps its normal behavior (it fails the same way
// it does without this stub) so unrelated tools see an unchanged environment.
// The frontend builds API URLs from location.origin (fetchAvailableModels does), and node has no
// location — without one those calls throw before the stub below ever sees them.
if (typeof (globalThis as { location?: unknown }).location === 'undefined') {
	Object.defineProperty(globalThis, 'location', {
		value: new URL('http://benchmark.local/'),
		configurable: true
	})
}

const ORIGINAL_FETCH = globalThis.fetch
globalThis.fetch = (async (input: unknown, init?: RequestInit) => {
	const url = typeof input === 'string' ? input : ((input as Request | URL | null)?.url ?? '')
	if (typeof url === 'string' && hasBenchmarkApiHandler(url)) {
		return handleBenchmarkApiFetch(url, init)
	}
	// A global-mode case runs against a real workspace: send its calls there, as its user.
	const real = typeof url === 'string' ? realWorkspaceRoute(url) : undefined
	if (real) {
		const headers = new Headers(init?.headers)
		headers.set('Authorization', `Bearer ${real.token}`)
		return ORIGINAL_FETCH(real.url, { ...init, headers })
	}
	// The docs tools read the documentation bundled into the backend: hand them to the
	// benchmark's real backend, logged in like the AI proxy calls.
	if (typeof url === 'string' && url.startsWith('/api/docs/')) {
		return new WindmillBackendClient(resolveWindmillBackendSettings()).request(url.slice('/api'.length), init)
	}
	// The parsers behind inferArgs load their wasm from a vite `?url` path, which only a dev
	// server serves. Unserved, every script schema infers as empty, and the tools then tell the
	// model its arguments are undeclared, so it rewrites a correct script until it runs out of turns.
	const wasmPath = typeof url === 'string' ? wasmFilePath(url) : undefined
	if (wasmPath) {
		return new Response(await readFile(wasmPath), {
			headers: { 'Content-Type': 'application/wasm' }
		})
	}
	return ORIGINAL_FETCH(input as Parameters<typeof fetch>[0], init)
}) as typeof fetch

function wasmFilePath(url: string): string | undefined {
	const raw = url.split(/[?#]/)[0]
	if (!raw.endsWith('.wasm')) return undefined
	const pathname = decodeURIComponent(raw)
	// Located past whatever base path the config prefixes.
	const fs = pathname.indexOf('/@fs/')
	if (fs !== -1) return pathname.slice(fs + '/@fs'.length)
	const modules = pathname.indexOf('/node_modules/')
	if (modules !== -1) return resolve(FRONTEND_DIR, `.${pathname.slice(modules)}`)
	return undefined
}

vi.mock('monaco-editor', () => ({
	editor: {},
	languages: {},
	KeyCode: {},
	Uri: {
		parse: (value: string) => ({ toString: () => value })
	},
	MarkerSeverity: {
		Error: 8,
		Warning: 4,
		Info: 2,
		Hint: 1
	}
}))

vi.mock('@codingame/monaco-vscode-standalone-typescript-language-features', () => ({
	getTypeScriptWorker: async () => async () => ({}),
	typescriptVersion: 'test'
}))

vi.mock('@codingame/monaco-vscode-languages-service-override', () => ({
	default: () => ({})
}))

vi.mock('$lib/components/vscode', () => ({}))

// get_db_schema reads a database's tables by running a query job; a benchmark database
// answers with the tables its fixture seeds instead.
vi.mock('$lib/components/apps/components/display/dbtable/metadata', async () => {
	const actual = await vi.importActual<any>(
		'$lib/components/apps/components/display/dbtable/metadata'
	)
	const { getBenchmarkDbSchemaSeed } = await import('./mockBackend')
	const { stringifySchema } = await import('$lib/components/copilot/lib')
	const { resourceTypeToLang } = await import(
		'$lib/components/apps/components/display/dbtable/utils'
	)
	return {
		...actual,
		getDbSchemas: async (
			resourceType: string,
			resourcePath: string,
			workspace: string | undefined,
			...rest: unknown[]
		) => {
			const seed = workspace ? getBenchmarkDbSchemaSeed(workspace, resourcePath) : null
			if (!seed) return actual.getDbSchemas(resourceType, resourcePath, workspace, ...rest)
			const dbSchema = {
				lang: resourceTypeToLang(seed.resourceType),
				schema: seed.schema,
				publicOnly: !!seed.schema.public
			}
			return { ...dbSchema, stringified: stringifySchema(dbSchema) }
		}
	}
})

vi.mock('$lib/gen', async () => {
	const actual = await vi.importActual<any>('$lib/gen')
	const {
		getBenchmarkCompletedJob,
		getBenchmarkFlowByPath,
		getBenchmarkScriptByHash,
		getBenchmarkScriptByPath,
		hasBenchmarkWorkspace,
		listBenchmarkResourceTypeNames,
		queryBenchmarkResourceTypes,
		listBenchmarkFlows,
		listBenchmarkScripts,
		createBenchmarkHttpTrigger,
		createBenchmarkSchedule,
		previewBenchmarkSchedule,
		runBenchmarkFlowByPath,
		runBenchmarkFlowPreview,
		runBenchmarkScriptPreview
	} = await import('./mockBackend')

	function wrapService<T extends object>(target: T, overrides: Record<string, unknown>): T {
		return new Proxy(target, {
			get(source, property, receiver) {
				if (typeof property === 'string' && property in overrides) {
					return overrides[property]
				}
				return Reflect.get(source, property, receiver)
			}
		})
	}

	return {
		...actual,
		ScriptService: wrapService(actual.ScriptService, {
			listScripts: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? (listBenchmarkScripts(data.workspace) ?? [])
					: actual.ScriptService.listScripts(data),
			getScriptByPath: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					const script = getBenchmarkScriptByPath(data.workspace, data.path)
					if (!script) {
						throw Object.assign(
							new Error(`Script "${data.path}" not found in benchmark workspace`),
							{ status: 404 }
						)
					}
					return script
				}
				return actual.ScriptService.getScriptByPath(data)
			},
			getScriptByHash: async (data: { workspace: string; hash: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					const script = getBenchmarkScriptByHash(data.workspace, data.hash)
					if (!script) {
						throw new Error(`Script hash "${data.hash}" not found in benchmark workspace`)
					}
					return script
				}
				return actual.ScriptService.getScriptByHash(data)
			}
		}),
		FlowService: wrapService(actual.FlowService, {
			listFlows: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? (listBenchmarkFlows(data.workspace) ?? [])
					: actual.FlowService.listFlows(data),
			getFlowByPath: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					const flow = getBenchmarkFlowByPath(data.workspace, data.path)
					if (!flow) {
						throw Object.assign(
							new Error(`Flow "${data.path}" not found in benchmark workspace`),
							{ status: 404 }
						)
					}
					return flow
				}
				return actual.FlowService.getFlowByPath(data)
			}
		}),
		JobService: wrapService(actual.JobService, {
			runScriptPreview: async (data: {
				workspace: string
				requestBody?: {
					content?: string
					language?: string
					args?: Record<string, unknown>
					path?: string
				}
			}) =>
				hasBenchmarkWorkspace(data.workspace)
					? runBenchmarkScriptPreview({
							workspace: data.workspace,
							requestBody: data.requestBody ?? {}
						})
					: actual.JobService.runScriptPreview(data),
			runFlowPreview: async (data: {
				workspace: string
				requestBody?: { path?: string; args?: unknown }
			}) =>
				hasBenchmarkWorkspace(data.workspace)
					? runBenchmarkFlowPreview(data)
					: actual.JobService.runFlowPreview(data as any),
			runFlowByPath: async (data: {
				workspace: string
				path: string
				requestBody?: Record<string, unknown>
			}) =>
				hasBenchmarkWorkspace(data.workspace)
					? runBenchmarkFlowByPath({
							workspace: data.workspace,
							path: data.path,
							args: data.requestBody
						})
					: actual.JobService.runFlowByPath(data),
			getJob: async (data: { workspace: string; id: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					const job = getBenchmarkJob(data.workspace, data.id)
					if (!job) {
						throw new Error(`Job "${data.id}" not found in benchmark workspace`)
					}
					return job
				}
				return actual.JobService.getJob(data)
			}
		}),
		// A benchmark workspace holds no resources and no AI settings: answer as an empty
		// workspace would, rather than send its temp-directory id to the real backend.
		WorkspaceService: wrapService(actual.WorkspaceService, {
			getWorkspaceSettingsDocument: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? getBenchmarkWorkspaceSettingsDocument(data.workspace)
					: actual.WorkspaceService.getWorkspaceSettingsDocument(data),
			applyWorkspaceSettingsDocument: async (data: any) =>
				hasBenchmarkWorkspace(data.workspace)
					? applyBenchmarkWorkspaceSettingsDocument(data)
					: actual.WorkspaceService.applyWorkspaceSettingsDocument(data),
			getCopilotInfo: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? {} : actual.WorkspaceService.getCopilotInfo(data)
		}),
		WorkerService: wrapService(actual.WorkerService, {
			listWorkers: async (data: Record<string, unknown>) =>
				listBenchmarkWorkers() ?? actual.WorkerService.listWorkers(data)
		}),
		ScheduleService: wrapService(actual.ScheduleService, {
			previewSchedule: async (data: { requestBody?: Record<string, unknown> }) =>
				previewBenchmarkSchedule(data),
			createSchedule: async (data: { workspace: string; requestBody: Record<string, unknown> }) =>
				hasBenchmarkWorkspace(data.workspace)
					? createBenchmarkSchedule(data)
					: actual.ScheduleService.createSchedule(data)
		}),
		ResourceService: wrapService(actual.ResourceService, {
			listResource: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? [] : actual.ResourceService.listResource(data),
			getResource: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					throw new Error(`Resource "${data.path}" not found in benchmark workspace`)
				}
				return actual.ResourceService.getResource(data)
			},
			queryResourceTypes: async (data: { workspace: string; text: string; limit?: number }) =>
				hasBenchmarkWorkspace(data.workspace)
					? queryBenchmarkResourceTypes(data.text, data.limit)
					: actual.ResourceService.queryResourceTypes(data),
			listResourceTypeNames: async (data: { workspace: string }) =>
				listBenchmarkResourceTypeNames(data.workspace) ??
				actual.ResourceService.listResourceTypeNames(data)
		}),
		HttpTriggerService: wrapService(actual.HttpTriggerService, {
			createHttpTrigger: async (data: { workspace: string; requestBody: Record<string, unknown> }) =>
				hasBenchmarkWorkspace(data.workspace)
					? createBenchmarkHttpTrigger(data)
					: actual.HttpTriggerService.createHttpTrigger(data)
		})
	}
})

const benchmarkOutputPath = process.env.WMILL_FRONTEND_AI_EVAL_OUTPUT_PATH
const benchmarkIt = benchmarkOutputPath ? it : it.skip

benchmarkIt(
	'runs the frontend benchmark adapter from environment input',
	async () => {
		const { resetBenchmarkMockBackend } = await import('./mockBackend')
		resetBenchmarkMockBackend()
		// The tools swallow inference failures, so a wasm the fetch stub stops serving would
		// only show up as empty schemas the model chases, never as an error.
		const { inferArgs } = await import('$lib/infer')
		const { emptySchema } = await import('$lib/utils')
		const probe = emptySchema()
		await inferArgs('bun', 'export async function main(name: string) {}', probe)
		expect(Object.keys(probe.properties)).toEqual(['name'])
		const { runFrontendBenchmarkFromEnv } = await import('./benchmarkRunner')
		try {
			const payload = await runFrontendBenchmarkFromEnv()
			const absoluteOutputPath = resolve(benchmarkOutputPath!)
			await mkdir(dirname(absoluteOutputPath), { recursive: true })
			await writeFile(absoluteOutputPath, JSON.stringify(payload, null, 2) + '\n', 'utf8')

			expect(payload.cases.length).toBeGreaterThan(0)
		} finally {
			resetBenchmarkMockBackend()
		}
	},
	// A full suite on a slow reasoning model runs for hours (gpt-6-astra on global went past
	// 2h); the CI job's own timeout is the real bound.
	18_000_000
)
