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
	// Keys whose `updateDraft` / draft reads throw a non-404 (network/5xx);
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
			getScriptByHash: vi.fn(async () => {
				throw new Error('getScriptByHash mock not configured')
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
			getFlowVersion: vi.fn(async () => {
				throw new Error('getFlowVersion mock not configured')
			}),
			getFlowLatestVersion: vi.fn(async () => ({ id: 1 })),
			listFlows: vi.fn(async () => [])
		}),
		ScheduleService: wrapService(actual.ScheduleService, {
			existsSchedule: vi.fn(async () => false),
			getSchedule: vi.fn(async () => {
				throw new Error('getSchedule mock not configured')
			}),
			createSchedule: vi.fn(async () => 'created'),
			updateSchedule: vi.fn(async () => 'updated')
		}),
		HttpTriggerService: wrapService(actual.HttpTriggerService, {
			existsHttpTrigger: vi.fn(async () => false),
			getHttpTrigger: vi.fn(async () => {
				throw new Error('getHttpTrigger mock not configured')
			}),
			createHttpTrigger: vi.fn(async () => 'created'),
			updateHttpTrigger: vi.fn(async () => 'updated')
		}),
		AppService: wrapService(actual.AppService, {
			existsApp: vi.fn(async () => false),
			createAppRaw: vi.fn(async () => 'created'),
			updateAppRaw: vi.fn(async () => 'updated'),
			getAppByPath: vi.fn(async () => {
				throw new Error('getAppByPath mock not configured')
			}),
			getAppByVersion: vi.fn(async () => {
				throw new Error('getAppByVersion mock not configured')
			}),
			listApps: vi.fn(async () => [])
		}),
		ResourceService: wrapService(actual.ResourceService, {
			existsResource: vi.fn(async () => false),
			getResource: vi.fn(async () => {
				throw new Error('getResource mock not configured')
			}),
			createResource: vi.fn(async () => 'created'),
			updateResource: vi.fn(async () => 'updated')
		}),
		VariableService: wrapService(actual.VariableService, {
			existsVariable: vi.fn(async () => false),
			getVariable: vi.fn(async () => {
				throw new Error('getVariable mock not configured')
			}),
			createVariable: vi.fn(async () => 'created'),
			updateVariable: vi.fn(async () => 'updated')
		}),
		FolderService: wrapService(actual.FolderService, {
			createFolder: vi.fn(async () => 'created')
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
				// The real endpoint rejects drawer kinds up front (drafts for
				// schedule/trigger/resource/variable are private to their owner) —
				// mirror it so a caller regressing to this route for those kinds
				// fails in tests the same way it does against the backend.
				if (!['script', 'flow', 'app', 'raw_app'].includes(kind))
					throw Object.assign(new Error('drafts for this item kind are private to their owner'), {
						status: 404
					})
				const key = `${kind}:${path}`
				if (failingReads.has(key)) throw Object.assign(new Error('server error'), { status: 500 })
				if (!backendDrafts.has(key))
					throw Object.assign(new Error('no draft for that owner at that path'), { status: 404 })
				return { value: backendDrafts.get(key), created_at: '2026-06-15T00:00:00Z' }
			}),
			getOwnDraft: vi.fn(async ({ kind, path }: any) => {
				const key = `${kind}:${path}`
				if (failingReads.has(key)) throw Object.assign(new Error('server error'), { status: 500 })
				// The real endpoint returns 200 with null when the user has no draft.
				if (!backendDrafts.has(key)) return null
				return { value: backendDrafts.get(key), created_at: '2026-06-15T00:00:00Z' }
			}),
			listDrafts: vi.fn(async () =>
				Array.from(backendDrafts.entries()).map(([key, value]) => {
					const idx = key.indexOf(':')
					const path = key.slice(idx + 1)
					// Like the real endpoint: friendly path from the draft JSON, only
					// when set and different from the storage path.
					const draftPath = (value as any)?.draft_path
					return {
						kind: key.slice(0, idx),
						path,
						summary: (value as any)?.summary,
						...(draftPath && draftPath !== path ? { draft_path: draftPath } : {}),
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

vi.mock('$lib/infer', async () => ({
	...(await vi.importActual<any>('$lib/infer')),
	// Avoid the wasm parser in unit tests: the script deploy path infers the arg
	// schema but tolerates failure, and these tests don't assert on the schema.
	inferArgs: vi.fn(async () => {})
}))

import {
	buildOpenPageUrl,
	globalTools,
	globalToolsFor,
	prepareGlobalSystemMessage,
	prepareGlobalUserMessage,
	setDeployedInSessionHandler,
	setGetPreviewStatusHandler,
	setGetRuntimeLogsHandler,
	setGetDomHandler,
	setListAppRunsHandler,
	setOpenPreviewHandler
} from './core'
import { UserDraft, __resetUserDraftForTesting } from '$lib/userDraft.svelte'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import {
	clearGlobalDrafts,
	deleteGlobalDraft,
	listGlobalDrafts,
	persistGlobalDraft,
	readGlobalDraftValue,
	saveGlobalAppDraft
} from './userDraftAdapter'
import { bundleRawAppDraft } from './rawAppBundlerBridge'
import {
	AppService,
	FlowService,
	FolderService,
	HttpTriggerService,
	JobService,
	ResourceService,
	ScheduleService,
	ScriptService,
	VariableService
} from '$lib/gen'
import { userStore } from '$lib/stores'
import { get } from 'svelte/store'
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

	it('deploys every field of a script draft (not just content/summary)', async () => {
		// The deploy delegates to the shared deployer, which reads the full persisted
		// draft via getScriptByPath(getDraft) and deploys all of it. Config fields
		// (tag/priority/schema/description/concurrency) were previously dropped,
		// sourced from the deployed version instead.
		seedBackendDraft(
			'script',
			'f/scripts/full',
			{ path: 'f/scripts/full', content: 'export async function main() {}', language: 'bun' },
			{ workspace: WORKSPACE }
		)
		vi.mocked(ScriptService.getScriptByPath).mockResolvedValueOnce({
			hash: 1234,
			path: 'f/scripts/full',
			summary: 'Full script',
			description: 'desc',
			content: 'export async function main() {}',
			schema: { foo: 'bar' },
			language: 'bun',
			kind: 'script',
			tag: 'custom-tag',
			priority: 7,
			concurrent_limit: 3,
			draft_only: true
		} as any)

		await callGlobalTool('deploy_workspace_item', { type: 'script', path: 'f/scripts/full' })

		expect(ScriptService.createScript).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: expect.objectContaining({
				path: 'f/scripts/full',
				content: 'export async function main() {}',
				summary: 'Full script',
				description: 'desc',
				schema: { foo: 'bar' },
				language: 'bun',
				tag: 'custom-tag',
				priority: 7,
				concurrent_limit: 3,
				parent_hash: 1234
			})
		})
		// Editor-only / server-managed draft keys must not leak into the deploy body.
		const calls = vi.mocked(ScriptService.createScript).mock.calls
		const body = calls[calls.length - 1][0].requestBody as any
		expect(body.draft_only).toBeUndefined()
	})

	it('deploys every config field of a flow draft via createFlow', async () => {
		seedBackendDraft(
			'flow',
			'f/flows/full',
			{ summary: 'Full flow', description: 'flow desc', value: { modules: [] }, schema: {} },
			{ workspace: WORKSPACE }
		)
		vi.mocked(FlowService.getFlowByPath).mockResolvedValueOnce({
			path: 'f/flows/full',
			summary: 'Full flow',
			description: 'flow desc',
			value: { modules: [] },
			schema: { x: 1 },
			tag: 'flow-tag',
			dedicated_worker: true
		} as any)

		await callGlobalTool('deploy_workspace_item', { type: 'flow', path: 'f/flows/full' })

		// No deployed flow row (existsFlowByPath defaults to false) → create.
		expect(FlowService.createFlow).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: expect.objectContaining({
				path: 'f/flows/full',
				summary: 'Full flow',
				description: 'flow desc',
				value: { modules: [] },
				schema: { x: 1 },
				tag: 'flow-tag',
				dedicated_worker: true
			})
		})
		expect(FlowService.updateFlow).not.toHaveBeenCalled()
	})

	it('deploys an editor draft_only script at its chosen path, not its synthetic storage key', async () => {
		// A new script created in the editor lives at a synthetic `u/{user}/draft_{uuid}`
		// storage key while its chosen path is in the draft value. The chat addresses
		// it by the chosen (display) path; deploy must resolve to the storage key so the
		// shared deployer can read the draft via getScriptByPath, then deploy at the
		// chosen path. Reading at the chosen path would 404.
		const storageKey = 'u/admin/draft_abc123'
		const chosenPath = 'f/team/chosen_path'
		seedBackendDraft(
			'script',
			storageKey,
			{
				path: chosenPath,
				summary: 'New script',
				description: '',
				content: 'export async function main() {}',
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
			storagePath: storageKey,
			effectivePath: chosenPath
		})
		vi.mocked(ScriptService.getScriptByPath).mockResolvedValueOnce({
			path: chosenPath,
			summary: 'New script',
			description: '',
			content: 'export async function main() {}',
			schema: {},
			language: 'bun',
			kind: 'script'
		} as any)

		const flushSpy = vi.spyOn(UserDraftDbSyncer, 'flush')

		await callGlobalTool('deploy_workspace_item', { type: 'script', path: chosenPath })

		// Any pending editor autosave is flushed at the storage key before delegating,
		// so the shared deployer reads the latest value, not a stale persisted draft.
		expect(flushSpy).toHaveBeenCalledWith(
			expect.objectContaining({ workspace: WORKSPACE, itemKind: 'script', path: storageKey })
		)
		// The draft is read at the STORAGE key (the chosen path would 404)…
		expect(ScriptService.getScriptByPath).toHaveBeenCalledWith(
			expect.objectContaining({ workspace: WORKSPACE, path: storageKey, getDraft: true })
		)
		// …and deployed at the chosen path.
		expect(ScriptService.createScript).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: expect.objectContaining({ path: chosenPath })
		})
	})

	it('aborts deploy when the pre-deploy draft flush hit a conflict', async () => {
		// flush() resolves even when the save recorded a conflict; deploy must abort
		// rather than publish the stale persisted draft.
		seedBackendDraft(
			'script',
			'f/scripts/conflicted',
			{
				path: 'f/scripts/conflicted',
				summary: '',
				description: '',
				content: 'export async function main() {}',
				schema: {},
				is_template: false,
				language: 'bun',
				kind: 'script'
			},
			{ workspace: WORKSPACE }
		)
		const conflictSpy = vi
			.spyOn(UserDraftDbSyncer, 'getConflict')
			.mockReturnValue({ conflict: { serverTimestamp: '2026', localLastSync: null } } as any)

		await expect(
			callGlobalTool('deploy_workspace_item', { type: 'script', path: 'f/scripts/conflicted' })
		).rejects.toThrow(/conflicting/)
		expect(ScriptService.createScript).not.toHaveBeenCalled()
		conflictSpy.mockRestore()
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

	it('does not echo the app value back to the model on write', async () => {
		const sentinel = 'SENTINEL_DO_NOT_ECHO_DEADBEEF'
		seedBackendDraft(
			'raw_app',
			'',
			{
				summary: 'Echo check',
				files: { '/src/App.tsx': `export default function App() { return '${sentinel}' }` },
				runnables: {},
				data: { tables: [] }
			},
			{ workspace: WORKSPACE }
		)
		UserDraft.setLiveEditorDraft({
			workspace: WORKSPACE,
			itemKind: 'raw_app',
			storagePath: '',
			effectivePath: 'u/admin/echo_app'
		})

		const raw = await callGlobalTool('write_app_file', {
			path: 'u/admin/echo_app',
			file_path: '/src/New.tsx',
			content: 'export default function New() { return null }'
		})

		const parsed = JSON.parse(raw)
		expect(parsed.success).toBe(true)
		expect(parsed.item).toBeUndefined()
		// Neither the pre-existing file body nor the just-written one is resent.
		expect(raw).not.toContain(sentinel)
		expect(raw).not.toContain('function New')
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

	// Schedule drafts (like all drawer kinds) are private to their owner, so the
	// cross-user draft route 404s on them. Reading them back must go through the
	// own-draft route, else a freshly written schedule draft is listed but can
	// never be read or deployed.
	it('reads and deploys a schedule draft written by the chat', async () => {
		await callGlobalTool('write_schedule', {
			path: 'u/admin/test_schedule_greet',
			schedule: '0 0 9 * * *',
			timezone: 'UTC',
			script_path: 'f/scripts/greet',
			is_flow: false,
			args: {}
		})

		const readRaw = await callGlobalTool('read_workspace_item', {
			type: 'schedule',
			path: 'u/admin/test_schedule_greet'
		})
		expect(JSON.parse(readRaw)).toMatchObject({
			type: 'schedule',
			path: 'u/admin/test_schedule_greet',
			isDraft: true
		})

		await callGlobalTool('deploy_workspace_item', {
			type: 'schedule',
			path: 'u/admin/test_schedule_greet'
		})
		expect(ScheduleService.createSchedule).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: expect.objectContaining({
				path: 'u/admin/test_schedule_greet',
				schedule: '0 0 9 * * *',
				script_path: 'f/scripts/greet'
			})
		})
		// The draft is consumed by the deploy.
		expect(
			getBackendDraft('trigger_schedule', 'u/admin/test_schedule_greet', {
				workspace: WORKSPACE
			})
		).toBeUndefined()
	})

	// Same private-owner read path as schedules, for the trigger drawer kinds.
	it('reads and deploys a trigger draft written by the chat', async () => {
		await callGlobalTool('write_trigger', {
			kind: 'http',
			config: {
				path: 'u/admin/fresh_route',
				script_path: 'f/scripts/handler',
				is_flow: false,
				route_path: 'api/fresh',
				http_method: 'get',
				authentication_method: 'none',
				is_static_website: false
			}
		})

		const readRaw = await callGlobalTool('read_workspace_item', {
			type: 'trigger',
			trigger_kind: 'http',
			path: 'u/admin/fresh_route'
		})
		expect(JSON.parse(readRaw)).toMatchObject({
			type: 'trigger',
			triggerKind: 'http',
			path: 'u/admin/fresh_route',
			isDraft: true
		})

		await callGlobalTool('deploy_workspace_item', {
			type: 'trigger',
			trigger_kind: 'http',
			path: 'u/admin/fresh_route'
		})
		expect(HttpTriggerService.createHttpTrigger).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: expect.objectContaining({
				path: 'u/admin/fresh_route',
				route_path: 'api/fresh',
				script_path: 'f/scripts/handler'
			})
		})
		expect(
			getBackendDraft('trigger_http', 'u/admin/fresh_route', { workspace: WORKSPACE })
		).toBeUndefined()
	})

	// Same private-owner read path as schedules, for the resource drawer kind.
	it('reads and deploys a resource draft written by the chat', async () => {
		await callGlobalTool('write_resource', {
			path: 'u/admin/fresh_db',
			value: { host: 'db.example.com', port: 5432 },
			resource_type: 'postgresql',
			description: 'fresh database'
		})

		const readRaw = await callGlobalTool('read_workspace_item', {
			type: 'resource',
			path: 'u/admin/fresh_db'
		})
		expect(JSON.parse(readRaw)).toMatchObject({
			type: 'resource',
			path: 'u/admin/fresh_db',
			isDraft: true
		})

		await callGlobalTool('deploy_workspace_item', {
			type: 'resource',
			path: 'u/admin/fresh_db'
		})
		expect(ResourceService.createResource).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: expect.objectContaining({
				path: 'u/admin/fresh_db',
				resource_type: 'postgresql',
				value: { host: 'db.example.com', port: 5432 }
			})
		})
		expect(
			getBackendDraft('resource', 'u/admin/fresh_db', { workspace: WORKSPACE })
		).toBeUndefined()
	})

	// Same private-owner read path as schedules, for the variable drawer kind.
	// Secret variables deploy through the ephemeral in-memory value instead
	// (see the ephemeral-value tests above); this pins the plain-value cycle.
	it('reads and deploys a non-secret variable draft written by the chat', async () => {
		await callGlobalTool('write_variable', {
			path: 'u/admin/fresh_config',
			value: 'plain-value',
			is_secret: false,
			description: 'fresh config'
		})

		const readRaw = await callGlobalTool('read_workspace_item', {
			type: 'variable',
			path: 'u/admin/fresh_config'
		})
		expect(JSON.parse(readRaw)).toMatchObject({
			type: 'variable',
			path: 'u/admin/fresh_config',
			isDraft: true
		})

		await callGlobalTool('deploy_workspace_item', {
			type: 'variable',
			path: 'u/admin/fresh_config'
		})
		expect(VariableService.createVariable).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: expect.objectContaining({
				path: 'u/admin/fresh_config',
				value: 'plain-value',
				is_secret: false,
				description: 'fresh config'
			})
		})
		expect(
			getBackendDraft('variable', 'u/admin/fresh_config', { workspace: WORKSPACE })
		).toBeUndefined()
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

	describe('stale-draft deploy guard and rebase', () => {
		// The suite's beforeEach only clears mock calls (not implementations), so
		// restore the script-service mocks these tests override back to their factory
		// defaults; otherwise a persistent resolved value leaks into later tests.
		afterEach(() => {
			vi.mocked(ScriptService.existsScriptByPath).mockResolvedValue(false)
			vi.mocked(ScriptService.getScriptByPath).mockImplementation(async () => {
				throw new Error('getScriptByPath mock not configured')
			})
			vi.mocked(ScriptService.getScriptByHash).mockImplementation(async () => {
				throw new Error('getScriptByHash mock not configured')
			})
			vi.mocked(FlowService.existsFlowByPath).mockResolvedValue(false)
			vi.mocked(FlowService.getFlowByPath).mockImplementation(async () => {
				throw new Error('getFlowByPath mock not configured')
			})
			vi.mocked(FlowService.getFlowVersion).mockImplementation(async () => {
				throw new Error('getFlowVersion mock not configured')
			})
			vi.mocked(FlowService.getFlowLatestVersion).mockResolvedValue({ id: 1 } as any)
			vi.mocked(AppService.existsApp).mockResolvedValue(false)
			vi.mocked(AppService.getAppByPath).mockImplementation(async () => {
				throw new Error('getAppByPath mock not configured')
			})
			vi.mocked(AppService.getAppByVersion).mockImplementation(async () => {
				throw new Error('getAppByVersion mock not configured')
			})
		})

		function seedStaleScriptDraft(path: string, parentHash: string, content = 'draft content') {
			seedBackendDraft('script', path, {
				path,
				summary: 's',
				description: '',
				content,
				language: 'bun',
				kind: 'script',
				parent_hash: parentHash,
				schema: {}
			})
		}

		function mockDeployedScript(path: string, hash: string, content = 'latest deployed') {
			vi.mocked(ScriptService.existsScriptByPath).mockResolvedValue(true)
			vi.mocked(ScriptService.getScriptByPath).mockResolvedValue({
				path,
				hash,
				content,
				language: 'bun',
				summary: 's'
			} as any)
		}

		it('blocks deploying a script draft started from an older deployed version', async () => {
			seedStaleScriptDraft('f/scripts/stale', 'base-hash')
			mockDeployedScript('f/scripts/stale', 'new-hash')

			await expect(
				callGlobalTool('deploy_workspace_item', { type: 'script', path: 'f/scripts/stale' })
			).rejects.toThrow(/older deployed version/)
			expect(ScriptService.createScript).not.toHaveBeenCalled()
		})

		it('deploys a stale script draft when force is set', async () => {
			seedStaleScriptDraft('f/scripts/stale', 'base-hash')
			mockDeployedScript('f/scripts/stale', 'new-hash')

			const result = JSON.parse(
				await callGlobalTool('deploy_workspace_item', {
					type: 'script',
					path: 'f/scripts/stale',
					force: true
				})
			)
			expect(result.success).toBe(true)
			expect(ScriptService.createScript).toHaveBeenCalled()
		})

		it('deploys a script draft that is based on the current deployed head', async () => {
			seedStaleScriptDraft('f/scripts/fresh', 'head-hash')
			mockDeployedScript('f/scripts/fresh', 'head-hash')

			const result = JSON.parse(
				await callGlobalTool('deploy_workspace_item', { type: 'script', path: 'f/scripts/fresh' })
			)
			expect(result.success).toBe(true)
			expect(ScriptService.createScript).toHaveBeenCalled()
		})

		it('rebase_draft discards the stale draft and surfaces the changes to replay', async () => {
			seedStaleScriptDraft('f/scripts/stale', 'base-hash', 'base content\nmy added line\n')
			mockDeployedScript('f/scripts/stale', 'new-hash', 'latest deployed content\n')
			vi.mocked(ScriptService.getScriptByHash).mockResolvedValue({
				hash: 'base-hash',
				content: 'base content\n',
				language: 'bun'
			} as any)

			const result = JSON.parse(
				await callGlobalTool('rebase_draft', { type: 'script', path: 'f/scripts/stale' })
			)
			expect(result.success).toBe(true)
			expect(result.latest_hash).toBe('new-hash')
			// The diff surfaces the draft's own change over its fork base.
			expect(result.your_changes).toContain('my added line')

			// The stale draft is discarded (not reset), so a premature deploy fails
			// cleanly rather than silently shipping the latest unchanged.
			expect(getBackendDraft('script', 'f/scripts/stale', { workspace: WORKSPACE })).toBeUndefined()
			await expect(
				callGlobalTool('deploy_workspace_item', { type: 'script', path: 'f/scripts/stale' })
			).rejects.toThrow(/No .*draft/)
			expect(ScriptService.createScript).not.toHaveBeenCalled()

			// Re-applying re-bases onto the current head; the deploy then passes.
			await callGlobalTool('write_script', {
				path: 'f/scripts/stale',
				summary: 's',
				language: 'bun',
				content: 'latest deployed content\nmy added line\n'
			})
			const deploy = JSON.parse(
				await callGlobalTool('deploy_workspace_item', { type: 'script', path: 'f/scripts/stale' })
			)
			expect(deploy.success).toBe(true)
			expect(ScriptService.createScript).toHaveBeenCalled()
		})

		function seedStaleFlowDraft(path: string, versionId: number, modules: any[] = []) {
			seedBackendDraft('flow', path, {
				path,
				summary: 'f',
				description: '',
				version_id: versionId,
				value: { modules },
				schema: {}
			})
		}

		function mockDeployedFlow(path: string, versionId: number) {
			vi.mocked(FlowService.existsFlowByPath).mockResolvedValue(true)
			vi.mocked(FlowService.getFlowByPath).mockResolvedValue({
				path,
				summary: 'f',
				version_id: versionId,
				value: { modules: [] },
				schema: {}
			} as any)
		}

		it('blocks deploying a flow draft started from an older deployed version', async () => {
			seedStaleFlowDraft('f/flows/stale', 1)
			mockDeployedFlow('f/flows/stale', 2)

			await expect(
				callGlobalTool('deploy_workspace_item', { type: 'flow', path: 'f/flows/stale' })
			).rejects.toThrow(/older deployed version/)
			expect(FlowService.updateFlow).not.toHaveBeenCalled()
			expect(FlowService.createFlow).not.toHaveBeenCalled()
		})

		it('rebase_draft discards the stale flow draft and surfaces the changes to replay', async () => {
			seedStaleFlowDraft('f/flows/stale', 1, [{ id: 'a', value: { type: 'identity' } }])
			mockDeployedFlow('f/flows/stale', 2)
			vi.mocked(FlowService.getFlowVersion).mockResolvedValue({
				value: { modules: [] }
			} as any)

			const result = JSON.parse(
				await callGlobalTool('rebase_draft', { type: 'flow', path: 'f/flows/stale' })
			)
			expect(result.success).toBe(true)
			expect(result.latest_version).toBe(2)
			expect(result.your_changes).toContain('identity')

			// The stale draft is discarded, so a premature deploy fails cleanly.
			expect(getBackendDraft('flow', 'f/flows/stale', { workspace: WORKSPACE })).toBeUndefined()
			await expect(
				callGlobalTool('deploy_workspace_item', { type: 'flow', path: 'f/flows/stale' })
			).rejects.toThrow(/No .*draft/)
			expect(FlowService.updateFlow).not.toHaveBeenCalled()
		})

		function seedStaleAppDraft(path: string, parentVersion: number, file = 'old') {
			seedBackendDraft('raw_app', path, {
				summary: 'a',
				files: { '/index.tsx': file },
				runnables: {},
				data: { tables: [] },
				parent_version: parentVersion
			})
		}

		function mockDeployedApp(path: string, versionId: number, file = 'latest') {
			vi.mocked(AppService.existsApp).mockResolvedValue(true)
			vi.mocked(AppService.getAppByPath).mockResolvedValue({
				path,
				summary: 'a',
				versions: [versionId],
				value: { files: { '/index.tsx': file }, runnables: {}, data: { tables: [] } },
				policy: { execution_mode: 'publisher' }
			} as any)
		}

		it('grafts the fork-base version onto a new app draft and keeps it through the save whitelist', async () => {
			// No draft yet: the first app edit projects the deployed app into a draft.
			// This exercises the runtime path types can't catch — the graft in
			// appSourceToDraftValue AND survival through normalizeAppDraftValue's whitelist.
			mockDeployedApp('f/apps/fresh', 5)

			await callGlobalTool('write_app_file', {
				path: 'f/apps/fresh',
				file_path: '/src/New.tsx',
				content: 'export default function New() { return null }'
			})

			const draft = getBackendDraft<any>('raw_app', 'f/apps/fresh', { workspace: WORKSPACE })
			expect(draft.parent_version).toBe(5)
		})

		it('keeps a draft-only app friendly draft_path through the save whitelist on chat edits', async () => {
			// A renamed draft-only app parks its typed name in the draft's
			// `draft_path`. A chat edit round-trips the value through
			// normalizeAppDraftValue — dropping the field there would rename the
			// app back to its `draft_<uuid>` storage key.
			seedBackendDraft('raw_app', 'u/admin/draft_abc', {
				summary: 'a',
				files: { '/index.tsx': 'old' },
				runnables: {},
				data: { tables: [] },
				draft_path: 'u/admin/my_pretty_app'
			})

			await callGlobalTool('write_app_file', {
				path: 'u/admin/draft_abc',
				file_path: '/index.tsx',
				content: 'new'
			})

			const draft = getBackendDraft<any>('raw_app', 'u/admin/draft_abc', { workspace: WORKSPACE })
			expect(draft.files['/index.tsx']).toBe('new')
			expect(draft.draft_path).toBe('u/admin/my_pretty_app')
		})

		it('lists a live raw app staged rename as draftPath even when registered at the storage key', async () => {
			// Flow/raw-app renames live in the value's `draft_path` while `path`
			// stays the storage key; a live registration whose effectivePath is the
			// storage key must not hide the staged rename from listGlobalDrafts —
			// the pickers regroup the item under it.
			const storageKey = 'u/admin/draft_live1'
			const staged = 'f/team/renamed_app'
			seedBackendDraft(
				'raw_app',
				storageKey,
				{
					summary: '',
					files: { '/App.tsx': 'export default () => null' },
					runnables: {},
					data: { tables: [] },
					draft_path: staged
				},
				{ workspace: WORKSPACE }
			)
			UserDraft.setLiveEditorDraft({
				workspace: WORKSPACE,
				itemKind: 'raw_app',
				storagePath: storageKey,
				effectivePath: storageKey
			})

			const items = await listGlobalDrafts(WORKSPACE)
			const app = items.find((i) => i.type === 'app' && i.path === storageKey)
			expect(app?.draftPath).toBe(staged)
			expect(app?.isLiveDraft).toBe(true)
		})

		it('blocks deploying an app draft started from an older deployed version', async () => {
			seedStaleAppDraft('f/apps/stale', 1)
			mockDeployedApp('f/apps/stale', 2)

			await expect(
				callGlobalTool('deploy_workspace_item', { type: 'app', path: 'f/apps/stale' })
			).rejects.toThrow(/older deployed version/)
			expect(AppService.createAppRaw).not.toHaveBeenCalled()
			expect(AppService.updateAppRaw).not.toHaveBeenCalled()
		})

		it('rebase_draft discards the stale app draft and surfaces the changes to replay', async () => {
			seedStaleAppDraft('f/apps/stale', 1, 'my-change')
			mockDeployedApp('f/apps/stale', 2, 'latest-deployed')
			vi.mocked(AppService.getAppByVersion).mockResolvedValue({
				value: { files: { '/index.tsx': 'base' }, runnables: {}, data: { tables: [] } }
			} as any)

			const result = JSON.parse(
				await callGlobalTool('rebase_draft', { type: 'app', path: 'f/apps/stale' })
			)
			expect(result.success).toBe(true)
			expect(result.latest_version).toBe(2)
			expect(result.your_changes).toContain('my-change')

			// The stale draft is discarded, so a premature deploy fails cleanly.
			expect(getBackendDraft('raw_app', 'f/apps/stale', { workspace: WORKSPACE })).toBeUndefined()
			await expect(
				callGlobalTool('deploy_workspace_item', { type: 'app', path: 'f/apps/stale' })
			).rejects.toThrow(/No .*draft/)
			expect(AppService.updateAppRaw).not.toHaveBeenCalled()
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

		expect(result).toContain('lines 1-1500 of 2000.')
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

		expect(result).toContain('lines 5-7 of 2000.')
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

		expect(result).toContain('lines 1-3 of 3, truncated to the first 50000 of 90002 chars.')
		expect(result).toContain('the file is likely minified')
		expect(result.split('\n\n')[1]).toHaveLength(50_000)
	})

	it('caps a single-line generated file at the character budget', async () => {
		const bigLine = 'x'.repeat(60_000)
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce(
			deployedAppWithFile('/generated.js', bigLine)
		)

		const result = await callGlobalTool('read_app_file', {
			path: 'f/apps/report',
			file_path: '/generated.js'
		})

		expect(result).toContain('lines 1-1 of 1, truncated to the first 50000 of 60000 chars.')
		expect(result).toContain('re-read with a smaller limit')
		expect(result.split('\n\n')[1]).toBe('x'.repeat(50_000))
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
		).resolves.toBe('offset 50 is past the end of the file (10 lines).')
	})

	// Deterministic micro-benchmark: measures how much context the read_app_file cap
	// saves over a realistic big-project read pattern, isolated from model
	// nondeterminism. "Baseline" is the old behavior (whole file returned on every
	// read); "actual" is the current line cap + char budget + paging. Asserting the
	// ratio also guards against a future change silently weakening the savings.
	it('micro-benchmark: the read cap cuts returned context for a realistic read pattern', async () => {
		const bigContent = Array.from({ length: 5000 }, (_, i) => `const row${i} = ${i};`).join('\n')
		const minified = 'a'.repeat(200_000) // single long line (e.g. a generated bundle)
		const appValue = {
			path: 'f/apps/report',
			summary: 'big app',
			versions: [5],
			value: { files: { '/big.tsx': bigContent, '/min.js': minified }, runnables: {}, data: {} }
		} as any
		const fullSize: Record<string, number> = {
			'/big.tsx': bigContent.length,
			'/min.js': minified.length
		}

		// A plausible pass over a large app: read a big file head, page deeper into it,
		// then hit a generated bundle (capped at the char budget). The old tool returned
		// every file in full on every read.
		const sequence = [
			{ file_path: '/big.tsx' }, // 1. big file head (line cap)
			{ file_path: '/big.tsx', offset: 1501 }, // 2. next line chunk
			{ file_path: '/min.js' } // 3. minified bundle head (char budget)
		]

		let baselineChars = 0
		let actualChars = 0
		const perRead: number[] = []
		for (const read of sequence) {
			baselineChars += fullSize[read.file_path]
			vi.mocked(AppService.getAppByPath).mockResolvedValueOnce(appValue)
			const out = await callGlobalTool('read_app_file', { path: 'f/apps/report', ...read })
			actualChars += out.length
			perRead.push(out.length)
		}

		const reductionPct = Math.round((1 - actualChars / baselineChars) * 100)
		// Surfaced when the suite runs so the benchmark is readable, not just asserted.
		// eslint-disable-next-line no-console
		console.log(
			`[read_app_file micro-benchmark] baseline=${baselineChars} chars, actual=${actualChars} chars ` +
				`(per-read ${perRead.join(', ')}), reduction=${reductionPct}%`
		)

		// Each capped read is far smaller than the whole file it came from:
		expect(perRead[0]).toBeLessThan(fullSize['/big.tsx']) // head slice < whole file
		expect(perRead[1]).toBeLessThan(fullSize['/big.tsx']) // a paged line chunk too
		expect(perRead[2]).toBeLessThan(51_000) // ~50k char budget + a short annotation
		// Overall: well under half the bytes the old tool would have returned.
		expect(actualChars).toBeLessThan(baselineChars * 0.5)
	})

	// A multi-file app: the revenue helper is referenced in three frontend files
	// and one inline backend runnable; a generated file also mentions it (and must
	// be excluded). Mirrors the analytics_dashboard fixture's "symbol spread".
	const searchAppValue = () =>
		({
			path: 'f/apps/report',
			summary: 'search app',
			versions: [5],
			value: {
				files: {
					'/lib/aggregations.ts': 'export function computeRevenue(o) {\n  return o.unitPrice\n}\n',
					'/components/SummaryPanel.tsx':
						'import { computeRevenue } from "../lib/aggregations"\nconst total = computeRevenue(order)\n',
					'/components/OrdersTable.tsx': 'const r = computeRevenue(row)\n// renders revenue\n',
					'/styles.css': '.revenue { color: green }\n',
					'/wmill.d.ts': 'declare function computeRevenue(o: any): number\n'
				},
				runnables: {
					computeSummary: {
						type: 'inline',
						inlineScript: {
							language: 'bun',
							content: 'export async function main() {\n  return computeRevenue\n}\n'
						}
					}
				},
				data: {}
			}
		}) as any

	it('greps across frontend files and inline runnables, returning file:line rows', async () => {
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce(searchAppValue())

		const result = await callGlobalTool('search_app', {
			path: 'f/apps/report',
			query: 'computeRevenue'
		})

		// header counts every match across the (non-generated) files, without echoing the query
		expect(result).toMatch(/\d+ match(?:es)? in \d+ files?/)
		// frontend rows use read_app_file's leading-slash addressing
		expect(result).toContain('/lib/aggregations.ts')
		expect(result).toContain('1: export function computeRevenue(o) {')
		expect(result).toContain('/components/SummaryPanel.tsx')
		// inline runnable rows use the backend/<key>/main.<ext> addressing
		expect(result).toContain('backend/computeSummary/main.ts')
		// generated files are never searched
		expect(result).not.toContain('/wmill.d.ts')
	})

	it('matches case-insensitively', async () => {
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce(searchAppValue())

		const result = await callGlobalTool('search_app', {
			path: 'f/apps/report',
			query: 'COMPUTEREVENUE' // upper-case query still matches computeRevenue
		})

		expect(result).toContain('/lib/aggregations.ts')
		expect(result).toContain('export function computeRevenue(o) {')
	})

	it('filters by a basename glob (matches nested files)', async () => {
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce(searchAppValue())

		const result = await callGlobalTool('search_app', {
			path: 'f/apps/report',
			query: 'computeRevenue',
			file_glob: '*.tsx'
		})

		expect(result).toContain('/components/SummaryPanel.tsx')
		expect(result).toContain('/components/OrdersTable.tsx')
		expect(result).not.toContain('/lib/aggregations.ts')
		expect(result).not.toContain('backend/computeSummary/main.ts')
	})

	it('filters by a path glob (e.g. backend/**)', async () => {
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce(searchAppValue())

		const result = await callGlobalTool('search_app', {
			path: 'f/apps/report',
			query: 'computeRevenue',
			file_glob: 'backend/**'
		})

		expect(result).toContain('backend/computeSummary/main.ts')
		expect(result).not.toContain('/lib/aggregations.ts')
	})

	it('reports zero matches with a hint instead of an empty result', async () => {
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce(searchAppValue())

		const result = await callGlobalTool('search_app', {
			path: 'f/apps/report',
			query: 'nonexistent_symbol_xyz'
		})

		expect(result).toContain('No matches')
		expect(result).toContain('Try a broader')
	})

	it('truncates very long matching lines to keep results sparse', async () => {
		const longLine = `const x = "${'q'.repeat(5000)} computeRevenue"`
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce({
			path: 'f/apps/report',
			summary: 'app',
			versions: [5],
			value: { files: { '/min.js': longLine }, runnables: {}, data: {} }
		} as any)

		const result = await callGlobalTool('search_app', {
			path: 'f/apps/report',
			query: 'computeRevenue'
		})

		expect(result).toContain('[line truncated]')
		expect(result.length).toBeLessThan(1000)
	})

	it('caps the number of match rows and says it truncated', async () => {
		const manyLines = Array.from({ length: 500 }, (_, i) => `hit ${i}`).join('\n')
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce({
			path: 'f/apps/report',
			summary: 'app',
			versions: [5],
			value: { files: { '/big.tsx': manyLines }, runnables: {}, data: {} }
		} as any)

		const result = await callGlobalTool('search_app', {
			path: 'f/apps/report',
			query: 'hit',
			max_matches: 50
		})

		expect(result).toContain('500 matches')
		expect(result).toContain('showing the first 50')
		// 50 capped match lines, each rendered with its fixed context window (deduped),
		// so the body is bounded near max_matches and nowhere near the 500 total.
		const rows = result.split('\n').filter((l) => /^\s+\d+: /.test(l)).length
		expect(rows).toBeGreaterThanOrEqual(50)
		expect(rows).toBeLessThan(60)
	})

	it('counts every file with a match, even matches past the render cap', async () => {
		// The first (sorted) file exhausts max_matches; the later file's match falls
		// past the cap but the symbol still lives there, so the header must count it.
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce({
			path: 'f/apps/report',
			summary: 'app',
			versions: [5],
			value: {
				files: { '/a.tsx': 'hit\nhit\nhit\nhit\nhit', '/b.tsx': 'hit' },
				runnables: {},
				data: {}
			}
		} as any)

		const result = await callGlobalTool('search_app', {
			path: 'f/apps/report',
			query: 'hit',
			max_matches: 3
		})

		expect(result).toContain('6 matches in 2 files')
		expect(result).toContain('showing the first 3')
	})

	// Deterministic micro-benchmark: how much context a single search_app call
	// saves over locating a symbol by reading the candidate files whole. Baseline
	// is the conservative "read only the files that actually contain the symbol"
	// path (a model without search must read at least those in full); the real
	// saving is larger because, lacking search, a model often reads non-matching
	// files too. Isolated from model nondeterminism so it can gate regressions.
	it('micro-benchmark: search_app locates a symbol far cheaper than reading files', async () => {
		const fileBodies: Record<string, string> = {}
		// 8 component files, 3 of which reference the symbol, each ~120 lines.
		for (let f = 0; f < 8; f++) {
			const lines = Array.from({ length: 120 }, (_, i) =>
				f < 3 && i === 60 ? `  return computeRevenue(order${f})` : `  const v${i} = ${i} // padding`
			)
			fileBodies[`/components/File${f}.tsx`] = lines.join('\n')
		}
		const appValue = {
			path: 'f/apps/report',
			summary: 'big app',
			versions: [5],
			value: {
				files: fileBodies,
				runnables: {},
				data: {}
			}
		} as any

		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce(appValue)
		const searchOut = await callGlobalTool('search_app', {
			path: 'f/apps/report',
			query: 'computeRevenue'
		})

		// Baseline: the bytes a model must pull to gather the same locations by
		// reading each matching file in full.
		const matchingFiles = Object.entries(fileBodies).filter(([, body]) =>
			body.includes('computeRevenue')
		)
		const baselineChars = matchingFiles.reduce((sum, [, body]) => sum + body.length, 0)
		const actualChars = searchOut.length
		const reductionPct = Math.round((1 - actualChars / baselineChars) * 100)
		// eslint-disable-next-line no-console
		console.log(
			`[search_app micro-benchmark] baseline=${baselineChars} chars (read ${matchingFiles.length} files whole), ` +
				`actual=${actualChars} chars (one search), reduction=${reductionPct}%`
		)

		// The search surfaced exactly the 3 locations…
		expect(matchingFiles.length).toBe(3)
		expect((searchOut.match(/computeRevenue/g) ?? []).length).toBeGreaterThanOrEqual(3)
		// …at a tiny fraction of reading those files whole.
		expect(actualChars).toBeLessThan(baselineChars * 0.15)
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

		// getAppByPath resolves with no draft_path → deploy at the item's own path.
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce({} as any)

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

	it('deploys an editor raw app draft at its draft_path, not its synthetic storage key', async () => {
		// An editor-created draft_only raw app lives at a synthetic storage key with
		// its chosen path in `draft_path`; deploy must resolve to the storage key,
		// read draft_path, and create the app there — not at the synthetic key.
		const storageKey = 'u/admin/draft_app999'
		const chosenPath = 'f/team/chosen_app'
		seedBackendDraft(
			'raw_app',
			storageKey,
			{
				summary: 'Editor app',
				files: { '/App.tsx': 'export default () => null' },
				runnables: {},
				data: { tables: [] },
				draft_path: chosenPath
			},
			{ workspace: WORKSPACE }
		)
		UserDraft.setLiveEditorDraft({
			workspace: WORKSPACE,
			itemKind: 'raw_app',
			storagePath: storageKey,
			effectivePath: chosenPath
		})
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce({
			draft: { draft_path: chosenPath }
		} as any)
		const flushSpy = vi.spyOn(UserDraftDbSyncer, 'flush')

		await callGlobalTool('deploy_workspace_item', { type: 'app', path: chosenPath })

		// The draft is flushed at the storage key before the draft_path read, so a
		// not-yet-saved editor rename isn't read stale.
		expect(flushSpy).toHaveBeenCalledWith(
			expect.objectContaining({ workspace: WORKSPACE, itemKind: 'raw_app', path: storageKey })
		)
		// draft_path is read from the backend draft at the storage key…
		expect(AppService.getAppByPath).toHaveBeenCalledWith(
			expect.objectContaining({
				workspace: WORKSPACE,
				path: storageKey,
				getDraft: true,
				rawApp: true
			})
		)
		// …and the app is created at the chosen path, not the synthetic key.
		expect(AppService.createAppRaw).toHaveBeenCalledWith(
			expect.objectContaining({
				formData: expect.objectContaining({
					app: expect.objectContaining({ path: chosenPath })
				})
			})
		)
	})

	it('aborts a raw app deploy when the draft_path lookup fails (non-404)', async () => {
		// A real lookup failure (network/5xx) must abort, not silently fall back to the
		// storage path and deploy there. Only a 404 justifies the storage-path fallback.
		seedBackendDraft(
			'raw_app',
			'u/admin/draft_appfail',
			{
				summary: 'Editor app',
				files: { '/App.tsx': 'export default () => null' },
				runnables: {},
				data: { tables: [] },
				draft_path: 'f/team/chosen_app'
			},
			{ workspace: WORKSPACE }
		)
		UserDraft.setLiveEditorDraft({
			workspace: WORKSPACE,
			itemKind: 'raw_app',
			storagePath: 'u/admin/draft_appfail',
			effectivePath: 'f/team/chosen_app'
		})
		vi.mocked(AppService.getAppByPath).mockRejectedValueOnce(
			Object.assign(new Error('server error'), { status: 500 })
		)

		await expect(
			callGlobalTool('deploy_workspace_item', { type: 'app', path: 'f/team/chosen_app' })
		).rejects.toThrow()
		expect(AppService.createAppRaw).not.toHaveBeenCalled()
		expect(AppService.updateAppRaw).not.toHaveBeenCalled()
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

		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce({} as any)

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

	it('forwards preserve_on_behalf_of when the deployed policy carries an on_behalf_of', async () => {
		// Without the flag the backend resets the policy's on_behalf_of to the
		// deploying user; this chat path has no on-behalf-of selector, so it must
		// preserve whatever the carried policy already holds.
		vi.mocked(AppService.existsApp).mockResolvedValueOnce(true)
		seedBackendDraft(
			'raw_app',
			'f/apps/obo',
			{
				summary: 'On-behalf app',
				files: { '/index.tsx': 'console.log("obo")' },
				runnables: {},
				data: { tables: [] },
				policy: {
					execution_mode: 'publisher',
					on_behalf_of: 'u/alice',
					on_behalf_of_email: 'alice@windmill.dev'
				}
			},
			{ workspace: WORKSPACE }
		)
		vi.mocked(AppService.getAppByPath).mockResolvedValueOnce({} as any)

		await callGlobalTool('deploy_workspace_item', { type: 'app', path: 'f/apps/obo' })

		expect(AppService.updateAppRaw).toHaveBeenCalledWith(
			expect.objectContaining({
				formData: expect.objectContaining({
					app: expect.objectContaining({ preserve_on_behalf_of: true })
				})
			})
		)
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

			vi.mocked(AppService.getAppByPath).mockResolvedValueOnce({} as any)

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

	it('writes and reads back free-floating flow notes', async () => {
		const writeResult = JSON.parse(
			await callGlobalTool('write_flow', {
				path: 'f/flows/with-notes',
				summary: 'Flow with notes',
				modules: JSON.stringify([
					{
						id: 'start',
						summary: 'Start',
						value: { type: 'identity' }
					}
				]),
				notes: JSON.stringify([
					{ id: 'n1', type: 'free', text: 'What this flow does', color: 'blue' }
				])
			})
		)

		expect(writeResult.success).toBe(true)

		const item = JSON.parse(
			await callGlobalTool('read_workspace_item', {
				type: 'flow',
				path: 'f/flows/with-notes'
			})
		)

		expect(item.value.notes).toHaveLength(1)
		expect(item.value.notes[0]).toMatchObject({
			id: 'n1',
			type: 'free',
			text: 'What this flow does',
			color: 'blue'
		})
		// Free notes with no explicit geometry get auto-placed/sized by validation.
		expect(item.value.notes[0].position).toBeDefined()
		expect(item.value.notes[0].size).toBeDefined()
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
			requestUserQuestion: vi.fn(async (_toolId, question) => [question.choices[1]])
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
				content: 'Asked: Which script language should be used? — python3',
				isLoading: false,
				result: 'python3',
				userQuestion: expect.objectContaining({ selectedChoices: ['python3'] })
			})
		)
	})

	it('returns a newline-bulleted list when several answers are selected', async () => {
		const callbacks: ToolCallbacks = {
			setToolStatus: vi.fn(),
			removeToolStatus: vi.fn(),
			requestUserQuestion: vi.fn(async (_toolId, question) => [
				question.choices[0],
				question.choices[2]
			])
		}

		const raw = await callGlobalTool(
			'askUserQuestion',
			{
				question: 'Which languages should be supported?',
				choices: ['bun', 'python3', 'go'],
				multiSelect: true
			},
			callbacks
		)

		// Model-facing return stays newline-bulleted; the header readback is a
		// compact comma list.
		expect(raw).toBe('- bun\n- go')
		expect(callbacks.requestUserQuestion).toHaveBeenCalledWith(
			'test-askUserQuestion',
			expect.objectContaining({ multiSelect: true })
		)
		expect(callbacks.setToolStatus).toHaveBeenLastCalledWith(
			'test-askUserQuestion',
			expect.objectContaining({
				content: 'Asked: Which languages should be supported? — bun, go',
				isLoading: false,
				result: '- bun\n- go',
				userQuestion: expect.objectContaining({ selectedChoices: ['bun', 'go'] })
			})
		)
	})

	it('allows up to ten proposed answers', async () => {
		const choices = Array.from({ length: 10 }, (_, index) => `choice-${index + 1}`)
		const callbacks: ToolCallbacks = {
			setToolStatus: vi.fn(),
			removeToolStatus: vi.fn(),
			requestUserQuestion: vi.fn(async (_toolId, question) => [question.choices[9]])
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
			requestUserQuestion: vi.fn(async () => ['use deno instead'])
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
				content: 'Asked: Which script language should be used? — use deno instead',
				result: 'use deno instead',
				userQuestion: expect.objectContaining({ selectedChoices: ['use deno instead'] })
			})
		)
	})
})

