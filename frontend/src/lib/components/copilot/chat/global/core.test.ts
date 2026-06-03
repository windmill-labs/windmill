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

	// In-memory stand-in for the backend `draft`/item tables so script/flow/app
	// "write then read" round-trips work the way the real DB does: write_script /
	// write_flow / write_app_* now persist via DraftService.createDraft (+ a
	// draft_only anchor), and reads resolve through get*ByPathWithDraft. The store
	// mirrors that: createDraft / createScript / createFlow / createAppRaw register
	// a readable row, and the *WithDraft reads return it (404 when absent).
	// `row`, when present, is the full object a get*ByPathWithDraft read returns
	// (used to model a *deployed* item that also carries a DB draft). When absent,
	// the read synthesizes a draft_only row from `value`.
	type StoredDraft = { typ: 'script' | 'flow' | 'app'; value: any; anchored: boolean; row?: any }
	const dbDraftStore = new Map<string, StoredDraft>()
	const dbKey = (typ: string, path: string) => `${typ}:${path}`

	function notFound404() {
		return Object.assign(new Error('not found'), { status: 404 })
	}

	return {
		...actual,
		// Lets the test reset the in-memory DB-draft store between cases.
		__resetDbDraftStore: () => dbDraftStore.clear(),
		// Seeds a full get*ByPathWithDraft row (e.g. a deployed item + DB draft) so
		// every read in a test sees the same fixture without leaking mock impls.
		__seedDbRow: (typ: 'script' | 'flow' | 'app', path: string, row: any) =>
			dbDraftStore.set(dbKey(typ, path), { typ, value: row?.draft ?? row, anchored: true, row }),
		// A 404 from the generated client surfaces as a thrown ApiError-shaped
		// object with `.status === 404`. `readScriptDbDraft`/`readFlowDbDraft`/
		// `readAppDbDraft` catch *only* this case and treat it as "no item exists".
		ScriptService: wrapService(actual.ScriptService, {
			existsScriptByPath: vi.fn(async () => false),
			createScript: vi.fn(async ({ path, requestBody }: any) => {
				const p = path ?? requestBody?.path
				const existing = dbDraftStore.get(dbKey('script', p))
				dbDraftStore.set(dbKey('script', p), {
					typ: 'script',
					value: existing?.value,
					anchored: true
				})
				return 'created'
			}),
			getScriptByPath: vi.fn(async () => {
				throw new Error('getScriptByPath mock not configured')
			}),
			getScriptByHash: vi.fn(async () => {
				throw new Error('getScriptByHash mock not configured')
			}),
			getScriptByPathWithDraft: vi.fn(async ({ path }: any) => {
				const stored = dbDraftStore.get(dbKey('script', path))
				if (!stored) throw notFound404()
				if (stored.row) return stored.row
				return {
					path,
					hash: undefined,
					draft_only: true,
					draft: stored.value ?? null,
					...(stored.value ?? {})
				}
			}),
			queryHubScripts: vi.fn(async () => []),
			getHubScriptContentByPath: vi.fn(async () => ''),
			deleteScriptByPath: vi.fn(async () => 'deleted'),
			listScripts: vi.fn(async () => [])
		}),
		DraftService: wrapService(actual.DraftService, {
			createDraft: vi.fn(async ({ requestBody }: any) => {
				const { path, typ, value } = requestBody
				if (typ === 'script' || typ === 'flow' || typ === 'app') {
					const existing = dbDraftStore.get(dbKey(typ, path))
					dbDraftStore.set(dbKey(typ, path), {
						typ,
						value,
						anchored: existing?.anchored ?? false
					})
				}
				return 'created'
			}),
			// Deploy must NOT call this — the backend deletes the DB draft row on a
			// non-draft create/update. Mocked so deploy tests can assert it stays unused.
			deleteDraft: vi.fn(async () => undefined)
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
			createFlow: vi.fn(async ({ path, requestBody }: any) => {
				const p = path ?? requestBody?.path
				const existing = dbDraftStore.get(dbKey('flow', p))
				dbDraftStore.set(dbKey('flow', p), {
					typ: 'flow',
					value: existing?.value,
					anchored: true
				})
				return 'created'
			}),
			updateFlow: vi.fn(async () => 'updated'),
			getFlowByPath: vi.fn(async () => {
				throw new Error('getFlowByPath mock not configured')
			}),
			getFlowByPathWithDraft: vi.fn(async ({ path }: any) => {
				const stored = dbDraftStore.get(dbKey('flow', path))
				if (!stored) throw notFound404()
				if (stored.row) return stored.row
				return {
					path,
					draft_only: true,
					draft: stored.value ?? null,
					...(stored.value ?? {})
				}
			}),
			getFlowLatestVersion: vi.fn(async () => ({ id: 1 })),
			deleteFlowByPath: vi.fn(async () => 'deleted'),
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
			createAppRaw: vi.fn(async ({ path, formData }: any) => {
				// Mirrors the backend: a raw-app create registers a readable row (the
				// `draft_only` anchor when `formData.app.draft_only`). Preserve any
				// draft already upserted at this path so a later read resolves it.
				const p = path ?? formData?.app?.path
				const existing = dbDraftStore.get(dbKey('app', p))
				dbDraftStore.set(dbKey('app', p), {
					typ: 'app',
					value: existing?.value,
					anchored: true
				})
				return 'created'
			}),
			updateAppRaw: vi.fn(async () => 'updated'),
			getAppByPathWithDraft: vi.fn(async ({ path }: any) => {
				const stored = dbDraftStore.get(dbKey('app', path))
				if (!stored) throw notFound404()
				if (stored.row) return stored.row
				// Synthesize a draft_only row from the upserted draft value. The draft
				// `value` is `{ value: rawApp, path, summary, policy, custom_path }`,
				// exactly what `appSourceToDraftValue` reads back via `app.draft`.
				return {
					path,
					draft_only: true,
					draft: stored.value ?? null,
					...(stored.value ?? {})
				}
			}),
			deleteApp: vi.fn(async () => 'deleted'),
			listApps: vi.fn(async () => [])
		}),
		ResourceService: wrapService(actual.ResourceService, {
			existsResource: vi.fn(async () => false),
			getResource: vi.fn(async () => {
				throw new Error('getResource mock not configured')
			}),
			listResource: vi.fn(async () => [])
		}),
		VariableService: wrapService(actual.VariableService, {
			existsVariable: vi.fn(async () => false),
			getVariable: vi.fn(async () => {
				throw new Error('getVariable mock not configured')
			}),
			createVariable: vi.fn(async () => 'created'),
			updateVariable: vi.fn(async () => 'updated')
		})
	}
})

vi.mock('./rawAppBundlerBridge', () => ({
	bundleRawAppDraft: vi.fn(async () => ({
		js: 'bundled js',
		css: 'bundled css'
	}))
}))

