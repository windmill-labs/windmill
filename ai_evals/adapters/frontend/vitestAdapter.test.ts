import { expect, it, vi } from 'vitest'
// @ts-ignore - Node.js fs/promises
import { mkdir, writeFile } from 'fs/promises'
// @ts-ignore - Node.js path
import { dirname, resolve } from 'path'

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

vi.mock('$lib/gen', async () => {
	const actual = await vi.importActual<any>('$lib/gen')
	const {
		getBenchmarkCompletedJob,
		getBenchmarkFlowByPath,
		getBenchmarkScriptByHash,
		getBenchmarkScriptByPath,
		hasBenchmarkWorkspace,
		listBenchmarkFlows,
		listBenchmarkScripts,
		createBenchmarkHttpTrigger,
		createBenchmarkSchedule,
		previewBenchmarkSchedule,
		runBenchmarkFlowByPath,
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
						throw new Error(`Script "${data.path}" not found in benchmark workspace`)
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
						throw new Error(`Flow "${data.path}" not found in benchmark workspace`)
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
					const job = getBenchmarkCompletedJob(data.workspace, data.id)
					if (!job) {
						throw new Error(`Job "${data.id}" not found in benchmark workspace`)
					}
					return job
				}
				return actual.JobService.getJob(data)
			}
		}),
		ScheduleService: wrapService(actual.ScheduleService, {
			previewSchedule: async (data: { requestBody?: Record<string, unknown> }) =>
				previewBenchmarkSchedule(data),
			createSchedule: async (data: { workspace: string; requestBody: Record<string, unknown> }) =>
				hasBenchmarkWorkspace(data.workspace)
					? createBenchmarkSchedule(data)
					: actual.ScheduleService.createSchedule(data)
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
	600_000
)
