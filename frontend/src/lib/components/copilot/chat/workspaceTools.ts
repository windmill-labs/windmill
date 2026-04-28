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
} from './workspaceToolsZod'
import { z } from 'zod'
import { createToolDef, type Tool } from './shared'

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

const createScheduleToolDef = createToolDef(
	scheduleRequestSchema,
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
	const body =
		typeof error?.body === 'string'
			? error.body
			: error?.body !== undefined
				? JSON.stringify(error.body)
				: undefined
	const message = body || error?.message || String(error)
	return error?.status ? `HTTP ${error.status}: ${message}` : message
}

const createScheduleTool: Tool<any> = {
	def: createScheduleToolDef,
	requiresConfirmation: true,
	confirmationMessage: 'Create schedule',
	showDetails: true,
	fn: async ({ args, workspace, toolCallbacks, toolId }) => {
		const requestBody = parseWithExplicitErrors(
			scheduleRequestSchema as z.ZodType<NewSchedule>,
			args,
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
			toolCallbacks.setToolStatus(toolId, {
				content: `Created schedule "${requestBody.path}"`,
				result
			})
			return JSON.stringify({
				success: true,
				path: requestBody.path,
				target_path: requestBody.script_path,
				target_kind: requestBody.is_flow ? 'flow' : 'script',
				result
			})
		} catch (error) {
			throw new Error(`Failed to create schedule "${requestBody.path}": ${formatApiError(error)}`)
		}
	}
}

const createTriggerTool: Tool<any> = {
	def: createTriggerToolDef,
	requiresConfirmation: true,
	confirmationMessage: 'Create trigger',
	showDetails: true,
	fn: async ({ args, workspace, toolCallbacks, toolId }) => {
		const parsedArgs = parseWithExplicitErrors(createTriggerToolSchema, args, 'Trigger')
		const triggerConfig = triggerConfigs[parsedArgs.kind]
		const requestBody = parseWithExplicitErrors(
			triggerConfig.requestSchema as z.ZodType<TriggerRequestBody>,
			{
				...parsedArgs.config,
				path: parsedArgs.path
			},
			triggerConfig.label
		)

		toolCallbacks.setToolStatus(toolId, {
			content: `Creating ${triggerConfig.label} "${requestBody.path}"...`
		})
		try {
			const result = await triggerConfig.create({ workspace, requestBody } as never)
			toolCallbacks.setToolStatus(toolId, {
				content: `Created ${triggerConfig.label} "${requestBody.path}"`,
				result
			})
			return JSON.stringify({
				success: true,
				kind: parsedArgs.kind,
				path: requestBody.path,
				target_path: requestBody.script_path,
				target_kind: requestBody.is_flow ? 'flow' : 'script',
				result
			})
		} catch (error) {
			throw new Error(
				`Failed to create ${triggerConfig.label} "${requestBody.path}": ${formatApiError(error)}`
			)
		}
	}
}

const workspaceMutationTools = [createScheduleTool, createTriggerTool]

export function createWorkspaceMutationTools<T>(): Tool<T>[] {
	return workspaceMutationTools as Tool<T>[]
}
