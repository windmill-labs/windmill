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
	$AuthenticationMethod,
	$AwsAuthResourceType,
	$AzureMode,
	$DeliveryType,
	$HttpMethod,
	$HttpRequestType,
	$MqttClientVersion,
	$MqttQoS,
	$SubscriptionMode,
	$TriggerMode,
	type AuthenticationMethod,
	type AzureMode,
	type AzureTriggerData,
	type AwsAuthResourceType,
	type DeliveryType,
	type GcpTriggerData,
	type HttpMethod,
	type HttpRequestType,
	type JobTriggerKind,
	type MqttClientVersion,
	type MqttQoS,
	type NewHttpTrigger,
	type NewKafkaTrigger,
	type NewMqttTrigger,
	type NewNatsTrigger,
	type NewPostgresTrigger,
	type NewSchedule,
	type NewSqsTrigger,
	type NewWebsocketTrigger,
	type SubscriptionMode,
	type TriggerMode
} from '$lib/gen'
import { z } from 'zod'
import { createToolDef, type Tool } from './shared'

const triggerKinds = [
	'http',
	'websocket',
	'kafka',
	'nats',
	'postgres',
	'mqtt',
	'sqs',
	'gcp',
	'azure'
] as const satisfies readonly JobTriggerKind[]
type TriggerKind = (typeof triggerKinds)[number]

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

function generatedEnumSchema<T extends string>(schema: { enum: readonly T[] }) {
	return z.enum(schema.enum as [T, ...T[]])
}

const triggerModeSchema = generatedEnumSchema<TriggerMode>($TriggerMode)
const httpMethodSchema = generatedEnumSchema<HttpMethod>($HttpMethod)
const httpRequestTypeSchema = generatedEnumSchema<HttpRequestType>($HttpRequestType)
const authenticationMethodSchema = generatedEnumSchema<AuthenticationMethod>($AuthenticationMethod)
const mqttClientVersionSchema = generatedEnumSchema<MqttClientVersion>($MqttClientVersion)
const mqttQosSchema = generatedEnumSchema<MqttQoS>($MqttQoS)
const awsAuthResourceTypeSchema = generatedEnumSchema<AwsAuthResourceType>($AwsAuthResourceType)
const subscriptionModeSchema = generatedEnumSchema<SubscriptionMode>($SubscriptionMode)
const deliveryTypeSchema = generatedEnumSchema<DeliveryType>($DeliveryType)
const azureModeSchema = generatedEnumSchema<AzureMode>($AzureMode)

const runnableFields = {
	path: z.string().min(1).describe('The unique path identifier for this trigger'),
	script_path: z.string().min(1).describe('Path to the script or flow to execute'),
	is_flow: z.boolean().describe('True if script_path points to a flow')
}
const scriptArgsSchema = z.record(z.string(), z.unknown())
const labelsSchema = z.array(z.string())
const retrySchema = z
	.object({
		constant: z
			.object({
				attempts: z.number().int().optional(),
				seconds: z.number().int().optional()
			})
			.optional(),
		exponential: z
			.object({
				attempts: z.number().int().optional(),
				multiplier: z.number().int().optional(),
				seconds: z.number().int().min(1).optional(),
				random_factor: z.number().int().min(0).max(100).optional()
			})
			.optional(),
		retry_if: z
			.object({
				expr: z.string()
			})
			.optional()
	})
	.passthrough()

const commonTriggerFields = {
	mode: triggerModeSchema.default('enabled'),
	error_handler_path: z.string().optional(),
	error_handler_args: scriptArgsSchema.optional(),
	retry: retrySchema.optional(),
	permissioned_as: z.string().optional(),
	preserve_permissioned_as: z.boolean().optional(),
	labels: labelsSchema.optional()
}

const filterSchema = z
	.object({
		key: z.string(),
		value: z.unknown()
	})
	.passthrough()

const websocketInitialMessageSchema = z.union([
	z.object({ raw_message: z.string() }),
	z.object({
		runnable_result: z.object({
			path: z.string(),
			args: scriptArgsSchema,
			is_flow: z.boolean()
		})
	})
])