// The real `inferArgs` loads a wasm parser that isn't available under vitest; stub
// it to populate the passed schema in place (mirroring the real mutation) so the
// deploy schema-inference step is exercised deterministically.
vi.mock('$lib/infer', () => ({
	inferArgs: vi.fn(async (_language: unknown, code: string, schema: any) => {
		const match = /main\(([^)]*)\)/.exec(code)
		const name = match?.[1]?.split(':')[0]?.trim()
		if (name) {
			schema.properties = { [name]: { type: 'number' } }
			schema.required = [name]
		}
		return { auto_kind: null, has_preprocessor: null }
	})
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
	DraftService,
	FlowService,
	HttpTriggerService,
	JobService,
	ResourceService,
	ScheduleService,
	ScriptService,
	VariableService
} from '$lib/gen'
import * as genModule from '$lib/gen'
import type { Tool, ToolCallbacks } from '../shared'

// Reset/seed helpers for the in-memory DB-draft store backing the script/flow
// $lib/gen mocks.
const { __resetDbDraftStore, __seedDbRow } = genModule as unknown as {
	__resetDbDraftStore: () => void
	__seedDbRow: (typ: 'script' | 'flow' | 'app', path: string, row: any) => void
}

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
		clearGlobalDrafts(WORKSPACE)
		__resetDbDraftStore()
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

		expect(UserDraft.get<any>('resource', 'f/resources/db', { workspace: WORKSPACE })).toEqual({
			path: 'f/resources/db',
			description: 'existing database',
			args: { host: 'new.example.com', port: 5432 },
			labels: ['prod'],
			wsSpecific: true,
			resource_type: 'postgresql'
		})
		expect(UserDraft.getMeta('resource', 'f/resources/db', { workspace: WORKSPACE })).toEqual({
			remoteRev: '2026-05-22T09:30:00Z'
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

		expect(UserDraft.get<any>('variable', 'f/secrets/api_key', { workspace: WORKSPACE })).toEqual({
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
		expect(UserDraft.getMeta('variable', 'f/secrets/api_key', { workspace: WORKSPACE })).toEqual({
			remoteRev: '2026-05-22T09:30:00Z'
		})
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
			UserDraft.get<any>('variable', 'f/secrets/api_key', { workspace: WORKSPACE })
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
		expect(UserDraft.get('variable', 'f/secrets/api_key', { workspace: WORKSPACE })).toBeUndefined()
		expect(localStorageSnapshot()).not.toContain('new-secret-token')
	})

	it('does not deploy a secret variable draft when the ephemeral value is gone', async () => {
		UserDraft.save(
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

	it('writes a brand-new script as a DB draft with a draft_only anchor', async () => {
		const content = 'export async function main() {\n\treturn "hello"\n}'

		const raw = await callGlobalTool('write_script', {
			path: 'f/scripts/hello',
			summary: 'Hello script',
			language: 'bun',
			content
		})

		// Brand-new item (getScriptByPathWithDraft 404s) -> a draft_only anchor is
		// created so the draft row is readable, then the draft row is upserted.
		expect(ScriptService.createScript).toHaveBeenCalledTimes(1)
		const anchor = vi.mocked(ScriptService.createScript).mock.calls[0][0]
		expect(anchor.workspace).toBe(WORKSPACE)
		expect(anchor.requestBody.draft_only).toBe(true)
		expect(anchor.requestBody.path).toBe('f/scripts/hello')

		expect(DraftService.createDraft).toHaveBeenCalledTimes(1)
		const draftCall = vi.mocked(DraftService.createDraft).mock.calls[0][0]
		expect(draftCall.workspace).toBe(WORKSPACE)
		expect(draftCall.requestBody).toMatchObject({
			path: 'f/scripts/hello',
			typ: 'script'
		})
		expect(draftCall.requestBody.value).toMatchObject({
			path: 'f/scripts/hello',
			summary: 'Hello script',
			language: 'bun',
			content
		})

		// No live editor open -> nothing mirrored to localStorage.
		expect(UserDraft.get('script', 'f/scripts/hello', { workspace: WORKSPACE })).toBeUndefined()

		// The tool result is built from the persisted value (not localStorage).
		const result = JSON.parse(raw)
		expect(result.success).toBe(true)
		expect(result.message).toContain('as a workspace draft')
		expect(result.message).toContain('not deployed')
		expect(result.item).toMatchObject({
			type: 'script',
			path: 'f/scripts/hello',
			summary: 'Hello script',
			language: 'bun',
			value: content,
			isDraft: true
		})
	})

	it('skips the draft_only anchor when the script item already exists', async () => {
		__seedDbRow('script', 'f/scripts/existing-anchor', {
			path: 'f/scripts/existing-anchor',
			hash: 'deployed-hash',
			summary: 'deployed',
			description: '',
			content: 'old',
			language: 'bun',
			kind: 'script'
		})

		await callGlobalTool('write_script', {
			path: 'f/scripts/existing-anchor',
			summary: 'Updated',
			language: 'bun',
			content: 'export async function main() { return 1 }'
		})

		expect(ScriptService.createScript).not.toHaveBeenCalled()
		expect(DraftService.createDraft).toHaveBeenCalledTimes(1)
	})

	it('mirrors a script write to localStorage when a live editor is open', async () => {
		// Open a live script editor whose effective path matches the write target.
		UserDraft.save(
			'script',
			'',
			{
				path: 'u/admin/live_mirror',
				summary: 'Live script',
				description: '',
				content: 'export async function main() { return 1 }',
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
			effectivePath: 'u/admin/live_mirror'
		})

		const content = 'export async function main() { return 2 }'
		await callGlobalTool('write_script', {
			path: 'u/admin/live_mirror',
			summary: 'Live script',
			language: 'bun',
			content
		})

		// DB draft is always written...
		expect(DraftService.createDraft).toHaveBeenCalledTimes(1)
		// ...and a live editor open -> the value is mirrored to localStorage at the
		// live editor's storage path ('') so the open editor updates immediately.
		expect(UserDraft.get<any>('script', '', { workspace: WORKSPACE })).toMatchObject({
			path: 'u/admin/live_mirror',
			content
		})
	})

	it('reads a deployed script with isDraft false and a DB draft with isDraft true', async () => {
		__seedDbRow('script', 'f/scripts/deployed-read', {
			path: 'f/scripts/deployed-read',
			hash: 'deployed-hash',
			draft_only: false,
			summary: 'Deployed',
			description: '',
			content: 'export async function main() { return 1 }',
			language: 'bun',
			kind: 'script'
		})

		const deployedRaw = await callGlobalTool('read_workspace_item', {
			type: 'script',
			path: 'f/scripts/deployed-read'
		})
		expect(JSON.parse(deployedRaw)).toMatchObject({
			type: 'script',
			path: 'f/scripts/deployed-read',
			isDraft: false
		})

		// Now write a draft over it; the read should flag isDraft true.
		await callGlobalTool('write_script', {
			path: 'f/scripts/deployed-read',
			summary: 'Deployed',
			language: 'bun',
			content: 'export async function main() { return 2 }'
		})
		const draftRaw = await callGlobalTool('read_workspace_item', {
			type: 'script',
			path: 'f/scripts/deployed-read'
		})
		const draftItem = JSON.parse(draftRaw)
		expect(draftItem).toMatchObject({
			type: 'script',
			path: 'f/scripts/deployed-read',
			value: 'export async function main() { return 2 }',
			isDraft: true
		})
	})

	it('reads a draft-only DB script anchor (no draft row) as a draft', async () => {
		// A bare draft_only anchor with no draft row resolves to source 'deployed'
		// (its own value is the only version), yet it IS a draft.
		__seedDbRow('script', 'f/scripts/draft-only', {
			path: 'f/scripts/draft-only',
			hash: 'draft-anchor-hash',
			draft_only: true,
			summary: 'draft-only summary',
			description: 'draft-only description',
			content: 'draft-only content',
			language: 'bun',
			kind: 'script'
		})

		const raw = await callGlobalTool('read_workspace_item', {
			type: 'script',
			path: 'f/scripts/draft-only'
		})
		expect(JSON.parse(raw)).toEqual({
			type: 'script',
			path: 'f/scripts/draft-only',
			summary: 'draft-only summary',
			language: 'bun',
			value: 'draft-only content',
			isDraft: true
		})
	})

	it('reads a draft-only DB flow anchor (no draft row) as a draft', async () => {
		__seedDbRow('flow', 'f/flows/draft-only', {
			path: 'f/flows/draft-only',
			draft_only: true,
			summary: 'draft-only flow summary',
			value: { modules: [{ id: 'draft_only_step', value: { type: 'identity' } }] },
			schema: { type: 'object', properties: { draftOnly: { type: 'boolean' } } }
		})

		const raw = await callGlobalTool('read_workspace_item', {
			type: 'flow',
			path: 'f/flows/draft-only'
		})
		expect(JSON.parse(raw)).toMatchObject({
			type: 'flow',
			path: 'f/flows/draft-only',
			summary: 'draft-only flow summary',
			isDraft: true,
			value: {
				modules: [{ id: 'draft_only_step', value: { type: 'identity' } }],
				schema: { type: 'object', properties: { draftOnly: { type: 'boolean' } } }
			}
		})
	})

	it('reads a draft-only DB app anchor (no draft row) as a draft', async () => {
		// The app anchor's `value` nests the raw app the way appSourceToDraftValue reads.
		__seedDbRow('app', 'f/apps/draft-only', {
			path: 'f/apps/draft-only',
			draft_only: true,
			summary: 'draft-only app summary',
			value: { files: { '/index.tsx': 'console.log("draft")' }, runnables: {}, data: { tables: [] } }
		})

		const raw = await callGlobalTool('read_workspace_item', {
			type: 'app',
			path: 'f/apps/draft-only'
		})
		expect(JSON.parse(raw)).toMatchObject({
			type: 'app',
			path: 'f/apps/draft-only',
			summary: 'draft-only app summary',
			isDraft: true
		})
	})

	it('applies path_prefix to local drafts before enforcing the result limit', async () => {
		// Group-B (resource/variable/…) drafts are localStorage-only and always
		// listed; use them to exercise path_prefix filtering happening BEFORE the
		// result limit (so a matching draft isn't dropped for a non-matching one).
		UserDraft.save(
			'resource',
			'f/other/outside',
			{ path: 'f/other/outside', description: '', args: {}, labels: [], wsSpecific: false },
			{ workspace: WORKSPACE }
		)
		UserDraft.save(
			'resource',
			'f/matching/inside',
			{ path: 'f/matching/inside', description: '', args: {}, labels: [], wsSpecific: false },
			{ workspace: WORKSPACE }
		)

		const raw = await callGlobalTool('list_workspace_items', {
			types: ['resource'],
			path_prefix: 'f/matching/',
			limit: 1
		})

		expect(JSON.parse(raw)).toEqual([
			expect.objectContaining({
				type: 'resource',
				path: 'f/matching/inside',
				isDraft: true
			})
		])
	})

	it('flags backend draft_only items as drafts and never double-lists them with localStorage', async () => {
		// The backend list (`includeDraftOnly: true`) already returns Group-A DB
		// drafts, including draft-only items that were never deployed. Those must be
		// flagged `isDraft: true`, deployed items `isDraft: false`, and a path that
		// ALSO has a localStorage live-editor mirror must appear exactly once.
		vi.mocked(ScriptService.listScripts).mockResolvedValueOnce([
			{
				path: 'f/list/deployed',
				summary: 'Deployed script',
				language: 'bun',
				content: '',
				draft_only: false
			},
			{
				path: 'f/list/db-draft-only',
				summary: 'Draft-only script',
				language: 'bun',
				content: '',
				draft_only: true
			},
			{
				path: 'f/list/live-mirrored',
				summary: 'Backend copy',
				language: 'bun',
				content: '',
				draft_only: true
			}
		] as any)

		// A live-editor localStorage draft for a path the backend list already
		// returns — the merge must dedupe by key, not list it twice.
		UserDraft.save(
			'script',
			'f/list/live-mirrored',
			{
				path: 'f/list/live-mirrored',
				summary: 'Live mirror',
				description: '',
				content: 'export async function main() { return 1 }',
				schema: {},
				is_template: false,
				language: 'bun',
				kind: 'script'
			},
			{ workspace: WORKSPACE }
		)

		const raw = await callGlobalTool('list_workspace_items', { types: ['script'] })
		const items = JSON.parse(raw) as Array<{ path: string; isDraft: boolean }>

		const byPath = (p: string) => items.filter((i) => i.path === p)
		expect(byPath('f/list/deployed')).toEqual([
			expect.objectContaining({ path: 'f/list/deployed', isDraft: false })
		])
		expect(byPath('f/list/db-draft-only')).toEqual([
			expect.objectContaining({ path: 'f/list/db-draft-only', isDraft: true })
		])
		// Exactly one entry despite appearing in BOTH the backend list and localStorage.
		expect(byPath('f/list/live-mirrored')).toHaveLength(1)
		expect(byPath('f/list/live-mirrored')[0]).toMatchObject({
			path: 'f/list/live-mirrored',
			isDraft: true
		})
	})

	it('list_workspace_items shows the fresh DB draft summary, not the stale anchor', async () => {
		// The draft_only anchor is intentionally NOT updated on a re-save, so its
		// list summary goes stale. A newer DB draft exists; the list must surface
		// the draft's summary and flag the row as a draft.
		__seedDbRow('script', 'f/scripts/stale-list-summary', {
			path: 'f/scripts/stale-list-summary',
			draft_only: true,
			draft: {
				path: 'f/scripts/stale-list-summary',
				summary: 'Fresh DB draft summary',
				description: '',
				content: 'export async function main() { return "fresh" }',
				language: 'bun',
				kind: 'script'
			}
		})
		vi.mocked(ScriptService.listScripts).mockResolvedValueOnce([
			{
				path: 'f/scripts/stale-list-summary',
				summary: 'Old deployed summary',
				language: 'bun',
				has_draft: true
			}
		] as any)

		const raw = await callGlobalTool('list_workspace_items', {
			types: ['script'],
			query: 'Fresh DB draft'
		})

		expect(JSON.parse(raw)).toEqual([
			expect.objectContaining({
				type: 'script',
				path: 'f/scripts/stale-list-summary',
				summary: 'Fresh DB draft summary',
				isDraft: true
			})
		])
		expect(raw).not.toContain('Old deployed summary')
	})

	it('lists a draft-only app (includeDraftOnly) and flags it as a draft', async () => {
		// `init_app` persists a `draft_only` app row; the backend excludes those
		// unless `includeDraftOnly` is passed, so the app list must request it or a
		// just-created app draft is invisible (scripts/flows already pass it).
		vi.mocked(AppService.listApps).mockResolvedValueOnce([
			{ path: 'f/apps/draft-only', summary: 'App draft', draft_only: true },
			{ path: 'f/apps/deployed', summary: 'Deployed app', draft_only: false }
		] as any)

		const raw = await callGlobalTool('list_workspace_items', { types: ['app'] })
		const items = JSON.parse(raw) as Array<{ path: string; isDraft: boolean }>

		expect(items.find((i) => i.path === 'f/apps/draft-only')).toMatchObject({ isDraft: true })
		expect(items.find((i) => i.path === 'f/apps/deployed')).toMatchObject({ isDraft: false })
		expect(vi.mocked(AppService.listApps).mock.calls[0]?.[0]).toMatchObject({
			includeDraftOnly: true
		})
	})

	it('does not list a stale (non-live) Group-A localStorage draft', async () => {
		// No live editor is registered, so `loadDraft` (read/deploy) ignores this
		// localStorage entry — the list must not advertise a draft they won't use.
		UserDraft.save(
			'script',
			'f/list/stale-local',
			{
				path: 'f/list/stale-local',
				summary: 'Stale local',
				description: '',
				content: '',
				schema: {},
				is_template: false,
				language: 'bun',
				kind: 'script'
			},
			{ workspace: WORKSPACE }
		)

		const raw = await callGlobalTool('list_workspace_items', { types: ['script'] })
		const items = JSON.parse(raw) as Array<{ path: string }>
		expect(items.find((i) => i.path === 'f/list/stale-local')).toBeUndefined()
	})

	it('lists a live-editor Group-A localStorage draft', async () => {
		UserDraft.setLiveEditorDraft({
			workspace: WORKSPACE,
			itemKind: 'script',
			storagePath: 'f/list/live-only'
		})
		UserDraft.save(
			'script',
			'f/list/live-only',
			{
				path: 'f/list/live-only',
				summary: 'Live only',
				description: '',
				content: '',
				schema: {},
				is_template: false,
				language: 'bun',
				kind: 'script'
			},
			{ workspace: WORKSPACE }
		)

		const raw = await callGlobalTool('list_workspace_items', { types: ['script'] })
		const items = JSON.parse(raw) as Array<{ path: string; isDraft: boolean }>
		expect(items.find((i) => i.path === 'f/list/live-only')).toMatchObject({ isDraft: true })
	})

	it('lists and edits the live script editor draft through its effective path', async () => {
		UserDraft.save(
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

		expect(UserDraft.get<any>('script', '', { workspace: WORKSPACE })).toMatchObject({
			path: 'u/admin/amazed_script',
			content: 'export async function main(a: number, b: number) {\n\treturn a * b\n}'
		})
		expect(
			UserDraft.get('script', 'u/admin/amazed_script', { workspace: WORKSPACE })
		).toBeUndefined()
	})

	it('lists and writes the live flow editor draft through its effective path', async () => {
		UserDraft.save(
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

		expect(UserDraft.get<any>('flow', '', { workspace: WORKSPACE })).toMatchObject({
			path: 'u/admin/live_flow',
			summary: 'Updated live flow',
			value: { modules: [{ id: 'step', value: { type: 'identity' } }] }
		})
		expect(UserDraft.get('flow', 'u/admin/live_flow', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('writes the live raw app editor draft through its effective path', async () => {
		UserDraft.save(
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

		expect(UserDraft.get<any>('raw_app', '', { workspace: WORKSPACE })).toMatchObject({
			files: {
				'/src/App.tsx': 'export default function App() { return null }',
				'/src/New.tsx': 'export default function New() { return null }'
			}
		})
		expect(UserDraft.get('raw_app', 'u/admin/live_app', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('discards a local draft without deleting the workspace item', async () => {
		// For Group B (and the localStorage live-editor mirror), discard_draft
		// operates on the localStorage layer, so seed a localStorage draft directly.
		UserDraft.save(
			'script',
			'f/scripts/discard-me',
			{
				path: 'f/scripts/discard-me',
				summary: 'Temporary draft',
				description: '',
				content: 'export async function main() { return 1 }',
				schema: {},
				is_template: false,
				language: 'bun',
				kind: 'script'
			},
			{ workspace: WORKSPACE }
		)

		expect(UserDraft.get('script', 'f/scripts/discard-me', { workspace: WORKSPACE })).toBeDefined()

		const raw = await callGlobalTool('discard_draft', {
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
			UserDraft.get('script', 'f/scripts/discard-me', { workspace: WORKSPACE })
		).toBeUndefined()
	})

	it('requires trigger_kind when discarding a trigger draft', async () => {
		await expect(
			callGlobalTool('discard_draft', {
				type: 'trigger',
				path: 'f/routes/missing-kind'
			})
		).rejects.toThrow('trigger_kind is required')
	})

	it('discards a DB draft over a deployed script without deleting the deployed item', async () => {
		// A deployed script that also carries a DB draft. Discard must remove only
		// the draft row and leave the deployed item intact.
		__seedDbRow('script', 'f/scripts/with-deployed', {
			path: 'f/scripts/with-deployed',
			hash: 'deployed-hash',
			draft_only: false,
			summary: 'deployed summary',
			content: 'old deployed content',
			language: 'bun',
			kind: 'script',
			draft: {
				path: 'f/scripts/with-deployed',
				summary: 'draft summary',
				content: 'export async function main() { return 1 }',
				language: 'bun',
				kind: 'script'
			}
		})

		const raw = await callGlobalTool('discard_draft', {
			type: 'script',
			path: 'f/scripts/with-deployed'
		})

		expect(DraftService.deleteDraft).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			kind: 'script',
			path: 'f/scripts/with-deployed'
		})
		// Deployed version exists -> the anchor/item must NOT be deleted.
		expect(ScriptService.deleteScriptByPath).not.toHaveBeenCalled()
		expect(
			UserDraft.get('script', 'f/scripts/with-deployed', { workspace: WORKSPACE })
		).toBeUndefined()
		expect(JSON.parse(raw)).toMatchObject({
			success: true,
			type: 'script',
			path: 'f/scripts/with-deployed'
		})
		expect(raw).toContain('deployed')
	})

	it('discards a draft_only script and deletes the anchor item', async () => {
		// Only a DB draft exists (no deployed version) -> delete the draft AND the
		// draft_only anchor so no empty shell remains.
		__seedDbRow('script', 'f/scripts/db-only', {
			path: 'f/scripts/db-only',
			draft_only: true,
			draft: {
				path: 'f/scripts/db-only',
				summary: 'DB draft script',
				content: 'export async function main() { return 1 }',
				language: 'bun',
				kind: 'script'
			}
		})

		await callGlobalTool('discard_draft', {
			type: 'script',
			path: 'f/scripts/db-only'
		})

		expect(DraftService.deleteDraft).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			kind: 'script',
			path: 'f/scripts/db-only'
		})
		expect(ScriptService.deleteScriptByPath).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			path: 'f/scripts/db-only',
			keepCaptures: true
		})
		// Anchor deletion happens BEFORE the draft row deletion (failure-resilient).
		expect(vi.mocked(ScriptService.deleteScriptByPath).mock.invocationCallOrder[0]).toBeLessThan(
			vi.mocked(DraftService.deleteDraft).mock.invocationCallOrder[0]
		)
	})

	it('keeps the DB draft when draft_only anchor deletion fails', async () => {
		// A draft_only script whose anchor delete is rejected (e.g. deployment rules).
		// Because the anchor is deleted first, the draft row must remain untouched.
		__seedDbRow('script', 'f/scripts/blocked-discard', {
			path: 'f/scripts/blocked-discard',
			draft_only: true,
			draft: {
				path: 'f/scripts/blocked-discard',
				summary: 'Temporary draft',
				content: 'export async function main() { return 1 }',
				language: 'bun',
				kind: 'script'
			}
		})
		vi.mocked(ScriptService.deleteScriptByPath).mockRejectedValueOnce(
			new Error('deployment rules blocked deletion')
		)

		await expect(
			callGlobalTool('discard_draft', {
				type: 'script',
				path: 'f/scripts/blocked-discard'
			})
		).rejects.toThrow('deployment rules blocked deletion')

		expect(DraftService.deleteDraft).not.toHaveBeenCalled()
	})

	it('discards a DB draft over a deployed flow without deleting the deployed item', async () => {
		__seedDbRow('flow', 'f/flows/with-deployed', {
			path: 'f/flows/with-deployed',
			draft_only: false,
			summary: 'deployed summary',
			value: { modules: [] },
			draft: {
				path: 'f/flows/with-deployed',
				summary: 'draft summary',
				value: { modules: [] }
			}
		})

		await callGlobalTool('discard_draft', {
			type: 'flow',
			path: 'f/flows/with-deployed'
		})

		expect(DraftService.deleteDraft).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			kind: 'flow',
			path: 'f/flows/with-deployed'
		})
		expect(FlowService.deleteFlowByPath).not.toHaveBeenCalled()
	})

	it('discards a draft_only flow and deletes the anchor item', async () => {
		__seedDbRow('flow', 'f/flows/db-only', {
			path: 'f/flows/db-only',
			draft_only: true,
			draft: {
				path: 'f/flows/db-only',
				summary: 'DB draft flow',
				value: { modules: [] }
			}
		})

		await callGlobalTool('discard_draft', {
			type: 'flow',
			path: 'f/flows/db-only'
		})

		expect(DraftService.deleteDraft).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			kind: 'flow',
			path: 'f/flows/db-only'
		})
		expect(FlowService.deleteFlowByPath).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			path: 'f/flows/db-only',
			keepCaptures: true
		})
	})

	it('discards a DB draft over a deployed app without deleting the deployed item', async () => {
		__seedDbRow('app', 'f/apps/with-deployed', {
			path: 'f/apps/with-deployed',
			draft_only: false,
			summary: 'deployed report',
			versions: [1],
			value: { files: {}, runnables: {}, data: { tables: [] } },
			policy: { execution_mode: 'publisher' },
			draft: {
				summary: 'Updated report',
				value: { files: { '/index.tsx': 'x' }, runnables: {}, data: { tables: [] } }
			}
		})

		await callGlobalTool('discard_draft', {
			type: 'app',
			path: 'f/apps/with-deployed'
		})

		expect(DraftService.deleteDraft).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			kind: 'app',
			path: 'f/apps/with-deployed'
		})
		expect(AppService.deleteApp).not.toHaveBeenCalled()
	})

	it('discards a draft_only app and deletes the anchor item', async () => {
		__seedDbRow('app', 'f/apps/db-only', {
			path: 'f/apps/db-only',
			draft_only: true,
			draft: {
				summary: 'AI report',
				value: { files: { '/index.tsx': 'x' }, runnables: {}, data: { tables: [] } }
			}
		})

		await callGlobalTool('discard_draft', {
			type: 'app',
			path: 'f/apps/db-only'
		})

		expect(DraftService.deleteDraft).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			kind: 'app',
			path: 'f/apps/db-only'
		})
		expect(AppService.deleteApp).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			path: 'f/apps/db-only'
		})
	})

	it('errors when discarding a Group A draft that does not exist', async () => {
		await expect(
			callGlobalTool('discard_draft', {
				type: 'script',
				path: 'f/scripts/missing'
			})
		).rejects.toThrow('No draft found')
		expect(DraftService.deleteDraft).not.toHaveBeenCalled()
		expect(ScriptService.deleteScriptByPath).not.toHaveBeenCalled()
	})

	it('discards a Group B (resource) draft via localStorage only', async () => {
		UserDraft.save(
			'resource',
			'f/resources/discard-me',
			{
				path: 'f/resources/discard-me',
				value: { token: 'secret' },
				resource_type: 'c_custom'
			},
			{ workspace: WORKSPACE }
		)
		expect(
			UserDraft.get('resource', 'f/resources/discard-me', { workspace: WORKSPACE })
		).toBeDefined()

		await callGlobalTool('discard_draft', {
			type: 'resource',
			path: 'f/resources/discard-me'
		})

		// Group B never touches the DB-draft / item services.
		expect(DraftService.deleteDraft).not.toHaveBeenCalled()
		expect(
			UserDraft.get('resource', 'f/resources/discard-me', { workspace: WORKSPACE })
		).toBeUndefined()
	})

	it('merges the existing DB draft and anchors to the deployed hash on a script write', async () => {
		// A deployed script that already carries a DB draft. `loadDraft` resolves
		// to the DB draft as the merge base; the deployed hash becomes parent_hash.
		__seedDbRow('script', 'f/scripts/existing', {
			path: 'f/scripts/existing',
			hash: 'deployed-hash',
			draft_only: false,
			draft_created_at: '2026-05-22T10:00:00Z',
			summary: 'deployed summary',
			description: 'deployed description',
			content: 'old deployed content',
			language: 'bun',
			kind: 'script',
			draft: {
				path: 'f/scripts/existing',
				summary: 'db draft summary',
				description: 'db draft description',
				content: 'old draft content',
				language: 'bun',
				kind: 'script'
			}
		})

		await callGlobalTool('write_script', {
			path: 'f/scripts/existing',
			summary: 'new summary',
			language: 'bun',
			content: 'new content'
		})

		// Item already exists -> no anchor; only the draft row is upserted.
		expect(ScriptService.createScript).not.toHaveBeenCalled()
		expect(DraftService.createDraft).toHaveBeenCalledTimes(1)
		const value = vi.mocked(DraftService.createDraft).mock.calls[0][0].requestBody.value as any
		expect(value).toMatchObject({
			path: 'f/scripts/existing',
			parent_hash: 'deployed-hash',
			summary: 'new summary',
			description: 'db draft description',
			content: 'new content',
			language: 'bun'
		})
		// No live editor -> nothing mirrored to localStorage.
		expect(UserDraft.get('script', 'f/scripts/existing', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('merges the existing DB draft on a flow write', async () => {
		vi.mocked(FlowService.getFlowLatestVersion).mockResolvedValueOnce({ id: 42 } as any)
		__seedDbRow('flow', 'f/flows/existing', {
			path: 'f/flows/existing',
			draft_only: false,
			summary: 'deployed summary',
			description: 'deployed description',
			value: { modules: [] },
			schema: { properties: { deployed: { type: 'boolean' } } },
			edited_by: 'admin',
			edited_at: '2026-05-22T09:00:00Z',
			archived: false,
			extra_perms: {},
			draft_created_at: '2026-05-22T10:00:00Z',
			draft: {
				path: 'f/flows/existing',
				summary: 'db draft summary',
				description: 'db draft description',
				value: { modules: [] },
				schema: { properties: { draft: { type: 'string' } } },
				edited_by: 'admin',
				edited_at: '2026-05-22T09:30:00Z',
				archived: false,
				extra_perms: {}
			}
		})

		await callGlobalTool('write_flow', {
			path: 'f/flows/existing',
			summary: 'new summary',
			modules: JSON.stringify([{ id: 'step', value: { type: 'identity' } }])
		})

		expect(FlowService.createFlow).not.toHaveBeenCalled()
		expect(DraftService.createDraft).toHaveBeenCalledTimes(1)
		const value = vi.mocked(DraftService.createDraft).mock.calls[0][0].requestBody.value as any
		expect(value).toMatchObject({
			path: 'f/flows/existing',
			summary: 'new summary',
			description: 'db draft description',
			value: { modules: [{ id: 'step', value: { type: 'identity' } }] }
		})
		expect(UserDraft.get('flow', 'f/flows/existing', { workspace: WORKSPACE })).toBeUndefined()
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
			UserDraft.get<any>('trigger_schedule', 'f/schedules/nightly', { workspace: WORKSPACE })
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
			UserDraft.get<any>('trigger_schedule', 'f/schedules/nightly', { workspace: WORKSPACE })
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

		const draft = UserDraft.get<any>('trigger_http', 'f/routes/api', { workspace: WORKSPACE })
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

	it('upserts the DB draft (no anchor, no localStorage) when an app already has a draft', async () => {
		// A deployed app that already carries a DB draft. `loadDraft` resolves the
		// existing DB draft as the merge base; `write_app_file` adds a file and
		// re-upserts the draft row. No live editor -> nothing mirrored to localStorage.
		__seedDbRow('app', 'f/apps/report', {
			path: 'f/apps/report',
			summary: 'deployed app',
			versions: [3, 4],
			draft_created_at: '2026-05-22T10:30:00Z',
			value: {
				files: { '/src/App.tsx': 'deployed content' },
				runnables: {},
				data: { tables: [] }
			},
			policy: { execution_mode: 'publisher' },
			custom_path: 'report',
			draft: {
				summary: 'saved app draft',
				value: {
					files: { '/src/App.tsx': 'draft content' },
					runnables: {
						main: {
							type: 'inline',
							inlineScript: { language: 'bun', content: 'export async function main() {}' }
						}
					},
					data: { tables: ['orders'], datatable: 'db', schema: 'public' }
				},
				policy: { execution_mode: 'anonymous' }
			}
		})

		await callGlobalTool('write_app_file', {
			path: 'f/apps/report',
			file_path: '/src/New.tsx',
			content: 'export default function New() { return null }'
		})

		// Existing item -> no anchor created; only the draft row is upserted.
		expect(AppService.createAppRaw).not.toHaveBeenCalled()
		expect(DraftService.createDraft).toHaveBeenCalledTimes(1)
		const draftCall = vi.mocked(DraftService.createDraft).mock.calls[0][0]
		expect(draftCall.workspace).toBe(WORKSPACE)
		expect(draftCall.requestBody).toMatchObject({ path: 'f/apps/report', typ: 'app' })
		// The DB draft value nests the raw app under `.value`, like the raw-app editor.
		const dbValue = draftCall.requestBody.value as any
		expect(dbValue.value).toMatchObject({
			files: {
				'/src/App.tsx': 'draft content',
				'/src/New.tsx': 'export default function New() { return null }'
			},
			runnables: {
				main: {
					type: 'inline',
					inlineScript: { language: 'bun', content: 'export async function main() {}' }
				}
			},
			data: { tables: ['orders'], datatable: 'db', schema: 'public' }
		})
		expect(dbValue.summary).toBe('saved app draft')
		expect(dbValue.policy).toEqual({ execution_mode: 'anonymous' })
		expect(dbValue.custom_path).toBe('report')

		// No live editor open -> nothing mirrored to localStorage.
		expect(UserDraft.get('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('creates a draft_only raw-app anchor (bundling) + a DB draft for a brand-new app', async () => {
		// `init_app` is the only entry that creates a brand-new app: no row exists yet
		// (existsApp false, no draft), so the DB-draft write must bundle and create a
		// `draft_only` anchor so the draft row is readable, then upsert the draft row.
		const raw = await callGlobalTool('init_app', {
			path: 'f/apps/fresh',
			summary: 'Fresh app',
			framework: 'react19'
		})

		// A brand-new app -> the bundle runs and a draft_only raw-app anchor is created.
		expect(bundleRawAppDraft).toHaveBeenCalledTimes(1)
		expect(AppService.createAppRaw).toHaveBeenCalledTimes(1)
		const anchor = vi.mocked(AppService.createAppRaw).mock.calls[0][0] as any
		expect(anchor.workspace).toBe(WORKSPACE)
		expect(anchor.formData.app.draft_only).toBe(true)
		expect(anchor.formData.app.path).toBe('f/apps/fresh')
		// A policy is required to create the anchor; `init_app` computes it first.
		expect(anchor.formData.app.policy).toBeTruthy()
		expect(anchor.formData.js).toBe('bundled js')
		expect(anchor.formData.css).toBe('bundled css')

		// ...then the draft row is upserted under typ 'app'.
		expect(DraftService.createDraft).toHaveBeenCalledTimes(1)
		const draftCall = vi.mocked(DraftService.createDraft).mock.calls[0][0]
		expect(draftCall.requestBody).toMatchObject({ path: 'f/apps/fresh', typ: 'app' })

		// No live editor open -> nothing mirrored to localStorage.
		expect(UserDraft.get('raw_app', 'f/apps/fresh', { workspace: WORKSPACE })).toBeUndefined()

		const result = JSON.parse(raw)
		expect(result.success).toBe(true)
		expect(result.item).toMatchObject({ type: 'app', path: 'f/apps/fresh', isDraft: true })
	})

	it('prefers the DB draft over the deployed value when reading an app file', async () => {
		// Deployed app that also carries a DB draft. `loadDraft` resolves the DB draft,
		// so `read_app_file` returns the draft content, not the deployed content.
		__seedDbRow('app', 'f/apps/prefers-draft', {
			path: 'f/apps/prefers-draft',
			summary: 'deployed app',
			versions: [7],
			value: {
				files: { '/src/App.tsx': 'deployed content' },
				runnables: {},
				data: { tables: [] }
			},
			draft: {
				summary: 'saved app draft',
				value: {
					files: { '/src/App.tsx': 'draft content' },
					runnables: {},
					data: { tables: [] }
				}
			}
		})

		await expect(
			callGlobalTool('read_app_file', {
				path: 'f/apps/prefers-draft',
				file_path: '/src/App.tsx'
			})
		).resolves.toBe('draft content')
	})

	it('mirrors an app write to localStorage when a live editor is open', async () => {
		// Open a live raw-app editor whose effective path matches the write target.
		UserDraft.save(
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
			effectivePath: 'u/admin/mirror_app'
		})

		await callGlobalTool('write_app_file', {
			path: 'u/admin/mirror_app',
			file_path: '/src/New.tsx',
			content: 'export default function New() { return null }'
		})

		// DB draft is always written...
		expect(DraftService.createDraft).toHaveBeenCalledTimes(1)
		expect(vi.mocked(DraftService.createDraft).mock.calls[0][0].requestBody.typ).toBe('app')
		// ...and a live editor open -> the value is mirrored to localStorage at the
		// live editor's storage path ('') so the open editor updates immediately.
		expect(UserDraft.get<any>('raw_app', '', { workspace: WORKSPACE })).toMatchObject({
			files: {
				'/src/App.tsx': 'export default function App() { return null }',
				'/src/New.tsx': 'export default function New() { return null }'
			}
		})
		// Nothing written at the path-keyed slot (the non-live mirror location).
		expect(
			UserDraft.get('raw_app', 'u/admin/mirror_app', { workspace: WORKSPACE })
		).toBeUndefined()
	})

	it('summarizes a live-editor raw app draft in read_workspace_item', async () => {
		// App reads route through `loadDraft`, which only reads the localStorage
		// mirror when a live editor is open (source 'live'); otherwise it reads the
		// DB. Seed the mirror AND open a live editor so this exercises the live path.
		UserDraft.save(
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
		UserDraft.setLiveEditorDraft({
			workspace: WORKSPACE,
			itemKind: 'raw_app',
			storagePath: 'f/apps/local',
			effectivePath: 'f/apps/local'
		})

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

	it('summarizes backend raw app drafts from the same source as file reads', async () => {
		const appWithDraft = {
			path: 'f/apps/report',
			summary: 'deployed app',
			versions: [5],
			value: {
				files: { '/src/App.tsx': 'deployed content' },
				runnables: {},
				data: { tables: ['deployed'] }
			},
			draft: {
				summary: 'saved app draft',
				value: {
					files: {
						'/src/App.tsx': 'draft content',
						'/src/DraftOnly.tsx': 'draft-only content'
					},
					runnables: {
						main: {
							type: 'inline',
							inlineScript: {
								language: 'bun',
								content: 'export async function main() { return "draft" }'
							}
						}
					},
					data: { tables: ['draft'] }
				}
			}
		}
		vi.mocked(AppService.getAppByPathWithDraft)
			.mockResolvedValueOnce(appWithDraft as any)
			.mockResolvedValueOnce(appWithDraft as any)

		const raw = await callGlobalTool('read_workspace_item', {
			type: 'app',
			path: 'f/apps/report'
		})
		const item = JSON.parse(raw)

		expect(raw).not.toContain('draft-only content')
		expect(item).toMatchObject({
			type: 'app',
			path: 'f/apps/report',
			summary: 'saved app draft',
			value: {
				frontend: [
					{ path: '/src/App.tsx', size: 'draft content'.length },
					{ path: '/src/DraftOnly.tsx', size: 'draft-only content'.length }
				],
				backend: [
					expect.objectContaining({
						key: 'main',
						name: 'main',
						type: 'inline',
						language: 'bun',
						contentSize: 'export async function main() { return "draft" }'.length
					})
				],
				data: { tables: ['draft'] }
			},
			// The value resolved from the DB draft (not the deployed version), so the
			// item is flagged as a draft.
			isDraft: true
		})

		await expect(
			callGlobalTool('read_app_file', {
				path: 'f/apps/report',
				file_path: '/src/DraftOnly.tsx'
			})
		).resolves.toBe('draft-only content')
		expect(UserDraft.get('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('reads raw app files without creating a local draft', async () => {
		vi.mocked(AppService.getAppByPathWithDraft).mockResolvedValueOnce({
			path: 'f/apps/report',
			summary: 'deployed app',
			versions: [5],
			value: {
				files: { '/src/App.tsx': 'deployed content' },
				runnables: {},
				data: { tables: [] }
			},
			draft: {
				summary: 'saved app draft',
				value: {
					files: { '/src/App.tsx': 'draft content' },
					runnables: {},
					data: { tables: [] }
				}
			}
		} as any)

		await expect(
			callGlobalTool('read_app_file', {
				path: 'f/apps/report',
				file_path: '/src/App.tsx'
			})
		).resolves.toBe('draft content')
		expect(UserDraft.get('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('does not persist a raw app draft when patch_app_file validation fails', async () => {
		vi.mocked(AppService.getAppByPathWithDraft).mockResolvedValueOnce({
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
		expect(UserDraft.get('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('does not persist a raw app draft when delete_app_file validation fails', async () => {
		vi.mocked(AppService.getAppByPathWithDraft).mockResolvedValueOnce({
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
		expect(UserDraft.get('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('does not persist a raw app draft when delete_app_runnable validation fails', async () => {
		vi.mocked(AppService.getAppByPathWithDraft).mockResolvedValueOnce({
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
		expect(UserDraft.get('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('deploys a DB-only script draft via createScript with inferred schema (no deployed version)', async () => {
		// A brand-new draft_only script: only a DB draft exists, no deployed version.
		__seedDbRow('script', 'f/scripts/db-only', {
			path: 'f/scripts/db-only',
			draft_only: true,
			draft: {
				path: 'f/scripts/db-only',
				summary: 'DB draft script',
				description: '',
				content: 'export async function main(x: number) { return x }',
				language: 'bun',
				kind: 'script'
			}
		})

		await callGlobalTool('deploy_workspace_item', {
			type: 'script',
			path: 'f/scripts/db-only',
			deployment_message: 'ship it'
		})

		// Deploys the DB draft content (no deployed version -> no parent_hash). The
		// schema is inferred from the content like the editor does.
		expect(ScriptService.createScript).toHaveBeenCalledTimes(1)
		const call = vi.mocked(ScriptService.createScript).mock.calls[0][0] as any
		expect(call.workspace).toBe(WORKSPACE)
		expect(call.requestBody).toMatchObject({
			path: 'f/scripts/db-only',
			summary: 'DB draft script',
			content: 'export async function main(x: number) { return x }',
			language: 'bun',
			deployment_message: 'ship it'
		})
		expect(call.requestBody.parent_hash).toBeUndefined()
		expect(call.requestBody.schema?.properties).toHaveProperty('x')
		// draft_only on a create would re-create an anchor; deploy must NOT set it.
		expect(call.requestBody.draft_only).toBeUndefined()

		// The backend deletes the DB draft row on deploy; the frontend must NOT.
		expect(DraftService.deleteDraft).not.toHaveBeenCalled()
		// localStorage stays clean (no live editor was open).
		expect(UserDraft.get('script', 'f/scripts/db-only', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('anchors a deployed script to its parent_hash when deploying a DB draft over it', async () => {
		// A deployed script that also carries a DB draft. The deployed hash becomes
		// the parent_hash on the new deploy.
		__seedDbRow('script', 'f/scripts/with-deployed', {
			path: 'f/scripts/with-deployed',
			hash: 'deployed-hash',
			draft_only: false,
			summary: 'deployed summary',
			description: 'deployed description',
			content: 'old deployed content',
			language: 'bun',
			kind: 'script',
			draft: {
				path: 'f/scripts/with-deployed',
				summary: 'draft summary',
				description: '',
				content: 'export async function main() { return 1 }',
				language: 'bun',
				kind: 'script'
			}
		})
		vi.mocked(ScriptService.existsScriptByPath).mockResolvedValueOnce(true)
		vi.mocked(ScriptService.getScriptByPath).mockResolvedValueOnce({
			path: 'f/scripts/with-deployed',
			hash: 'deployed-hash',
			summary: 'deployed summary',
			description: 'deployed description',
			content: 'old deployed content',
			language: 'bun',
			kind: 'script'
		} as any)

		await callGlobalTool('deploy_workspace_item', {
			type: 'script',
			path: 'f/scripts/with-deployed'
		})

		expect(ScriptService.createScript).toHaveBeenCalledTimes(1)
		const call = vi.mocked(ScriptService.createScript).mock.calls[0][0] as any
		expect(call.requestBody).toMatchObject({
			path: 'f/scripts/with-deployed',
			content: 'export async function main() { return 1 }',
			parent_hash: 'deployed-hash'
		})
		expect(DraftService.deleteDraft).not.toHaveBeenCalled()
	})

	it('deploys DB script drafts with the draft metadata (not the deployed version)', async () => {
		// A deployed script with a DB draft whose metadata (tag/timeout/labels)
		// differs. The deploy must carry the DRAFT's metadata, parent on the
		// deployed hash, and re-infer the schema from the draft content.
		__seedDbRow('script', 'f/scripts/existing', {
			path: 'f/scripts/existing',
			hash: 'deployed-hash',
			draft_only: false,
			summary: 'deployed summary',
			description: 'deployed description',
			content: 'deployed content',
			language: 'bun',
			kind: 'script',
			tag: 'deployed-tag',
			envs: ['DEPLOYED_ENV'],
			timeout: 60,
			visible_to_runner_only: true,
			labels: ['deployed'],
			draft: {
				path: 'f/scripts/existing',
				summary: 'db draft summary',
				description: 'db draft description',
				content: 'export async function main(x: number) { return x }',
				language: 'bun',
				kind: 'script',
				tag: 'draft-tag',
				envs: [],
				timeout: 0,
				visible_to_runner_only: false,
				labels: []
			}
		})

		await callGlobalTool('deploy_workspace_item', {
			type: 'script',
			path: 'f/scripts/existing',
			deployment_message: 'ship db draft'
		})

		expect(ScriptService.createScript).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: expect.objectContaining({
				path: 'f/scripts/existing',
				parent_hash: 'deployed-hash',
				summary: 'db draft summary',
				description: 'db draft description',
				content: 'export async function main(x: number) { return x }',
				language: 'bun',
				tag: 'draft-tag',
				envs: [],
				timeout: 0,
				visible_to_runner_only: false,
				labels: [],
				deployment_message: 'ship db draft'
			})
		})
		// Schema is re-inferred from the draft content.
		const body = vi.mocked(ScriptService.createScript).mock.calls[0][0].requestBody as any
		expect(body.schema?.properties).toHaveProperty('x')
		expect(DraftService.deleteDraft).not.toHaveBeenCalled()
	})

	it('deploys a renamed script draft to the draft target path', async () => {
		// The DB draft carries a renamed path; deploy must target that path and
		// clean up the stale old-lookup-path draft + draft_only anchor.
		__seedDbRow('script', 'f/scripts/old-name', {
			path: 'f/scripts/old-name',
			hash: 'old-script-hash',
			draft_only: false,
			summary: 'deployed summary',
			description: 'deployed description',
			content: 'deployed content',
			language: 'bun',
			kind: 'script',
			draft: {
				path: 'f/scripts/new-name',
				summary: 'renamed draft summary',
				description: 'renamed draft description',
				content: 'renamed draft content',
				language: 'bun',
				kind: 'script'
			}
		})

		const raw = await callGlobalTool('deploy_workspace_item', {
			type: 'script',
			path: 'f/scripts/old-name'
		})

		expect(ScriptService.createScript).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: expect.objectContaining({
				path: 'f/scripts/new-name',
				parent_hash: 'old-script-hash',
				summary: 'renamed draft summary',
				description: 'renamed draft description',
				content: 'renamed draft content'
			})
		})
		// Deployed version existed -> the old anchor is NOT deleted, only the stale
		// draft row at the old path.
		expect(ScriptService.deleteScriptByPath).not.toHaveBeenCalled()
		expect(DraftService.deleteDraft).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			kind: 'script',
			path: 'f/scripts/old-name'
		})
		expect(JSON.parse(raw)).toMatchObject({
			success: true,
			type: 'script',
			path: 'f/scripts/new-name'
		})
	})

	it('removes the old draft_only anchor when deploying a renamed draft-only script', async () => {
		// A never-deployed (draft_only) script renamed in its draft. Deploy targets
		// the new path and must remove the stale old-path draft AND its anchor.
		__seedDbRow('script', 'f/scripts/old-only', {
			path: 'f/scripts/old-only',
			draft_only: true,
			draft: {
				path: 'f/scripts/new-only',
				summary: 'renamed draft-only summary',
				description: '',
				content: 'export async function main() { return 1 }',
				language: 'bun',
				kind: 'script'
			}
		})

		await callGlobalTool('deploy_workspace_item', {
			type: 'script',
			path: 'f/scripts/old-only'
		})

		expect(ScriptService.createScript).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			requestBody: expect.objectContaining({ path: 'f/scripts/new-only' })
		})
		expect(DraftService.deleteDraft).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			kind: 'script',
			path: 'f/scripts/old-only'
		})
		expect(ScriptService.deleteScriptByPath).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			path: 'f/scripts/old-only',
			keepCaptures: true
		})
	})

	it('does NOT call deleteDraft on a non-rename deploy (the backend handles it)', async () => {
		__seedDbRow('script', 'f/scripts/same-path', {
			path: 'f/scripts/same-path',
			draft_only: true,
			draft: {
				path: 'f/scripts/same-path',
				summary: 'DB draft',
				description: '',
				content: 'export async function main() { return 1 }',
				language: 'bun',
				kind: 'script'
			}
		})

		await callGlobalTool('deploy_workspace_item', {
			type: 'script',
			path: 'f/scripts/same-path'
		})

		expect(ScriptService.createScript).toHaveBeenCalledTimes(1)
		expect(DraftService.deleteDraft).not.toHaveBeenCalled()
		expect(ScriptService.deleteScriptByPath).not.toHaveBeenCalled()
	})

	it('errors when deploying a script that has only a deployed version (no draft)', async () => {
		__seedDbRow('script', 'f/scripts/deployed-only', {
			path: 'f/scripts/deployed-only',
			hash: 'deployed-hash',
			draft_only: false,
			summary: 'deployed',
			description: '',
			content: 'export async function main() { return 1 }',
			language: 'bun',
			kind: 'script'
			// no `draft` -> source resolves to 'deployed'
		})

		await expect(
			callGlobalTool('deploy_workspace_item', {
				type: 'script',
				path: 'f/scripts/deployed-only'
			})
		).rejects.toThrow('No draft changes to deploy')
		expect(ScriptService.createScript).not.toHaveBeenCalled()
		expect(DraftService.deleteDraft).not.toHaveBeenCalled()
	})

	it('errors when deploying a script with no draft at all', async () => {
		await expect(
			callGlobalTool('deploy_workspace_item', {
				type: 'script',
				path: 'f/scripts/missing'
			})
		).rejects.toThrow('No draft found')
		expect(ScriptService.createScript).not.toHaveBeenCalled()
	})

	it('deploys a DB-only flow draft via createFlow (no deployed version)', async () => {
		__seedDbRow('flow', 'f/flows/db-only', {
			path: 'f/flows/db-only',
			draft_only: true,
			draft: {
				path: 'f/flows/db-only',
				summary: 'DB draft flow',
				description: '',
				value: { modules: [{ id: 'step', value: { type: 'identity' } }] },
				schema: { properties: { a: { type: 'string' } } }
			}
		})

		await callGlobalTool('deploy_workspace_item', {
			type: 'flow',
			path: 'f/flows/db-only',
			deployment_message: 'ship flow'
		})

		// No deployed version -> createFlow (not updateFlow).
		expect(FlowService.createFlow).toHaveBeenCalledTimes(1)
		expect(FlowService.updateFlow).not.toHaveBeenCalled()
		const call = vi.mocked(FlowService.createFlow).mock.calls[0][0] as any
		expect(call.workspace).toBe(WORKSPACE)
		expect(call.requestBody).toMatchObject({
			path: 'f/flows/db-only',
			summary: 'DB draft flow',
			value: { modules: [{ id: 'step', value: { type: 'identity' } }] },
			schema: { properties: { a: { type: 'string' } } },
			deployment_message: 'ship flow'
		})
		expect(DraftService.deleteDraft).not.toHaveBeenCalled()
		expect(UserDraft.get('flow', 'f/flows/db-only', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('deploys a DB flow draft over a deployed flow via updateFlow', async () => {
		__seedDbRow('flow', 'f/flows/with-deployed', {
			path: 'f/flows/with-deployed',
			draft_only: false,
			summary: 'deployed summary',
			description: 'deployed description',
			value: { modules: [] },
			schema: { properties: { deployed: { type: 'boolean' } } },
			draft: {
				path: 'f/flows/with-deployed',
				summary: 'draft summary',
				description: '',
				value: { modules: [{ id: 'step', value: { type: 'identity' } }] },
				schema: { properties: { a: { type: 'string' } } }
			}
		})
		vi.mocked(FlowService.existsFlowByPath).mockResolvedValueOnce(true)
		vi.mocked(FlowService.getFlowByPath).mockResolvedValueOnce({
			path: 'f/flows/with-deployed',
			summary: 'deployed summary',
			description: 'deployed description',
			value: { modules: [] },
			schema: { properties: { deployed: { type: 'boolean' } } }
		} as any)

		await callGlobalTool('deploy_workspace_item', {
			type: 'flow',
			path: 'f/flows/with-deployed'
		})

		// A deployed version exists -> updateFlow.
		expect(FlowService.updateFlow).toHaveBeenCalledTimes(1)
		expect(FlowService.createFlow).not.toHaveBeenCalled()
		const call = vi.mocked(FlowService.updateFlow).mock.calls[0][0] as any
		expect(call.workspace).toBe(WORKSPACE)
		expect(call.path).toBe('f/flows/with-deployed')
		expect(call.requestBody).toMatchObject({
			path: 'f/flows/with-deployed',
			summary: 'draft summary',
			value: { modules: [{ id: 'step', value: { type: 'identity' } }] }
		})
		expect(DraftService.deleteDraft).not.toHaveBeenCalled()
	})

	it('deploys DB flow drafts with the draft metadata (not the deployed version)', async () => {
		vi.mocked(FlowService.existsFlowByPath).mockResolvedValueOnce(true)
		__seedDbRow('flow', 'f/flows/existing', {
			path: 'f/flows/existing',
			draft_only: false,
			summary: 'deployed summary',
			description: 'deployed description',
			value: { modules: [{ id: 'deployed_step', value: { type: 'identity' } }] },
			schema: { type: 'object', properties: { deployed: { type: 'boolean' } } },
			tag: 'deployed-tag',
			timeout: 60,
			visible_to_runner_only: true,
			labels: ['deployed'],
			draft: {
				path: 'f/flows/existing',
				summary: 'db draft summary',
				description: 'db draft description',
				value: { modules: [{ id: 'draft_step', value: { type: 'identity' } }] },
				schema: { type: 'object', properties: { draft: { type: 'string' } } },
				tag: 'draft-tag',
				timeout: 0,
				visible_to_runner_only: false,
				labels: []
			}
		})

		await callGlobalTool('deploy_workspace_item', {
			type: 'flow',
			path: 'f/flows/existing',
			deployment_message: 'ship db draft'
		})

		expect(FlowService.updateFlow).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			path: 'f/flows/existing',
			requestBody: expect.objectContaining({
				path: 'f/flows/existing',
				summary: 'db draft summary',
				description: 'db draft description',
				schema: { type: 'object', properties: { draft: { type: 'string' } } },
				tag: 'draft-tag',
				timeout: 0,
				visible_to_runner_only: false,
				labels: [],
				deployment_message: 'ship db draft'
			})
		})
		expect(vi.mocked(FlowService.updateFlow).mock.calls[0]?.[0].requestBody.value.modules).toEqual([
			{ id: 'draft_step', value: { type: 'identity' } }
		])
		expect(DraftService.deleteDraft).not.toHaveBeenCalled()
	})

	it('deploys a renamed flow draft through the original route path', async () => {
		vi.mocked(FlowService.existsFlowByPath).mockResolvedValueOnce(true)
		__seedDbRow('flow', 'f/flows/old-name', {
			path: 'f/flows/old-name',
			draft_only: false,
			summary: 'deployed summary',
			description: 'deployed description',
			value: { modules: [{ id: 'deployed_step', value: { type: 'identity' } }] },
			schema: { type: 'object', properties: { deployed: { type: 'boolean' } } },
			draft: {
				path: 'f/flows/new-name',
				summary: 'renamed draft summary',
				description: 'renamed draft description',
				value: { modules: [{ id: 'draft_step', value: { type: 'identity' } }] },
				schema: { type: 'object', properties: { draft: { type: 'string' } } }
			}
		})

		const raw = await callGlobalTool('deploy_workspace_item', {
			type: 'flow',
			path: 'f/flows/old-name'
		})

		// Original path in the URL, renamed path in the body.
		expect(FlowService.updateFlow).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			path: 'f/flows/old-name',
			requestBody: expect.objectContaining({
				path: 'f/flows/new-name',
				summary: 'renamed draft summary',
				description: 'renamed draft description',
				schema: { type: 'object', properties: { draft: { type: 'string' } } }
			})
		})
		expect(vi.mocked(FlowService.updateFlow).mock.calls[0]?.[0].requestBody.value.modules).toEqual([
			{ id: 'draft_step', value: { type: 'identity' } }
		])
		// Deployed version existed -> only the stale old-path draft is removed.
		expect(FlowService.deleteFlowByPath).not.toHaveBeenCalled()
		expect(DraftService.deleteDraft).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			kind: 'flow',
			path: 'f/flows/old-name'
		})
		expect(JSON.parse(raw)).toMatchObject({
			success: true,
			type: 'flow',
			path: 'f/flows/new-name'
		})
	})

	it('errors when deploying a flow that has only a deployed version (no draft)', async () => {
		__seedDbRow('flow', 'f/flows/deployed-only', {
			path: 'f/flows/deployed-only',
			draft_only: false,
			summary: 'deployed',
			value: { modules: [] }
			// no `draft` -> source resolves to 'deployed'
		})

		await expect(
			callGlobalTool('deploy_workspace_item', {
				type: 'flow',
				path: 'f/flows/deployed-only'
			})
		).rejects.toThrow('No draft changes to deploy')
		expect(FlowService.createFlow).not.toHaveBeenCalled()
		expect(FlowService.updateFlow).not.toHaveBeenCalled()
	})

	it('deploys a new raw app draft by bundling files and creating a raw app', async () => {
		__seedDbRow('app', 'f/apps/report', {
			path: 'f/apps/report',
			draft_only: true,
			draft: {
				summary: 'AI report',
				value: {
					files: {
						'/index.tsx': 'console.log("app")',
						'/package.json': '{"dependencies":{"react":"19.0.0"}}'
					},
					runnables: {},
					data: { tables: [] }
				}
			}
		})
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
		expect(UserDraft.get('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
		expect(JSON.parse(raw)).toMatchObject({
			success: true,
			type: 'app',
			path: 'f/apps/report'
		})
	})

	it('deploys an existing raw app draft by bundling files and updating the raw app', async () => {
		vi.mocked(AppService.existsApp).mockResolvedValueOnce(true)
		// A deployed app with a DB draft carrying the edited value + policy/custom_path.
		__seedDbRow('app', 'f/apps/report', {
			path: 'f/apps/report',
			summary: 'deployed report',
			versions: [1],
			value: { files: {}, runnables: {}, data: { tables: [] } },
			policy: { execution_mode: 'publisher' },
			draft: {
				summary: 'Updated report',
				value: {
					files: { '/index.tsx': 'console.log("updated")' },
					runnables: {},
					data: { tables: ['orders'] }
				},
				policy: { execution_mode: 'anonymous' },
				custom_path: 'kept-by-backend'
			}
		})

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
		expect(UserDraft.get('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
	})

	it('deploys a renamed app draft through the original route path', async () => {
		vi.mocked(AppService.existsApp).mockResolvedValueOnce(true)
		// A deployed app whose DB draft carries a renamed path. The draft `value`
		// nests the raw app + the renamed `path`, the way appSourceToDraftValue reads.
		__seedDbRow('app', 'f/apps/old-report', {
			path: 'f/apps/old-report',
			summary: 'deployed report',
			versions: [1],
			value: { files: {}, runnables: {}, data: { tables: [] } },
			policy: { execution_mode: 'anonymous' },
			draft: {
				path: 'f/apps/new-report',
				summary: 'Renamed report',
				value: {
					files: { '/index.tsx': 'console.log("renamed")' },
					runnables: {},
					data: { tables: ['renamed'] }
				},
				policy: { execution_mode: 'anonymous' }
			}
		})

		const raw = await callGlobalTool('deploy_workspace_item', {
			type: 'app',
			path: 'f/apps/old-report'
		})

		// Original path in the URL, renamed path in the body.
		expect(AppService.updateAppRaw).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			path: 'f/apps/old-report',
			formData: {
				app: {
					path: 'f/apps/new-report',
					value: {
						files: { '/index.tsx': 'console.log("renamed")' },
						runnables: {},
						data: { tables: ['renamed'] }
					},
					summary: 'Renamed report',
					policy: expect.objectContaining({ execution_mode: 'anonymous' }),
					deployment_message: undefined
				},
				js: 'bundled js',
				css: 'bundled css'
			}
		})
		expect(AppService.createAppRaw).not.toHaveBeenCalled()
		// Deployed version existed -> only the stale old-path draft is removed.
		expect(AppService.deleteApp).not.toHaveBeenCalled()
		expect(DraftService.deleteDraft).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			kind: 'app',
			path: 'f/apps/old-report'
		})
		expect(JSON.parse(raw)).toMatchObject({
			success: true,
			type: 'app',
			path: 'f/apps/new-report'
		})
	})

	it('notifies the session preview (as raw_app) after deploying a raw app', async () => {
		const onDeployed = vi.fn()
		setDeployedInSessionHandler(onDeployed)
		try {
			__seedDbRow('app', 'f/apps/report', {
				path: 'f/apps/report',
				draft_only: true,
				draft: {
					summary: 'AI report',
					value: {
						files: { '/index.tsx': 'console.log("app")' },
						runnables: {},
						data: { tables: [] }
					}
				}
			})

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

	it('test_run_script previews deployed script content when no draft exists', async () => {
		// Deployed-only row (no draft) — `loadDraft` falls through to the deployed
		// value, sourced from getScriptByPathWithDraft (not getScriptByPath).
		__seedDbRow('script', 'f/scripts/deployed-test', {
			path: 'f/scripts/deployed-test',
			hash: 'deployed-hash',
			draft_only: false,
			summary: 'Deployed test script',
			content: 'def main(name):\n    return name',
			language: 'python3',
			kind: 'script'
		})

		await withCompletedTestJob(() =>
			callGlobalTool('test_run_script', {
				path: 'f/scripts/deployed-test',
				args: { name: 'Grace' }
			})
		)

		expect(ScriptService.getScriptByPath).not.toHaveBeenCalled()
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

	it('test_run_flow previews deployed flow content when no draft exists', async () => {
		const modules = [{ id: 'deployed_start', value: { type: 'identity' } }]
		// Deployed-only row (no draft) — `loadDraft` falls through to the deployed
		// value, sourced from getFlowByPathWithDraft (not getFlowByPath).
		__seedDbRow('flow', 'f/flows/deployed-test', {
			path: 'f/flows/deployed-test',
			draft_only: false,
			summary: 'Deployed test flow',
			value: { modules },
			schema: {}
		})

		await withCompletedTestJob(() =>
			callGlobalTool('test_run_flow', {
				path: 'f/flows/deployed-test',
				args: { name: 'Grace' }
			})
		)

		expect(FlowService.getFlowByPath).not.toHaveBeenCalled()
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
		UserDraft.save(
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
		UserDraft.save(
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

		expect(content).toContain('Draft tools create or update drafts only')
		expect(content).toContain(
			'Use discard_draft to remove a draft, including the matching open editor draft'
		)
		expect(content).toContain(
			'create or update a workspace draft: a persisted draft that is visible to editors'
		)
		expect(content).toContain(
			'deploy_workspace_item deploys a draft to the workspace'
		)
		expect(content).toContain(
			'they are staged locally in the browser until you deploy them'
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
		const discard = getGlobalTool('discard_draft')
		const deleteItem = getGlobalTool('delete_workspace_item')

		expect(discard.def.function.description).toBe(
			'Discard a draft only; the deployed item is never changed. For script/flow/app it discards the workspace draft (and removes a draft-only item that was never deployed), leaving any deployed version untouched. For schedule/trigger/resource/variable it discards the locally staged draft. Also clears the matching open editor draft if one is mounted.'
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
