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
import { UserDraft, __resetUserDraftForTesting } from '$lib/userDraft.svelte'
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
			isDraft: false
		})
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
			isDraft: false
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
				isDraft: false
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

		expect(item.isDraft).toBe(false)
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