const websocketHeartbeatSchema = z.object({
	interval_secs: z.number().int(),
	message: z.string(),
	state_field: z.string().optional()
})

const httpTriggerRequestSchema = z
	.object({
		...runnableFields,
		...commonTriggerFields,
		route_path: z
			.string()
			.min(1)
			.describe("The URL route path that will trigger this endpoint. Must not start with '/'."),
		static_asset_config: z
			.object({
				s3: z.string(),
				storage: z.string().optional(),
				filename: z.string().optional()
			})
			.nullable()
			.optional(),
		http_method: httpMethodSchema,
		authentication_resource_path: z.string().nullable().optional(),
		summary: z.string().nullable().optional(),
		description: z.string().nullable().optional(),
		request_type: httpRequestTypeSchema.default('sync'),
		authentication_method: authenticationMethodSchema.default('none'),
		is_static_website: z.boolean().default(false),
		workspaced_route: z.boolean().default(false),
		wrap_body: z.boolean().default(false),
		raw_string: z.boolean().default(false)
	})
	.passthrough()

const websocketTriggerRequestSchema = z
	.object({
		...runnableFields,
		...commonTriggerFields,
		url: z.string().min(1),
		filters: z.array(filterSchema).default([]),
		filter_logic: z.enum(['and', 'or']).default('and'),
		initial_messages: z.array(websocketInitialMessageSchema).nullable().optional(),
		url_runnable_args: scriptArgsSchema.nullable().optional(),
		can_return_message: z.boolean().default(false),
		can_return_error_result: z.boolean().default(false),
		heartbeat: websocketHeartbeatSchema.nullable().optional()
	})
	.passthrough()

const kafkaTriggerRequestSchema = z
	.object({
		...runnableFields,
		...commonTriggerFields,
		kafka_resource_path: z.string().min(1),
		group_id: z.string().min(1),
		topics: z.array(z.string()),
		filters: z.array(filterSchema).default([]),
		filter_logic: z.enum(['and', 'or']).default('and'),
		auto_offset_reset: z.enum(['latest', 'earliest']).default('latest'),
		auto_commit: z.boolean().default(true)
	})
	.passthrough()

const natsTriggerRequestSchema = z
	.object({
		...runnableFields,
		...commonTriggerFields,
		nats_resource_path: z.string().min(1),
		use_jetstream: z.boolean().default(false),
		stream_name: z.string().nullable().optional(),
		consumer_name: z.string().nullable().optional(),
		subjects: z.array(z.string())
	})
	.passthrough()

const postgresTriggerRequestSchema = z
	.object({
		...runnableFields,
		...commonTriggerFields,
		postgres_resource_path: z.string().min(1),
		replication_slot_name: z.string().optional(),
		publication_name: z.string().optional(),
		publication: z.record(z.string(), z.unknown()).optional()
	})
	.passthrough()

const mqttTriggerRequestSchema = z
	.object({
		...runnableFields,
		...commonTriggerFields,
		mqtt_resource_path: z.string().min(1),
		subscribe_topics: z.array(
			z.object({
				qos: mqttQosSchema,
				topic: z.string()
			})
		),
		client_id: z.string().nullable().optional(),
		v3_config: z
			.object({
				clean_session: z.boolean().optional()
			})
			.nullable()
			.optional(),
		v5_config: z
			.object({
				clean_start: z.boolean().optional(),
				topic_alias_maximum: z.number().optional(),
				session_expiry_interval: z.number().optional()
			})
			.nullable()
			.optional(),
		client_version: mqttClientVersionSchema.nullable().optional()
	})
	.passthrough()

const sqsTriggerRequestSchema = z
	.object({
		...runnableFields,
		...commonTriggerFields,
		queue_url: z.string().min(1),
		aws_auth_resource_type: awsAuthResourceTypeSchema.default('credentials'),
		aws_resource_path: z.string().min(1),
		message_attributes: z.array(z.string()).nullable().optional()
	})
	.passthrough()

