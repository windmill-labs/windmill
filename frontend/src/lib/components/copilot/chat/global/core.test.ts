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

// In-memory stand-in for the per-user DB draft layer (`DraftService`).
// Post #9351 the global mode is headless and persists drafts through the
// DB endpoints, so the tools read/write here instead of localStorage.
// Hoisted so the `$lib/gen` mock factory (itself hoisted) can close over it.
const draftDb = vi.hoisted(() => {
	const store = new Map<string, { value: unknown; saved_at: string }>()
	let clock = 0
	const key = (kind: string, path: string) => `${kind}/${path}`
	return {
		store,
		key,
		reset() {
			store.clear()
			clock = 0
		},
		set(kind: string, path: string, value: unknown): string {
			clock += 1
			const saved_at = `2026-01-01T00:00:${String(clock).padStart(2, '0')}.000Z`
			store.set(key(kind, path), { value, saved_at })
			return saved_at
		}
	}
})

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
			}))
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
			saveDraft: vi.fn(async ({ kind, path, requestBody }: any) => {
				if (requestBody.value === null) {
					draftDb.store.delete(draftDb.key(kind, path))
					return { status: 'saved', current_timestamp: '1970-01-01T00:00:00.000Z' }
				}
				const saved_at = draftDb.set(kind, path, requestBody.value)
				return { status: 'saved', current_timestamp: saved_at }
			}),
			getDraft: vi.fn(async ({ kind, path }: any) => {
				const entry = draftDb.store.get(draftDb.key(kind, path))
				if (!entry) {
					throw new actual.ApiError(
						{ method: 'GET', url: '' },
						{ url: '', ok: false, status: 404, statusText: 'Not Found', body: undefined },
						'no draft for the current user at that path'
					)
				}
				return { value: entry.value, saved_at: entry.saved_at }
			}),
			listDrafts: vi.fn(async () =>
				[...draftDb.store.entries()].map(([k, v]) => {
					const slash = k.indexOf('/')
					return { typ: k.slice(0, slash), path: k.slice(slash + 1), saved_at: v.saved_at }
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
	setOpenPreviewHandler
} from './core'
import { UserDraft, __resetUserDraftForTesting } from '$lib/userDraft.svelte'
import { clearGlobalDrafts } from './userDraftAdapter'
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

// Read the value a tool persisted into the per-user DB draft layer. Post
// #9351 the headless global tools write through `DraftService`
// (mocked above as `draftDb`), not the in-tab `UserDraft` cache, so draft
// assertions read here instead of `UserDraft.get`.
function dbDraftValue<T = any>(kind: string, path: string): T | undefined {
	return draftDb.store.get(draftDb.key(kind, path))?.value as T | undefined
}

// Seed a draft directly into the DB layer (replaces test setup that used
// `UserDraft.save` to pre-populate a draft — that now debounces to the DB
// asynchronously, whereas this lands synchronously). The trailing
// `{ workspace }` arg the old `UserDraft.save` calls passed is accepted and
// ignored so the call sites convert by rename alone.
function seedDbDraft(kind: string, path: string, value: unknown, _opts?: unknown): void {
	draftDb.set(kind, path, value)
}

// Serialized view of every persisted draft value — used to assert secret
// values never reach the draft store.
function dbSnapshot(): string {
	return [...draftDb.store.entries()]
		.map(([k, v]) => `${k}: ${JSON.stringify(v.value)}`)
		.join('\n')
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
	beforeEach(async () => {
		__resetUserDraftForTesting()
		localStorage.clear()
		draftDb.reset()
		await clearGlobalDrafts(WORKSPACE)
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
		expect(dbSnapshot()).not.toContain('super-secret-token')
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

		expect(dbDraftValue('resource', 'f/resources/db')).toEqual({
			path: 'f/resources/db',
			description: 'existing database',
			args: { host: 'new.example.com', port: 5432 },
			labels: ['prod'],
			wsSpecific: true,
			resource_type: 'postgresql'
		})
		// Rev metadata (remoteRev) is in-memory only post #9351 — not persisted to the DB draft layer.
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

		expect(dbDraftValue('variable', 'f/secrets/api_key')).toEqual({
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
		})
		// Rev metadata (remoteRev) is in-memory only post #9351 — not persisted to the DB draft layer.
		expect(localStorageSnapshot()).not.toContain('new-secret-token')
		expect(dbSnapshot()).not.toContain('new-secret-token')
	})

	it('deploys secret variable drafts with ephemeral values only', async () => {
		await callGlobalTool('write_variable', {
			path: 'f/secrets/api_key',
			value: 'new-secret-token',
			is_secret: true,
			description: 'new description'
		})

		expect(
			dbDraftValue('variable', 'f/secrets/api_key')
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
		expect(dbSnapshot()).not.toContain('new-secret-token')

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
		expect(dbDraftValue('variable', 'f/secrets/api_key')).toBeUndefined()
		expect(localStorageSnapshot()).not.toContain('new-secret-token')
		expect(dbSnapshot()).not.toContain('new-secret-token')
	})

	it('does not deploy a secret variable draft when the ephemeral value is gone', async () => {
		seedDbDraft(
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

		expect(dbDraftValue('script', 'f/scripts/hello')).toMatchObject(
			{
				path: 'f/scripts/hello',
				summary: 'Hello script',
				language: 'bun',
				content
			}
		)
	})

	it('flags backend draft scripts and forwards path_prefix + limit', async () => {
		// Post #9351 the list tool sources script drafts from `listScripts`
		// (`includeDraftOnly` + the `draft_only`/`is_draft` flags), not a local
		// store — so path_prefix (pathStart) and limit (perPage) are forwarded to
		// the backend query, which does the filtering. The backend now filters
		// draft-only rows by `path_start` too (the (c) fix in scripts/flows/apps),
		// so a draft-only row under the prefix is returned and flagged here.
		vi.mocked(ScriptService.listScripts).mockResolvedValueOnce([
			{ path: 'f/matching/inside', summary: 'Inside draft', language: 'bun', draft_only: true }
		] as any)

		const raw = await callGlobalTool('list_workspace_items', {
			types: ['script'],
			path_prefix: 'f/matching/',
			limit: 1
		})

		expect(ScriptService.listScripts).toHaveBeenCalledWith(
			expect.objectContaining({ pathStart: 'f/matching/', perPage: 1, includeDraftOnly: true })
		)
		expect(JSON.parse(raw)).toEqual([
			expect.objectContaining({
				type: 'script',
				path: 'f/matching/inside',
				isDraft: true
			})
		])
	})

	it('requests draft-only apps and flags them via is_draft', async () => {
		// Regression guard: `listApps` must be called with `includeDraftOnly`
		// (the app call previously omitted it, so DB-only app drafts were missed),
		// and a draft row maps to `isDraft: true` (draft-only apps carry
		// `is_draft === true`).
		vi.mocked(AppService.listApps).mockResolvedValueOnce([
			{ path: 'f/apps/inflight', summary: 'In-flight app', draft_only: true, is_draft: true }
		] as any)

		const raw = await callGlobalTool('list_workspace_items', { types: ['app'] })

		expect(AppService.listApps).toHaveBeenCalledWith(
			expect.objectContaining({ includeDraftOnly: true })
		)
		expect(JSON.parse(raw)).toEqual([
			expect.objectContaining({ type: 'app', path: 'f/apps/inflight', isDraft: true })
		])
	})

	it('lists and edits the live script editor draft through its effective path', async () => {
		seedDbDraft(
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

		expect(dbDraftValue('script', '')).toMatchObject({
			path: 'u/admin/amazed_script',
			content: 'export async function main(a: number, b: number) {\n\treturn a * b\n}'
		})
		expect(
			dbDraftValue('script', 'u/admin/amazed_script')
		).toBeUndefined()
	})

	it('lists and writes the live flow editor draft through its effective path', async () => {
		seedDbDraft(
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

		expect(dbDraftValue('flow', '')).toMatchObject({
			path: 'u/admin/live_flow',
			summary: 'Updated live flow',
			value: { modules: [{ id: 'step', value: { type: 'identity' } }] }
		})
		expect(dbDraftValue('flow', 'u/admin/live_flow')).toBeUndefined()
	})

	it('writes the live raw app editor draft through its effective path', async () => {
		seedDbDraft(
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

		expect(dbDraftValue('raw_app', '')).toMatchObject({
			files: {
				'/src/App.tsx': 'export default function App() { return null }',
				'/src/New.tsx': 'export default function New() { return null }'
			}
		})
		expect(dbDraftValue('raw_app', 'u/admin/live_app')).toBeUndefined()
	})

	it('discards a local draft without deleting the workspace item', async () => {
		await callGlobalTool('write_script', {
			path: 'f/scripts/discard-me',
			summary: 'Temporary draft',
			language: 'bun',
			content: 'export async function main() { return 1 }'
		})

		expect(dbDraftValue('script', 'f/scripts/discard-me')).toBeDefined()

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
			dbDraftValue('script', 'f/scripts/discard-me')
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
			dbDraftValue('script', 'f/scripts/existing')
		).toMatchObject({
			path: 'f/scripts/existing',
			parent_hash: 'deployed-hash',
			summary: 'new summary',
			description: 'deployed description',
			content: 'new content',
			language: 'bun'
		})
		// Rev metadata (remoteRev) is in-memory only post #9351 — not persisted to the DB draft layer.
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

		expect(dbDraftValue('flow', 'f/flows/existing')).toMatchObject({
			path: 'f/flows/existing',
			summary: 'new summary',
			description: 'deployed description',
			value: { modules: [{ id: 'step', value: { type: 'identity' } }] }
		})
		// Rev metadata (remoteRev) is in-memory only post #9351 — not persisted to the DB draft layer.
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
			dbDraftValue('trigger_schedule', 'f/schedules/nightly')
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
			dbDraftValue('trigger_schedule', 'f/schedules/nightly')
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

		const draft = dbDraftValue('trigger_http', 'f/routes/api')
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

		const draft = dbDraftValue('raw_app', 'f/apps/report')
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
		// Rev metadata (remoteRev) is in-memory only post #9351 — not persisted to the DB draft layer.
	})

	it('summarizes local raw app drafts in read_workspace_item', async () => {
		seedDbDraft(
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
		expect(dbDraftValue('raw_app', 'f/apps/report')).toBeUndefined()
	})

	it('reads raw app files without creating a local draft', async () => {
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
		expect(dbDraftValue('raw_app', 'f/apps/report')).toBeUndefined()
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
		expect(dbDraftValue('raw_app', 'f/apps/report')).toBeUndefined()
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
		expect(dbDraftValue('raw_app', 'f/apps/report')).toBeUndefined()
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
		expect(dbDraftValue('raw_app', 'f/apps/report')).toBeUndefined()
	})

	it('deploys a new raw app draft by bundling files and creating a raw app', async () => {
		seedDbDraft(
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
		expect(dbDraftValue('raw_app', 'f/apps/report')).toBeUndefined()
		expect(JSON.parse(raw)).toMatchObject({
			success: true,
			type: 'app',
			path: 'f/apps/report'
		})
	})

	it('deploys an existing raw app draft by bundling files and updating the raw app', async () => {
		vi.mocked(AppService.existsApp).mockResolvedValueOnce(true)
		seedDbDraft(
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
		expect(dbDraftValue('raw_app', 'f/apps/report')).toBeUndefined()
	})

	it('notifies the session preview (as raw_app) after deploying a raw app', async () => {
		const onDeployed = vi.fn()
		setDeployedInSessionHandler(onDeployed)
		try {
			seedDbDraft(
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

		expect(writeResult.item.value.value).toBeUndefined()

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

	it('test_run_script previews local draft script content by path', async () => {
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

	it('test_run_script previews deployed script content when no local draft exists', async () => {
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

	it('test_run_flow previews local draft flow content by path', async () => {
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

	it('test_run_flow previews deployed flow content when no local draft exists', async () => {
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
		seedDbDraft(
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
		seedDbDraft(
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

	it('test_run_step previews rawscript steps from the local draft flow', async () => {
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

	it('test_run_step previews local draft subflows for flow steps', async () => {
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

		expect(content).toContain('Draft tools create or update local drafts only')
		expect(content).toContain(
			'Use discard_local_draft to remove an unsaved local draft, including the matching open editor draft'
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
			'Discard a local draft only. Does not mutate deployed workspace items, but clears the matching open editor draft if one is mounted.'
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
})

describe('session-only preview tools gating', () => {
	const toolNames = (sessionPreview: boolean) =>
		globalToolsFor({ sessionPreview }).map((t) => t.def.function.name)

	it('excludes open_preview / get_preview_status outside a session', () => {
		const names = toolNames(false)
		expect(names).not.toContain('open_preview')
		expect(names).not.toContain('get_preview_status')
		// other tools are still present
		expect(names).toContain('write_script')
	})

	it('includes open_preview / get_preview_status inside a session', () => {
		const names = toolNames(true)
		expect(names).toContain('open_preview')
		expect(names).toContain('get_preview_status')
		// session set is the full globalTools
		expect(names.length).toBe(globalTools.length)
	})

	it('mentions open_preview in the system prompt only when preview tools are enabled', () => {
		const off = prepareGlobalSystemMessage(undefined, { previewTools: false }).content as string
		const on = prepareGlobalSystemMessage(undefined, { previewTools: true }).content as string
		expect(off).not.toContain('open_preview')
		expect(on).toContain('open_preview')
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
