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

vi.mock('svelte', async (importOriginal) => {
	const actual = (await importOriginal()) as Record<string, unknown>
	return {
		...actual,
		onDestroy: vi.fn()
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
			getScriptByPath: vi.fn(),
			getScriptByPathWithDraft: vi.fn(),
			listScripts: vi.fn(async () => [])
		}),
		FlowService: wrapService(actual.FlowService, {
			existsFlowByPath: vi.fn(async () => false),
			listFlows: vi.fn(async () => [])
		}),
		ScheduleService: wrapService(actual.ScheduleService, {
			existsSchedule: vi.fn(async () => false),
			listSchedules: vi.fn(async () => [])
		}),
		HttpTriggerService: wrapService(actual.HttpTriggerService, {
			existsHttpTrigger: vi.fn(async () => false),
			listHttpTriggers: vi.fn(async () => [])
		}),
		WebsocketTriggerService: wrapService(actual.WebsocketTriggerService, {
			existsWebsocketTrigger: vi.fn(async () => false),
			listWebsocketTriggers: vi.fn(async () => [])
		}),
		KafkaTriggerService: wrapService(actual.KafkaTriggerService, {
			existsKafkaTrigger: vi.fn(async () => false),
			listKafkaTriggers: vi.fn(async () => [])
		}),
		NatsTriggerService: wrapService(actual.NatsTriggerService, {
			existsNatsTrigger: vi.fn(async () => false),
			listNatsTriggers: vi.fn(async () => [])
		}),
		PostgresTriggerService: wrapService(actual.PostgresTriggerService, {
			existsPostgresTrigger: vi.fn(async () => false),
			listPostgresTriggers: vi.fn(async () => [])
		}),
		MqttTriggerService: wrapService(actual.MqttTriggerService, {
			existsMqttTrigger: vi.fn(async () => false),
			listMqttTriggers: vi.fn(async () => [])
		}),
		SqsTriggerService: wrapService(actual.SqsTriggerService, {
			existsSqsTrigger: vi.fn(async () => false),
			listSqsTriggers: vi.fn(async () => [])
		}),
		GcpTriggerService: wrapService(actual.GcpTriggerService, {
			existsGcpTrigger: vi.fn(async () => false),
			listGcpTriggers: vi.fn(async () => [])
		}),
		AzureTriggerService: wrapService(actual.AzureTriggerService, {
			existsAzureTrigger: vi.fn(async () => false),
			listAzureTriggers: vi.fn(async () => [])
		}),
		AppService: wrapService(actual.AppService, {
			existsApp: vi.fn(async () => false)
		}),
		ResourceService: wrapService(actual.ResourceService, {
			existsResource: vi.fn(async () => false),
			listResource: vi.fn(async () => []),
			queryResourceTypes: vi.fn(async () => [])
		}),
		VariableService: wrapService(actual.VariableService, {
			existsVariable: vi.fn(async () => false),
			getVariable: vi.fn(async ({ path }: { path: string }) => ({
				path,
				value: 'super-secret-token',
				is_secret: true,
				description: 'API key'
			})),
			listVariable: vi.fn(async () => [])
		})
	}
})

import { globalTools, prepareGlobalUserMessage } from './core'
import { deleteGlobalDraft, listGlobalDrafts } from './userDraftAdapter'
import { UserDraft, __resetUserDraftForTesting } from '$lib/userDraft.svelte'
import { ScriptService } from '$lib/gen'
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

