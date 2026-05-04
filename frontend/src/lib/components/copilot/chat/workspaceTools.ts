import {
	AzureTriggerService,
	GcpTriggerService,
	HttpTriggerService,
	KafkaTriggerService,
	MqttTriggerService,
	NatsTriggerService,
	PostgresTriggerService,
	ScheduleService,
	SqsTriggerService,
	WebsocketTriggerService,
	type AzureTriggerData,
	type GcpTriggerData,
	type NewHttpTrigger,
	type NewKafkaTrigger,
	type NewMqttTrigger,
	type NewNatsTrigger,
	type NewPostgresTrigger,
	type NewSchedule,
	type NewSqsTrigger,
	type NewWebsocketTrigger
} from '$lib/gen'
import {
	createTriggerToolSchema,
	scheduleRequestSchema,
	triggerRequestSchemas
} from './workspaceToolsZod.gen'
import { z } from 'zod'
import { createToolDef, type Tool, type ToolCallbacks } from './shared'
import { emptyString } from '$lib/utils'

type TriggerKind = keyof typeof triggerRequestSchemas

type TriggerRequestByKind = {
	http: NewHttpTrigger
	websocket: NewWebsocketTrigger
	kafka: NewKafkaTrigger
	nats: NewNatsTrigger
	postgres: NewPostgresTrigger
	mqtt: NewMqttTrigger
	sqs: NewSqsTrigger
	gcp: GcpTriggerData
	azure: AzureTriggerData
}
type TriggerRequestBody = TriggerRequestByKind[TriggerKind]

export type WorkspaceMutationTarget = {
	kind: 'script' | 'flow'
	path?: string
	deployed?: boolean
}

type WorkspaceMutationHelpers = {
	getWorkspaceMutationTarget?: () => WorkspaceMutationTarget
}

const createScheduleToolSchema = scheduleRequestSchema.omit({ script_path: true, is_flow: true })

function getWorkspaceMutationTarget(helpers: unknown): WorkspaceMutationTarget | undefined {
	return (helpers as WorkspaceMutationHelpers | undefined)?.getWorkspaceMutationTarget?.()
}

function getWorkspaceMutationTargetError(target: WorkspaceMutationTarget | undefined): string | undefined {
	if (!target) {
		return 'the script or flow needs to be deployed before doing this action'
	}
	if (!emptyString(target.path) && target.deployed) {
		return undefined
	}
	return `the ${target.kind} needs to be deployed before doing this action`
}

function validateWorkspaceMutationTarget(helpers: unknown): string | undefined {
	return getWorkspaceMutationTargetError(getWorkspaceMutationTarget(helpers))
}

function requireWorkspaceMutationTarget(helpers: unknown): WorkspaceMutationTarget & { path: string } {
	const target = getWorkspaceMutationTarget(helpers)
	const error = getWorkspaceMutationTargetError(target)
	if (error) {
		throw new Error(error)
	}
	return target as WorkspaceMutationTarget & { path: string }
}

function getWorkspaceMutationTargetFields(helpers: unknown): Pick<NewSchedule, 'script_path' | 'is_flow'> {
	const target = requireWorkspaceMutationTarget(helpers)
	return {
		script_path: target.path,
		is_flow: target.kind === 'flow'
	}
}

const createScheduleToolDef = createToolDef(
	createScheduleToolSchema,
	'create_schedule',
	'Create a schedule for the current script or flow.',
	{ strict: false }
)

const createTriggerToolDef = createToolDef(
	createTriggerToolSchema,
	'create_trigger',
	'Create a trigger for the current script or flow.',
	{ strict: false }
)

