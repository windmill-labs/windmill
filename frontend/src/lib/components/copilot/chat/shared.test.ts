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
		const { createTriggerToolSchema } = await import('./workspaceToolsZod.gen')
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

	it('does not expose runnable target fields on workspace mutation tools', async () => {
		const { createWorkspaceMutationTools } = await import('./workspaceTools')
		const [scheduleTool, triggerTool] = createWorkspaceMutationTools()

		const scheduleParameters = scheduleTool.def.function.parameters as any
		expect(scheduleParameters?.properties?.script_path).toBeUndefined()
		expect(scheduleParameters?.properties?.is_flow).toBeUndefined()

		const triggerParameters = triggerTool.def.function.parameters as any
		const triggerConfigVariants = triggerParameters?.properties?.config?.anyOf ?? []
		expect(triggerConfigVariants.length).toBeGreaterThan(1)
		for (const variant of triggerConfigVariants) {
			expect(variant?.properties?.script_path).toBeUndefined()
			expect(variant?.properties?.is_flow).toBeUndefined()
		}
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

	it('injects runnable target fields into schedule and trigger requests', async () => {
		const gen = (await import('$lib/gen')) as any
		const { processToolCall } = await import('./shared')
		const { createWorkspaceMutationTools } = await import('./workspaceTools')
		const workspaceMutationTools = createWorkspaceMutationTools()

		gen.ScheduleService.previewSchedule.mockReset()
		gen.ScheduleService.createSchedule.mockReset()
		gen.HttpTriggerService.createHttpTrigger.mockReset()
		gen.ScheduleService.previewSchedule.mockResolvedValue({})
		gen.ScheduleService.createSchedule.mockResolvedValue('schedule-created')
		gen.HttpTriggerService.createHttpTrigger.mockResolvedValue('trigger-created')

		const scheduleSetToolStatus = vi.fn()
		const scheduleResult = await processToolCall({
			tools: workspaceMutationTools,
			toolCall: {
				id: 'call_5',
				type: 'function',
				function: {
					name: 'create_schedule',
					arguments: JSON.stringify({
						path: 'f/schedules/current',
						schedule: '0 0 12 * * *',
						timezone: 'UTC',
						args: null
					})
				}
			},
			helpers: {
				getWorkspaceMutationTarget: () => ({
					kind: 'script',
					path: 'f/scripts/current',
					deployed: true
				})
			},
			workspace: 'test-workspace',
			toolCallbacks: {
				setToolStatus: scheduleSetToolStatus,
				removeToolStatus: vi.fn(),
				requestConfirmation: vi.fn().mockResolvedValue(true)
			}
		})

		expect(gen.ScheduleService.createSchedule).toHaveBeenCalledWith({
			workspace: 'test-workspace',
			requestBody: expect.objectContaining({
				script_path: 'f/scripts/current',
				is_flow: false
			})
		})
		expect(scheduleSetToolStatus).toHaveBeenCalledWith(
			'call_5',
			expect.objectContaining({
				result: expect.objectContaining({
					success: true,
					path: 'f/schedules/current',
					target_path: 'f/scripts/current',
					target_kind: 'script',
					backend_result: 'schedule-created'
				})
			})
		)
		expect(JSON.parse(scheduleResult.content as string)).toEqual(
			expect.objectContaining({
				success: true,
				path: 'f/schedules/current',
				target_path: 'f/scripts/current',
				target_kind: 'script',
				backend_result: 'schedule-created'
			})
		)

		const triggerSetToolStatus = vi.fn()
		const triggerResult = await processToolCall({
			tools: workspaceMutationTools,
			toolCall: {
				id: 'call_6',
				type: 'function',
				function: {
					name: 'create_trigger',
					arguments: JSON.stringify({
						kind: 'http',
						path: 'f/triggers/current',
						config: {
							route_path: 'api/current',
							http_method: 'post',
							authentication_method: 'none',
							is_static_website: false
						}
					})
				}
			},
			helpers: {
				getWorkspaceMutationTarget: () => ({
					kind: 'flow',
					path: 'f/flows/current',
					deployed: true
				})
			},
			workspace: 'test-workspace',
			toolCallbacks: {
				setToolStatus: triggerSetToolStatus,
				removeToolStatus: vi.fn(),
				requestConfirmation: vi.fn().mockResolvedValue(true)
			}
		})

		expect(gen.HttpTriggerService.createHttpTrigger).toHaveBeenCalledWith({
			workspace: 'test-workspace',
			requestBody: expect.objectContaining({
				script_path: 'f/flows/current',
				is_flow: true
			})
		})
		expect(triggerSetToolStatus).toHaveBeenCalledWith(
			'call_6',
			expect.objectContaining({
				result: expect.objectContaining({
					success: true,
					kind: 'http',
					path: 'f/triggers/current',
					target_path: 'f/flows/current',
					target_kind: 'flow',
					backend_result: 'trigger-created'
				})
			})
		)
		expect(JSON.parse(triggerResult.content as string)).toEqual(
			expect.objectContaining({
				success: true,
				kind: 'http',
				path: 'f/triggers/current',
				target_path: 'f/flows/current',
				target_kind: 'flow',
				backend_result: 'trigger-created'
			})
		)
	})

	it('surfaces workspace mutation tool execution errors to the user', async () => {
		const gen = (await import('$lib/gen')) as any
		const { processToolCall } = await import('./shared')
		const { createWorkspaceMutationTools } = await import('./workspaceTools')
		const workspaceMutationTools = createWorkspaceMutationTools()

		gen.ScheduleService.previewSchedule.mockReset()
		gen.ScheduleService.createSchedule.mockReset()
		gen.HttpTriggerService.createHttpTrigger.mockReset()
		gen.ScheduleService.previewSchedule.mockRejectedValue(new Error('backend rejected schedule'))

		const scheduleSetToolStatus = vi.fn()
		const scheduleError = 'Invalid schedule or timezone: backend rejected schedule'
		const scheduleResult = await processToolCall({
			tools: workspaceMutationTools,
			toolCall: {
				id: 'call_7',
				type: 'function',
				function: {
					name: 'create_schedule',
					arguments: JSON.stringify({
						path: 'f/schedules/current',
						schedule: '0 0 12 * * *',
						timezone: 'UTC',
						args: null
					})
				}
			},
			helpers: {
				getWorkspaceMutationTarget: () => ({
					kind: 'script',
					path: 'f/scripts/current',
					deployed: true
				})
			},
			workspace: 'test-workspace',
			toolCallbacks: {
				setToolStatus: scheduleSetToolStatus,
				removeToolStatus: vi.fn(),
				requestConfirmation: vi.fn().mockResolvedValue(true)
			}
		})

		expect(scheduleSetToolStatus).toHaveBeenCalledWith(
			'call_7',
			expect.objectContaining({
				content: scheduleError,
				error: scheduleError,
				isLoading: false
			})
		)
		expect(scheduleResult.content).toBe(`Error while calling tool: ${scheduleError}`)
		expect(scheduleSetToolStatus).not.toHaveBeenCalledWith(
			'call_7',
			expect.objectContaining({
				error: 'An error occurred while calling the tool'
			})
		)

		gen.ScheduleService.previewSchedule.mockResolvedValue({})
		gen.HttpTriggerService.createHttpTrigger.mockRejectedValue(
			new Error('backend rejected trigger')
		)

		const triggerSetToolStatus = vi.fn()
		const triggerError =
			'Failed to create HTTP trigger "f/triggers/current": backend rejected trigger'
		const triggerResult = await processToolCall({
			tools: workspaceMutationTools,
			toolCall: {
				id: 'call_8',
				type: 'function',
				function: {
					name: 'create_trigger',
					arguments: JSON.stringify({
						kind: 'http',
						path: 'f/triggers/current',
						config: {
							route_path: 'api/current',
							http_method: 'post',
							authentication_method: 'none',
							is_static_website: false
						}
					})
				}
			},
			helpers: {
				getWorkspaceMutationTarget: () => ({
					kind: 'flow',
					path: 'f/flows/current',
					deployed: true
				})
			},
			workspace: 'test-workspace',
			toolCallbacks: {
				setToolStatus: triggerSetToolStatus,
				removeToolStatus: vi.fn(),
				requestConfirmation: vi.fn().mockResolvedValue(true)
			}
		})

		expect(triggerSetToolStatus).toHaveBeenCalledWith(
			'call_8',
			expect.objectContaining({
				content: triggerError,
				error: triggerError,
				isLoading: false
			})
		)
		expect(triggerResult.content).toBe(`Error while calling tool: ${triggerError}`)
		expect(triggerSetToolStatus).not.toHaveBeenCalledWith(
			'call_8',
			expect.objectContaining({
				error: 'An error occurred while calling the tool'
			})
		)
	})
})