describe('folder tools', () => {
	beforeEach(() => {
		vi.clearAllMocks()
		userStore.set(undefined)
	})
	afterEach(() => {
		userStore.set(undefined)
	})

	it('create_folder requires confirmation', () => {
		const tool = getGlobalTool('create_folder')
		expect(tool.requiresConfirmation).toBe(true)
		expect(tool.confirmationMessage).toBe('Create folder')
	})

	it('create_folder creates the folder and reflects it in the path context', async () => {
		userStore.set({ username: 'bob', is_admin: false, folders: ['existing'] } as any)
		const raw = await callGlobalTool('create_folder', { name: 'analytics', summary: 'team data' })

		expect(vi.mocked(FolderService.createFolder)).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: { name: 'analytics', summary: 'team data' }
		})
		const parsed = JSON.parse(raw)
		expect(parsed.success).toBe(true)
		expect(parsed.message).toContain('f/analytics')
		expect((get(userStore) as any)?.folders).toContain('analytics')
	})

	it('create_folder rejects an invalid name without calling the API', async () => {
		const raw = await callGlobalTool('create_folder', { name: 'bad name!' })
		expect(vi.mocked(FolderService.createFolder)).not.toHaveBeenCalled()
		const parsed = JSON.parse(raw)
		expect(parsed.success).toBe(false)
		expect(parsed.error).toContain('alphanumeric')
	})

	it('create_folder surfaces a backend error (e.g. name conflict)', async () => {
		vi.mocked(FolderService.createFolder).mockRejectedValueOnce(new Error('Folder already exists'))
		const raw = await callGlobalTool('create_folder', { name: 'taken' })
		const parsed = JSON.parse(raw)
		expect(parsed.success).toBe(false)
		expect(parsed.error).toContain('Folder already exists')
	})
})

