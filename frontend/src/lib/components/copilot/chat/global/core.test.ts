import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

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

// In-memory stand-in for the per-user draft backend. The chat now persists/reads
// drafts through DraftService (no in-tab cell in unit tests), so this Map is the
// source of truth the write/read tools round-trip against. `vi.hoisted` makes it
// available inside the hoisted `vi.mock` factory and the test body alike.
const { backendDrafts, serverTimestamps, failingWrites, failingReads } = vi.hoisted(() => ({
	backendDrafts: new Map<string, unknown>(),
	// Per-row server timestamp, only set by tests that want to simulate a
	// concurrent writer advancing the row; otherwise empty, so the conflict
	// branch in `updateDraft` stays inert for every pre-existing test.
	serverTimestamps: new Map<string, string>(),
	// Keys whose `updateDraft` / `getDraftForUser` throw a non-404 (network/5xx);
	// only set by the error-handling tests, empty otherwise.
	failingWrites: new Set<string>(),
	failingReads: new Set<string>()
}))

vi.mock('$lib/gen', async () => {
	const actual = await vi.importActual<any>('$lib/gen')

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
			existsScriptByPath: vi.fn(async () => false),
			createScript: vi.fn(async () => 'created'),
			getScriptByPath: vi.fn(async () => {
				throw new Error('getScriptByPath mock not configured')
			}),
			queryHubScripts: vi.fn(async () => []),
			getHubScriptContentByPath: vi.fn(async () => ''),
			listScripts: vi.fn(async () => [])
		}),
		JobService: wrapService(actual.JobService, {
			runScriptPreview: vi.fn(async () => 'job-script-preview'),
			runFlowPreview: vi.fn(async () => 'job-flow-preview'),
			runFlowByPath: vi.fn(async () => 'job-flow-by-path'),
			getJob: vi.fn(async () => ({
				type: 'CompletedJob',
				success: true,
				result: { ok: true },
				logs: 'test logs'
			})),
			getJobLogs: vi.fn(async () => 'job log line 1\njob log line 2'),
			listJobs: vi.fn(async () => [
				{
					type: 'CompletedJob',
					id: 'completed-1',
					job_kind: 'script',
					script_path: 'f/team/runner',
					created_by: 'alice',
					created_at: '2026-06-09T10:00:00Z',
					started_at: '2026-06-09T10:00:01Z',
					duration_ms: 1200,
					success: true,
					canceled: false,
					is_flow_step: false,
					tag: 'default',
					// fields that must be stripped from the summary
					logs: 'verbose logs',
					args: { secret: 'do-not-leak' },
					result: { value: 42 }
				},
				{
					type: 'QueuedJob',
					id: 'queued-1',
					job_kind: 'flow',
					script_path: 'f/team/pipeline',
					created_by: 'bob',
					created_at: '2026-06-09T10:05:00Z',
					running: true,
					canceled: false,
					is_flow_step: false,
					tag: 'default'
				}
			])
		}),
		FlowService: wrapService(actual.FlowService, {
			existsFlowByPath: vi.fn(async () => false),
			createFlow: vi.fn(async () => 'created'),
			updateFlow: vi.fn(async () => 'updated'),
			getFlowByPath: vi.fn(async () => {
				throw new Error('getFlowByPath mock not configured')
			}),
			getFlowLatestVersion: vi.fn(async () => ({ id: 1 })),
			listFlows: vi.fn(async () => [])
		}),
		ScheduleService: wrapService(actual.ScheduleService, {
			existsSchedule: vi.fn(async () => false),
			getSchedule: vi.fn(async () => {
				throw new Error('getSchedule mock not configured')
			})
		}),
		HttpTriggerService: wrapService(actual.HttpTriggerService, {
			existsHttpTrigger: vi.fn(async () => false),
			getHttpTrigger: vi.fn(async () => {
				throw new Error('getHttpTrigger mock not configured')
			})
		}),
		AppService: wrapService(actual.AppService, {
			existsApp: vi.fn(async () => false),
			createAppRaw: vi.fn(async () => 'created'),
			updateAppRaw: vi.fn(async () => 'updated'),
			getAppByPath: vi.fn(async () => {
				throw new Error('getAppByPath mock not configured')
			}),
			listApps: vi.fn(async () => [])
		}),
		ResourceService: wrapService(actual.ResourceService, {
			existsResource: vi.fn(async () => false),
			getResource: vi.fn(async () => {
				throw new Error('getResource mock not configured')
			})
		}),
		VariableService: wrapService(actual.VariableService, {
			existsVariable: vi.fn(async () => false),
			getVariable: vi.fn(async () => {
				throw new Error('getVariable mock not configured')
			}),
			createVariable: vi.fn(async () => 'created'),
			updateVariable: vi.fn(async () => 'updated')
		}),
		DraftService: wrapService(actual.DraftService, {
			updateDraft: vi.fn(async ({ kind, path, requestBody }: any) => {
				const key = `${kind}:${path}`
				if (failingWrites.has(key)) throw Object.assign(new Error('server error'), { status: 500 })
				// A non-force save whose last_sync no longer matches the row's
				// server timestamp is rejected (optimistic concurrency). Inert
				// unless a test set serverTimestamps for this key.
				const serverTs = serverTimestamps.get(key)
				if (
					!requestBody?.force &&
					requestBody?.last_sync != null &&
					serverTs != null &&
					requestBody.last_sync !== serverTs
				) {
					return { status: 'conflict', current_timestamp: serverTs }
				}
				if (requestBody?.value == null) backendDrafts.delete(key)
				else backendDrafts.set(key, requestBody.value)
				return { status: 'saved', current_timestamp: '2026-06-15T00:00:00Z' }
			}),
			getDraftForUser: vi.fn(async ({ kind, path }: any) => {
				const key = `${kind}:${path}`
				if (failingReads.has(key)) throw Object.assign(new Error('server error'), { status: 500 })
				// 404-shaped (status) like the real ApiError, so the adapter's
				// narrowed catch treats it as "no draft" rather than re-throwing.
				if (!backendDrafts.has(key))
					throw Object.assign(new Error('no draft for that owner at that path'), { status: 404 })
				return { value: backendDrafts.get(key), created_at: '2026-06-15T00:00:00Z' }
			}),
			listDrafts: vi.fn(async () =>
				Array.from(backendDrafts.entries()).map(([key, value]) => {
					const idx = key.indexOf(':')
					return {
						kind: key.slice(0, idx),
						path: key.slice(idx + 1),
						summary: (value as any)?.summary,
						draft_only: true,
						created_at: '2026-06-15T00:00:00Z'
					}
				})
			)
		})
	}
})

vi.mock('./rawAppBundlerBridge', () => ({
	bundleRawAppDraft: vi.fn(async () => ({
		js: 'bundled js',
		css: 'bundled css'
	}))
}))

import {
	globalTools,
	globalToolsFor,
	prepareGlobalSystemMessage,
	prepareGlobalUserMessage,
	setDeployedInSessionHandler,
	setGetPreviewStatusHandler,
	setGetRuntimeLogsHandler,
	setListAppRunsHandler,
	setOpenPreviewHandler
} from './core'
import { UserDraft, __resetUserDraftForTesting } from '$lib/userDraft.svelte'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import {
	clearGlobalDrafts,
	deleteGlobalDraft,
	persistGlobalDraft,
	readGlobalDraftValue,
	saveGlobalAppDraft
} from './userDraftAdapter'
import { bundleRawAppDraft } from './rawAppBundlerBridge'
import {
	AppService,
	FlowService,
	HttpTriggerService,
	JobService,
	ResourceService,
	ScheduleService,
	ScriptService,
	VariableService
} from '$lib/gen'
import type { Tool, ToolCallbacks } from '../shared'

const WORKSPACE = 'global-core-test'

// Seed/read the backend draft store directly (keyed exactly like the syncer:
// `${itemKind}:${storagePath}`). Drop-in replacements for the old in-tab
// `UserDraft.save`/`UserDraft.get` round-trip the tests used before the drafts
// moved to the backend. Extra opts arg is ignored (kept for call-site parity).
function seedBackendDraft(kind: string, path: string, value: unknown, _opts?: unknown): void {
	backendDrafts.set(`${kind}:${path}`, value)
}
function getBackendDraft<V = any>(kind: string, path: string, _opts?: unknown): V | undefined {
	return backendDrafts.get(`${kind}:${path}`) as V | undefined
}

const toolCallbacks: ToolCallbacks = {
	setToolStatus: vi.fn(),
	removeToolStatus: vi.fn()
}

function getGlobalTool(name: string): Tool<{}> {
	const tool = globalTools.find((candidate) => candidate.def.function.name === name)
	if (!tool) {
		throw new Error(`Missing global tool "${name}"`)
	}
	return tool
}

async function callGlobalTool(
	name: string,
	args: Record<string, unknown>,
	callbacks: ToolCallbacks = toolCallbacks,
	helpers: Record<string, unknown> = {}
): Promise<string> {
	return getGlobalTool(name).fn({
		args,
		workspace: WORKSPACE,
		helpers,
		toolCallbacks: callbacks,
		toolId: `test-${name}`
	})
}

function localStorageSnapshot(): string {
	const values: string[] = []
	for (let i = 0; i < localStorage.length; i++) {
		const key = localStorage.key(i)
		if (key) values.push(`${key}: ${localStorage.getItem(key)}`)
	}
	return values.join('\n')
}