const gcpTriggerRequestSchema = z
	.object({
		...runnableFields,
		...commonTriggerFields,
		gcp_resource_path: z.string().min(1),
		subscription_mode: subscriptionModeSchema,
		topic_id: z.string().min(1),
		subscription_id: z.string().optional(),
		base_endpoint: z.string().optional(),
		delivery_type: deliveryTypeSchema.optional(),
		delivery_config: z
			.object({
				authenticate: z.boolean(),
				audience: z.string().optional()
			})
			.nullable()
			.optional(),
		auto_acknowledge_msg: z.boolean().default(true),
		ack_deadline: z.number().optional()
	})
	.passthrough()

const azureTriggerRequestSchema = z
	.object({
		...runnableFields,
		...commonTriggerFields,
		azure_resource_path: z.string().min(1),
		azure_mode: azureModeSchema,
		scope_resource_id: z.string().min(1),
		topic_name: z.string().nullable().optional(),
		subscription_name: z.string().min(1),
		base_endpoint: z.string().optional(),
		event_type_filters: z.array(z.string()).optional()
	})
	.passthrough()

const createScheduleRequestSchema = z
	.object({
		path: z.string().min(1).describe('The unique path identifier for this schedule'),
		schedule: z.string().min(1).describe('Cron expression with 6 fields, including seconds'),
		timezone: z.string().min(1).describe("IANA timezone, for example 'UTC' or 'Europe/Paris'"),
		script_path: z.string().min(1).describe('Path to the script or flow to execute'),
		is_flow: z.boolean().describe('True if script_path points to a flow'),
		args: scriptArgsSchema.nullable().default({}),
		enabled: z.boolean().default(true),
		on_failure: z.string().nullable().optional(),
		on_failure_times: z.number().nullable().optional(),
		on_failure_exact: z.boolean().nullable().optional(),
		on_failure_extra_args: scriptArgsSchema.nullable().optional(),
		on_recovery: z.string().nullable().optional(),
		on_recovery_times: z.number().nullable().optional(),
		on_recovery_extra_args: scriptArgsSchema.nullable().optional(),
		on_success: z.string().nullable().optional(),
		on_success_extra_args: scriptArgsSchema.nullable().optional(),
		ws_error_handler_muted: z.boolean().optional(),
		retry: retrySchema.nullable().optional(),
		no_flow_overlap: z.boolean().optional(),
		summary: z.string().nullable().optional(),
		description: z.string().nullable().optional(),
		tag: z.string().nullable().optional(),
		paused_until: z.string().nullable().optional(),
		cron_version: z.string().nullable().optional(),
		dynamic_skip: z.string().nullable().optional(),
		permissioned_as: z.string().optional(),
		preserve_permissioned_as: z.boolean().optional(),
		labels: labelsSchema.optional()
	})
	.passthrough()

const createScheduleToolSchema = createScheduleRequestSchema

const triggerConfigSchemas = {
	http: httpTriggerRequestSchema.omit({ path: true }),
	websocket: websocketTriggerRequestSchema.omit({ path: true }),
	kafka: kafkaTriggerRequestSchema.omit({ path: true }),
	nats: natsTriggerRequestSchema.omit({ path: true }),
	postgres: postgresTriggerRequestSchema.omit({ path: true }),
	mqtt: mqttTriggerRequestSchema.omit({ path: true }),
	sqs: sqsTriggerRequestSchema.omit({ path: true }),
	gcp: gcpTriggerRequestSchema.omit({ path: true }),
	azure: azureTriggerRequestSchema.omit({ path: true })
} satisfies Record<TriggerKind, z.ZodType>

