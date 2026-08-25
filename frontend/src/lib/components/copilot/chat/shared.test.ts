import { describe, expect, it, vi } from 'vitest'
import { z } from 'zod'
import type { DisplayMessage, ToolDisplayMessage } from './shared'
import { openItemPreviewAction } from './shared'

vi.mock('monaco-editor', () => ({
	editor: {}
}))

vi.mock('$lib/utils/featureUsage', () => ({
	logFeatureUsage: vi.fn(),
	logHubScriptPick: vi.fn()
}))

const userHolder = vi.hoisted(() => ({
	current: { is_super_admin: true } as { is_super_admin: boolean }
}))

vi.mock('$lib/stores', () => ({
	workspaceStore: { subscribe: () => () => undefined },
	userStore: {
		subscribe: (run: (value: { is_super_admin: boolean }) => void) => {
			run(userHolder.current)
			return () => {}
		}
	}
}))

vi.mock('$lib/components/triggers/email/utils', () => ({
	getEmailAddress: (localPart: string, _wlp: boolean, _wsId: string, domain: string) =>
		`${localPart}@${domain}`
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
	AzureTriggerService: { createAzureTrigger: vi.fn() },
	AmqpTriggerService: { createAmqpTrigger: vi.fn() },
	EmailTriggerService: { createEmailTrigger: vi.fn() },
	SettingService: { getGlobal: vi.fn() }
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
		// config stays open-ended; get_trigger_schema serves the per-kind schemas instead.
		expect(parameters?.properties?.config?.anyOf).toBeUndefined()
		expect(JSON.stringify(toolDef).length).toBeLessThan(2000)
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
		const { triggerConfigSchemas } = await import('./workspaceToolsZod.gen')
		const [scheduleTool] = createWorkspaceMutationTools()

		const scheduleParameters = scheduleTool.def.function.parameters as any
		expect(scheduleParameters?.properties?.script_path).toBeUndefined()
		expect(scheduleParameters?.properties?.is_flow).toBeUndefined()

		// These come from the runnable the chat is editing, so the model must not see them
		// on any kind, including in the schemas get_trigger_schema serves on demand.
		const getTriggerSchema = createWorkspaceMutationTools().find(
			(tool) => tool.def.function.name === 'get_trigger_schema'
		)!
		for (const kind of Object.keys(triggerConfigSchemas)) {
			const served = JSON.parse(await getTriggerSchema.fn({ args: { kind } } as any))
			expect(served.properties?.script_path).toBeUndefined()
			expect(served.properties?.is_flow).toBeUndefined()
			expect(served.properties?.path).toBeUndefined()
		}

		// Same for the schedule options served on demand. The runnable target still wins
		// on merge, so this is about not advertising a field the model cannot influence.
		const getScheduleSchema = createWorkspaceMutationTools().find(
			(tool) => tool.def.function.name === 'get_schedule_schema'
		)!
		const schedule = JSON.parse(await getScheduleSchema.fn({ args: {} } as any))
		expect(schedule.properties?.script_path).toBeUndefined()
		expect(schedule.properties?.is_flow).toBeUndefined()
		expect(schedule.properties).toHaveProperty('retry')
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

		expect(requestConfirmation).toHaveBeenCalledWith('call_2', 'create_schedule')
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

	// create_schedule duplicates the global `advanced` merge and guard, so it needs its
	// own coverage: the fold must reach the request body and a mis-shape must stop the
	// write rather than create a schedule with an inert policy.
	it('folds advanced schedule options into create_schedule and refuses a mis-shaped one', async () => {
		const gen = (await import('$lib/gen')) as any
		const { processToolCall } = await import('./shared')
		const { createWorkspaceMutationTools } = await import('./workspaceTools')
		const workspaceMutationTools = createWorkspaceMutationTools()

		gen.ScheduleService.previewSchedule.mockReset()
		gen.ScheduleService.createSchedule.mockReset()
		gen.ScheduleService.previewSchedule.mockResolvedValue({})
		gen.ScheduleService.createSchedule.mockResolvedValue('schedule-created')

		const target = {
			getWorkspaceMutationTarget: () => ({
				kind: 'script' as const,
				path: 'f/scripts/current',
				deployed: true
			})
		}
		const callbacks = () => ({
			setToolStatus: vi.fn(),
			removeToolStatus: vi.fn(),
			requestConfirmation: vi.fn().mockResolvedValue(true)
		})

		await processToolCall({
			tools: workspaceMutationTools,
			toolCall: {
				id: 'call_adv',
				type: 'function',
				function: {
					name: 'create_schedule',
					arguments: JSON.stringify({
						path: 'f/schedules/adv',
						schedule: '0 0 12 * * *',
						timezone: 'UTC',
						args: null,
						advanced: { tag: 'nightly', retry: { constant: { attempts: 2, seconds: 30 } } }
					})
				}
			},
			helpers: target,
			workspace: 'test-workspace',
			toolCallbacks: callbacks()
		})

		expect(gen.ScheduleService.createSchedule).toHaveBeenCalledWith({
			workspace: 'test-workspace',
			requestBody: expect.objectContaining({
				tag: 'nightly',
				retry: { constant: { attempts: 2, seconds: 30 } }
			})
		})

		gen.ScheduleService.createSchedule.mockReset()
		const badStatus = vi.fn()
		await processToolCall({
			tools: workspaceMutationTools,
			toolCall: {
				id: 'call_bad',
				type: 'function',
				function: {
					name: 'create_schedule',
					arguments: JSON.stringify({
						path: 'f/schedules/bad',
						schedule: '0 0 12 * * *',
						timezone: 'UTC',
						args: null,
						advanced: { retry: { constant: { attempts: 2, seconds_typo: 30 } } }
					})
				}
			},
			helpers: target,
			workspace: 'test-workspace',
			toolCallbacks: { ...callbacks(), setToolStatus: badStatus }
		})

		expect(gen.ScheduleService.createSchedule).not.toHaveBeenCalled()
		expect(JSON.stringify(badStatus.mock.calls)).toContain('get_schedule_schema')
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

	it('email trigger: guides the user to set up email triggering when unconfigured', async () => {
		const gen = (await import('$lib/gen')) as any
		const { processToolCall } = await import('./shared')
		const { createWorkspaceMutationTools } = await import('./workspaceTools')
		const tools = createWorkspaceMutationTools()

		gen.SettingService.getGlobal.mockReset()
		gen.SettingService.getGlobal.mockResolvedValue(null)
		gen.EmailTriggerService.createEmailTrigger.mockReset()

		const call = (id: string) =>
			processToolCall({
				tools,
				toolCall: {
					id,
					type: 'function',
					function: {
						name: 'create_trigger',
						arguments: JSON.stringify({
							kind: 'email',
							path: 'f/triggers/email_current',
							config: { local_part: 'orders' }
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
					setToolStatus: vi.fn(),
					removeToolStatus: vi.fn(),
					requestConfirmation: vi.fn().mockResolvedValue(true)
				}
			})

		userHolder.current = { is_super_admin: true }
		const superadminResult = await call('call_email_super')
		expect(gen.EmailTriggerService.createEmailTrigger).not.toHaveBeenCalled()
		expect(superadminResult.content).toContain('not set up')
		expect(superadminResult.content).toContain('As a superadmin')

		userHolder.current = { is_super_admin: false }
		const memberResult = await call('call_email_member')
		expect(gen.EmailTriggerService.createEmailTrigger).not.toHaveBeenCalled()
		expect(memberResult.content).toContain('Ask an instance superadmin')
	})

	it('email trigger: creates it and reports the address when email triggering is configured', async () => {
		const gen = (await import('$lib/gen')) as any
		const { processToolCall } = await import('./shared')
		const { createWorkspaceMutationTools } = await import('./workspaceTools')
		const tools = createWorkspaceMutationTools()

		gen.SettingService.getGlobal.mockReset()
		gen.SettingService.getGlobal.mockResolvedValue('mail.example.com')
		gen.EmailTriggerService.createEmailTrigger.mockReset()
		gen.EmailTriggerService.createEmailTrigger.mockResolvedValue('email-created')

		const result = await processToolCall({
			tools,
			toolCall: {
				id: 'call_email_ok',
				type: 'function',
				function: {
					name: 'create_trigger',
					arguments: JSON.stringify({
						kind: 'email',
						path: 'f/triggers/email_current',
						config: { local_part: 'orders' }
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
				setToolStatus: vi.fn(),
				removeToolStatus: vi.fn(),
				requestConfirmation: vi.fn().mockResolvedValue(true)
			}
		})

		expect(gen.EmailTriggerService.createEmailTrigger).toHaveBeenCalledWith({
			workspace: 'test-workspace',
			requestBody: expect.objectContaining({
				local_part: 'orders',
				// defaulted before the request is sent; the backend column is NOT NULL
				workspaced_local_part: false,
				script_path: 'f/flows/current',
				is_flow: true
			})
		})
		expect(JSON.parse(result.content as string)).toEqual(
			expect.objectContaining({
				success: true,
				kind: 'email',
				email_address: 'orders@mail.example.com',
				backend_result: 'email-created'
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

	// The counter is silently dropped by the backend when the key is malformed, so nothing
	// here fails loudly if a path stops logging or logs the wrong status.
	it('logs one feature-usage outcome per tool call, keyed <tool>:<status>', async () => {
		const { createToolDef, processToolCall } = await import('./shared')
		const { logFeatureUsage } = await import('$lib/utils/featureUsage')

		const outcomeKeys = async (
			tool: Partial<import('./shared').Tool<any>> = {},
			toolCallbacks: Partial<import('./shared').ToolCallbacks> = {}
		) => {
			vi.mocked(logFeatureUsage).mockClear()
			await runToolCall(
				{
					def: createToolDef(z.object({}), 'run_script', 'Run script'),
					fn: vi.fn().mockResolvedValue('done'),
					...tool
				},
				toolCallbacks
			)
			return vi
				.mocked(logFeatureUsage)
				.mock.calls.map(([feature, kind, opts]) => [feature, kind, opts?.key])
		}

		expect(await outcomeKeys()).toEqual([['ai_chat', 'tool', 'run_script:ok']])
		expect(await outcomeKeys({ fn: vi.fn().mockRejectedValue(new Error('boom')) })).toEqual([
			['ai_chat', 'tool', 'run_script:error']
		])
		expect(await outcomeKeys({ validateBeforeConfirmation: () => 'not deployed' })).toEqual([
			['ai_chat', 'tool', 'run_script:rejected']
		])
		expect(
			await outcomeKeys(
				{ requiresConfirmation: true },
				{ requestConfirmation: vi.fn().mockResolvedValue(false) }
			)
		).toEqual([['ai_chat', 'tool', 'run_script:declined']])
		expect(await outcomeKeys({}, { isPlanModeActive: () => true })).toEqual([
			['ai_chat', 'tool', 'run_script:blocked_plan_mode']
		])

		// A name the model invented resolves to no tool, and must never reach telemetry.
		vi.mocked(logFeatureUsage).mockClear()
		await processToolCall({
			tools: [{ def: createToolDef(z.object({}), 'run_script', 'Run script'), fn: vi.fn() }],
			toolCall: {
				id: 'call_ghost',
				type: 'function',
				function: { name: 'hallucinated_tool', arguments: '{}' }
			},
			helpers: {},
			workspace: 'test-workspace',
			toolCallbacks: { setToolStatus: vi.fn(), removeToolStatus: vi.fn() }
		})
		expect(logFeatureUsage).not.toHaveBeenCalled()
	})
})

async function runToolCall(
	tool: Partial<import('./shared').Tool<any>> & {
		def: import('./shared').Tool<any>['def']
		fn: import('./shared').Tool<any>['fn']
	},
	toolCallbacks: Partial<import('./shared').ToolCallbacks>,
	args: Record<string, unknown> = {}
) {
	const { processToolCall } = await import('./shared')
	return processToolCall({
		tools: [tool as import('./shared').Tool<any>],
		toolCall: {
			id: 'call_plan',
			type: 'function',
			function: { name: tool.def.function.name, arguments: JSON.stringify(args) }
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

describe('processToolCall plan-mode gate', () => {
	it('blocks an untagged tool while plan mode is active', async () => {
		const { createToolDef } = await import('./shared')
		const fn = vi.fn().mockResolvedValue('ran')
		const onToolBlockedByPlanMode = vi.fn()

		const result = await runToolCall(
			{ def: createToolDef(z.object({}), 'write_script', 'Write script'), fn },
			{ isPlanModeActive: () => true, onToolBlockedByPlanMode }
		)

		expect(fn).not.toHaveBeenCalled()
		expect(onToolBlockedByPlanMode).toHaveBeenCalledOnce()
		expect(result.content).toContain('plan mode is active')
	})

	it('blocks a mutating tool before its own validator gets to run', async () => {
		const { createToolDef } = await import('./shared')
		const fn = vi.fn().mockResolvedValue('ran')
		const validateBeforeConfirmation = vi.fn().mockResolvedValue('target is undeployed')
		const onToolBlockedByPlanMode = vi.fn()

		const result = await runToolCall(
			{
				def: createToolDef(z.object({}), 'write_script', 'Write script'),
				validateBeforeConfirmation,
				fn
			},
			{ isPlanModeActive: () => true, onToolBlockedByPlanMode }
		)

		expect(validateBeforeConfirmation).not.toHaveBeenCalled()
		expect(onToolBlockedByPlanMode).toHaveBeenCalledOnce()
		expect(result.content).toContain('plan mode is active')
	})

	it('allows a plan-mode-safe tool while plan mode is active', async () => {
		const { createToolDef } = await import('./shared')
		const fn = vi.fn().mockResolvedValue('ok')

		const result = await runToolCall(
			{ def: createToolDef(z.object({}), 'read_file', 'Read file'), planModeSafe: true, fn },
			{ isPlanModeActive: () => true }
		)

		expect(fn).toHaveBeenCalled()
		expect(result.content).toBe('ok')
	})

	it('runs an untagged tool normally when plan mode is inactive', async () => {
		const { createToolDef } = await import('./shared')
		const fn = vi.fn().mockResolvedValue('ran')

		const result = await runToolCall(
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
		const setToolStatus = vi.fn()

		const result = await runToolCall(
			{
				def: createToolDef(z.object({ summary: z.string() }), 'exit_plan_mode', 'Exit plan mode'),
				planModeSafe: true,
				requiresConfirmation: true,
				cancellationMessage: 'keep planning',
				fn
			},
			{
				isPlanModeActive: () => true,
				requestConfirmation: vi.fn().mockResolvedValue(false),
				setToolStatus
			}
		)

		expect(fn).not.toHaveBeenCalled()
		expect(result.content).toBe('keep planning')
		// The one place a decline is recorded: planCardState reads it to tell a plan the
		// user turned down apart from a call that merely errored.
		expect(setToolStatus).toHaveBeenCalledWith(
			'call_plan',
			expect.objectContaining({ declinedByUser: true })
		)
	})

	it('blocks a mutating tool if plan mode is entered while its confirmation is pending', async () => {
		const { createToolDef } = await import('./shared')
		const fn = vi.fn().mockResolvedValue('ran')
		let planActive = false
		// The user switches into plan mode while the confirmation card is open, then
		// approves it: requestConfirmation flips the posture, then resolves true.
		const requestConfirmation = vi.fn().mockImplementation(async () => {
			planActive = true
			return true
		})

		const result = await runToolCall(
			{
				def: createToolDef(z.object({}), 'write_script', 'Write script'),
				requiresConfirmation: true,
				fn
			},
			{ isPlanModeActive: () => planActive, requestConfirmation }
		)

		expect(requestConfirmation).toHaveBeenCalled()
		expect(fn).not.toHaveBeenCalled()
		expect(result.content).toContain('plan mode is active')
	})

	it('refuses only the arguments a tagged tool names, in its own words', async () => {
		const { createToolDef } = await import('./shared')
		const fn = vi.fn().mockResolvedValue('ran')
		const setToolStatus = vi.fn()
		const tool = {
			def: createToolDef(z.object({ id: z.string() }), 'update_doc', 'Update a doc'),
			planModeSafe: true,
			refuseInPlanMode: ({ args }: { args: any }) =>
				args.id === 'the-plan'
					? { label: 'Not that one', result: 'Write anything but the plan.' }
					: undefined,
			fn
		}

		const refused = await runToolCall(
			tool,
			{ isPlanModeActive: () => true, setToolStatus },
			{
				id: 'the-plan'
			}
		)
		expect(fn).not.toHaveBeenCalled()
		expect(refused.content).toBe('Write anything but the plan.')
		expect(setToolStatus).toHaveBeenCalledWith(
			'call_plan',
			expect.objectContaining({ content: 'Not that one', blockedByPlanMode: true })
		)

		const allowed = await runToolCall(tool, { isPlanModeActive: () => true }, { id: 'a-note' })
		expect(fn).toHaveBeenCalled()
		expect(allowed.content).toBe('ran')
	})

	it('never asks a tool which arguments it refuses once plan mode is over', async () => {
		// The posture is the only thing that makes this hook relevant: consulted outside it, a
		// tool that narrows itself for planning would narrow itself for every other mode too.
		const { createToolDef } = await import('./shared')
		const fn = vi.fn().mockResolvedValue('ran')
		const refuseInPlanMode = vi.fn().mockReturnValue('refused')

		const result = await runToolCall(
			{
				def: createToolDef(z.object({ id: z.string() }), 'update_doc', 'Update a doc'),
				planModeSafe: true,
				refuseInPlanMode,
				fn
			},
			{ isPlanModeActive: () => false },
			{ id: 'the-plan' }
		)

		expect(refuseInPlanMode).not.toHaveBeenCalled()
		expect(fn).toHaveBeenCalled()
		expect(result.content).toBe('ran')
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

describe('pendingUserAction', () => {
	const toolMessage = (overrides: Partial<ToolDisplayMessage> = {}): ToolDisplayMessage => ({
		role: 'tool',
		tool_call_id: 'call_p',
		content: 'running',
		isLoading: true,
		...overrides
	})

	const question = toolMessage({ userQuestion: { question: 'Pick one', choices: ['a'] } })

	it('distinguishes an unanswered question from a staged confirmation', async () => {
		const { pendingUserAction } = await import('./shared')
		expect(pendingUserAction([question])).toBe('question')
		expect(pendingUserAction([toolMessage({ needsConfirmation: true })])).toBe('confirmation')
	})

	it('is undefined for a tool the AI is running on its own', async () => {
		const { pendingUserAction } = await import('./shared')
		expect(pendingUserAction([toolMessage()])).toBe(undefined)
		expect(pendingUserAction([toolMessage({ needsConfirmation: true, isLoading: false })])).toBe(
			undefined
		)
	})

	// A multi-tool turn creates every card before running the calls one at a time,
	// so the blocked card is not the last message.
	it('finds a blocked card sitting behind queued ones', async () => {
		const { pendingUserAction } = await import('./shared')
		expect(pendingUserAction([question, toolMessage(), toolMessage()])).toBe('question')
		expect(pendingUserAction([toolMessage({ needsConfirmation: true }), toolMessage()])).toBe(
			'confirmation'
		)
	})

	// Text emitted between two tool calls lands as an assistant card between them.
	it('finds a blocked card behind an interleaved assistant card', async () => {
		const { pendingUserAction } = await import('./shared')
		const assistant: DisplayMessage = { role: 'assistant', content: 'and also…' }
		expect(pendingUserAction([question, assistant, toolMessage()])).toBe('question')
	})

	it('stops at the previous turn rather than reviving its resolved cards', async () => {
		const { pendingUserAction } = await import('./shared')
		const userMessage: DisplayMessage = { role: 'user', index: 0, content: 'go on' }
		expect(pendingUserAction([question, userMessage, toolMessage()])).toBe(undefined)
	})

	// The composer answers the parked question by id, so the scan must name the
	// blocked card and not one of the queued ones sharing the turn.
	it('names the blocked card so a caller can resolve it', async () => {
		const { pendingUserActionDetail } = await import('./shared')
		const blocked = toolMessage({
			tool_call_id: 'call_ask',
			userQuestion: { question: 'Pick one', choices: ['a'] }
		})
		expect(pendingUserActionDetail([blocked, toolMessage({ tool_call_id: 'call_next' })])).toEqual({
			action: 'question',
			toolCallId: 'call_ask'
		})
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

describe('processToolCall preAction', () => {
	// preAction's "-ing" label must land at execution start, not stream time —
	// firing it earlier would relabel a still-queued card as active.
	it('invokes preAction at promotion, before the tool fn runs', async () => {
		const { processToolCall } = await import('./shared')
		const calls: string[] = []
		const tool = {
			def: { type: 'function' as const, function: { name: 'patch_app_file', parameters: {} } },
			preAction: () => calls.push('preAction'),
			fn: vi.fn().mockImplementation(async () => {
				calls.push('fn')
				return 'ok'
			})
		}
		await processToolCall({
			tools: [tool] as any,
			toolCall: {
				id: 'call_1',
				type: 'function',
				function: { name: 'patch_app_file', arguments: '{}' }
			},
			helpers: {},
			toolCallbacks: { setToolStatus: vi.fn() } as any,
			workspace: 'test'
		})
		expect(calls).toEqual(['preAction', 'fn'])
	})
})

describe('queuedToolStatus', () => {
	const tool = (extra: Record<string, unknown> = {}) => ({
		def: { type: 'function' as const, function: { name: 'run_script', parameters: {} } },
		fn: vi.fn(),
		...extra
	})

	it('humanizes snake_case and camelCase tool names by default', async () => {
		const { queuedToolStatus } = await import('./shared')
		expect(queuedToolStatus([], 'run_script', '{}')).toMatchObject({
			isLoading: false,
			isQueued: true,
			isStreamingArguments: false,
			content: 'Run script'
		})
		expect(queuedToolStatus([], 'askUserQuestion', '{}').content).toBe('Ask user question')
	})

	it('derives the label from parsed args via queuedLabel', async () => {
		const { queuedToolStatus } = await import('./shared')
		const t = tool({ queuedLabel: (args: any) => `Test ${args.path}` })
		expect(queuedToolStatus([t] as any, 'run_script', '{"path": "u/admin/x"}').content).toBe(
			'Test u/admin/x'
		)
	})

	it('falls back to the humanized name when args are truncated', async () => {
		const { queuedToolStatus } = await import('./shared')
		const t = tool({ queuedLabel: (args: any) => `Test ${args.path}` })
		expect(queuedToolStatus([t] as any, 'run_script', '{"path": "u/adm').content).toBe('Run script')
	})
})

describe('appendPendingToolImages', () => {
	// Tool results are string-only, so tool-produced images ride a follow-up
	// user message appended after the whole tool batch. It must land in BOTH
	// arrays (messages = sent next iteration, addedMessages = committed to
	// history) and drain the buffer exactly once — a second flush appending the
	// same screenshots again would duplicate them in history.
	it('appends one user message to both arrays and drains the buffer once', async () => {
		const { appendPendingToolImages } = await import('./shared')
		let pending = [{ dataUrl: 'data:image/png;base64,SHOT', mediaType: 'image/png' as const }]
		const toolCallbacks = {
			setToolStatus: vi.fn(),
			takePendingToolImages: () => {
				const taken = pending
				pending = []
				return taken
			}
		}
		const messages: any[] = []
		const addedMessages: any[] = []

		appendPendingToolImages(messages, addedMessages, toolCallbacks as any)

		expect(messages).toHaveLength(1)
		expect(messages[0]).toBe(addedMessages[0])
		expect(messages[0].role).toBe('user')
		expect(messages[0].content[1]).toEqual({
			type: 'image_url',
			image_url: { url: 'data:image/png;base64,SHOT' }
		})

		appendPendingToolImages(messages, addedMessages, toolCallbacks as any)
		expect(messages).toHaveLength(1)
		expect(addedMessages).toHaveLength(1)
	})
})

describe('openItemPreviewAction', () => {
	// The action's `type` is the key the sessions page registers its handler under,
	// so it must stay 'open_item_preview'; `previewKind`/`path` are passed verbatim
	// to previewTargetForSessionTarget.
	it('carries the kind and path through to the dispatch action', () => {
		expect(openItemPreviewAction('flow', 'f/team/etl')).toEqual({
			id: 'open-item-preview:flow:f/team/etl',
			type: 'open_item_preview',
			label: 'Open flow preview',
			previewKind: 'flow',
			path: 'f/team/etl'
		})
	})

	// raw_app is the internal kind; the user-facing label says "app".
	it('labels raw_app as "app"', () => {
		expect(openItemPreviewAction('raw_app', 'u/me/dash').label).toBe('Open app preview')
	})
})

describe('createSearchHubScriptsTool', () => {
	const hit = (version_id: number, app: string, summary: string) => ({
		version_id,
		app,
		summary,
		ask_id: version_id,
		id: version_id,
		kind: 'script' as const,
		score: 1
	})

	async function runWithContent(getHubScriptByPath: ReturnType<typeof vi.fn>) {
		const { ScriptService } = await import('$lib/gen')
		Object.assign(ScriptService, {
			queryHubScripts: vi.fn(async () => [
				hit(1, 'discord', 'Send a message'),
				hit(2, 'slack', 'Post a message')
			]),
			getHubScriptByPath
		})
		const { createSearchHubScriptsTool } = await import('./shared')
		const raw = await createSearchHubScriptsTool(true).fn({
			args: { query: 'send a message' },
			toolId: 't1',
			toolCallbacks: { setToolStatus: vi.fn() }
		} as any)
		return JSON.parse(raw)
	}

	it('reports each script language alongside its content', async () => {
		const results = await runWithContent(
			vi.fn(async ({ path }: { path: string }) => ({
				content: `// ${path}`,
				language: path.startsWith('hub/1/') ? 'bunnative' : 'python3'
			}))
		)

		expect(results).toEqual([
			{
				path: 'hub/1/discord/send_a_message',
				summary: 'Send a message',
				language: 'bunnative',
				content: '// hub/1/discord/send_a_message'
			},
			{
				path: 'hub/2/slack/post_a_message',
				summary: 'Post a message',
				language: 'python3',
				content: '// hub/2/slack/post_a_message'
			}
		])
	})

	it('keeps the other results when one content fetch fails', async () => {
		const results = await runWithContent(
			vi.fn(async ({ path }: { path: string }) => {
				if (path.startsWith('hub/1/')) throw new Error('hub unreachable')
				return { content: 'ok', language: 'python3' }
			})
		)

		expect(results[0].error).toContain('hub unreachable')
		expect(results[0].content).toBeUndefined()
		expect(results[1].content).toBe('ok')
	})
})

describe('processToolCall confirmation hooks', () => {
	async function hookedTool() {
		const { createToolDef } = await import('./shared')
		return {
			def: createToolDef(z.object({}), 'apply_change', 'Apply change'),
			requiresConfirmation: true,
			fn: vi.fn().mockResolvedValue('ok'),
			onConfirmationRequested: vi.fn()
		}
	}

	it('requests before the card resolves', async () => {
		const tool = await hookedTool()
		let requestedBeforeResolve = false
		const requestConfirmation = vi.fn(async () => {
			requestedBeforeResolve = tool.onConfirmationRequested.mock.calls.length === 1
			return false
		})

		await runToolCall(tool, { requestConfirmation })

		expect(requestedBeforeResolve).toBe(true)
		// The hook needs the id and callbacks to attach what it sets up to this card.
		expect(tool.onConfirmationRequested).toHaveBeenCalledWith(
			expect.objectContaining({ toolId: 'call_plan', toolCallbacks: expect.any(Object) })
		)
		expect(tool.fn).not.toHaveBeenCalled()
	})

	it('fires no hook when the confirmation is auto-accepted', async () => {
		const tool = await hookedTool()

		await runToolCall(tool, {
			requestConfirmation: vi.fn(async () => true),
			shouldAutoAcceptToolConfirmations: () => true
		})

		expect(tool.onConfirmationRequested).not.toHaveBeenCalled()
		expect(tool.fn).toHaveBeenCalled()
	})
})