async function withCompletedTestJob<T>(run: () => Promise<T>): Promise<T> {
	vi.useFakeTimers()
	try {
		const promise = run()
		await vi.advanceTimersByTimeAsync(1000)
		return await promise
	} finally {
		vi.useRealTimers()
	}
}

describe('global AI tools', () => {
	beforeEach(() => {
		__resetUserDraftForTesting()
		localStorage.clear()
		backendDrafts.clear()
		serverTimestamps.clear()
		failingWrites.clear()
		failingReads.clear()
		clearGlobalDrafts(WORKSPACE)
		vi.clearAllMocks()
	})

	it('defaults the datatable instruction subject to the TypeScript SQL SDK', async () => {
		const result = await callGlobalTool('get_instructions', { subject: 'datatable' })
		expect(result).toContain('wmill.datatable(')
		expect(result).toContain('TypeScript Datatable API')
		expect(result).toContain('fetchOne')
		// Defaults to TypeScript only — no Python noise.
		expect(result).not.toContain('Python Datatable API')
	})

	it('returns only the requested language SDK for the datatable subject', async () => {
		const ts = await callGlobalTool('get_instructions', { subject: 'datatable', language: 'bun' })
		expect(ts).toContain('TypeScript Datatable API')
		expect(ts).not.toContain('Python Datatable API')

		const py = await callGlobalTool('get_instructions', {
			subject: 'datatable',
			language: 'python3'
		})
		expect(py).toContain('Python Datatable API')
		expect(py).not.toContain('TypeScript Datatable API')
	})

	it('exposes hub search and path-aware test tools', () => {
		const names = globalTools.map((tool) => tool.def.function.name)

		expect(names).toContain('search_hub_scripts')
		expect(names).toContain('test_run_script')
		expect(names).toContain('test_run_flow')
		expect(names).toContain('test_run_step')
		expect(names).toContain('get_job_logs')
		expect(names).toContain('list_runs')
	})

	it('lists recent runs with compact summaries and forwarded filters', async () => {
		const result = await callGlobalTool('list_runs', {
			path: 'f/team/runner',
			created_by: 'alice',
			success: true,
			limit: 10
		})

		expect(JobService.listJobs).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			scriptPathExact: 'f/team/runner',
			createdBy: 'alice',
			label: undefined,
			success: true,
			running: undefined,
			perPage: 10
		})

		const runs = JSON.parse(result)
		expect(runs).toHaveLength(2)
		expect(runs[0]).toMatchObject({
			id: 'completed-1',
			status: 'success',
			path: 'f/team/runner',
			duration_ms: 1200
		})
		expect(runs[1]).toMatchObject({ id: 'queued-1', status: 'running' })
		// Heavy / sensitive fields must not leak into the summary.
		expect(result).not.toContain('verbose logs')
		expect(result).not.toContain('do-not-leak')
		// The result must be surfaced to the tool display, otherwise the details
		// panel shows "No result yet" even though the call succeeded.
		expect(toolCallbacks.setToolStatus).toHaveBeenCalledWith(
			'test-list_runs',
			expect.objectContaining({ result })
		)
	})

	it('defaults list_runs to 30 results when no limit is given', async () => {
		await callGlobalTool('list_runs', {})
		expect(JobService.listJobs).toHaveBeenCalledWith(
			expect.objectContaining({ workspace: WORKSPACE, perPage: 30 })
		)
	})

	it('fetches job logs by id and always suppresses the backend ansi hint line', async () => {
		const result = await callGlobalTool('get_job_logs', { id: 'job-123' })

		expect(JobService.getJobLogs).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			id: 'job-123',
			removeAnsiWarnings: true
		})
		expect(result).toBe('job log line 1\njob log line 2')
		// The logs must be surfaced as the tool result so the details panel shows
		// them rather than "No result yet".
		expect(toolCallbacks.setToolStatus).toHaveBeenCalledWith(
			'test-get_job_logs',
			expect.objectContaining({ result: 'job log line 1\njob log line 2' })
		)
	})

	it('reports when a job has no logs', async () => {
		vi.mocked(JobService.getJobLogs).mockResolvedValueOnce('   ')

		const result = await callGlobalTool('get_job_logs', { id: 'job-empty' })

		expect(result).toBe('No logs available for this job.')
	})

	it('searches hub scripts without fetching script contents', async () => {
		vi.mocked(ScriptService.queryHubScripts).mockResolvedValueOnce([
			{
				version_id: 7,
				app: 'slack',
				summary: 'Send Message'
			}
		] as any)

		const raw = await callGlobalTool('search_hub_scripts', {
			query: 'slack message'
		})

		expect(ScriptService.queryHubScripts).toHaveBeenCalledWith({
			text: 'slack message',
			kind: 'script'
		})
		expect(ScriptService.getHubScriptContentByPath).not.toHaveBeenCalled()
		expect(JSON.parse(raw)).toEqual([
			{
				path: 'hub/7/slack/send_message',
				summary: 'Send Message'
			}
		])
	})

	it('redacts variable draft values when reading workspace items', async () => {
		await callGlobalTool('write_variable', {
			path: 'f/secrets/api_key',
			value: 'super-secret-token',
			is_secret: true,
			description: 'API key'
		})

		const raw = await callGlobalTool('read_workspace_item', {
			type: 'variable',
			path: 'f/secrets/api_key'
		})
		const item = JSON.parse(raw)

		expect(raw).not.toContain('super-secret-token')
		expect(localStorageSnapshot()).not.toContain('super-secret-token')
		expect(item).toEqual({
			type: 'variable',
			path: 'f/secrets/api_key',
			summary: 'API key',
			isDraft: true
		})
	})

	it('writes resource drafts in the editor UserDraft shape', async () => {
		vi.mocked(ResourceService.existsResource).mockResolvedValueOnce(true)
		vi.mocked(ResourceService.getResource).mockResolvedValueOnce({
			path: 'f/resources/db',
			description: 'existing database',
			value: { host: 'old.example.com', port: 5432 },
			resource_type: 'postgresql',
			labels: ['prod'],
			ws_specific: true,
			edited_at: '2026-05-22T09:30:00Z'
		} as any)

		await callGlobalTool('write_resource', {
			path: 'f/resources/db',
			value: { host: 'new.example.com', port: 5432 },
			resource_type: 'postgresql'
		})

		expect(getBackendDraft<any>('resource', 'f/resources/db', { workspace: WORKSPACE })).toEqual({
			path: 'f/resources/db',
			description: 'existing database',
			args: { host: 'new.example.com', port: 5432 },
			labels: ['prod'],
			wsSpecific: true,
			resource_type: 'postgresql'
		})
	})

	it('writes variable drafts in the editor UserDraft shape', async () => {
		vi.mocked(VariableService.existsVariable).mockResolvedValueOnce(true)
		vi.mocked(VariableService.getVariable).mockResolvedValueOnce({
			path: 'f/secrets/api_key',
			value: undefined,
			is_secret: true,
			description: 'old description',
			account: 123,
			is_oauth: true,
			expires_at: '2026-06-22T09:30:00Z',
			labels: ['prod'],
			ws_specific: true,
			edited_at: '2026-05-22T09:30:00Z'
		} as any)

		await callGlobalTool('write_variable', {
			path: 'f/secrets/api_key',
			value: 'new-secret-token',
			is_secret: true,
			description: 'new description'
		})

		expect(getBackendDraft<any>('variable', 'f/secrets/api_key', { workspace: WORKSPACE })).toEqual(
			{
				path: 'f/secrets/api_key',
				variable: {
					value: '',
					is_secret: true,
					description: 'new description'
				},
				labels: ['prod'],
				wsSpecific: true,
				account: 123,
				is_oauth: true,
				expires_at: '2026-06-22T09:30:00Z'
			}
		)
		expect(localStorageSnapshot()).not.toContain('new-secret-token')
	})

	it('deploys secret variable drafts with ephemeral values only', async () => {
		await callGlobalTool('write_variable', {
			path: 'f/secrets/api_key',
			value: 'new-secret-token',
			is_secret: true,
			description: 'new description'
		})

		expect(
			getBackendDraft<any>('variable', 'f/secrets/api_key', { workspace: WORKSPACE })
		).toMatchObject({
			path: 'f/secrets/api_key',
			variable: {
				value: '',
				is_secret: true,
				description: 'new description'
			},
			wsSpecific: false
		})
		expect(localStorageSnapshot()).not.toContain('new-secret-token')

		await callGlobalTool('deploy_workspace_item', {
			type: 'variable',
			path: 'f/secrets/api_key'
		})

		expect(VariableService.createVariable).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: expect.objectContaining({
				path: 'f/secrets/api_key',
				value: 'new-secret-token',
				is_secret: true,
				description: 'new description',
				ws_specific: false
			})
		})
		expect(
			getBackendDraft('variable', 'f/secrets/api_key', { workspace: WORKSPACE })
		).toBeUndefined()
		expect(localStorageSnapshot()).not.toContain('new-secret-token')
	})

	it('does not deploy a secret variable draft when the ephemeral value is gone', async () => {
		seedBackendDraft(
			'variable',
			'f/secrets/api_key',
			{
				path: 'f/secrets/api_key',
				variable: {
					value: '',
					is_secret: true,
					description: 'new description'
				},
				labels: undefined,
				wsSpecific: false
			},
			{ workspace: WORKSPACE }
		)

		await expect(
			callGlobalTool('deploy_workspace_item', {
				type: 'variable',
				path: 'f/secrets/api_key'
			})
		).rejects.toThrow('secret draft values are kept only in memory')
		expect(VariableService.createVariable).not.toHaveBeenCalled()
		expect(VariableService.updateVariable).not.toHaveBeenCalled()
	})

	it('writes script drafts into UserDraft', async () => {
		const content = 'export async function main() {\n\treturn "hello"\n}'

		await callGlobalTool('write_script', {
			path: 'f/scripts/hello',
			summary: 'Hello script',
			language: 'bun',
			content
		})

		expect(
			getBackendDraft<any>('script', 'f/scripts/hello', { workspace: WORKSPACE })
		).toMatchObject({
			path: 'f/scripts/hello',
			summary: 'Hello script',
			language: 'bun',
			content
		})
	})

	it('applies path_prefix to drafts before enforcing the result limit', async () => {
		await callGlobalTool('write_script', {
			path: 'f/other/outside',
			summary: 'Outside draft',
			language: 'bun',
			content: 'export async function main() { return "outside" }'
		})
		await callGlobalTool('write_script', {
			path: 'f/matching/inside',
			summary: 'Inside draft',
			language: 'bun',
			content: 'export async function main() { return "inside" }'
		})

		const raw = await callGlobalTool('list_workspace_items', {
			types: ['script'],
			path_prefix: 'f/matching/',
			limit: 1
		})

		expect(JSON.parse(raw)).toEqual([
			expect.objectContaining({
				type: 'script',
				path: 'f/matching/inside',
				isDraft: true
			})
		])
	})

	it('lists and edits the live script editor draft through its effective path', async () => {
		seedBackendDraft(
			'script',
			'',
			{
				path: 'u/admin/amazed_script',
				summary: 'Live script',
				description: '',
				content: 'export async function main(a: number, b: number) {\n\treturn a + b\n}',
				schema: {},
				is_template: false,
				language: 'bun',
				kind: 'script'
			},
			{ workspace: WORKSPACE }
		)
		UserDraft.setLiveEditorDraft({
			workspace: WORKSPACE,
			itemKind: 'script',
			storagePath: '',
			effectivePath: 'u/admin/amazed_script'
		})

		const listRaw = await callGlobalTool('list_workspace_items', { types: ['script'] })
		expect(JSON.parse(listRaw)).toContainEqual(
			expect.objectContaining({
				type: 'script',
				path: 'u/admin/amazed_script',
				isDraft: true,
				isLiveDraft: true
			})
		)

		await callGlobalTool('edit_script', {
			path: 'u/admin/amazed_script',
			old_string: 'return a + b',
			new_string: 'return a * b'
		})

		expect(getBackendDraft<any>('script', '', { workspace: WORKSPACE })).toMatchObject({
			path: 'u/admin/amazed_script',
			content: 'export async function main(a: number, b: number) {\n\treturn a * b\n}'
		})
		expect(
			getBackendDraft('script', 'u/admin/amazed_script', { workspace: WORKSPACE })
		).toBeUndefined()
	})

	it('lists and writes the live flow editor draft through its effective path', async () => {
		seedBackendDraft(
			'flow',
			'',
			{
				path: '',
				summary: 'Live flow',
				value: { modules: [] },
				schema: {},
				edited_by: '',
				edited_at: '',
				archived: false,
				extra_perms: {}
			},
			{ workspace: WORKSPACE }
		)
		UserDraft.setLiveEditorDraft({
			workspace: WORKSPACE,
			itemKind: 'flow',
			storagePath: '',
			effectivePath: 'u/admin/live_flow'
		})

		const listRaw = await callGlobalTool('list_workspace_items', { types: ['flow'] })
		expect(JSON.parse(listRaw)).toContainEqual(
			expect.objectContaining({
				type: 'flow',
				path: 'u/admin/live_flow',
				isDraft: true,
				isLiveDraft: true
			})
		)

		await callGlobalTool('write_flow', {
			path: 'u/admin/live_flow',
			summary: 'Updated live flow',
			modules: JSON.stringify([{ id: 'step', value: { type: 'identity' } }])
		})

		expect(getBackendDraft<any>('flow', '', { workspace: WORKSPACE })).toMatchObject({
			path: 'u/admin/live_flow',
			summary: 'Updated live flow',
			value: { modules: [{ id: 'step', value: { type: 'identity' } }] }
		})
		expect(getBackendDraft('flow', 'u/admin/live_flow', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('writes the live raw app editor draft through its effective path', async () => {
		seedBackendDraft(
			'raw_app',
			'',
			{
				summary: 'Live app',
				files: { '/src/App.tsx': 'export default function App() { return null }' },
				runnables: {},
				data: { tables: [] }
			},
			{ workspace: WORKSPACE }
		)
		UserDraft.setLiveEditorDraft({
			workspace: WORKSPACE,
			itemKind: 'raw_app',
			storagePath: '',
			effectivePath: 'u/admin/live_app'
		})

		await callGlobalTool('write_app_file', {
			path: 'u/admin/live_app',
			file_path: '/src/New.tsx',
			content: 'export default function New() { return null }'
		})

		expect(getBackendDraft<any>('raw_app', '', { workspace: WORKSPACE })).toMatchObject({
			files: {
				'/src/App.tsx': 'export default function App() { return null }',
				'/src/New.tsx': 'export default function New() { return null }'
			}
		})
		expect(getBackendDraft('raw_app', 'u/admin/live_app', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('discards a draft without deleting the workspace item', async () => {
		await callGlobalTool('write_script', {
			path: 'f/scripts/discard-me',
			summary: 'Temporary draft',
			language: 'bun',
			content: 'export async function main() { return 1 }'
		})

		expect(
			getBackendDraft('script', 'f/scripts/discard-me', { workspace: WORKSPACE })
		).toBeDefined()

		const raw = await callGlobalTool('discard_local_draft', {
			type: 'script',
			path: 'f/scripts/discard-me'
		})

		expect(JSON.parse(raw)).toMatchObject({
			success: true,
			type: 'script',
			path: 'f/scripts/discard-me'
		})
		expect(raw).toContain('The deployed workspace item was not changed')
		expect(
			getBackendDraft('script', 'f/scripts/discard-me', { workspace: WORKSPACE })
		).toBeUndefined()
	})

	// Covers the conflict-on-save / override branch of `persistGlobalDraft`
	// directly: a non-force save whose recorded baseline is older than the
	// server row is rejected with `status:'conflict'`, and `override` (force)
	// pushes our version through. NB: this targets persistGlobalDraft, not the
	// write_* tools — those re-read the backend first (readGlobalDraftValue ->
	// recordRemoteSync), which re-seeds the baseline and so can only surface a
	// conflict when a live editor cell is mounted (not the case in unit tests).
	it('persistGlobalDraft surfaces a conflict on a stale baseline and override forces it', async () => {
		const path = 'f/scripts/conflicted'
		const key = `script:${path}`
		const v1 = {
			path,
			summary: 'v1',
			description: '',
			content: 'export function main() {}',
			language: 'bun'
		}
		seedBackendDraft('script', path, v1)
		// A concurrent writer advanced the row past the baseline we recorded.
		serverTimestamps.set(key, '2026-06-15T00:01:00Z')
		UserDraftDbSyncer.recordRemoteSync(
			{ workspace: WORKSPACE, itemKind: 'script', path },
			'2026-06-15T00:00:00Z'
		)

		const v2 = { ...v1, summary: 'v2', content: 'export function main() { return 1 }' }
		const conflict = await persistGlobalDraft(WORKSPACE, 'script', path, v2)
		expect(conflict.status).toBe('conflict')
		if (conflict.status === 'conflict') {
			expect(conflict.serverTimestamp).toBe('2026-06-15T00:01:00Z')
		}
		// The rejected write left the stored draft untouched.
		expect(getBackendDraft<any>('script', path, { workspace: WORKSPACE })).toMatchObject({
			summary: 'v1'
		})

		// override:true bypasses the check and persists our version.
		const forced = await persistGlobalDraft(WORKSPACE, 'script', path, v2, { force: true })
		expect(forced.status).toBe('saved')
		expect(getBackendDraft<any>('script', path, { workspace: WORKSPACE })).toMatchObject({
			summary: 'v2',
			content: 'export function main() { return 1 }'
		})
	})

	// A backend save failure (network/5xx) is recorded in the syncer's failure
	// map, not thrown — persistGlobalDraft must report 'error', never 'saved'.
	it('persistGlobalDraft reports an error (not saved) when the backend save fails', async () => {
		const path = 'f/scripts/savefail'
		failingWrites.add(`script:${path}`)
		const v = {
			path,
			summary: 's',
			description: '',
			content: 'export function main() {}',
			language: 'bun'
		}
		const res = await persistGlobalDraft(WORKSPACE, 'script', path, v)
		expect(res.status).toBe('error')
		if (res.status === 'error') expect(res.message).toBeTruthy()
		// Nothing was persisted.
		expect(getBackendDraft('script', path, { workspace: WORKSPACE })).toBeUndefined()
	})

	// A non-404 read failure must propagate, not collapse to "no draft" — else
	// the write merge falls through to the deployed item, losing draft edits.
	it('a non-404 backend read failure propagates instead of returning undefined', async () => {
		const path = 'f/scripts/readfail'
		failingReads.add(`script:${path}`)
		await expect(readGlobalDraftValue(WORKSPACE, 'script', path)).rejects.toThrow()
	})

	// Raw-app writes go through saveGlobalAppDraft, which must carry the conflict
	// status so write_app_* tools don't report a stale write as saved.
	it('saveGlobalAppDraft surfaces a conflict on a stale baseline', async () => {
		const path = 'u/admin/conflictedapp'
		const key = `raw_app:${path}`
		seedBackendDraft('raw_app', path, { summary: 'v1', files: {}, runnables: {} })
		serverTimestamps.set(key, '2026-06-15T00:01:00Z')
		UserDraftDbSyncer.recordRemoteSync(
			{ workspace: WORKSPACE, itemKind: 'raw_app', path },
			'2026-06-15T00:00:00Z'
		)
		const res = await saveGlobalAppDraft(WORKSPACE, path, {
			summary: 'v2',
			files: {},
			runnables: {}
		} as any)
		expect(res.status).toBe('conflict')
	})

	// A failed server delete must surface (throw), not silently report removed —
	// the same guard the write path got, applied to the delete path.
	it('deleteGlobalDraft throws when the server delete fails', async () => {
		const path = 'f/scripts/delfail'
		seedBackendDraft('script', path, {
			path,
			summary: 's',
			content: 'export function main() {}',
			language: 'bun'
		})
		failingWrites.add(`script:${path}`)
		await expect(deleteGlobalDraft(WORKSPACE, 'script', path)).rejects.toThrow()
	})

	// `override` is a tool-only conflict flag and must not leak into the persisted
	// schedule draft value.
	it('does not persist the tool-only override flag into a schedule draft', async () => {
		await callGlobalTool('write_schedule', {
			path: 'f/schedules/ov',
			schedule: '0 0 9 * * *',
			timezone: 'UTC',
			script_path: 'f/scripts/run',
			is_flow: false,
			args: {},
			override: true
		})
		const draft = getBackendDraft<any>('trigger_schedule', 'f/schedules/ov', {
			workspace: WORKSPACE
		})
		expect(draft).toBeTruthy()
		expect(draft).not.toHaveProperty('override')
	})

	it('requires trigger_kind when discarding a trigger draft', async () => {
		await expect(
			callGlobalTool('discard_local_draft', {
				type: 'trigger',
				path: 'f/routes/missing-kind'
			})
		).rejects.toThrow('trigger_kind is required')
	})

	it('preserves existing script metadata and seeds freshness on first script write', async () => {
		vi.mocked(ScriptService.existsScriptByPath).mockResolvedValueOnce(true)
		vi.mocked(ScriptService.getScriptByPath).mockResolvedValueOnce({
			path: 'f/scripts/existing',
			hash: 'deployed-hash',
			summary: 'deployed summary',
			description: 'deployed description',
			content: 'old deployed content',
			language: 'bun',
			kind: 'script'
		} as any)

		await callGlobalTool('write_script', {
			path: 'f/scripts/existing',
			summary: 'new summary',
			language: 'bun',
			content: 'new content'
		})

		expect(
			getBackendDraft<any>('script', 'f/scripts/existing', { workspace: WORKSPACE })
		).toMatchObject({
			path: 'f/scripts/existing',
			parent_hash: 'deployed-hash',
			summary: 'new summary',
			description: 'deployed description',
			content: 'new content',
			language: 'bun'
		})
	})

	it('preserves existing flow metadata and seeds freshness on first flow write', async () => {
		vi.mocked(FlowService.existsFlowByPath).mockResolvedValueOnce(true)
		vi.mocked(FlowService.getFlowLatestVersion).mockResolvedValueOnce({ id: 42 } as any)
		vi.mocked(FlowService.getFlowByPath).mockResolvedValueOnce({
			path: 'f/flows/existing',
			summary: 'deployed summary',
			description: 'deployed description',
			value: { modules: [] },
			schema: { properties: { deployed: { type: 'boolean' } } },
			edited_by: 'admin',
			edited_at: '2026-05-22T09:00:00Z',
			archived: false,
			extra_perms: {}
		} as any)

		await callGlobalTool('write_flow', {
			path: 'f/flows/existing',
			summary: 'new summary',
			modules: JSON.stringify([{ id: 'step', value: { type: 'identity' } }])
		})

		expect(
			getBackendDraft<any>('flow', 'f/flows/existing', { workspace: WORKSPACE })
		).toMatchObject({
			path: 'f/flows/existing',
			summary: 'new summary',
			description: 'deployed description',
			value: { modules: [{ id: 'step', value: { type: 'identity' } }] }
		})
	})

	it('preserves editor schedule fields when writing over an existing schedule', async () => {
		vi.mocked(ScheduleService.existsSchedule).mockResolvedValueOnce(true)
		vi.mocked(ScheduleService.getSchedule).mockResolvedValueOnce({
			path: 'f/schedules/nightly',
			schedule: '0 0 0 * * *',
			timezone: 'UTC',
			enabled: true,
			script_path: 'f/scripts/old',
			is_flow: false,
			args: {},
			extra_perms: { 'u/viewer': true },
			email: 'admin@windmill.dev',
			permissioned_as: 'u/admin',
			edited_by: 'admin',
			edited_at: '2026-05-22T09:00:00Z',
			summary: 'old summary',
			description: 'keep this description',
			no_flow_overlap: true,
			cron_version: 'v2'
		} as any)

		await callGlobalTool('write_schedule', {
			path: 'f/schedules/nightly',
			schedule: '0 15 0 * * *',
			timezone: 'Europe/Paris',
			script_path: 'f/flows/new',
			is_flow: true,
			args: { limit: 5 }
		})

		expect(
			getBackendDraft<any>('trigger_schedule', 'f/schedules/nightly', { workspace: WORKSPACE })
		).toMatchObject({
			path: 'f/schedules/nightly',
			schedule: '0 15 0 * * *',
			timezone: 'Europe/Paris',
			script_path: 'f/flows/new',
			is_flow: true,
			args: { limit: 5 },
			extra_perms: { 'u/viewer': true },
			permissioned_as: 'u/admin',
			summary: 'old summary',
			description: 'keep this description',
			no_flow_overlap: true
		})
		expect(
			getBackendDraft<any>('trigger_schedule', 'f/schedules/nightly', { workspace: WORKSPACE })
		).not.toMatchObject({
			edited_by: expect.anything()
		})
	})

	it('preserves editor trigger fields when writing over an existing trigger', async () => {
		vi.mocked(HttpTriggerService.existsHttpTrigger).mockResolvedValueOnce(true)
		vi.mocked(HttpTriggerService.getHttpTrigger).mockResolvedValueOnce({
			path: 'f/routes/api',
			script_path: 'f/scripts/old',
			is_flow: false,
			route_path: 'api/old',
			http_method: 'post',
			request_type: 'sync',
			authentication_method: 'none',
			is_static_website: false,
			workspaced_route: false,
			wrap_body: false,
			raw_string: false,
			mode: 'enabled',
			extra_perms: { 'u/viewer': true },
			workspace_id: WORKSPACE,
			edited_by: 'admin',
			edited_at: '2026-05-22T09:00:00Z',
			permissioned_as: 'u/admin',
			summary: 'old route',
			description: 'keep route description'
		} as any)

		await callGlobalTool('write_trigger', {
			kind: 'http',
			config: {
				path: 'f/routes/api',
				script_path: 'f/flows/new',
				is_flow: true,
				route_path: 'api/new',
				http_method: 'get',
				authentication_method: 'windmill',
				is_static_website: false
			}
		})

		const draft = getBackendDraft<any>('trigger_http', 'f/routes/api', { workspace: WORKSPACE })
		expect(draft).toMatchObject({
			path: 'f/routes/api',
			script_path: 'f/flows/new',
			is_flow: true,
			route_path: 'api/new',
			http_method: 'get',
			authentication_method: 'windmill',
			extra_perms: { 'u/viewer': true },
			permissioned_as: 'u/admin',
			summary: 'old route',
			description: 'keep route description'
		})
		expect(draft).not.toMatchObject({
			workspace_id: expect.anything(),
			edited_by: expect.anything()
		})
	})

	it('seeds raw app draft metadata on first app write', async () => {
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce({
			path: 'f/apps/report',
			summary: 'deployed app',
			versions: [3, 4],
			value: {
				files: { '/src/App.tsx': 'deployed content' },
				runnables: {
					main: {
						type: 'inline',
						inlineScript: { language: 'bun', content: 'export async function main() {}' }
					}
				},
				data: { tables: ['orders'], datatable: 'db', schema: 'public' }
			},
			policy: { execution_mode: 'publisher' },
			custom_path: 'report'
		} as any)

		await callGlobalTool('write_app_file', {
			path: 'f/apps/report',
			file_path: '/src/New.tsx',
			content: 'export default function New() { return null }'
		})

		const draft = getBackendDraft<any>('raw_app', 'f/apps/report', { workspace: WORKSPACE })
		expect(draft).toMatchObject({
			summary: 'deployed app',
			files: {
				'/src/App.tsx': 'deployed content',
				'/src/New.tsx': 'export default function New() { return null }'
			},
			runnables: {
				main: {
					type: 'inline',
					inlineScript: { language: 'bun', content: 'export async function main() {}' }
				}
			},
			data: { tables: ['orders'], datatable: 'db', schema: 'public' },
			policy: { execution_mode: 'publisher' },
			custom_path: 'report'
		})
	})

	it('summarizes local raw app drafts in read_workspace_item', async () => {
		seedBackendDraft(
			'raw_app',
			'f/apps/local',
			{
				summary: 'local app',
				files: { '/src/App.tsx': 'const frontendSecret = "do-not-dump"' },
				runnables: {
					main: {
						type: 'inline',
						inlineScript: {
							language: 'bun',
							content: 'const backendSecret = "do-not-dump"'
						}
					}
				},
				data: { tables: ['orders'] }
			},
			{ workspace: WORKSPACE }
		)

		const raw = await callGlobalTool('read_workspace_item', {
			type: 'app',
			path: 'f/apps/local'
		})
		const item = JSON.parse(raw)

		expect(raw).not.toContain('frontendSecret')
		expect(raw).not.toContain('backendSecret')
		expect(item).toMatchObject({
			type: 'app',
			path: 'f/apps/local',
			summary: 'local app',
			isDraft: true,
			value: {
				frontend: [{ path: '/src/App.tsx', size: 'const frontendSecret = "do-not-dump"'.length }],
				backend: [
					expect.objectContaining({
						key: 'main',
						name: 'main',
						type: 'inline',
						language: 'bun',
						contentSize: 'const backendSecret = "do-not-dump"'.length
					})
				],
				data: { tables: ['orders'] }
			}
		})
		expect(item.value.backend[0]).not.toHaveProperty('content')
	})

	it('summarizes backend raw apps from the same source as file reads', async () => {
		const deployedApp = {
			path: 'f/apps/report',
			summary: 'deployed app',
			versions: [5],
			value: {
				files: {
					'/src/App.tsx': 'deployed content',
					'/src/Helper.tsx': 'helper content'
				},
				runnables: {
					main: {
						type: 'inline',
						inlineScript: {
							language: 'bun',
							content: 'export async function main() { return "deployed" }'
						}
					}
				},
				data: { tables: ['deployed'] }
			}
		}
		vi.mocked(AppService.getAppByPath)
			.mockResolvedValueOnce(deployedApp as any)
			.mockResolvedValueOnce(deployedApp as any)

		const raw = await callGlobalTool('read_workspace_item', {
			type: 'app',
			path: 'f/apps/report'
		})
		const item = JSON.parse(raw)

		expect(item).toMatchObject({
			type: 'app',
			path: 'f/apps/report',
			summary: 'deployed app',
			value: {
				frontend: [
					{ path: '/src/App.tsx', size: 'deployed content'.length },
					{ path: '/src/Helper.tsx', size: 'helper content'.length }
				],
				backend: [
					expect.objectContaining({
						key: 'main',
						name: 'main',
						type: 'inline',
						language: 'bun',
						contentSize: 'export async function main() { return "deployed" }'.length
					})
				],
				data: { tables: ['deployed'] }
			},
			isDraft: false
		})

		await expect(
			callGlobalTool('read_app_file', {
				path: 'f/apps/report',
				file_path: '/src/Helper.tsx'
			})
		).resolves.toBe('helper content')
		expect(getBackendDraft('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('reads raw app files without creating a draft', async () => {
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce({
			path: 'f/apps/report',
			summary: 'deployed app',
			versions: [5],
			value: {
				files: { '/src/App.tsx': 'deployed content' },
				runnables: {},
				data: { tables: [] }
			}
		} as any)

		await expect(
			callGlobalTool('read_app_file', {
				path: 'f/apps/report',
				file_path: '/src/App.tsx'
			})
		).resolves.toBe('deployed content')
		expect(getBackendDraft('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
	})

	const deployedAppWithFile = (filePath: string, content: string) =>
		({
			path: 'f/apps/report',
			summary: 'deployed app',
			versions: [5],
			value: { files: { [filePath]: content }, runnables: {}, data: {} }
		}) as any

	it('truncates a large frontend file to a head slice with a paging annotation', async () => {
		const lines = Array.from({ length: 2000 }, (_, i) => `line ${i + 1}`)
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce(
			deployedAppWithFile('/big.tsx', lines.join('\n'))
		)

		const result = await callGlobalTool('read_app_file', {
			path: 'f/apps/report',
			file_path: '/big.tsx'
		})

		expect(result).toContain('[read_app_file] /big.tsx: lines 1-1500 of 2000.')
		expect(result).toContain('offset=1501')
		expect(result).toContain('line 1500')
		expect(result).not.toContain('line 1501')
	})

	it('returns the requested window when offset and limit are given', async () => {
		const lines = Array.from({ length: 2000 }, (_, i) => `line ${i + 1}`)
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce(
			deployedAppWithFile('/big.tsx', lines.join('\n'))
		)

		const result = await callGlobalTool('read_app_file', {
			path: 'f/apps/report',
			file_path: '/big.tsx',
			offset: 5,
			limit: 3
		})

		expect(result).toContain('[read_app_file] /big.tsx: lines 5-7 of 2000.')
		expect(result).toContain('line 5\nline 6\nline 7')
		expect(result).not.toContain('line 4')
		expect(result).not.toContain('line 8')
	})

	it('truncates at the character budget for files with very long lines', async () => {
		const bigLine = 'x'.repeat(30_000)
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce(
			deployedAppWithFile('/min.tsx', [bigLine, bigLine, bigLine].join('\n'))
		)

		const result = await callGlobalTool('read_app_file', {
			path: 'f/apps/report',
			file_path: '/min.tsx'
		})

		// 50k-char budget keeps only the first 30k line.
		expect(result).toContain('[read_app_file] /min.tsx: lines 1-1 of 3.')
		expect(result).toContain('offset=2')
	})

	it('reports an offset past the end of the file plainly', async () => {
		const lines = Array.from({ length: 10 }, (_, i) => `line ${i + 1}`)
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce(
			deployedAppWithFile('/small.tsx', lines.join('\n'))
		)

		await expect(
			callGlobalTool('read_app_file', {
				path: 'f/apps/report',
				file_path: '/small.tsx',
				offset: 50
			})
		).resolves.toBe('[read_app_file] /small.tsx: offset 50 is past the end of the file (10 lines).')
	})

	it('returns a stub when re-reading an unchanged file still in context', async () => {
		vi.mocked(AppService.getAppByPath)
			.mockResolvedValueOnce(deployedAppWithFile('/src/App.tsx', 'unchanged body'))
			.mockResolvedValueOnce(deployedAppWithFile('/src/App.tsx', 'unchanged body'))
		const helpers = { appReadLedger: new Map(), isToolResultRetained: () => true }

		const first = await callGlobalTool(
			'read_app_file',
			{ path: 'f/apps/report', file_path: '/src/App.tsx' },
			toolCallbacks,
			helpers
		)
		expect(first).toBe('unchanged body')

		const second = await callGlobalTool(
			'read_app_file',
			{ path: 'f/apps/report', file_path: '/src/App.tsx' },
			toolCallbacks,
			helpers
		)
		expect(second).toContain('unchanged since you read it earlier')
		expect(second).not.toContain('unchanged body')
	})

	it('re-sends the body when the prior read is no longer in context', async () => {
		vi.mocked(AppService.getAppByPath)
			.mockResolvedValueOnce(deployedAppWithFile('/src/App.tsx', 'body again'))
			.mockResolvedValueOnce(deployedAppWithFile('/src/App.tsx', 'body again'))
		const helpers = { appReadLedger: new Map(), isToolResultRetained: () => false }

		await callGlobalTool(
			'read_app_file',
			{ path: 'f/apps/report', file_path: '/src/App.tsx' },
			toolCallbacks,
			helpers
		)
		const second = await callGlobalTool(
			'read_app_file',
			{ path: 'f/apps/report', file_path: '/src/App.tsx' },
			toolCallbacks,
			helpers
		)
		expect(second).toBe('body again')
	})

	it('does not persist a raw app draft when patch_app_file validation fails', async () => {
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce({
			path: 'f/apps/report',
			summary: 'deployed app',
			versions: [5],
			value: {
				files: { '/src/App.tsx': 'deployed content' },
				runnables: {},
				data: { tables: [] }
			}
		} as any)

		await expect(
			callGlobalTool('patch_app_file', {
				path: 'f/apps/report',
				file_path: '/src/App.tsx',
				old_string: 'missing content',
				new_string: 'replacement',
				replace_all: false
			})
		).rejects.toThrow()
		expect(getBackendDraft('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('does not persist a raw app draft when delete_app_file validation fails', async () => {
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce({
			path: 'f/apps/report',
			summary: 'deployed app',
			versions: [5],
			value: {
				files: { '/src/App.tsx': 'deployed content' },
				runnables: {},
				data: { tables: [] }
			}
		} as any)

		await expect(
			callGlobalTool('delete_app_file', {
				path: 'f/apps/report',
				file_path: '/src/Missing.tsx'
			})
		).rejects.toThrow('Frontend file "/src/Missing.tsx" not found in app "f/apps/report".')
		expect(getBackendDraft('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('does not persist a raw app draft when delete_app_runnable validation fails', async () => {
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce({
			path: 'f/apps/report',
			summary: 'deployed app',
			versions: [5],
			value: {
				files: { '/src/App.tsx': 'deployed content' },
				runnables: {
					main: {
						type: 'inline',
						inlineScript: { language: 'bun', content: 'export async function main() {}' }
					}
				},
				data: { tables: [] }
			}
		} as any)

		await expect(
			callGlobalTool('delete_app_runnable', {
				path: 'f/apps/report',
				key: 'missing'
			})
		).rejects.toThrow('Backend runnable "missing" not found in app "f/apps/report".')
		expect(getBackendDraft('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('deploys a new raw app draft by bundling files and creating a raw app', async () => {
		seedBackendDraft(
			'raw_app',
			'f/apps/report',
			{
				summary: 'AI report',
				files: {
					'/index.tsx': 'console.log("app")',
					'/package.json': '{"dependencies":{"react":"19.0.0"}}'
				},
				runnables: {},
				data: { tables: [] }
			},
			{ workspace: WORKSPACE }
		)

		const raw = await callGlobalTool('deploy_workspace_item', {
			type: 'app',
			path: 'f/apps/report',
			deployment_message: 'ship report'
		})

		expect(bundleRawAppDraft).toHaveBeenCalledWith(
			expect.objectContaining({
				workspace: WORKSPACE,
				files: expect.objectContaining({
					'/index.tsx': 'console.log("app")'
				})
			})
		)
		expect(AppService.createAppRaw).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			formData: {
				app: {
					path: 'f/apps/report',
					value: {
						files: {
							'/index.tsx': 'console.log("app")',
							'/package.json': '{"dependencies":{"react":"19.0.0"}}'
						},
						runnables: {},
						data: { tables: [] }
					},
					summary: 'AI report',
					policy: expect.objectContaining({ execution_mode: 'publisher' }),
					deployment_message: 'ship report',
					custom_path: undefined
				},
				js: 'bundled js',
				css: 'bundled css'
			}
		})
		expect(AppService.updateAppRaw).not.toHaveBeenCalled()
		expect(getBackendDraft('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
		expect(JSON.parse(raw)).toMatchObject({
			success: true,
			type: 'app',
			path: 'f/apps/report'
		})
	})

	it('deploys an existing raw app draft by bundling files and updating the raw app', async () => {
		vi.mocked(AppService.existsApp).mockResolvedValueOnce(true)
		seedBackendDraft(
			'raw_app',
			'f/apps/report',
			{
				summary: 'Updated report',
				files: { '/index.tsx': 'console.log("updated")' },
				runnables: {},
				data: { tables: ['orders'] },
				policy: { execution_mode: 'anonymous' },
				custom_path: 'kept-by-backend'
			},
			{ workspace: WORKSPACE }
		)

		await callGlobalTool('deploy_workspace_item', {
			type: 'app',
			path: 'f/apps/report'
		})

		expect(AppService.updateAppRaw).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			path: 'f/apps/report',
			formData: {
				app: {
					path: 'f/apps/report',
					value: {
						files: { '/index.tsx': 'console.log("updated")' },
						runnables: {},
						data: { tables: ['orders'] }
					},
					summary: 'Updated report',
					policy: expect.objectContaining({ execution_mode: 'anonymous' }),
					deployment_message: undefined
				},
				js: 'bundled js',
				css: 'bundled css'
			}
		})
		expect(AppService.createAppRaw).not.toHaveBeenCalled()
		expect(getBackendDraft('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('notifies the session preview (as raw_app) after deploying a raw app', async () => {
		const onDeployed = vi.fn()
		setDeployedInSessionHandler(onDeployed)
		try {
			seedBackendDraft(
				'raw_app',
				'f/apps/report',
				{
					summary: 'AI report',
					files: { '/index.tsx': 'console.log("app")' },
					runnables: {},
					data: { tables: [] }
				},
				{ workspace: WORKSPACE }
			)

			await callGlobalTool(
				'deploy_workspace_item',
				{ type: 'app', path: 'f/apps/report' },
				toolCallbacks,
				{
					sessionId: 'sess-123'
				}
			)

			// A raw app deploys under type 'app' but the preview addresses it as
			// 'raw_app'; the calling session id is threaded through so the deploy
			// reloads the issuing session's preview, not the UI-active one.
			expect(onDeployed).toHaveBeenCalledWith({
				sessionId: 'sess-123',
				kind: 'raw_app',
				path: 'f/apps/report'
			})
		} finally {
			setDeployedInSessionHandler(undefined)
		}
	})

	it('fills an empty rawscript module through set_flow_module_code', async () => {
		await callGlobalTool('write_flow', {
			path: 'f/flows/empty-module',
			summary: 'Flow with empty module',
			modules: JSON.stringify([
				{
					id: 'empty_step',
					value: {
						type: 'rawscript',
						language: 'bun',
						content: '',
						input_transforms: {}
					}
				}
			])
		})

		const code = 'export async function main() {\n\treturn 42\n}'

		await expect(
			callGlobalTool('set_flow_module_code', {
				path: 'f/flows/empty-module',
				module_id: 'empty_step',
				code
			})
		).resolves.toContain('Updated flow')

		await expect(
			callGlobalTool('read_flow_module_code', {
				path: 'f/flows/empty-module',
				module_id: 'empty_step'
			})
		).resolves.toBe(code)
	})

	it('writes flows with flow-mode arguments and reads compact flow value', async () => {
		const writeResult = JSON.parse(
			await callGlobalTool('write_flow', {
				path: 'f/flows/with-schema-and-groups',
				summary: 'Flow with schema and groups',
				modules: JSON.stringify([
					{
						id: 'start',
						summary: 'Start',
						value: {
							type: 'identity'
						}
					}
				]),
				schema: JSON.stringify({
					type: 'object',
					properties: {
						name: { type: 'string' }
					},
					required: ['name']
				}),
				groups: JSON.stringify([{ summary: 'Main', start_id: 'start', end_id: 'start' }])
			})
		)

		expect(writeResult.success).toBe(true)
		// Write results must not echo the flow value back to the model.
		expect(writeResult.item).toBeUndefined()

		const raw = await callGlobalTool('read_workspace_item', {
			type: 'flow',
			path: 'f/flows/with-schema-and-groups'
		})
		const item = JSON.parse(raw)

		expect(item.value).toMatchObject({
			modules: [
				{
					id: 'start',
					summary: 'Start',
					value: { type: 'identity' }
				}
			],
			schema: {
				type: 'object',
				properties: {
					name: { type: 'string' }
				},
				required: ['name']
			},
			preprocessor_module: null,
			failure_module: null,
			groups: [{ summary: 'Main', start_id: 'start', end_id: 'start' }]
		})
		expect(item.value.value).toBeUndefined()
	})

	it('test_run_script previews draft script content by path', async () => {
		const content = 'export async function main(name: string) {\n\treturn `hello ${name}`\n}'
		await callGlobalTool('write_script', {
			path: 'f/scripts/draft-test',
			summary: 'Draft test script',
			language: 'bun',
			content
		})

		const result = await withCompletedTestJob(() =>
			callGlobalTool('test_run_script', {
				path: 'f/scripts/draft-test',
				args: { name: 'Ada' }
			})
		)

		expect(JobService.runScriptPreview).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: {
				path: 'f/scripts/draft-test',
				content,
				args: { name: 'Ada' },
				language: 'bun'
			}
		})
		expect(ScriptService.getScriptByPath).not.toHaveBeenCalled()
		expect(result).toContain('Result (SUCCESS)')
		expect(result).toContain('test logs')
	})

	it('test_run_script previews deployed script content when no draft exists', async () => {
		vi.mocked(ScriptService.getScriptByPath).mockResolvedValueOnce({
			path: 'f/scripts/deployed-test',
			summary: 'Deployed test script',
			content: 'def main(name):\n    return name',
			language: 'python3'
		} as any)

		await withCompletedTestJob(() =>
			callGlobalTool('test_run_script', {
				path: 'f/scripts/deployed-test',
				args: { name: 'Grace' }
			})
		)

		expect(ScriptService.getScriptByPath).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			path: 'f/scripts/deployed-test'
		})
		expect(JobService.runScriptPreview).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: {
				path: 'f/scripts/deployed-test',
				content: 'def main(name):\n    return name',
				args: { name: 'Grace' },
				language: 'python3'
			}
		})
	})

	it('test_run_flow previews draft flow content by path', async () => {
		const modules = [{ id: 'start', value: { type: 'identity' } }]
		await callGlobalTool('write_flow', {
			path: 'f/flows/draft-test',
			summary: 'Draft test flow',
			modules: JSON.stringify(modules)
		})

		await withCompletedTestJob(() =>
			callGlobalTool('test_run_flow', {
				path: 'f/flows/draft-test',
				args: { name: 'Ada' }
			})
		)

		expect(FlowService.getFlowByPath).not.toHaveBeenCalled()
		expect(JobService.runFlowPreview).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: {
				path: 'f/flows/draft-test',
				value: { modules },
				args: { name: 'Ada' }
			}
		})
	})

	it('test_run_flow previews deployed flow content when no draft exists', async () => {
		const modules = [{ id: 'deployed_start', value: { type: 'identity' } }]
		vi.mocked(FlowService.getFlowByPath).mockResolvedValueOnce({
			path: 'f/flows/deployed-test',
			summary: 'Deployed test flow',
			value: { modules },
			schema: {}
		} as any)

		await withCompletedTestJob(() =>
			callGlobalTool('test_run_flow', {
				path: 'f/flows/deployed-test',
				args: { name: 'Grace' }
			})
		)

		expect(FlowService.getFlowByPath).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			path: 'f/flows/deployed-test'
		})
		expect(JobService.runFlowPreview).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: {
				path: 'f/flows/deployed-test',
				value: { modules },
				args: { name: 'Grace' }
			}
		})
	})

	it('test_run_flow uses the live flow editor test hook when the active editor matches the path', async () => {
		seedBackendDraft(
			'flow',
			'',
			{
				path: 'u/admin/live_flow',
				summary: 'Live flow',
				value: { modules: [{ id: 'live_step', value: { type: 'identity' } }] },
				schema: {},
				edited_by: '',
				edited_at: '',
				archived: false,
				extra_perms: {}
			},
			{ workspace: WORKSPACE }
		)
		UserDraft.setLiveEditorDraft({
			workspace: WORKSPACE,
			itemKind: 'flow',
			storagePath: '',
			effectivePath: 'u/admin/live_flow'
		})
		const testActiveFlow = vi.fn(async () => 'job-live-flow')

		const result = await withCompletedTestJob(() =>
			callGlobalTool(
				'test_run_flow',
				{
					path: 'u/admin/live_flow',
					args: { name: 'Ada' }
				},
				toolCallbacks,
				{ testActiveFlow }
			)
		)

		expect(testActiveFlow).toHaveBeenCalledWith({ name: 'Ada' })
		expect(FlowService.getFlowByPath).not.toHaveBeenCalled()
		expect(JobService.runFlowPreview).not.toHaveBeenCalled()
		expect(result).toContain('Result (SUCCESS)')
	})

	it('test_run_flow falls back to preview when the live flow editor test hook returns undefined', async () => {
		seedBackendDraft(
			'flow',
			'',
			{
				path: 'u/admin/live_flow_fallback',
				summary: 'Live flow fallback',
				value: { modules: [{ id: 'fallback_step', value: { type: 'identity' } }] },
				schema: {},
				edited_by: '',
				edited_at: '',
				archived: false,
				extra_perms: {}
			},
			{ workspace: WORKSPACE }
		)
		UserDraft.setLiveEditorDraft({
			workspace: WORKSPACE,
			itemKind: 'flow',
			storagePath: '',
			effectivePath: 'u/admin/live_flow_fallback'
		})
		const testActiveFlow = vi.fn(async () => undefined)

		await withCompletedTestJob(() =>
			callGlobalTool(
				'test_run_flow',
				{
					path: 'u/admin/live_flow_fallback',
					args: { name: 'Ada' }
				},
				toolCallbacks,
				{ testActiveFlow }
			)
		)

		expect(testActiveFlow).toHaveBeenCalledWith({ name: 'Ada' })
		expect(FlowService.getFlowByPath).not.toHaveBeenCalled()
		expect(JobService.runFlowPreview).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: {
				path: 'u/admin/live_flow_fallback',
				value: { modules: [{ id: 'fallback_step', value: { type: 'identity' } }] },
				args: { name: 'Ada' }
			}
		})
	})

	it('test_run_step previews rawscript steps from the draft flow', async () => {
		const content = 'export async function main(name: string) {\n\treturn name.toUpperCase()\n}'
		await callGlobalTool('write_flow', {
			path: 'f/flows/rawscript-step',
			summary: 'Flow with rawscript',
			modules: JSON.stringify([
				{
					id: 'format_name',
					value: {
						type: 'rawscript',
						language: 'bun',
						content,
						input_transforms: {}
					}
				}
			])
		})

		await withCompletedTestJob(() =>
			callGlobalTool('test_run_step', {
				path: 'f/flows/rawscript-step',
				stepId: 'format_name',
				args: { name: 'Ada' }
			})
		)

		expect(JobService.runScriptPreview).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: {
				content,
				language: 'bun',
				args: { name: 'Ada' }
			}
		})
	})

	it('test_run_step prefers local script drafts for script steps', async () => {
		const content = 'export async function main(name: string) {\n\treturn `draft ${name}`\n}'
		await callGlobalTool('write_script', {
			path: 'f/scripts/step-script',
			summary: 'Step script',
			language: 'bun',
			content
		})
		await callGlobalTool('write_flow', {
			path: 'f/flows/script-step',
			summary: 'Flow with script step',
			modules: JSON.stringify([
				{
					id: 'call_script',
					value: {
						type: 'script',
						path: 'f/scripts/step-script',
						input_transforms: {}
					}
				}
			])
		})

		await withCompletedTestJob(() =>
			callGlobalTool('test_run_step', {
				path: 'f/flows/script-step',
				stepId: 'call_script',
				args: { name: 'Ada' }
			})
		)

		expect(ScriptService.getScriptByPath).not.toHaveBeenCalled()
		expect(JobService.runScriptPreview).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: {
				path: 'f/scripts/step-script',
				content,
				language: 'bun',
				args: { name: 'Ada' }
			}
		})
	})

	it('test_run_step previews draft subflows for flow steps', async () => {
		const nestedModules = [{ id: 'nested_start', value: { type: 'identity' } }]
		await callGlobalTool('write_flow', {
			path: 'f/flows/nested-draft',
			summary: 'Nested draft flow',
			modules: JSON.stringify(nestedModules)
		})
		await callGlobalTool('write_flow', {
			path: 'f/flows/parent-flow',
			summary: 'Parent flow',
			modules: JSON.stringify([
				{
					id: 'call_flow',
					value: {
						type: 'flow',
						path: 'f/flows/nested-draft',
						input_transforms: {}
					}
				}
			])
		})

		await withCompletedTestJob(() =>
			callGlobalTool('test_run_step', {
				path: 'f/flows/parent-flow',
				stepId: 'call_flow',
				args: { name: 'Ada' }
			})
		)

		expect(JobService.runFlowByPath).not.toHaveBeenCalled()
		expect(JobService.runFlowPreview).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: {
				path: 'f/flows/nested-draft',
				value: { modules: nestedModules },
				args: { name: 'Ada' }
			}
		})
	})

	it('test_run_step lists nested step ids when a step is not found', async () => {
		await callGlobalTool('write_flow', {
			path: 'f/flows/nested-step-error',
			summary: 'Flow with nested step',
			modules: JSON.stringify([
				{
					id: 'loop_step',
					value: {
						type: 'forloopflow',
						iterator: { type: 'static', value: [1] },
						skip_failures: false,
						modules: [
							{
								id: 'nested_script_step',
								value: {
									type: 'rawscript',
									language: 'bun',
									content: 'export async function main() { return 1 }',
									input_transforms: {}
								}
							}
						]
					}
				}
			])
		})

		await expect(
			callGlobalTool('test_run_step', {
				path: 'f/flows/nested-step-error',
				stepId: 'missing_nested_step',
				args: {}
			})
		).rejects.toThrow(/Available steps: loop_step, nested_script_step/)
	})

	it('asks the user a question and returns the selected answer', async () => {
		const callbacks: ToolCallbacks = {
			setToolStatus: vi.fn(),
			removeToolStatus: vi.fn(),
			requestUserQuestion: vi.fn(async (_toolId, question) => question.choices[1])
		}

		const raw = await callGlobalTool(
			'askUserQuestion',
			{
				question: 'Which script language should be used?',
				choices: ['bun', 'python3']
			},
			callbacks
		)

		expect(raw).toBe('python3')
		expect(callbacks.requestUserQuestion).toHaveBeenCalledWith(
			'test-askUserQuestion',
			expect.objectContaining({
				question: 'Which script language should be used?',
				choices: ['bun', 'python3']
			})
		)
		expect(callbacks.setToolStatus).toHaveBeenLastCalledWith(
			'test-askUserQuestion',
			expect.objectContaining({
				content: 'User answered question: python3',
				isLoading: false,
				result: 'python3',
				userQuestion: expect.objectContaining({ selectedChoice: 'python3' })
			})
		)
	})

	it('allows up to ten proposed answers', async () => {
		const choices = Array.from({ length: 10 }, (_, index) => `choice-${index + 1}`)
		const callbacks: ToolCallbacks = {
			setToolStatus: vi.fn(),
			removeToolStatus: vi.fn(),
			requestUserQuestion: vi.fn(async (_toolId, question) => question.choices[9])
		}

		const raw = await callGlobalTool(
			'askUserQuestion',
			{
				question: 'Which option should be used?',
				choices
			},
			callbacks
		)

		expect(raw).toBe('choice-10')
		expect(callbacks.requestUserQuestion).toHaveBeenCalledWith(
			'test-askUserQuestion',
			expect.objectContaining({
				choices
			})
		)
	})

	it('rejects more than ten proposed answers', async () => {
		const callbacks: ToolCallbacks = {
			setToolStatus: vi.fn(),
			removeToolStatus: vi.fn(),
			requestUserQuestion: vi.fn()
		}

		await expect(
			callGlobalTool(
				'askUserQuestion',
				{
					question: 'Which option should be used?',
					choices: Array.from({ length: 11 }, (_, index) => `choice-${index + 1}`)
				},
				callbacks
			)
		).rejects.toThrow()
		expect(callbacks.requestUserQuestion).not.toHaveBeenCalled()
	})

	it('returns a custom answer that is not one of the proposed answers', async () => {
		const callbacks: ToolCallbacks = {
			setToolStatus: vi.fn(),
			removeToolStatus: vi.fn(),
			requestUserQuestion: vi.fn(async () => 'use deno instead')
		}

		const raw = await callGlobalTool(
			'askUserQuestion',
			{
				question: 'Which script language should be used?',
				choices: ['bun', 'python3']
			},
			callbacks
		)

		expect(raw).toBe('use deno instead')
		expect(callbacks.setToolStatus).toHaveBeenLastCalledWith(
			'test-askUserQuestion',
			expect.objectContaining({
				content: 'User answered question: use deno instead',
				result: 'use deno instead',
				userQuestion: expect.objectContaining({ selectedChoice: 'use deno instead' })
			})
		)
	})
})

