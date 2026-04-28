import { describe, expect, it, vi } from 'vitest'
import { z } from 'zod'

vi.mock('monaco-editor', () => ({
	editor: {}
}))

vi.mock('$lib/stores', () => ({
	workspaceStore: { subscribe: () => () => undefined }
}))

vi.mock('$lib/components/flows/flowTree', () => ({
	findModuleInModules: () => undefined
}))

vi.mock('$lib/gen', () => ({
	ScriptService: {},
	FlowService: {},
	JobService: {},
	ScheduleService: {
		previewSchedule: vi.fn(),
		createSchedule: vi.fn()
	},
	HttpTriggerService: { createHttpTrigger: vi.fn() },
	WebsocketTriggerService: { createWebsocketTrigger: vi.fn() },
	KafkaTriggerService: { createKafkaTrigger: vi.fn() },
	NatsTriggerService: { createNatsTrigger: vi.fn() },
	PostgresTriggerService: { createPostgresTrigger: vi.fn() },
	MqttTriggerService: { createMqttTrigger: vi.fn() },
	SqsTriggerService: { createSqsTrigger: vi.fn() },
	GcpTriggerService: { createGcpTrigger: vi.fn() },
	AzureTriggerService: { createAzureTrigger: vi.fn() }
}))

vi.mock('$lib/utils', () => ({
	emptyString: (value: string | undefined | null) => !value
}))

vi.mock('$lib/scripts', () => ({
	scriptLangToEditorLang: (language: string) => language
}))

vi.mock('$lib/aiStore', () => ({
	getCurrentModel: () => undefined
}))

vi.mock('@leeoniya/ufuzzy', () => ({
	default: class {
		search() {
			return [[], [], []]
		}
	}
}))

describe('createToolDef', () => {
	it('builds the create_trigger schema without top-level composition', async () => {
		const { createToolDef } = await import('./shared')
		const { createTriggerToolSchema } = await import('./workspaceToolsZod')
		const toolDef = createToolDef(createTriggerToolSchema, 'create_trigger', 'Create a trigger')

		const parameters = toolDef.function.parameters as any
		expect(parameters).toBeDefined()
		expect(parameters?.type).toBe('object')
		expect(parameters?.anyOf).toBeUndefined()
		expect(parameters?.oneOf).toBeUndefined()
		expect(parameters?.allOf).toBeUndefined()
		expect(parameters?.properties?.kind?.enum).toContain('http')
		expect(parameters?.properties?.config?.anyOf?.length).toBeGreaterThan(1)
	})
})

describe('processToolCall', () => {
	it('returns pre-confirmation validation errors without asking for confirmation', async () => {
		const { createToolDef, processToolCall } = await import('./shared')
		const error = 'the script needs to be deployed before doing this action'
		const fn = vi.fn()
		const requestConfirmation = vi.fn()
		const setToolStatus = vi.fn()

		const result = await processToolCall({
			tools: [
				{
					def: createToolDef(z.object({}), 'create_schedule', 'Create schedule'),
					requiresConfirmation: true,
					showDetails: true,
					validateBeforeConfirmation: () => error,
					fn
				}
			],
			toolCall: {
				id: 'call_1',
				type: 'function',
				function: { name: 'create_schedule', arguments: '{}' }
			},
			helpers: {},
			workspace: 'test-workspace',
			toolCallbacks: {
				setToolStatus,
				removeToolStatus: vi.fn(),
				requestConfirmation
			}
		})

		expect(requestConfirmation).not.toHaveBeenCalled()
		expect(fn).not.toHaveBeenCalled()
		expect(setToolStatus).toHaveBeenCalledWith(
			'call_1',
			expect.objectContaining({
				content: error,
				error,
				isLoading: false,
				needsConfirmation: false,
				showDetails: true
			})
		)
		expect(result.content).toBe(error)
	})

	it('continues to confirmation when pre-confirmation validation passes', async () => {
		const { createToolDef, processToolCall } = await import('./shared')
		const fn = vi.fn().mockResolvedValue('ok')
		const requestConfirmation = vi.fn().mockResolvedValue(true)
		const setToolStatus = vi.fn()

		const result = await processToolCall({
			tools: [
				{
					def: createToolDef(z.object({}), 'create_schedule', 'Create schedule'),
					requiresConfirmation: true,
					confirmationMessage: 'Create schedule',
					validateBeforeConfirmation: () => undefined,
					fn
				}
			],
			toolCall: {
				id: 'call_2',
				type: 'function',
				function: { name: 'create_schedule', arguments: '{}' }
			},
			helpers: {},
			workspace: 'test-workspace',
			toolCallbacks: {
				setToolStatus,
				removeToolStatus: vi.fn(),
				requestConfirmation
			}
		})

		expect(requestConfirmation).toHaveBeenCalledWith('call_2')
		expect(fn).toHaveBeenCalled()
		expect(result.content).toBe('ok')
	})

	it('blocks workspace mutation tools for undeployed scripts and flows', async () => {
		const { processToolCall } = await import('./shared')
		const { createWorkspaceMutationTools } = await import('./workspaceTools')
		const setToolStatus = vi.fn()
		const requestConfirmation = vi.fn()
		const workspaceMutationTools = createWorkspaceMutationTools()

		const scriptResult = await processToolCall({
			tools: workspaceMutationTools,
			toolCall: {
				id: 'call_3',
				type: 'function',
				function: { name: 'create_schedule', arguments: '{}' }
			},
			helpers: {
				getWorkspaceMutationTarget: () => ({ kind: 'script', path: '', deployed: false })
			},
			workspace: 'test-workspace',
			toolCallbacks: {
				setToolStatus,
				removeToolStatus: vi.fn(),
				requestConfirmation
			}
		})

		expect(scriptResult.content).toBe('the script needs to be deployed before doing this action')

		const flowResult = await processToolCall({
			tools: workspaceMutationTools,
			toolCall: {
				id: 'call_4',
				type: 'function',
				function: { name: 'create_trigger', arguments: '{}' }
			},
			helpers: {
				getWorkspaceMutationTarget: () => ({ kind: 'flow', path: 'f/flow', deployed: false })
			},
			workspace: 'test-workspace',
			toolCallbacks: {
				setToolStatus,
				removeToolStatus: vi.fn(),
				requestConfirmation
			}
		})

		expect(flowResult.content).toBe('the flow needs to be deployed before doing this action')
		expect(requestConfirmation).not.toHaveBeenCalled()
	})
})
