import { describe, expect, it, vi } from 'vitest'
import { z } from 'zod'
import type { DisplayMessage, ToolDisplayMessage } from './shared'

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
	JobService: { getJob: vi.fn() },
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

// deriveChatJobStatus's scheduled branch calls forLater; stub it deterministically
// (a real one pulls in stores/db-clock drift) — "later" = >5s in the future.
vi.mock('$lib/forLater', () => ({
	forLater: (scheduled: string | number | Date) => new Date(scheduled).getTime() > Date.now() + 5000
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

	it('disables strict mode for schemas with optional properties', async () => {
		const { createToolDef } = await import('./shared')
		const toolDef = createToolDef(
			z.object({
				subject: z.string(),
				language: z.string().optional()
			}),
			'get_instructions',
			'Get instructions'
		)

		const parameters = toolDef.function.parameters as any
		expect(toolDef.function.strict).toBe(false)
		expect(parameters.required).toEqual(['subject'])
		expect(parameters.properties.language.type).toBe('string')
	})

	it('keeps strict mode for schemas without optional properties', async () => {
		const { createToolDef } = await import('./shared')
		const toolDef = createToolDef(
			z.object({
				question: z.string(),
				choices: z.array(z.string())
			}),
			'askUserQuestion',
			'Ask a question'
		)

		const parameters = toolDef.function.parameters as any
		expect(toolDef.function.strict).toBe(true)
		expect(parameters.required).toEqual(['question', 'choices'])
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

describe('buildContextString', () => {
	it('serializes selected workspace items as references only', async () => {
		const { buildContextString } = await import('./shared')

		const context = buildContextString([
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

		expect(context).toContain('SELECTED WORKSPACE ITEMS:')
		expect(context).toContain('- type: script, path: f/scripts/report')
		expect(context).toContain('- type: flow, path: f/flows/reporting')
		expect(context).toContain('- type: raw_app, path: f/apps/dashboard')
		expect(context).not.toContain('Report script')
		expect(context).not.toContain('Reporting flow')
		expect(context).not.toContain('Dashboard raw app')
		expect(context).not.toContain('Code:')
		expect(context).not.toContain('Value:')
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
				isStreamingArguments: false,
				needsConfirmation: false,
				showDetails: true
			})
		)
		expect(result.content).toBe(error)
	})

	it('surfaces the real error in the tool status when the tool throws', async () => {
		const { createToolDef, processToolCall } = await import('./shared')
		const apiError = Object.assign(new Error('Bad Request'), {
			status: 400,
			body: { error: { message: 'script not found at path f/scripts/missing' } }
		})
		const setToolStatus = vi.fn()

		const result = await processToolCall({
			tools: [
				{
					def: createToolDef(z.object({}), 'run_script', 'Run script'),
					fn: vi.fn().mockRejectedValue(apiError)
				}
			],
			toolCall: {
				id: 'call_err',
				type: 'function',
				function: { name: 'run_script', arguments: '{}' }
			},
			helpers: {},
			workspace: 'test-workspace',
			toolCallbacks: {
				setToolStatus,
				removeToolStatus: vi.fn()
			}
		})

		const expectedError = 'script not found at path f/scripts/missing'
		expect(setToolStatus).toHaveBeenLastCalledWith(
			'call_err',
			expect.objectContaining({
				isLoading: false,
				error: expectedError
			})
		)
		expect(result.content).toBe(`Error while calling tool: ${expectedError}`)
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
					showDetails: true,
					autoCollapseDetails: false,
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
		expect(setToolStatus).toHaveBeenCalledWith(
			'call_2',
			expect.objectContaining({
				autoCollapseDetails: false,
				showDetails: true
			})
		)
		expect(setToolStatus).toHaveBeenLastCalledWith(
			'call_2',
			expect.objectContaining({
				isLoading: false,
				isStreamingArguments: false
			})
		)
		expect(result.content).toBe('ok')
	})

	it('auto-accepts required confirmations when yolo mode is active', async () => {
		const { createToolDef, processToolCall } = await import('./shared')
		const fn = vi.fn().mockResolvedValue('ok')
		const requestConfirmation = vi.fn()
		const setToolStatus = vi.fn()

		const result = await processToolCall({
			tools: [
				{
					def: createToolDef(z.object({}), 'create_schedule', 'Create schedule'),
					requiresConfirmation: true,
					confirmationMessage: 'Create schedule',
					fn
				}
			],
			toolCall: {
				id: 'call_yolo',
				type: 'function',
				function: { name: 'create_schedule', arguments: '{}' }
			},
			helpers: {},
			workspace: 'test-workspace',
			toolCallbacks: {
				setToolStatus,
				removeToolStatus: vi.fn(),
				requestConfirmation,
				shouldAutoAcceptToolConfirmations: () => true
			}
		})

		expect(requestConfirmation).not.toHaveBeenCalled()
		expect(fn).toHaveBeenCalled()
		expect(setToolStatus).toHaveBeenCalledWith(
			'call_yolo',
			expect.objectContaining({
				content: 'Create schedule',
				isLoading: true,
				needsConfirmation: false
			})
		)
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
				}),
				actions: [
					expect.objectContaining({
						id: 'open-created-schedule:f/schedules/current',
						type: 'open_created_resource',
						label: 'Open schedule',
						resource: 'schedule',
						path: 'f/schedules/current',
						targetKind: 'script'
					})
				]
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
				}),
				actions: [
					expect.objectContaining({
						id: 'open-created-trigger:http:f/triggers/current',
						type: 'open_created_resource',
						label: 'Open HTTP trigger',
						resource: 'trigger',
						triggerKind: 'http',
						path: 'f/triggers/current',
						targetKind: 'flow'
					})
				]
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

describe('processToolCall plan-mode gate', () => {
	async function run(
		tool: Partial<import('./shared').Tool<any>> & {
			def: import('./shared').Tool<any>['def']
			fn: import('./shared').Tool<any>['fn']
		},
		toolCallbacks: Partial<import('./shared').ToolCallbacks>
	) {
		const { processToolCall } = await import('./shared')
		return processToolCall({
			tools: [tool as import('./shared').Tool<any>],
			toolCall: {
				id: 'call_plan',
				type: 'function',
				function: { name: tool.def.function.name, arguments: '{}' }
			},
			helpers: {},
			workspace: 'test-workspace',
			toolCallbacks: {
				setToolStatus: vi.fn(),
				removeToolStatus: vi.fn(),
				...toolCallbacks
			}
		})
	}

	it('blocks a non-readonly tool while plan mode is active', async () => {
		const { createToolDef } = await import('./shared')
		const fn = vi.fn().mockResolvedValue('ran')
		const onToolBlockedByPlanMode = vi.fn()

		const result = await run(
			{ def: createToolDef(z.object({}), 'write_script', 'Write script'), fn },
			{ isPlanModeActive: () => true, onToolBlockedByPlanMode }
		)

		expect(fn).not.toHaveBeenCalled()
		expect(onToolBlockedByPlanMode).toHaveBeenCalledOnce()
		expect(result.content).toContain('plan mode is active')
	})

	it('allows a readonly-tagged tool while plan mode is active', async () => {
		const { createToolDef } = await import('./shared')
		const fn = vi.fn().mockResolvedValue('ok')

		const result = await run(
			{ def: createToolDef(z.object({}), 'read_file', 'Read file'), readonly: true, fn },
			{ isPlanModeActive: () => true }
		)

		expect(fn).toHaveBeenCalled()
		expect(result.content).toBe('ok')
	})

	it('exempts exit_plan_mode from the block even without a readonly tag', async () => {
		const { createToolDef } = await import('./shared')
		const fn = vi.fn().mockResolvedValue('Plan approved. You may now execute it.')
		const requestConfirmation = vi.fn().mockResolvedValue(true)

		const result = await run(
			{
				def: createToolDef(z.object({ summary: z.string() }), 'exit_plan_mode', 'Exit plan mode'),
				requiresConfirmation: true,
				fn
			},
			{ isPlanModeActive: () => true, requestConfirmation }
		)

		expect(fn).toHaveBeenCalled()
		expect(result.content).toBe('Plan approved. You may now execute it.')
	})

	it('runs a non-readonly tool normally when plan mode is inactive', async () => {
		const { createToolDef } = await import('./shared')
		const fn = vi.fn().mockResolvedValue('ran')

		const result = await run(
			{ def: createToolDef(z.object({}), 'write_script', 'Write script'), fn },
			{ isPlanModeActive: () => false }
		)

		expect(fn).toHaveBeenCalled()
		expect(result.content).toBe('ran')
	})

	it('does not block an unknown tool name — falls through to the unknown-tool error', async () => {
		const { processToolCall } = await import('./shared')
		const result = await processToolCall({
			tools: [],
			toolCall: {
				id: 'call_unknown',
				type: 'function',
				function: { name: 'made_up_tool', arguments: '{}' }
			},
			helpers: {},
			workspace: 'test-workspace',
			toolCallbacks: {
				setToolStatus: vi.fn(),
				removeToolStatus: vi.fn(),
				isPlanModeActive: () => true
			}
		})

		expect(result.content).not.toContain('plan mode is active')
		expect(result.content).toContain('Unknown tool call')
	})

	it('returns the tool cancellationMessage when the user rejects the confirmation', async () => {
		const { createToolDef } = await import('./shared')
		const fn = vi.fn()

		const result = await run(
			{
				def: createToolDef(z.object({ summary: z.string() }), 'exit_plan_mode', 'Exit plan mode'),
				requiresConfirmation: true,
				cancellationMessage: 'keep planning',
				fn
			},
			{ isPlanModeActive: () => true, requestConfirmation: vi.fn().mockResolvedValue(false) }
		)

		expect(fn).not.toHaveBeenCalled()
		expect(result.content).toBe('keep planning')
	})
})

describe('isActiveUserQuestion', () => {
	function toolMessage(overrides: Partial<ToolDisplayMessage> = {}): ToolDisplayMessage {
		return {
			role: 'tool',
			tool_call_id: 'call_q',
			content: 'asking a question',
			isLoading: true,
			userQuestion: { question: 'Pick one', choices: ['a', 'b'] },
			...overrides
		}
	}

	it('is true for a loading tool message with an unanswered question', async () => {
		const { isActiveUserQuestion } = await import('./shared')
		expect(isActiveUserQuestion(toolMessage())).toBe(true)
	})

	it('is false once choices have been selected', async () => {
		const { isActiveUserQuestion } = await import('./shared')
		expect(
			isActiveUserQuestion(
				toolMessage({
					userQuestion: { question: 'Pick one', choices: ['a', 'b'], selectedChoices: ['a'] }
				})
			)
		).toBe(false)
	})

	it('is false once a legacy scalar selectedChoice is present', async () => {
		const { isActiveUserQuestion } = await import('./shared')
		expect(
			isActiveUserQuestion(
				toolMessage({
					userQuestion: { question: 'Pick one', choices: ['a', 'b'], selectedChoice: 'a' }
				})
			)
		).toBe(false)
	})

	it('stays active when selectedChoices is present but empty', async () => {
		const { isActiveUserQuestion } = await import('./shared')
		expect(
			isActiveUserQuestion(
				toolMessage({
					userQuestion: { question: 'Pick one', choices: ['a', 'b'], selectedChoices: [] }
				})
			)
		).toBe(true)
	})

	it('is false when the question was canceled', async () => {
		const { isActiveUserQuestion } = await import('./shared')
		expect(
			isActiveUserQuestion(
				toolMessage({ userQuestion: { question: 'Pick one', choices: ['a', 'b'], canceled: true } })
			)
		).toBe(false)
	})

	it('is false when the tool errored', async () => {
		const { isActiveUserQuestion } = await import('./shared')
		expect(isActiveUserQuestion(toolMessage({ error: 'boom' }))).toBe(false)
	})

	it('is false when the tool is no longer loading', async () => {
		const { isActiveUserQuestion } = await import('./shared')
		expect(isActiveUserQuestion(toolMessage({ isLoading: false }))).toBe(false)
	})

	it('is false for a tool message without a question', async () => {
		const { isActiveUserQuestion } = await import('./shared')
		expect(isActiveUserQuestion(toolMessage({ userQuestion: undefined }))).toBe(false)
	})

	it('is false for non-tool messages and undefined', async () => {
		const { isActiveUserQuestion } = await import('./shared')
		const userMessage: DisplayMessage = { role: 'user', index: 0, content: 'hi' }
		const assistantMessage: DisplayMessage = { role: 'assistant', content: 'hi' }
		expect(isActiveUserQuestion(undefined)).toBe(false)
		expect(isActiveUserQuestion(userMessage)).toBe(false)
		expect(isActiveUserQuestion(assistantMessage)).toBe(false)
	})
})

describe('pollJobCompletion detach', () => {
	function makeCallbacks() {
		return {
			setToolStatus: vi.fn(),
			removeToolStatus: vi.fn(),
			onJobStatus: vi.fn()
		}
	}

	it('detaches immediately (no polling) when detachAfterMs is 0', async () => {
		const { pollJobCompletion } = await import('./shared')
		const { JobService } = await import('$lib/gen')
		const getJob = vi.mocked(JobService.getJob)
		getJob.mockReset()
		const cbs = makeCallbacks()

		const outcome = await pollJobCompletion('job1', 'w', 'tool1', cbs as any, { detachAfterMs: 0 })

		expect(outcome).toBe('detached')
		expect(getJob).not.toHaveBeenCalled()
	})

	it('detaches after the inline budget when the job is still running', async () => {
		vi.useFakeTimers()
		try {
			const { pollJobCompletion } = await import('./shared')
			const { JobService } = await import('$lib/gen')
			const getJob = vi.mocked(JobService.getJob)
			getJob.mockReset()
			getJob.mockResolvedValue({ type: 'QueuedJob', running: true } as any)
			const cbs = makeCallbacks()

			// detachAfterMs 2000 → 2 polls at 1s each, then detach.
			const promise = pollJobCompletion('job1', 'w', 'tool1', cbs as any, { detachAfterMs: 2000 })
			await vi.advanceTimersByTimeAsync(2000)

			expect(await promise).toBe('detached')
			// Status is reported as running during the wait (alongside the trimmed
			// Job snapshot that feeds JobStatusIcon).
			expect(cbs.onJobStatus).toHaveBeenCalledWith(
				'job1',
				expect.objectContaining({ status: 'running' })
			)
		} finally {
			vi.useRealTimers()
		}
	})

	it('returns the completed job when it finishes within the inline budget', async () => {
		vi.useFakeTimers()
		try {
			const { pollJobCompletion } = await import('./shared')
			const { JobService } = await import('$lib/gen')
			const getJob = vi.mocked(JobService.getJob)
			getJob.mockReset()
			const completed = { type: 'CompletedJob', success: true, result: 42 }
			getJob.mockResolvedValue(completed as any)
			const cbs = makeCallbacks()

			const promise = pollJobCompletion('job1', 'w', 'tool1', cbs as any, { detachAfterMs: 15000 })
			await vi.advanceTimersByTimeAsync(1000)

			expect(await promise).toBe(completed)
		} finally {
			vi.useRealTimers()
		}
	})

	it('legacy mode (no detach) throws a timeout error when the job never completes', async () => {
		vi.useFakeTimers()
		try {
			const { pollJobCompletion } = await import('./shared')
			const { JobService } = await import('$lib/gen')
			const getJob = vi.mocked(JobService.getJob)
			getJob.mockReset()
			getJob.mockResolvedValue({ type: 'QueuedJob', running: true } as any)
			const cbs = makeCallbacks()

			const promise = pollJobCompletion('job1', 'w', 'tool1', cbs as any)
			const assertion = expect(promise).rejects.toThrow('timed out')
			await vi.advanceTimersByTimeAsync(60000)
			await assertion
			expect(cbs.setToolStatus).toHaveBeenCalledWith(
				'tool1',
				expect.objectContaining({ error: expect.any(String) })
			)
		} finally {
			vi.useRealTimers()
		}
	})
})

describe('deriveChatJobStatus', () => {
	// CompletedJob is discriminated by the presence of a `success` key; the branch
	// order deliberately mirrors JobStatusIcon so the badge and scalar never drift.
	it('maps a canceled completed job to canceled (canceled wins over success=false)', async () => {
		const { deriveChatJobStatus } = await import('./shared')
		expect(deriveChatJobStatus({ success: false, canceled: true } as any)).toBe('canceled')
	})

	it('maps a successful completed job to success', async () => {
		const { deriveChatJobStatus } = await import('./shared')
		expect(deriveChatJobStatus({ success: true, canceled: false } as any)).toBe('success')
	})

	it('maps a non-canceled failed completed job to failure', async () => {
		const { deriveChatJobStatus } = await import('./shared')
		expect(deriveChatJobStatus({ success: false, canceled: false } as any)).toBe('failure')
	})

	it('maps a running suspended queued job to suspended', async () => {
		const { deriveChatJobStatus } = await import('./shared')
		expect(deriveChatJobStatus({ running: true, suspend: 1 } as any)).toBe('suspended')
	})

	it('maps a running queued job to running', async () => {
		const { deriveChatJobStatus } = await import('./shared')
		expect(deriveChatJobStatus({ running: true } as any)).toBe('running')
	})

	it('maps a future-scheduled queued job to scheduled', async () => {
		const { deriveChatJobStatus } = await import('./shared')
		const future = new Date(Date.now() + 3_600_000).toISOString()
		expect(deriveChatJobStatus({ running: false, scheduled_for: future } as any)).toBe('scheduled')
	})

	it('maps a plain (non-running, non-scheduled) queued job to queued', async () => {
		const { deriveChatJobStatus } = await import('./shared')
		expect(deriveChatJobStatus({ running: false } as any)).toBe('queued')
	})
})

describe('trimJob', () => {
	const HEAVY = ['logs', 'args', 'result', 'raw_code', 'raw_flow', 'flow_status']

	it('preserves the ABSENCE of a success key on a queued job (JobStatusIcon in-operator invariant)', async () => {
		const { trimJob } = await import('./shared')
		const queued = {
			id: 'j1',
			running: true,
			logs: 'x',
			args: {},
			result: 1,
			raw_code: 'c',
			raw_flow: {},
			flow_status: {}
		}
		const trimmed = trimJob(queued as any)
		// The load-bearing invariant: a running/queued job must NOT gain a `success`
		// key, or deriveChatJobStatus/JobStatusIcon would misread it as completed.
		expect('success' in trimmed).toBe(false)
		expect(trimmed.running).toBe(true)
		expect(trimmed.id).toBe('j1')
	})

	it('deletes the six heavy fields but keeps the status-discriminant scalar', async () => {
		const { trimJob } = await import('./shared')
		const job = {
			id: 'j1',
			success: true,
			logs: 'x',
			args: { a: 1 },
			result: [1],
			raw_code: 'c',
			raw_flow: { modules: [] },
			flow_status: { step: 0 }
		}
		const trimmed = trimJob(job as any) as Record<string, unknown>
		for (const k of HEAVY) expect(k in trimmed).toBe(false)
		expect('success' in trimmed).toBe(true)
		expect(trimmed.success).toBe(true)
	})

	it('does not mutate the input job', async () => {
		const { trimJob } = await import('./shared')
		const job = { id: 'j1', success: true, result: 42 }
		trimJob(job as any)
		expect(job.result).toBe(42)
	})
})