describe('prepareGlobalSystemMessage', () => {
	it('keeps global chat draft instructions concise and user-facing', () => {
		const message = prepareGlobalSystemMessage()
		const content = message.content

		expect(content).toContain('Draft tools create or update drafts only')
		expect(content).toContain(
			'Use discard_local_draft to remove a draft, including the matching open editor draft'
		)
		expect(content).toContain(
			'After creating or editing a script or flow draft, run test_run_script, test_run_flow, or test_run_step'
		)
		expect(content).toContain('If the user message includes an ACTIVE EDITOR section')
		expect(content).not.toContain('AI draft')
		expect(content).not.toContain('UserDraft')
		expect(content).not.toContain('localStorage')
		expect(content).not.toContain('frontend AI draft store')
	})

	it('exposes separate tools for discarding drafts and deleting workspace items', () => {
		const discard = getGlobalTool('discard_local_draft')
		const deleteItem = getGlobalTool('delete_workspace_item')

		expect(discard.def.function.description).toBe(
			'Discard a draft only. Does not mutate deployed workspace items, but clears the matching open editor draft if one is mounted.'
		)
		expect(deleteItem.def.function.description).toBe(
			'Delete a deployed workspace item. Mutates the workspace.'
		)
		expect(discard.requiresConfirmation).toBe(true)
		expect(deleteItem.requiresConfirmation).toBe(true)
	})

	describe('get_preview_status', () => {
		afterEach(() => {
			setGetPreviewStatusHandler(undefined)
			setOpenPreviewHandler(undefined)
		})

		it('takes no arguments', () => {
			const tool = getGlobalTool('get_preview_status')
			expect(tool.def.function.parameters).toMatchObject({
				type: 'object',
				properties: {},
				required: []
			})
		})

		it('returns the session-only error when no handler is registered', async () => {
			setGetPreviewStatusHandler(undefined)
			const result = await callGlobalTool('get_preview_status', {})
			expect(result).toBe('Error: get_preview_status is only available inside an AI session.')
		})

		it('dispatches to the registered session handler', async () => {
			setGetPreviewStatusHandler(() => 'The preview is currently open showing script "u/me/foo".')
			const result = await callGlobalTool('get_preview_status', {})
			expect(result).toBe('The preview is currently open showing script "u/me/foo".')
		})
	})

	describe('get_app_runtime_logs', () => {
		afterEach(() => {
			setGetRuntimeLogsHandler(undefined)
		})

		it('returns the session-only error when no handler is registered', async () => {
			setGetRuntimeLogsHandler(undefined)
			const result = await callGlobalTool('get_app_runtime_logs', {})
			expect(result).toContain(
				'Error: get_app_runtime_logs is only available inside an AI session.'
			)
			expect(result).toContain('open the raw app preview')
		})

		it('dispatches to the registered handler with the session id and default limit of 10', async () => {
			const callbacks: ToolCallbacks = { setToolStatus: vi.fn(), removeToolStatus: vi.fn() }
			const handler = vi.fn(async () => ({
				aiResult: 'logs output. Next step: inspect the browser error.',
				uiMessage: 'Read 1 runtime log',
				toolResult: '[{"level":"log","message":"log message","ts":1718000000000}]'
			}))
			setGetRuntimeLogsHandler(handler)
			const result = await callGlobalTool('get_app_runtime_logs', {}, callbacks, {
				sessionId: 'sess-logs'
			})
			expect(result).toBe('logs output. Next step: inspect the browser error.')
			expect(handler).toHaveBeenCalledWith({ sessionId: 'sess-logs', limit: 10 })
			expect(callbacks.setToolStatus).toHaveBeenLastCalledWith('test-get_app_runtime_logs', {
				content: 'Read 1 runtime log',
				result: '[{"level":"log","message":"log message","ts":1718000000000}]'
			})
		})

		it('passes an explicit limit through to the handler', async () => {
			const handler = vi.fn(async () => ({
				aiResult: 'logs output',
				uiMessage: 'Read runtime logs',
				toolResult: '[{"level":"log","message":"log message","ts":1718000000000}]'
			}))
			setGetRuntimeLogsHandler(handler)
			await callGlobalTool('get_app_runtime_logs', { limit: 3 }, toolCallbacks, {
				sessionId: 'sess-logs'
			})
			expect(handler).toHaveBeenCalledWith({ sessionId: 'sess-logs', limit: 3 })
		})
	})

	describe('list_app_runs', () => {
		afterEach(() => {
			setListAppRunsHandler(undefined)
		})

		it('returns the session-only error when no handler is registered', async () => {
			setListAppRunsHandler(undefined)
			const result = await callGlobalTool('list_app_runs', {})
			expect(result).toContain('Error: list_app_runs is only available inside an AI session.')
			expect(result).toContain('open the raw app preview')
		})

		it('dispatches to the registered handler with the session id and default limit of 20', async () => {
			const callbacks: ToolCallbacks = { setToolStatus: vi.fn(), removeToolStatus: vi.fn() }
			const handler = vi.fn(() => ({
				aiResult: 'runs output. Next step: call get_job_logs.',
				uiMessage: 'Listed 1 app run',
				toolResult:
					'[{"job_id":"job-1","component":"backend.1","status":"completed","created_at":1718000000000,"started_at":1718000000000,"duration_ms":1000}]'
			}))
			setListAppRunsHandler(handler)
			const result = await callGlobalTool('list_app_runs', {}, callbacks, {
				sessionId: 'sess-runs'
			})
			expect(result).toBe('runs output. Next step: call get_job_logs.')
			expect(handler).toHaveBeenCalledWith({ sessionId: 'sess-runs', limit: 20 })
			expect(callbacks.setToolStatus).toHaveBeenLastCalledWith('test-list_app_runs', {
				content: 'Listed 1 app run',
				result:
					'[{"job_id":"job-1","component":"backend.1","status":"completed","created_at":1718000000000,"started_at":1718000000000,"duration_ms":1000}]'
			})
		})

		it('passes an explicit limit through to the handler', async () => {
			const handler = vi.fn(() => ({
				aiResult: 'runs output',
				uiMessage: 'Listed app runs',
				toolResult:
					'[{"job_id":"job-1","component":"backend.1","status":"completed","created_at":1718000000000,"started_at":1718000000000,"duration_ms":1000}]'
			}))
			setListAppRunsHandler(handler)
			await callGlobalTool('list_app_runs', { limit: 5 }, toolCallbacks, {
				sessionId: 'sess-runs'
			})
			expect(handler).toHaveBeenCalledWith({ sessionId: 'sess-runs', limit: 5 })
		})
	})
})