describe('session pipeline gate', () => {
	const FLAG = 'wm_dev_session_pipelines'
	afterEach(() => {
		localStorage.removeItem(FLAG)
	})

	it('replaces the session prompt pipeline guidance with the alpha notice by default', () => {
		const content = prepareGlobalSystemMessage(undefined, { previewTools: true }).content as string
		expect(content).toContain('Data pipelines are in alpha and NOT yet available in this chat')
		expect(content).not.toContain('call get_instructions with subject "pipeline"')
		expect(content).not.toContain('Building a data pipeline: call open_preview')
	})

	it('restores the session pipeline guidance when the dev flag is set', () => {
		localStorage.setItem(FLAG, '1')
		const content = prepareGlobalSystemMessage(undefined, { previewTools: true }).content as string
		expect(content).toContain('call get_instructions with subject "pipeline"')
		expect(content).toContain('Building a data pipeline: call open_preview')
		expect(content).not.toContain('Data pipelines are in alpha and NOT yet available in this chat')
	})

	it('leaves the standalone (non-session) chat pipeline guidance ungated', () => {
		const content = prepareGlobalSystemMessage(undefined, { previewTools: false }).content as string
		expect(content).toContain('call get_instructions with subject "pipeline"')
		expect(content).not.toContain('Data pipelines are in alpha and NOT yet available in this chat')
	})

	it('refuses get_instructions(pipeline) in a session but not outside one', async () => {
		const inSession = await callGlobalTool('get_instructions', { subject: 'pipeline' }, undefined, {
			isSessionChat: true,
			sessionId: 'session-1'
		})
		expect(inSession).toContain('data pipelines are in alpha')

		const outsideSession = await callGlobalTool('get_instructions', { subject: 'pipeline' })
		expect(outsideSession).not.toContain('data pipelines are in alpha')

		// The eval harness passes a sessionId to standalone (non-session) chats;
		// only the explicit isSessionChat marker may engage the gate.
		const standaloneWithSessionId = await callGlobalTool(
			'get_instructions',
			{ subject: 'pipeline' },
			undefined,
			{ sessionId: 'eval-session' }
		)
		expect(standaloneWithSessionId).not.toContain('data pipelines are in alpha')
	})

	it('refuses open_preview(kind=pipeline) while gated', async () => {
		const handler = vi.fn(() => 'opened')
		setOpenPreviewHandler(handler)
		try {
			const gated = await callGlobalTool('open_preview', { kind: 'pipeline', path: 'my_folder' })
			expect(gated).toContain('data pipelines are in alpha')
			expect(handler).not.toHaveBeenCalled()

			localStorage.setItem(FLAG, '1')
			const opened = await callGlobalTool('open_preview', { kind: 'pipeline', path: 'my_folder' })
			expect(opened).toBe('opened')
		} finally {
			setOpenPreviewHandler(undefined)
		}
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

	it('honors user-supplied shared folder paths without asking first', () => {
		const content = prepareGlobalSystemMessage(undefined, {
			user: { username: 'admin', is_admin: true, folders: ['evals'] }
		}).content as string

		expect(content).toContain(
			'If the user supplies a fully qualified `f/<folder>/...` path, use that exact path'
		)
		expect(content).toContain('Do not ask for folder confirmation')
		expect(content).toContain('substitute a `u/admin/...` path unless a tool rejects it')
	})

	it('tells the model to create a folder only when the user explicitly asks', () => {
		const content = prepareGlobalSystemMessage().content as string
		expect(content).toContain(
			'create one with `create_folder` only when the user explicitly asks for a new folder'
		)
	})

	describe('folder guidance', () => {
		const guidanceOf = (user: {
			username: string
			is_admin?: boolean
			folders?: string[]
			folders_read?: string[]
		}) => prepareGlobalSystemMessage(undefined, { user }).content as string

		it('lists the writable folders for a non-admin', () => {
			const content = guidanceOf({
				username: 'bob',
				is_admin: false,
				folders: ['marketing', 'data_engineering'],
				folders_read: ['marketing', 'data_engineering']
			})
			expect(content).toContain(
				'Folders you can write to in this workspace: `f/marketing`, `f/data_engineering`.'
			)
			expect(content).not.toContain('You can see but NOT write to')
		})

		it('flags read-only folders a non-admin cannot write to', () => {
			const content = guidanceOf({
				username: 'bob',
				is_admin: false,
				folders: ['team_a'],
				folders_read: ['team_a', 'team_b']
			})
			expect(content).toContain('Folders you can write to in this workspace: `f/team_a`.')
			expect(content).toContain(
				'You can see but NOT write to: `f/team_b` — never create or deploy items there.'
			)
		})

		it('points a non-admin with no writable folders at the personal scope', () => {
			const content = guidanceOf({ username: 'bob', is_admin: false, folders: [] })
			expect(content).toContain(
				'You have no shared folders you can write to in this workspace, so use `u/bob/<name>`.'
			)
		})

		it('gives an admin permission-agnostic guidance with a non-exhaustive hint', () => {
			const content = guidanceOf({
				username: 'admin',
				is_admin: true,
				folders: ['marketing', 'data_engineering']
			})
			expect(content).toContain('As a workspace admin you can write to any existing folder.')
			expect(content).toContain(
				'Folders here include `f/marketing`, `f/data_engineering` (you can also write to others not listed).'
			)
			expect(content).toContain(
				'If the user names a folder, use it; if they explicitly ask for a new folder, create it with `create_folder`; otherwise ask them which folder to use rather than guessing or creating one unprompted.'
			)
			expect(content).not.toContain('Folders you can write to in this workspace')
		})

		it('omits the folder hint for an admin with no associated folders', () => {
			const content = guidanceOf({ username: 'admin', is_admin: true, folders: [] })
			expect(content).toContain(
				'- As a workspace admin you can write to any existing folder. If the user names a folder, use it; if they explicitly ask for a new folder, create it with `create_folder`; otherwise ask them which folder to use rather than guessing or creating one unprompted.'
			)
			expect(content).not.toContain('Folders here include')
		})

		it('caps the folder list and notes the remainder', () => {
			const folders = Array.from({ length: 45 }, (_, i) => `f${i}`)
			const content = guidanceOf({ username: 'bob', is_admin: false, folders })
			expect(content).toContain('(+5 more)')
		})

		it('emits no folder guidance when no user is available', () => {
			const content = prepareGlobalSystemMessage().content as string
			expect(content).not.toContain('Folders you can write to in this workspace')
			expect(content).not.toContain('As a workspace admin you can write to any existing folder')
			expect(content).not.toContain('You have no shared folders you can write to')
		})
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

	describe('search_dom / read_dom', () => {
		afterEach(() => {
			setGetDomHandler(undefined)
		})

		it('returns the session-only error when no handler is registered', async () => {
			setGetDomHandler(undefined)
			const searchResult = await callGlobalTool('search_dom', { pattern: 'foo' })
			expect(searchResult).toContain(
				'Error: search_dom and read_dom are only available inside an AI session.'
			)
			expect(searchResult).toContain('open the raw app preview')
			const readResult = await callGlobalTool('read_dom', {})
			expect(readResult).toContain(
				'Error: search_dom and read_dom are only available inside an AI session.'
			)
		})

		it('dispatches search_dom to the handler with a search query', async () => {
			const handler = vi.fn(async () => ({
				aiResult:
					'Live DOM for selector "button": Found 1 matching line(s):\n3: <button>Go</button>',
				uiMessage: 'Searched app DOM',
				toolResult: 'match'
			}))
			setGetDomHandler(handler)
			const result = await callGlobalTool(
				'search_dom',
				{ selector: 'button', pattern: 'Go', ignore_case: true },
				toolCallbacks,
				{ sessionId: 'sess-dom' }
			)
			expect(result).toContain('Found 1 matching line(s)')
			expect(handler).toHaveBeenCalledWith({
				sessionId: 'sess-dom',
				query: { mode: 'search', selector: 'button', pattern: 'Go', ignoreCase: true }
			})
		})

		it('dispatches read_dom to the handler with a read query (whole-page when no selector)', async () => {
			const handler = vi.fn(async () => ({
				aiResult: 'Live DOM for whole page (<body>): Showing lines 1-1 of 1.',
				uiMessage: 'Read app DOM',
				toolResult: 'dom'
			}))
			setGetDomHandler(handler)
			await callGlobalTool('read_dom', { start_line: 2, end_line: 40 }, toolCallbacks, {
				sessionId: 'sess-dom'
			})
			expect(handler).toHaveBeenCalledWith({
				sessionId: 'sess-dom',
				query: { mode: 'read', selector: undefined, startLine: 2, endLine: 40 }
			})
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
		expect(names).not.toContain('search_dom')
		expect(names).not.toContain('read_dom')
		// other tools are still present
		expect(names).toContain('write_script')
	})

	it('includes open_preview / get_preview_status / get_app_runtime_logs / list_app_runs inside a session', () => {
		const names = toolNames(true)
		expect(names).toContain('open_preview')
		expect(names).toContain('get_preview_status')
		expect(names).toContain('get_app_runtime_logs')
		expect(names).toContain('list_app_runs')
		expect(names).toContain('search_dom')
		expect(names).toContain('read_dom')
		// The session set is the full globalTools minus capability-gated tools:
		// this environment is not Chromium, so take_screenshot is withheld (DOM
		// capture is only faithful on Blink). search_dom / read_dom are not gated.
		expect(names).not.toContain('take_screenshot')
		expect(names.length).toBe(globalTools.length - 1)
	})

	it('offers take_screenshot inside a session only on Chromium', () => {
		vi.stubGlobal('navigator', {
			userAgentData: { brands: [{ brand: 'Chromium', version: '138' }] },
			userAgent: 'stubbed'
		})
		try {
			const names = toolNames(true)
			expect(names).toContain('take_screenshot')
			expect(names.length).toBe(globalTools.length)
			// still session-only, even on Chromium
			expect(toolNames(false)).not.toContain('take_screenshot')
		} finally {
			vi.unstubAllGlobals()
		}
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
		expect(off).not.toContain('search_dom')
		expect(on).toContain('search_dom')
		expect(on).toContain('read_dom')
	})

	it('renders a SELECTED DOM ELEMENTS block for app_dom_selector context', () => {
		const message = prepareGlobalUserMessage('Fix the button', [
			{
				type: 'app_dom_selector',
				selector: 'div.card > button.primary',
				appPath: 'u/admin/my_app',
				title: 'button.primary',
				tagName: 'button',
				className: 'primary'
			}
		])
		const content = message.content as string
		expect(content).toContain('## SELECTED DOM ELEMENTS')
		expect(content).toContain('div.card > button.primary')
		expect(content).toContain('search_dom')
	})

	// The instruction headers are matched by their distinctive parenthetical so the
	// guidance bullet (which references both block names) doesn't false-positive.
	const WS_HEADER = 'WORKSPACE INSTRUCTIONS (configured by a workspace admin'
	const USER_HEADER = "USER INSTRUCTIONS (this user's personal instructions"

	it('renders only the workspace block when given workspace instructions', () => {
		const content = prepareGlobalSystemMessage({ workspace: 'Always be terse.' }).content as string
		expect(content).toContain(WS_HEADER)
		expect(content).toContain('Always be terse.')
		expect(content).not.toContain(USER_HEADER)
	})

	it('renders the user block with the edit-tool mention when given user instructions', () => {
		const content = prepareGlobalSystemMessage({ user: 'Prefer Bun for new scripts.' })
			.content as string
		expect(content).toContain(USER_HEADER)
		expect(content).toContain('update_user_instructions')
		expect(content).toContain('Prefer Bun for new scripts.')
		expect(content).not.toContain(WS_HEADER)
	})

	it('renders the workspace block before the user block when both are present', () => {
		const content = prepareGlobalSystemMessage({ workspace: 'WS rule.', user: 'User rule.' })
			.content as string
		expect(content).toContain(WS_HEADER)
		expect(content.indexOf(USER_HEADER)).toBeGreaterThan(content.indexOf(WS_HEADER))
	})

	it('omits both instruction headers when none are provided', () => {
		const content = prepareGlobalSystemMessage().content as string
		expect(content).not.toContain(WS_HEADER)
		expect(content).not.toContain(USER_HEADER)
	})
})

describe('update_user_instructions', () => {
	function makeHelpers(initial = '') {
		let value = initial
		return {
			getUserInstructions: () => value,
			setUserInstructions: vi.fn((v: string) => {
				value = v
			})
		}
	}

	it('appends to empty instructions', async () => {
		const helpers = makeHelpers('')
		const res = await callGlobalTool(
			'update_user_instructions',
			{ operation: 'append', text: 'Prefer Bun for new scripts.' },
			toolCallbacks,
			helpers
		)
		expect(helpers.setUserInstructions).toHaveBeenCalledWith('Prefer Bun for new scripts.')
		expect(res).toContain('Added a personal instruction')
	})

	it('appends to existing instructions joined by a blank line', async () => {
		const helpers = makeHelpers('Existing rule.')
		await callGlobalTool(
			'update_user_instructions',
			{ operation: 'append', text: 'Another rule.' },
			toolCallbacks,
			helpers
		)
		expect(helpers.setUserInstructions).toHaveBeenCalledWith('Existing rule.\n\nAnother rule.')
	})

	it('returns only a short confirmation, not the resulting instructions', async () => {
		const helpers = makeHelpers('Existing rule.')
		const res = await callGlobalTool(
			'update_user_instructions',
			{ operation: 'append', text: 'Another rule.' },
			toolCallbacks,
			helpers
		)
		expect(res).not.toContain('Existing rule.')
		expect(res).not.toContain('Another rule.')
	})

	it('replaces an exact match', async () => {
		const helpers = makeHelpers('Prefer Bun.\n\nUse tabs.')
		await callGlobalTool(
			'update_user_instructions',
			{ operation: 'replace', old_string: 'Prefer Bun.', new_string: 'Prefer Deno.' },
			toolCallbacks,
			helpers
		)
		expect(helpers.setUserInstructions).toHaveBeenCalledWith('Prefer Deno.\n\nUse tabs.')
	})

	it('removes the matched text when new_string is empty', async () => {
		const helpers = makeHelpers('Keep this.\n\nDrop this.')
		await callGlobalTool(
			'update_user_instructions',
			{ operation: 'replace', old_string: '\n\nDrop this.', new_string: '' },
			toolCallbacks,
			helpers
		)
		expect(helpers.setUserInstructions).toHaveBeenCalledWith('Keep this.')
	})

	it('clears all instructions when the whole text is replaced with empty', async () => {
		const helpers = makeHelpers('Only rule.')
		const res = await callGlobalTool(
			'update_user_instructions',
			{ operation: 'replace', old_string: 'Only rule.', new_string: '' },
			toolCallbacks,
			helpers
		)
		expect(helpers.setUserInstructions).toHaveBeenCalledWith('')
		expect(res).toContain('Cleared your personal instructions')
	})

	it('errors without writing when old_string is not found, and echoes the current text for recovery', async () => {
		const helpers = makeHelpers('Existing rule.')
		const res = await callGlobalTool(
			'update_user_instructions',
			{ operation: 'replace', old_string: 'missing', new_string: 'x' },
			toolCallbacks,
			helpers
		)
		expect(helpers.setUserInstructions).not.toHaveBeenCalled()
		expect(res).toContain('not found')
		expect(res).toContain('Existing rule.')
	})

	it('rejects a result over the length cap without writing', async () => {
		const helpers = makeHelpers('')
		const res = await callGlobalTool(
			'update_user_instructions',
			{ operation: 'append', text: 'a'.repeat(5001) },
			toolCallbacks,
			helpers
		)
		expect(helpers.setUserInstructions).not.toHaveBeenCalled()
		expect(res).toContain('over the 5000')
	})

	it('fails gracefully when the context does not provide instruction helpers', async () => {
		const res = await callGlobalTool(
			'update_user_instructions',
			{ operation: 'append', text: 'x' },
			toolCallbacks,
			{}
		)
		expect(res).toContain('cannot modify user instructions')
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
			},
			{
				type: 'workspace_app',
				path: 'f/apps/dashboard',
				title: 'f/apps/dashboard',
				summary: 'Dashboard raw app'
			}
		])

		expect(message.content).toContain('## SELECTED CONTEXT')
		expect(message.content).toContain('- type: script, path: f/scripts/report')
		expect(message.content).toContain('- type: flow, path: f/flows/reporting')
		expect(message.content).toContain('- type: raw_app, path: f/apps/dashboard')
		expect(message.content).toContain('## INSTRUCTIONS:\nUpdate these items')
		expect(message.content).not.toContain('Report script')
		expect(message.content).not.toContain('Reporting flow')
		expect(message.content).not.toContain('Dashboard raw app')
	})

	it('omits selected context section when no workspace item is selected', () => {
		const message = prepareGlobalUserMessage('Create a draft')

		expect(message.content).toBe('## INSTRUCTIONS:\nCreate a draft')
	})
})

describe('buildOpenPageUrl compare selection', () => {
	const itemsOf = (url: string) => new URL(url, 'http://x').searchParams.get('items')

	it('explicit items win over the chat mask', () => {
		const url = buildOpenPageUrl(
			'compare',
			{ page: 'compare', items: ['script:f/a/b'] },
			{ workspaceId: 'ws', chatItems: ['flow:f/c/d'] }
		)
		expect(itemsOf(url)).toBe('script:f/a/b')
	})

	it('omitted items fall back to the chat-modified mask', () => {
		const url = buildOpenPageUrl(
			'compare',
			{ page: 'compare' },
			{ workspaceId: 'ws', chatItems: ['flow:f/c/d', 'script:f/a/b'] }
		)
		expect(itemsOf(url)).toBe('flow:f/c/d,script:f/a/b')
	})

	it('an empty or absent mask yields no items param (page select-all default)', () => {
		expect(
			itemsOf(
				buildOpenPageUrl('compare', { page: 'compare' }, { workspaceId: 'ws', chatItems: [] })
			)
		).toBeNull()
		expect(
			itemsOf(buildOpenPageUrl('compare', { page: 'compare' }, { workspaceId: 'ws' }))
		).toBeNull()
	})
})