const triggerConfigs = {
	http: {
		label: 'HTTP trigger',
		requestSchema: triggerRequestSchemas.http as z.ZodType<NewHttpTrigger>,
		create: (data: { workspace: string; requestBody: NewHttpTrigger }) =>
			HttpTriggerService.createHttpTrigger(data)
	},
	websocket: {
		label: 'WebSocket trigger',
		requestSchema: triggerRequestSchemas.websocket as z.ZodType<NewWebsocketTrigger>,
		create: (data: { workspace: string; requestBody: NewWebsocketTrigger }) =>
			WebsocketTriggerService.createWebsocketTrigger(data)
	},
	kafka: {
		label: 'Kafka trigger',
		requestSchema: triggerRequestSchemas.kafka as z.ZodType<NewKafkaTrigger>,
		create: (data: { workspace: string; requestBody: NewKafkaTrigger }) =>
			KafkaTriggerService.createKafkaTrigger(data)
	},
	nats: {
		label: 'NATS trigger',
		requestSchema: triggerRequestSchemas.nats as z.ZodType<NewNatsTrigger>,
		create: (data: { workspace: string; requestBody: NewNatsTrigger }) =>
			NatsTriggerService.createNatsTrigger(data)
	},
	postgres: {
		label: 'Postgres trigger',
		requestSchema: triggerRequestSchemas.postgres as z.ZodType<NewPostgresTrigger>,
		create: (data: { workspace: string; requestBody: NewPostgresTrigger }) =>
			PostgresTriggerService.createPostgresTrigger(data)
	},
	mqtt: {
		label: 'MQTT trigger',
		requestSchema: triggerRequestSchemas.mqtt as z.ZodType<NewMqttTrigger>,
		create: (data: { workspace: string; requestBody: NewMqttTrigger }) =>
			MqttTriggerService.createMqttTrigger(data)
	},
	sqs: {
		label: 'SQS trigger',
		requestSchema: triggerRequestSchemas.sqs as z.ZodType<NewSqsTrigger>,
		create: (data: { workspace: string; requestBody: NewSqsTrigger }) =>
			SqsTriggerService.createSqsTrigger(data)
	},
	gcp: {
		label: 'GCP Pub/Sub trigger',
		requestSchema: triggerRequestSchemas.gcp as z.ZodType<GcpTriggerData>,
		create: (data: { workspace: string; requestBody: GcpTriggerData }) =>
			GcpTriggerService.createGcpTrigger(data)
	},
	azure: {
		label: 'Azure Event Grid trigger',
		requestSchema: triggerRequestSchemas.azure as z.ZodType<AzureTriggerData>,
		create: (data: { workspace: string; requestBody: AzureTriggerData }) =>
			AzureTriggerService.createAzureTrigger(data)
	}
} satisfies {
	[K in TriggerKind]: {
		label: string
		requestSchema: z.ZodType<TriggerRequestByKind[K]>
		create: (data: { workspace: string; requestBody: TriggerRequestByKind[K] }) => Promise<string>
	}
}

function formatPath(path: (string | number | symbol)[]): string {
	if (path.length === 0) {
		return 'value'
	}
	return path
		.map((part) => (typeof part === 'number' ? `[${part}]` : String(part)))
		.join('.')
		.replaceAll('.[', '[')
}

function formatZodError(error: z.ZodError): string {
	return error.issues
		.slice(0, 8)
		.map((issue) => `${formatPath(issue.path)}: ${issue.message}`)
		.join('; ')
}

function parseWithExplicitErrors<T>(schema: z.ZodType<T>, value: unknown, label: string): T {
	const result = schema.safeParse(value)
	if (!result.success) {
		throw new Error(`${label} is invalid: ${formatZodError(result.error)}`)
	}
	return result.data
}

function formatApiError(error: any): string {
	const bodyMessage =
		error?.body?.error?.message ??
		error?.body?.message ??
		(typeof error?.body?.error === 'string' ? error.body.error : undefined)
	const body =
		bodyMessage ??
		(typeof error?.body === 'string'
			? error.body
			: error?.body !== undefined
				? stringifyErrorBody(error.body)
				: undefined)
	const message = body || error?.message || String(error)
	return error?.status ? `HTTP ${error.status}: ${message}` : message
}

function stringifyErrorBody(body: unknown): string {
	try {
		return JSON.stringify(body)
	} catch {
		return String(body)
	}
}

