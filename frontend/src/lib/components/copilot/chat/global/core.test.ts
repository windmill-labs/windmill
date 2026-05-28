import { beforeEach, describe, expect, it, vi } from 'vitest'

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

	return {
		...actual,
		ScriptService: wrapService(actual.ScriptService, {
			existsScriptByPath: vi.fn(async () => false),
			createScript: vi.fn(async () => 'created'),
			getScriptByPathWithDraft: vi.fn(async () => {
				throw new Error('getScriptByPathWithDraft mock not configured')
			}),
			listScripts: vi.fn(async () => [])
		}),
		FlowService: wrapService(actual.FlowService, {
			existsFlowByPath: vi.fn(async () => false),
			createFlow: vi.fn(async () => 'created'),
			updateFlow: vi.fn(async () => 'updated'),
			getFlowByPath: vi.fn(async () => {
				throw new Error('getFlowByPath mock not configured')
			}),
			getFlowByPathWithDraft: vi.fn(async () => {
				throw new Error('getFlowByPathWithDraft mock not configured')
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
			getAppByPathWithDraft: vi.fn(async () => {
				throw new Error('getAppByPathWithDraft mock not configured')
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
		})
	}
})

vi.mock('./rawAppBundlerBridge', () => ({
	bundleRawAppDraft: vi.fn(async () => ({
		js: 'bundled js',
		css: 'bundled css'
	}))
}))

import { globalTools, prepareGlobalSystemMessage, prepareGlobalUserMessage } from './core'
import { UserDraft, __resetUserDraftForTesting } from '$lib/userDraft.svelte'
import { clearGlobalDrafts } from './userDraftAdapter'
import { bundleRawAppDraft } from './rawAppBundlerBridge'
import {
	AppService,
	FlowService,
	HttpTriggerService,
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
	callbacks: ToolCallbacks = toolCallbacks
): Promise<string> {
	return getGlobalTool(name).fn({
		args,
		workspace: WORKSPACE,
		helpers: {},
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

describe('global AI tools', () => {
	beforeEach(() => {
		__resetUserDraftForTesting()
		localStorage.clear()
		clearGlobalDrafts(WORKSPACE)
		vi.clearAllMocks()
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

	it('writes script drafts into UserDraft', async () => {
		const content = 'export async function main() {\n\treturn "hello"\n}'

		await callGlobalTool('write_script', {
			path: 'f/scripts/hello',
			summary: 'Hello script',
			language: 'bun',
			content
		})

		expect(UserDraft.get<any>('script', 'f/scripts/hello', { workspace: WORKSPACE })).toMatchObject(
			{
				path: 'f/scripts/hello',
				summary: 'Hello script',
				language: 'bun',
				content
			}
		)
	})

	it('applies path_prefix to local drafts before enforcing the result limit', async () => {
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
		await callGlobalTool('write_script', {
			path: 'f/scripts/discard-me',
			summary: 'Temporary draft',
			language: 'bun',
			content: 'export async function main() { return 1 }'
		})

		expect(UserDraft.get('script', 'f/scripts/discard-me', { workspace: WORKSPACE })).toBeDefined()

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
			UserDraft.get('script', 'f/scripts/discard-me', { workspace: WORKSPACE })
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
		vi.mocked(ScriptService.getScriptByPathWithDraft).mockResolvedValueOnce({
			path: 'f/scripts/existing',
			hash: 'deployed-hash',
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
		} as any)

		await callGlobalTool('write_script', {
			path: 'f/scripts/existing',
			summary: 'new summary',
			language: 'bun',
			content: 'new content'
		})

		expect(
			UserDraft.get<any>('script', 'f/scripts/existing', { workspace: WORKSPACE })
		).toMatchObject({
			path: 'f/scripts/existing',
			parent_hash: 'deployed-hash',
			summary: 'new summary',
			description: 'db draft description',
			content: 'new content',
			language: 'bun'
		})
		expect(UserDraft.getMeta('script', 'f/scripts/existing', { workspace: WORKSPACE })).toEqual({
			remoteRev: 'deployed-hash',
			remoteDraftRev: '2026-05-22T10:00:00Z'
		})
	})

	it('preserves existing flow metadata and seeds freshness on first flow write', async () => {
		vi.mocked(FlowService.existsFlowByPath).mockResolvedValueOnce(true)
		vi.mocked(FlowService.getFlowLatestVersion).mockResolvedValueOnce({ id: 42 } as any)
		vi.mocked(FlowService.getFlowByPathWithDraft).mockResolvedValueOnce({
			path: 'f/flows/existing',
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
		} as any)

		await callGlobalTool('write_flow', {
			path: 'f/flows/existing',
			summary: 'new summary',
			modules: JSON.stringify([{ id: 'step', value: { type: 'identity' } }])
		})

		expect(UserDraft.get<any>('flow', 'f/flows/existing', { workspace: WORKSPACE })).toMatchObject({
			path: 'f/flows/existing',
			summary: 'new summary',
			description: 'db draft description',
			value: { modules: [{ id: 'step', value: { type: 'identity' } }] }
		})
		expect(UserDraft.getMeta('flow', 'f/flows/existing', { workspace: WORKSPACE })).toEqual({
			remoteRev: 42,
			remoteDraftRev: '2026-05-22T10:00:00Z'
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

	it('seeds raw app draft metadata on first app write', async () => {
		vi.mocked(AppService.getAppByPathWithDraft).mockResolvedValueOnce({
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
		} as any)

		await callGlobalTool('write_app_file', {
			path: 'f/apps/report',
			file_path: '/src/New.tsx',
			content: 'export default function New() { return null }'
		})

		const draft = UserDraft.get<any>('raw_app', 'f/apps/report', { workspace: WORKSPACE })
		expect(draft).toMatchObject({
			summary: 'saved app draft',
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
			data: { tables: ['orders'], datatable: 'db', schema: 'public' },
			policy: { execution_mode: 'anonymous' },
			custom_path: 'report'
		})
		expect(UserDraft.getMeta('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toEqual({
			remoteRev: 4,
			remoteDraftRev: '2026-05-22T10:30:00Z'
		})
	})

	it('summarizes local raw app drafts in read_workspace_item', async () => {
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
			isDraft: false
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

	it('deploys a new raw app draft by bundling files and creating a raw app', async () => {
		UserDraft.save(
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
		expect(UserDraft.get('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
		expect(JSON.parse(raw)).toMatchObject({
			success: true,
			type: 'app',
			path: 'f/apps/report'
		})
	})

	it('deploys an existing raw app draft by bundling files and updating the raw app', async () => {
		vi.mocked(AppService.existsApp).mockResolvedValueOnce(true)
		UserDraft.save(
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
		expect(UserDraft.get('raw_app', 'f/apps/report', { workspace: WORKSPACE })).toBeUndefined()
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
		).resolves.toContain('Updated local draft flow')

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