describe('global AI tools', () => {
	beforeEach(() => {
		__resetUserDraftForTesting()
		localStorage.clear()
		vi.clearAllMocks()
	})

	it('writes resource and variable drafts into the shared UserDraft store', async () => {
		await callGlobalTool('write_variable', {
			path: 'f/secrets/api_key',
			value: 'secret',
			is_secret: true,
			description: 'API key'
		})
		await callGlobalTool('write_resource', {
			path: 'f/resources/api',
			resource_type: 'http',
			description: 'API resource',
			value: {
				token: '$var:f/secrets/api_key'
			}
		})

		expect(
			UserDraft.get<any>('variable', 'f/secrets/api_key', { workspace: WORKSPACE })
		).toMatchObject({
			path: 'f/secrets/api_key',
			value: 'secret',
			is_secret: true,
			description: 'API key'
		})
		expect(
			UserDraft.get<any>('resource', 'f/resources/api', { workspace: WORKSPACE })
		).toMatchObject({
			path: 'f/resources/api',
			resource_type: 'http',
			description: 'API resource',
			value: {
				token: '$var:f/secrets/api_key'
			}
		})
	})

	it('writes script drafts into the shared UserDraft store', async () => {
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

		const raw = await callGlobalTool('read_workspace_item', {
			type: 'script',
			path: 'f/scripts/hello'
		})
		expect(JSON.parse(raw)).toMatchObject({
			type: 'script',
			path: 'f/scripts/hello',
			summary: 'Hello script',
			language: 'bun',
			value: content,
			isDraft: true
		})
	})

	it('reads backend draft-only scripts through the draft-aware endpoint', async () => {
		const content = 'export async function main() {\n\treturn "db draft"\n}'
		vi.mocked(ScriptService.getScriptByPathWithDraft).mockResolvedValueOnce({
			hash: 'draft-hash',
			path: 'f/scripts/backend-draft',
			summary: 'Backend draft',
			description: '',
			content,
			language: 'bun',
			draft_only: true
		} as any)

		const raw = await callGlobalTool('read_workspace_item', {
			type: 'script',
			path: 'f/scripts/backend-draft'
		})

		expect(JSON.parse(raw)).toMatchObject({
			type: 'script',
			path: 'f/scripts/backend-draft',
			summary: 'Backend draft',
			language: 'bun',
			value: content,
			isDraft: true
		})
		expect(ScriptService.getScriptByPathWithDraft).toHaveBeenCalledWith({
			workspace: WORKSPACE,
			path: 'f/scripts/backend-draft'
		})
		expect(ScriptService.getScriptByPath).not.toHaveBeenCalled()
	})

	it('reads a live UserDraft editor baseline as current context without marking it draft', async () => {
		const content = 'export async function main() {\n\treturn "open editor"\n}'
		const handle = UserDraft.use<any>('script', 'f/scripts/open-editor', { workspace: WORKSPACE })
		handle.setDraftAndMeta(
			{
				path: 'f/scripts/open-editor',
				summary: 'Open editor baseline',
				language: 'bun',
				content
			},
			{ remoteRev: 'v1' }
		)

		const raw = await callGlobalTool('read_workspace_item', {
			type: 'script',
			path: 'f/scripts/open-editor'
		})

		expect(JSON.parse(raw)).toMatchObject({
			type: 'script',
			path: 'f/scripts/open-editor',
			summary: 'Open editor baseline',
			language: 'bun',
			value: content,
			isDraft: false
		})
		expect(localStorage.getItem(`userdraft/w/${WORKSPACE}/script/f/scripts/open-editor`)).toBeNull()
	})

	it('does not treat a clean live editor baseline as a deployable global draft', async () => {
		const content = 'export async function main() {\n\treturn "baseline"\n}'
		const handle = UserDraft.use<any>('script', 'f/scripts/baseline', { workspace: WORKSPACE })
		handle.setDraftAndMeta(
			{
				path: 'f/scripts/baseline',
				summary: 'Clean baseline',
				language: 'bun',
				content
			},
			{ remoteRev: 'v1' }
		)

		expect(listGlobalDrafts(WORKSPACE)).toEqual([])
		await expect(
			callGlobalTool('deploy_workspace_item', {
				type: 'script',
				path: 'f/scripts/baseline'
			})
		).rejects.toThrow('No AI draft found for script "f/scripts/baseline".')
	})

	it('does not treat a persisted editor baseline as a deployable global draft', async () => {
		const key = `userdraft/w/${WORKSPACE}/script/f/scripts/persisted-baseline`
		localStorage.setItem(
			key,
			JSON.stringify({
				value: {
					path: 'f/scripts/persisted-baseline',
					summary: 'Persisted baseline',
					language: 'bun',
					content: 'export async function main() {\n\treturn "baseline"\n}',
					parent_hash: 'h1'
				},
				remoteRev: 'h1'
			})
		)

		expect(listGlobalDrafts(WORKSPACE)).toEqual([])
		await expect(
			callGlobalTool('deploy_workspace_item', {
				type: 'script',
				path: 'f/scripts/persisted-baseline'
			})
		).rejects.toThrow('No AI draft found for script "f/scripts/persisted-baseline".')
	})

	it('persists AI writes into a live editor handle so they remain deployable', async () => {
		const handle = UserDraft.use<any>('script', 'f/scripts/live-ai', { workspace: WORKSPACE })
		handle.setDraftAndMeta(
			{
				path: 'f/scripts/live-ai',
				summary: 'Clean baseline',
				language: 'bun',
				content: 'export async function main() {\n\treturn 1\n}'
			},
			{ remoteRev: 'v1' }
		)

		await callGlobalTool('write_script', {
			path: 'f/scripts/live-ai',
			summary: 'AI edit',
			language: 'bun',
			content: 'export async function main() {\n\treturn 2\n}'
		})

		expect(handle.draft?.content).toContain('return 2')
		expect(listGlobalDrafts(WORKSPACE)).toEqual([
			expect.objectContaining({
				type: 'script',
				path: 'f/scripts/live-ai',
				summary: 'AI edit',
				isDraft: true
			})
		])
		expect(localStorage.getItem(`userdraft/w/${WORKSPACE}/script/f/scripts/live-ai`)).not.toBeNull()
	})

	it('preserves NewScript metadata from existing scripts without copying backend read fields', async () => {
		vi.mocked(ScriptService.existsScriptByPath).mockResolvedValueOnce(true)
		vi.mocked(ScriptService.getScriptByPath).mockResolvedValueOnce({
			workspace_id: WORKSPACE,
			hash: 'old-hash',
			path: 'f/scripts/existing',
			parent_hashes: ['older-hash'],
			summary: 'Existing script',
			description: 'Existing description',
			content: 'export async function main() { return 1 }',
			created_by: 'admin',
			created_at: '2026-01-01T00:00:00Z',
			archived: false,
			deleted: false,
			extra_perms: {},
			is_template: true,
			language: 'bun',
			kind: 'script',
			starred: true,
			has_draft: true,
			schema: {
				type: 'object',
				properties: {
					name: { type: 'string' }
				}
			},
			concurrent_limit: 3,
			cache_ttl: 30,
			cache_ignore_s3_path: true,
			has_preprocessor: true,
			assets: [{ path: 'asset.txt', kind: 's3object' }],
			labels: ['important']
		} as any)

		await callGlobalTool('write_script', {
			path: 'f/scripts/existing',
			summary: 'AI update',
			language: 'python3',
			content: 'def main():\n    return 2'
		})

		const draft = UserDraft.get<any>('script', 'f/scripts/existing', { workspace: WORKSPACE })
		expect(draft).toMatchObject({
			path: 'f/scripts/existing',
			parent_hash: 'old-hash',
			summary: 'AI update',
			description: 'Existing description',
			content: 'def main():\n    return 2',
			language: 'python3',
			is_template: true,
			schema: {
				properties: {
					name: { type: 'string' }
				}
			},
			concurrent_limit: 3,
			cache_ttl: 30,
			cache_ignore_s3_path: true,
			has_preprocessor: true,
			assets: [{ path: 'asset.txt', kind: 's3object' }],
			labels: ['important']
		})
		expect(draft).not.toHaveProperty('hash')
		expect(draft).not.toHaveProperty('workspace_id')
		expect(draft).not.toHaveProperty('created_by')
		expect(draft).not.toHaveProperty('starred')
		expect(draft).not.toHaveProperty('extra_perms')
	})

	it('deleteGlobalDraft clears both persisted storage and any live handle state', async () => {
		const handle = UserDraft.use<any>('script', 'f/scripts/delete-live', { workspace: WORKSPACE })

		await callGlobalTool('write_script', {
			path: 'f/scripts/delete-live',
			summary: 'Delete me',
			language: 'bun',
			content: 'export async function main() {\n\treturn "delete"\n}'
		})
		expect(handle.draft).toBeDefined()
		expect(localStorage.getItem(`userdraft/w/${WORKSPACE}/script/f/scripts/delete-live`)).not.toBeNull()

		deleteGlobalDraft(WORKSPACE, 'script', 'f/scripts/delete-live')

		expect(handle.draft).toBeUndefined()
		expect(localStorage.getItem(`userdraft/w/${WORKSPACE}/script/f/scripts/delete-live`)).toBeNull()
	})

	it('lists and edits a new script draft stored under the empty add-editor key by its assigned path', async () => {
		const content = 'export async function main() {\n\treturn 1\n}'
		const handle = UserDraft.use<any>('script', '', { workspace: WORKSPACE })
		handle.setDraftAndMeta(
			{
				path: 'u/admin/assigned_script',
				summary: '',
				language: 'bun',
				content
			},
			{}
		)

		const raw = await callGlobalTool('list_workspace_items', {
			types: ['script']
		})

		expect(JSON.parse(raw)).toEqual([
			expect.objectContaining({
				type: 'script',
				path: 'u/admin/assigned_script',
				isDraft: false
			})
		])

		UserDraft.get<any>('script', '', { workspace: WORKSPACE }).uncloneable = () =>
			'runtime-only editor field'

		await callGlobalTool('edit_script', {
			path: 'u/admin/assigned_script',
			old_string: 'return 1',
			new_string: 'return 2',
			replace_all: false
		})

		expect(UserDraft.get<any>('script', '', { workspace: WORKSPACE })?.content).toContain(
			'return 2'
		)
		expect(
			UserDraft.get<any>('script', 'u/admin/assigned_script', { workspace: WORKSPACE })
		).toBeUndefined()
	})

	it('edits a new script draft stored under the empty add-editor key before listing it', async () => {
		const content = 'export async function main() {\n\treturn 1\n}'
		const handle = UserDraft.use<any>('script', '', { workspace: WORKSPACE })
		handle.setDraftAndMeta(
			{
				path: 'u/admin/direct_edit_script',
				summary: '',
				language: 'bun',
				content
			},
			{}
		)

		UserDraft.get<any>('script', '', { workspace: WORKSPACE }).uncloneable = () =>
			'runtime-only editor field'

		await callGlobalTool('edit_script', {
			path: 'u/admin/direct_edit_script',
			old_string: 'return 1',
			new_string: 'return 2',
			replace_all: false
		})

		expect(UserDraft.get<any>('script', '', { workspace: WORKSPACE })?.content).toContain(
			'return 2'
		)
		expect(
			UserDraft.get<any>('script', 'u/admin/direct_edit_script', { workspace: WORKSPACE })
		).toBeUndefined()
	})

	it('overlays shared UserDraft entries in list_workspace_items', async () => {
		await callGlobalTool('write_script', {
			path: 'f/scripts/listed',
			summary: 'Listed script',
			language: 'bun',
			content: 'export async function main() { return 1 }'
		})

		const raw = await callGlobalTool('list_workspace_items', {
			types: ['script']
		})

		expect(JSON.parse(raw)).toEqual([
			expect.objectContaining({
				type: 'script',
				path: 'f/scripts/listed',
				summary: 'Listed script',
				isDraft: true
			})
		])
	})

	it('keeps local UserDraft overlays marked as drafts after path_prefix filtering', async () => {
		await callGlobalTool('write_script', {
			path: 'f/scripts/listed',
			summary: 'Listed script',
			language: 'bun',
			content: 'export async function main() { return 1 }'
		})

		const raw = await callGlobalTool('list_workspace_items', {
			types: ['script'],
			path_prefix: 'f/scripts'
		})

		expect(JSON.parse(raw)).toEqual([
			expect.objectContaining({
				type: 'script',
				path: 'f/scripts/listed',
				isDraft: true
			})
		])
	})

	it('applies path_prefix to local UserDraft overlays in list_workspace_items', async () => {
		await callGlobalTool('write_script', {
			path: 'f/scripts/listed',
			summary: 'Listed script',
			language: 'bun',
			content: 'export async function main() { return 1 }'
		})

		const raw = await callGlobalTool('list_workspace_items', {
			types: ['script'],
			path_prefix: 'f/other'
		})

		expect(JSON.parse(raw)).toEqual([])
	})

	it('does not inspect unrelated partial flow UserDraft entries when listing scripts', async () => {
		UserDraft.save<any>('flow', 'f/flows/partial', {}, { workspace: WORKSPACE })

		const raw = await callGlobalTool('list_workspace_items', {
			types: ['script']
		})

		expect(JSON.parse(raw)).toEqual([])
	})

	it('lists legacy top-level flow UserDraft values without crashing', async () => {
		UserDraft.save<any>(
			'flow',
			'f/flows/legacy',
			{
				modules: [
					{
						id: 'start',
						value: { type: 'identity' }
					}
				]
			},
			{ workspace: WORKSPACE }
		)

		const raw = await callGlobalTool('list_workspace_items', {
			types: ['flow']
		})

		expect(JSON.parse(raw)).toEqual([
			expect.objectContaining({
				type: 'flow',
				path: 'f/flows/legacy',
				isDraft: false
			})
		])
	})

	it('writes schedule drafts into the shared UserDraft store', async () => {
		await callGlobalTool('write_schedule', {
			path: 'f/schedules/daily',
			summary: 'Daily schedule',
			schedule: '0 0 12 * * *',
			timezone: 'UTC',
			script_path: 'f/scripts/hello',
			is_flow: false,
			args: { name: 'Ada' },
			enabled: true
		})

		expect(
			UserDraft.get<any>('trigger_schedule', 'f/schedules/daily', { workspace: WORKSPACE })
		).toMatchObject({
			path: 'f/schedules/daily',
			summary: 'Daily schedule',
			script_path: 'f/scripts/hello',
			is_flow: false
		})

		const raw = await callGlobalTool('read_workspace_item', {
			type: 'schedule',
			path: 'f/schedules/daily'
		})

		expect(JSON.parse(raw)).toMatchObject({
			type: 'schedule',
			path: 'f/schedules/daily',
			summary: 'Daily schedule',
			value: expect.objectContaining({
				schedule: '0 0 12 * * *',
				timezone: 'UTC'
			}),
			isDraft: true
		})
	})

	it('writes trigger drafts into the shared trigger UserDraft kind', async () => {
		await callGlobalTool('write_trigger', {
			kind: 'http',
			config: {
				path: 'f/triggers/hook',
				summary: 'Hook trigger',
				script_path: 'f/scripts/hello',
				route_path: 'api/hook',
				is_flow: false,
				http_method: 'post',
				authentication_method: 'none',
				is_static_website: false
			}
		})

		expect(
			UserDraft.get<any>('trigger_http', 'f/triggers/hook', { workspace: WORKSPACE })
		).toMatchObject({
			path: 'f/triggers/hook',
			summary: 'Hook trigger',
			route_path: 'api/hook',
			is_flow: false
		})

		const raw = await callGlobalTool('list_workspace_items', {
			types: ['trigger']
		})

		expect(JSON.parse(raw)).toEqual([
			expect.objectContaining({
				type: 'trigger',
				triggerKind: 'http',
				path: 'f/triggers/hook',
				summary: 'Hook trigger',
				isDraft: true
			})
		])
	})

	it('redacts existing variable values when reading workspace items', async () => {
		const raw = await callGlobalTool('read_workspace_item', {
			type: 'variable',
			path: 'f/secrets/api_key'
		})
		const item = JSON.parse(raw)

		expect(raw).not.toContain('super-secret-token')
		expect(item).toEqual({
			type: 'variable',
			path: 'f/secrets/api_key',
			summary: 'API key',
			isDraft: false
		})
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

		expect(
			UserDraft.get<any>('flow', 'f/flows/empty-module', { workspace: WORKSPACE })
		).toMatchObject({
			path: 'f/flows/empty-module',
			summary: 'Flow with empty module',
			value: {
				modules: [
					expect.objectContaining({
						id: 'empty_step'
					})
				]
			}
		})

		const code = 'export async function main() {\n\treturn 42\n}'

		await expect(
			callGlobalTool('set_flow_module_code', {
				path: 'f/flows/empty-module',
				module_id: 'empty_step',
				code
			})
		).resolves.toContain('Updated AI draft flow')

		await expect(
			callGlobalTool('read_flow_module_code', {
				path: 'f/flows/empty-module',
				module_id: 'empty_step'
			})
		).resolves.toBe(code)
	})

	it('stores raw app drafts in the shared raw_app UserDraft kind', async () => {
		await callGlobalTool('init_app', {
			path: 'f/apps/demo',
			summary: 'Demo app',
			framework: 'react19'
		})

		await callGlobalTool('write_app_file', {
			path: 'f/apps/demo',
			file_path: '/App.tsx',
			content: 'export default function App() { return <main>Demo</main> }'
		})

		const draft = UserDraft.get<any>('raw_app', 'f/apps/demo', { workspace: WORKSPACE })
		expect(draft).toMatchObject({
			summary: 'Demo app',
			files: expect.objectContaining({
				'/App.tsx': 'export default function App() { return <main>Demo</main> }'
			})
		})
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

		expect(item.isDraft).toBe(true)
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

	it('asks the user a multiple-choice question and returns the selected answer', async () => {
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
})

describe('prepareGlobalUserMessage', () => {
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
