import { expect, it, vi } from 'vitest'
// @ts-ignore - Node.js fs/promises
import { mkdir, writeFile } from 'fs/promises'
// @ts-ignore - Node.js path
import { dirname, resolve } from 'path'
import { handleBenchmarkApiFetch } from './mockBackend'

// The API catalog executor issues relative fetch('/api/...') calls, which have
// no meaning in the vitest environment — serve them from the benchmark handler.
const ORIGINAL_FETCH = globalThis.fetch
globalThis.fetch = (async (input: unknown, init?: RequestInit) => {
	const url = typeof input === 'string' ? input : ((input as Request | URL | null)?.url ?? '')
	if (typeof url === 'string' && url.startsWith('/api/')) {
		return handleBenchmarkApiFetch(url)
	}
	return ORIGINAL_FETCH(input as Parameters<typeof fetch>[0], init)
}) as typeof fetch

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
		getBenchmarkAppByPath,
		getBenchmarkCompletedJob,
		getBenchmarkCompletedJobResultMaybe,
		getBenchmarkDatatableSchema,
		getBenchmarkDraftForUser,
		getBenchmarkFlowByPath,
		getBenchmarkJobLogs,
		getBenchmarkOwnDraft,
		getBenchmarkScriptByHash,
		getBenchmarkScriptByPath,
		hasBenchmarkWorkspace,
		listBenchmarkApps,
		listBenchmarkDatatables,
		listBenchmarkDrafts,
		listBenchmarkFlows,
		listBenchmarkJobs,
		listBenchmarkScripts,
		createBenchmarkFolder,
		createBenchmarkHttpTrigger,
		createBenchmarkSchedule,
		previewBenchmarkSchedule,
		runBenchmarkDatatableSql,
		runBenchmarkFlowByPath,
		runBenchmarkScriptPreview,
		updateBenchmarkDraft,
		listBenchmarkMcpTools
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
		DraftService: wrapService(actual.DraftService, {
			updateDraft: async (data: {
				workspace: string
				kind: any
				path: string
				requestBody?: { value?: unknown }
			}) =>
				hasBenchmarkWorkspace(data.workspace)
					? updateBenchmarkDraft(data)
					: actual.DraftService.updateDraft(data),
			getDraftForUser: async (data: { workspace: string; kind: any; path: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? getBenchmarkDraftForUser(data)
					: actual.DraftService.getDraftForUser(data),
			getOwnDraft: async (data: { workspace: string; kind: any; path: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? getBenchmarkOwnDraft(data)
					: actual.DraftService.getOwnDraft(data),
			listDrafts: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? listBenchmarkDrafts(data.workspace)
					: actual.DraftService.listDrafts(data)
		}),
		FolderService: wrapService(actual.FolderService, {
			createFolder: async (data: { workspace: string; requestBody: { name: string } }) =>
				hasBenchmarkWorkspace(data.workspace)
					? createBenchmarkFolder(data.workspace, data.requestBody.name)
					: actual.FolderService.createFolder(data)
		}),
		ScriptService: wrapService(actual.ScriptService, {
			listScripts: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? (listBenchmarkScripts(data.workspace) ?? [])
					: actual.ScriptService.listScripts(data),
			existsScriptByPath: async (data: { workspace: string; path: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? Boolean(getBenchmarkScriptByPath(data.workspace, data.path))
					: actual.ScriptService.existsScriptByPath(data),
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
			getScriptByPathWithDraft: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					const script = getBenchmarkScriptByPath(data.workspace, data.path)
					if (!script) {
						throw new Error(`Script "${data.path}" not found in benchmark workspace`)
					}
					return script
				}
				return actual.ScriptService.getScriptByPathWithDraft(data)
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
			existsFlowByPath: async (data: { workspace: string; path: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? Boolean(getBenchmarkFlowByPath(data.workspace, data.path))
					: actual.FlowService.existsFlowByPath(data),
			getFlowByPath: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					const flow = getBenchmarkFlowByPath(data.workspace, data.path)
					if (!flow) {
						throw new Error(`Flow "${data.path}" not found in benchmark workspace`)
					}
					return flow
				}
				return actual.FlowService.getFlowByPath(data)
			},
			getFlowByPathWithDraft: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					const flow = getBenchmarkFlowByPath(data.workspace, data.path)
					if (!flow) {
						throw new Error(`Flow "${data.path}" not found in benchmark workspace`)
					}
					return flow
				}
				return actual.FlowService.getFlowByPathWithDraft(data)
			},
			getFlowLatestVersion: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					const flow = getBenchmarkFlowByPath(data.workspace, data.path)
					if (!flow) {
						throw new Error(`Flow "${data.path}" not found in benchmark workspace`)
					}
					return { id: 1 }
				}
				return actual.FlowService.getFlowLatestVersion(data)
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
			}) => {
				if (!hasBenchmarkWorkspace(data.workspace)) {
					return actual.JobService.runScriptPreview(data)
				}
				const requestBody = data.requestBody ?? {}
				const database = requestBody.args?.database
				// Datatable SQL runs as a `postgresql` preview against `datatable://<name>`.
				// Execute it through the canned-SQL mock instead of linting it as a script.
				if (
					requestBody.language === 'postgresql' &&
					typeof database === 'string' &&
					database.startsWith('datatable://')
				) {
					return runBenchmarkDatatableSql({
						workspace: data.workspace,
						datatableName: database.slice('datatable://'.length),
						sql: requestBody.content ?? ''
					})
				}
				return runBenchmarkScriptPreview({ workspace: data.workspace, requestBody })
			},
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
			},
			getCompletedJobResultMaybe: async (data: { workspace: string; id: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? getBenchmarkCompletedJobResultMaybe({ workspace: data.workspace, id: data.id })
					: actual.JobService.getCompletedJobResultMaybe(data),
			listJobs: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? (listBenchmarkJobs(data.workspace) ?? [])
					: actual.JobService.listJobs(data),
			getJobLogs: async (data: { workspace: string; id: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? getBenchmarkJobLogs(data.workspace, data.id)
					: actual.JobService.getJobLogs(data)
		}),
		WorkspaceService: wrapService(actual.WorkspaceService, {
			listDataTableTables: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? (listBenchmarkDatatables(data.workspace) ?? [])
					: actual.WorkspaceService.listDataTableTables(data),
			getDataTableTableSchema: async (data: {
				workspace: string
				datatableName: string
				schemaName: string
				tableName: string
			}) =>
				hasBenchmarkWorkspace(data.workspace)
					? getBenchmarkDatatableSchema({
							workspace: data.workspace,
							datatableName: data.datatableName,
							schemaName: data.schemaName,
							tableName: data.tableName
						})
					: actual.WorkspaceService.getDataTableTableSchema(data)
		}),
		ScheduleService: wrapService(actual.ScheduleService, {
			existsSchedule: async (data: { workspace: string; path: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? false : actual.ScheduleService.existsSchedule(data),
			listSchedules: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? [] : actual.ScheduleService.listSchedules(data),
			getSchedule: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					throw new Error(`Schedule "${data.path}" not found in benchmark workspace`)
				}
				return actual.ScheduleService.getSchedule(data)
			},
			previewSchedule: async (data: { requestBody?: Record<string, unknown> }) =>
				previewBenchmarkSchedule(data),
			createSchedule: async (data: { workspace: string; requestBody: Record<string, unknown> }) =>
				hasBenchmarkWorkspace(data.workspace)
					? createBenchmarkSchedule(data)
					: actual.ScheduleService.createSchedule(data)
		}),
		ResourceService: wrapService(actual.ResourceService, {
			existsResource: async (data: { workspace: string; path: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? false : actual.ResourceService.existsResource(data),
			listResource: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? [] : actual.ResourceService.listResource(data),
			getResource: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					throw new Error(`Resource "${data.path}" not found in benchmark workspace`)
				}
				return actual.ResourceService.getResource(data)
			},
			queryResourceTypes: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? [] : actual.ResourceService.queryResourceTypes(data)
		}),
		McpService: wrapService(actual.McpService, {
			listMcpTools: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? listBenchmarkMcpTools()
					: actual.McpService.listMcpTools(data)
		}),
		VariableService: wrapService(actual.VariableService, {
			existsVariable: async (data: { workspace: string; path: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? false : actual.VariableService.existsVariable(data),
			listVariable: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? [] : actual.VariableService.listVariable(data),
			getVariable: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					throw new Error(`Variable "${data.path}" not found in benchmark workspace`)
				}
				return actual.VariableService.getVariable(data)
			}
		}),
		AppService: wrapService(actual.AppService, {
			existsApp: async (data: { workspace: string; path: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? Boolean(getBenchmarkAppByPath(data.workspace, data.path))
					: actual.AppService.existsApp(data),
			listApps: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? (listBenchmarkApps(data.workspace) ?? [])
					: actual.AppService.listApps(data),
			getAppByPath: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					const app = getBenchmarkAppByPath(data.workspace, data.path)
					if (!app) {
						throw new Error(`App "${data.path}" not found in benchmark workspace`)
					}
					return app
				}
				return actual.AppService.getAppByPath(data)
			}
		}),
		HttpTriggerService: wrapService(actual.HttpTriggerService, {
			existsHttpTrigger: async (data: { workspace: string; path: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? false : actual.HttpTriggerService.existsHttpTrigger(data),
			listHttpTriggers: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? [] : actual.HttpTriggerService.listHttpTriggers(data),
			getHttpTrigger: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					throw new Error(`HTTP trigger "${data.path}" not found in benchmark workspace`)
				}
				return actual.HttpTriggerService.getHttpTrigger(data)
			},
			createHttpTrigger: async (data: { workspace: string; requestBody: Record<string, unknown> }) =>
				hasBenchmarkWorkspace(data.workspace)
					? createBenchmarkHttpTrigger(data)
					: actual.HttpTriggerService.createHttpTrigger(data)
		}),
		WebsocketTriggerService: wrapService(actual.WebsocketTriggerService, {
			existsWebsocketTrigger: async (data: { workspace: string; path: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? false
					: actual.WebsocketTriggerService.existsWebsocketTrigger(data),
			listWebsocketTriggers: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? []
					: actual.WebsocketTriggerService.listWebsocketTriggers(data),
			getWebsocketTrigger: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					throw new Error(`Websocket trigger "${data.path}" not found in benchmark workspace`)
				}
				return actual.WebsocketTriggerService.getWebsocketTrigger(data)
			}
		}),
		KafkaTriggerService: wrapService(actual.KafkaTriggerService, {
			existsKafkaTrigger: async (data: { workspace: string; path: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? false
					: actual.KafkaTriggerService.existsKafkaTrigger(data),
			listKafkaTriggers: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? [] : actual.KafkaTriggerService.listKafkaTriggers(data),
			getKafkaTrigger: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					throw new Error(`Kafka trigger "${data.path}" not found in benchmark workspace`)
				}
				return actual.KafkaTriggerService.getKafkaTrigger(data)
			}
		}),
		NatsTriggerService: wrapService(actual.NatsTriggerService, {
			existsNatsTrigger: async (data: { workspace: string; path: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? false : actual.NatsTriggerService.existsNatsTrigger(data),
			listNatsTriggers: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? [] : actual.NatsTriggerService.listNatsTriggers(data),
			getNatsTrigger: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					throw new Error(`NATS trigger "${data.path}" not found in benchmark workspace`)
				}
				return actual.NatsTriggerService.getNatsTrigger(data)
			}
		}),
		PostgresTriggerService: wrapService(actual.PostgresTriggerService, {
			existsPostgresTrigger: async (data: { workspace: string; path: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? false
					: actual.PostgresTriggerService.existsPostgresTrigger(data),
			listPostgresTriggers: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? []
					: actual.PostgresTriggerService.listPostgresTriggers(data),
			getPostgresTrigger: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					throw new Error(`Postgres trigger "${data.path}" not found in benchmark workspace`)
				}
				return actual.PostgresTriggerService.getPostgresTrigger(data)
			}
		}),
		MqttTriggerService: wrapService(actual.MqttTriggerService, {
			existsMqttTrigger: async (data: { workspace: string; path: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? false : actual.MqttTriggerService.existsMqttTrigger(data),
			listMqttTriggers: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? [] : actual.MqttTriggerService.listMqttTriggers(data),
			getMqttTrigger: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					throw new Error(`MQTT trigger "${data.path}" not found in benchmark workspace`)
				}
				return actual.MqttTriggerService.getMqttTrigger(data)
			}
		}),
		SqsTriggerService: wrapService(actual.SqsTriggerService, {
			existsSqsTrigger: async (data: { workspace: string; path: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? false : actual.SqsTriggerService.existsSqsTrigger(data),
			listSqsTriggers: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? [] : actual.SqsTriggerService.listSqsTriggers(data),
			getSqsTrigger: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					throw new Error(`SQS trigger "${data.path}" not found in benchmark workspace`)
				}
				return actual.SqsTriggerService.getSqsTrigger(data)
			}
		}),
		GcpTriggerService: wrapService(actual.GcpTriggerService, {
			existsGcpTrigger: async (data: { workspace: string; path: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? false : actual.GcpTriggerService.existsGcpTrigger(data),
			listGcpTriggers: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? [] : actual.GcpTriggerService.listGcpTriggers(data),
			getGcpTrigger: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					throw new Error(`GCP trigger "${data.path}" not found in benchmark workspace`)
				}
				return actual.GcpTriggerService.getGcpTrigger(data)
			}
		}),
		AzureTriggerService: wrapService(actual.AzureTriggerService, {
			existsAzureTrigger: async (data: { workspace: string; path: string }) =>
				hasBenchmarkWorkspace(data.workspace)
					? false
					: actual.AzureTriggerService.existsAzureTrigger(data),
			listAzureTriggers: async (data: { workspace: string }) =>
				hasBenchmarkWorkspace(data.workspace) ? [] : actual.AzureTriggerService.listAzureTriggers(data),
			getAzureTrigger: async (data: { workspace: string; path: string }) => {
				if (hasBenchmarkWorkspace(data.workspace)) {
					throw new Error(`Azure trigger "${data.path}" not found in benchmark workspace`)
				}
				return actual.AzureTriggerService.getAzureTrigger(data)
			}
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
	// Full-suite runs (30+ cases at concurrency 2-3) routinely exceed 10 minutes.
	7_200_000
)