describe('session-only preview tools gating', () => {
	const toolNames = (sessionPreview: boolean) =>
		globalToolsFor({ sessionPreview }).map((t) => t.def.function.name)

	it('excludes open_preview / get_preview_status / get_app_runtime_logs / list_app_runs outside a session', () => {
		const names = toolNames(false)
		expect(names).not.toContain('open_preview')
		expect(names).not.toContain('get_preview_status')
		expect(names).not.toContain('get_app_runtime_logs')
		expect(names).not.toContain('list_app_runs')
		// other tools are still present
		expect(names).toContain('write_script')
	})

	it('includes open_preview / get_preview_status / get_app_runtime_logs / list_app_runs inside a session', () => {
		const names = toolNames(true)
		expect(names).toContain('open_preview')
		expect(names).toContain('get_preview_status')
		expect(names).toContain('get_app_runtime_logs')
		expect(names).toContain('list_app_runs')
		// session set is the full globalTools
		expect(names.length).toBe(globalTools.length)
	})

	it('mentions open_preview / get_app_runtime_logs / list_app_runs in the system prompt only when preview tools are enabled', () => {
		const off = prepareGlobalSystemMessage(undefined, { previewTools: false }).content as string
		const on = prepareGlobalSystemMessage(undefined, { previewTools: true }).content as string
		expect(off).not.toContain('open_preview')
		expect(off).not.toContain('get_app_runtime_logs')
		expect(off).not.toContain('list_app_runs')
		expect(on).toContain('open_preview')
		expect(on).toContain('get_app_runtime_logs')
		expect(on).toContain('list_app_runs')
	})
})