function setToolError(toolCallbacks: ToolCallbacks, toolId: string, error: unknown): string {
	const errorMessage = error instanceof Error ? error.message : String(error)
	toolCallbacks.setToolStatus(toolId, {
		content: errorMessage,
		error: errorMessage,
		isLoading: false,
		needsConfirmation: false
	})
	return `Error while calling tool: ${errorMessage}`
}

const createScheduleTool: Tool<any> = {
	def: createScheduleToolDef,
	requiresConfirmation: true,
	confirmationMessage: 'Create schedule',
	showDetails: true,
	validateBeforeConfirmation: ({ helpers }) => validateWorkspaceMutationTarget(helpers),
	fn: async ({ args, workspace, helpers, toolCallbacks, toolId }) => {
		try {
			const requestBody = parseWithExplicitErrors(
				scheduleRequestSchema as z.ZodType<NewSchedule>,
				{
					...args,
					...getWorkspaceMutationTargetFields(helpers)
				},
				'Schedule'
			)

			toolCallbacks.setToolStatus(toolId, {
				content: `Validating schedule "${requestBody.path}"...`
			})
			try {
				await ScheduleService.previewSchedule({
					requestBody: {
						schedule: requestBody.schedule,
						timezone: requestBody.timezone,
						cron_version: requestBody.cron_version ?? undefined
					}
				})
			} catch (error) {
				throw new Error(`Invalid schedule or timezone: ${formatApiError(error)}`)
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Creating schedule "${requestBody.path}"...`
			})
			try {
				const result = await ScheduleService.createSchedule({ workspace, requestBody })
				const toolResult = {
					success: true,
					path: requestBody.path,
					target_path: requestBody.script_path,
					target_kind: requestBody.is_flow ? 'flow' : 'script',
					backend_result: result
				}
				toolCallbacks.setToolStatus(toolId, {
					content: `Created schedule "${requestBody.path}"`,
					result: toolResult
				})
				return JSON.stringify(toolResult)
			} catch (error) {
				throw new Error(`Failed to create schedule "${requestBody.path}": ${formatApiError(error)}`)
			}
		} catch (error) {
			return setToolError(toolCallbacks, toolId, error)
		}
	}
}

const createTriggerTool: Tool<any> = {
	def: createTriggerToolDef,
	requiresConfirmation: true,
	confirmationMessage: 'Create trigger',
	showDetails: true,
	validateBeforeConfirmation: ({ helpers }) => validateWorkspaceMutationTarget(helpers),
	fn: async ({ args, workspace, helpers, toolCallbacks, toolId }) => {
		try {
			const parsedArgs = parseWithExplicitErrors(createTriggerToolSchema, args, 'Trigger')
			const triggerConfig = triggerConfigs[parsedArgs.kind]
			const requestBody = parseWithExplicitErrors(
				triggerConfig.requestSchema as z.ZodType<TriggerRequestBody>,
				{
					...parsedArgs.config,
					path: parsedArgs.path,
					...getWorkspaceMutationTargetFields(helpers)
				},
				triggerConfig.label
			)

			toolCallbacks.setToolStatus(toolId, {
				content: `Creating ${triggerConfig.label} "${requestBody.path}"...`
			})
			try {
				const result = await triggerConfig.create({ workspace, requestBody } as never)
				const toolResult = {
					success: true,
					kind: parsedArgs.kind,
					path: requestBody.path,
					target_path: requestBody.script_path,
					target_kind: requestBody.is_flow ? 'flow' : 'script',
					backend_result: result
				}
				toolCallbacks.setToolStatus(toolId, {
					content: `Created ${triggerConfig.label} "${requestBody.path}"`,
					result: toolResult
				})
				return JSON.stringify(toolResult)
			} catch (error) {
				throw new Error(
					`Failed to create ${triggerConfig.label} "${requestBody.path}": ${formatApiError(error)}`
				)
			}
		} catch (error) {
			return setToolError(toolCallbacks, toolId, error)
		}
	}
}

const workspaceMutationTools = [createScheduleTool, createTriggerTool]

export function createWorkspaceMutationTools<T>(): Tool<T>[] {
	return workspaceMutationTools as Tool<T>[]
}