const createTriggerToolSchema = z.discriminatedUnion('kind', [
	z.object({
		kind: z.literal('http'),
		path: z.string().min(1).describe("The new trigger's Windmill path"),
		config: triggerConfigSchemas.http
	}),
	z.object({
		kind: z.literal('websocket'),
		path: z.string().min(1).describe("The new trigger's Windmill path"),
		config: triggerConfigSchemas.websocket
	}),
	z.object({
		kind: z.literal('kafka'),
		path: z.string().min(1).describe("The new trigger's Windmill path"),
		config: triggerConfigSchemas.kafka
	}),
	z.object({
		kind: z.literal('nats'),
		path: z.string().min(1).describe("The new trigger's Windmill path"),
		config: triggerConfigSchemas.nats
	}),
	z.object({
		kind: z.literal('postgres'),
		path: z.string().min(1).describe("The new trigger's Windmill path"),
		config: triggerConfigSchemas.postgres
	}),
	z.object({
		kind: z.literal('mqtt'),
		path: z.string().min(1).describe("The new trigger's Windmill path"),
		config: triggerConfigSchemas.mqtt
	}),
	z.object({
		kind: z.literal('sqs'),
		path: z.string().min(1).describe("The new trigger's Windmill path"),
		config: triggerConfigSchemas.sqs
	}),
	z.object({
		kind: z.literal('gcp'),
		path: z.string().min(1).describe("The new trigger's Windmill path"),
		config: triggerConfigSchemas.gcp
	}),
	z.object({
		kind: z.literal('azure'),
		path: z.string().min(1).describe("The new trigger's Windmill path"),
		config: triggerConfigSchemas.azure
	})
])

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
		requestSchema: httpTriggerRequestSchema as z.ZodType<NewHttpTrigger>,
		create: (data: { workspace: string; requestBody: NewHttpTrigger }) =>
			HttpTriggerService.createHttpTrigger(data)
	},
	websocket: {
		label: 'WebSocket trigger',
		requestSchema: websocketTriggerRequestSchema as z.ZodType<NewWebsocketTrigger>,
		create: (data: { workspace: string; requestBody: NewWebsocketTrigger }) =>
			WebsocketTriggerService.createWebsocketTrigger(data)
	},
	kafka: {
		label: 'Kafka trigger',
		requestSchema: kafkaTriggerRequestSchema as z.ZodType<NewKafkaTrigger>,
		create: (data: { workspace: string; requestBody: NewKafkaTrigger }) =>
			KafkaTriggerService.createKafkaTrigger(data)
	},
	nats: {
		label: 'NATS trigger',
		requestSchema: natsTriggerRequestSchema as z.ZodType<NewNatsTrigger>,
		create: (data: { workspace: string; requestBody: NewNatsTrigger }) =>
			NatsTriggerService.createNatsTrigger(data)
	},
	postgres: {
		label: 'Postgres trigger',
		requestSchema: postgresTriggerRequestSchema as z.ZodType<NewPostgresTrigger>,
		create: (data: { workspace: string; requestBody: NewPostgresTrigger }) =>
			PostgresTriggerService.createPostgresTrigger(data)
	},
	mqtt: {
		label: 'MQTT trigger',
		requestSchema: mqttTriggerRequestSchema as z.ZodType<NewMqttTrigger>,
		create: (data: { workspace: string; requestBody: NewMqttTrigger }) =>
			MqttTriggerService.createMqttTrigger(data)
	},
	sqs: {
		label: 'SQS trigger',
		requestSchema: sqsTriggerRequestSchema as z.ZodType<NewSqsTrigger>,
		create: (data: { workspace: string; requestBody: NewSqsTrigger }) =>
			SqsTriggerService.createSqsTrigger(data)
	},
	gcp: {
		label: 'GCP Pub/Sub trigger',
		requestSchema: gcpTriggerRequestSchema as z.ZodType<GcpTriggerData>,
		create: (data: { workspace: string; requestBody: GcpTriggerData }) =>
			GcpTriggerService.createGcpTrigger(data)
	},
	azure: {
		label: 'Azure Event Grid trigger',
		requestSchema: azureTriggerRequestSchema as z.ZodType<AzureTriggerData>,
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
		const parsedArgs = parseWithExplicitErrors(createScheduleToolSchema, args, 'Schedule')
		const requestBody = parseWithExplicitErrors(
			createScheduleRequestSchema as z.ZodType<NewSchedule>,
			parsedArgs,
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