describe('prepareGlobalUserMessage', () => {
	it('injects the active editor reference without contents', () => {
		__resetUserDraftForTesting()
		localStorage.clear()
		UserDraft.setLiveEditorDraft({
			workspace: WORKSPACE,
			itemKind: 'script',
			storagePath: '',
			effectivePath: 'f/scripts/live_greeting'
		})

		const message = prepareGlobalUserMessage('Update this script', [], { workspace: WORKSPACE })

		expect(message.content).toContain('## ACTIVE EDITOR')
		expect(message.content).toContain('type: script')
		expect(message.content).toContain('path: f/scripts/live_greeting')
		expect(message.content).toContain('isLiveDraft: true')
		expect(message.content).toContain('## INSTRUCTIONS:\nUpdate this script')
		expect(message.content).not.toContain('When the user says')
		expect(message.content).not.toContain('content')
	})

	it('includes selected workspace item references without contents', () => {
		const message = prepareGlobalUserMessage('Update these items', [
			{
				type: 'workspace_script',
				path: 'f/scripts/report',
				title: 'f/scripts/report',
				summary: 'Report script'
			},
			{
				type: 'workspace_flow',
				path: 'f/flows/reporting',
				title: 'f/flows/reporting',
				summary: 'Reporting flow'
			}
		])

		expect(message.content).toContain('## SELECTED CONTEXT')
		expect(message.content).toContain('- type: script, path: f/scripts/report')
		expect(message.content).toContain('- type: flow, path: f/flows/reporting')
		expect(message.content).toContain('## INSTRUCTIONS:\nUpdate these items')
		expect(message.content).not.toContain('Report script')
		expect(message.content).not.toContain('Reporting flow')
	})

	it('omits selected context section when no workspace item is selected', () => {
		const message = prepareGlobalUserMessage('Create a draft')

		expect(message.content).toBe('## INSTRUCTIONS:\nCreate a draft')
	})
})
