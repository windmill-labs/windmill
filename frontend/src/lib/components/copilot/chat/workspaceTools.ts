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
	WebsocketTriggerService
} from '$lib/gen'
import { mcpEndpointTools } from '$lib/mcpEndpointTools'
import type { ChatCompletionFunctionTool } from 'openai/resources/index.mjs'
import YAML from 'yaml'
import type { Tool } from './shared'

import azureTriggerSchemaYaml from '$system_prompts/schemas/azure_trigger.schema.yaml?raw'
import gcpTriggerSchemaYaml from '$system_prompts/schemas/gcp_trigger.schema.yaml?raw'
import httpTriggerSchemaYaml from '$system_prompts/schemas/http_trigger.schema.yaml?raw'
import kafkaTriggerSchemaYaml from '$system_prompts/schemas/kafka_trigger.schema.yaml?raw'
import mqttTriggerSchemaYaml from '$system_prompts/schemas/mqtt_trigger.schema.yaml?raw'
import natsTriggerSchemaYaml from '$system_prompts/schemas/nats_trigger.schema.yaml?raw'
import postgresTriggerSchemaYaml from '$system_prompts/schemas/postgres_trigger.schema.yaml?raw'
import sqsTriggerSchemaYaml from '$system_prompts/schemas/sqs_trigger.schema.yaml?raw'
import websocketTriggerSchemaYaml from '$system_prompts/schemas/websocket_trigger.schema.yaml?raw'

type JsonSchema = {
	type?: string | string[]
	properties?: Record<string, JsonSchema>
	required?: string[]
	items?: JsonSchema
	enum?: unknown[]
	nullable?: boolean
	oneOf?: JsonSchema[]
	anyOf?: JsonSchema[]
	allOf?: JsonSchema[]
	$ref?: string
	description?: string
}

type CurrentRunnable = {
	path: string | undefined
	isFlow: boolean
	label: 'script' | 'flow'
}

type TriggerKind =
	| 'http'
	| 'websocket'
	| 'kafka'
	| 'nats'
	| 'postgres'
	| 'mqtt'
	| 'sqs'
	| 'gcp'
	| 'azure'

const INFERRED_FIELDS = ['script_path', 'is_flow'] as const
const TRIGGER_REQUIRED_OPTIONAL_AT_RUNTIME = ['script_path', 'is_flow', 'permissioned_as']

function cloneSchema(schema: JsonSchema): JsonSchema {
	return JSON.parse(JSON.stringify(schema))
}

function omitPropertiesFromSchema(schema: JsonSchema, fields: string[]): JsonSchema {
	const cloned = cloneSchema(schema)
	for (const field of fields) {
		delete cloned.properties?.[field]
	}
	cloned.required = cloned.required?.filter((field) => !fields.includes(field)) ?? []
	return cloned
}

function parseYamlSchema(yaml: string): JsonSchema {
	return YAML.parse(yaml) as JsonSchema
}

function getEndpointBodySchema(name: string): JsonSchema {
	const endpoint = mcpEndpointTools.find((tool) => tool.name === name)
	if (!endpoint?.bodySchema) {
		throw new Error(`Internal error: missing generated schema for ${name}`)
	}
	return endpoint.bodySchema as JsonSchema
}

function formatPath(path: (string | number)[]): string {
	if (path.length === 0) {
		return 'value'
	}
	return path
		.map((part) => (typeof part === 'number' ? `[${part}]` : part))
		.join('.')
		.replaceAll('.[', '[')
}

function isStandardJsonSchemaType(type: string): boolean {
	return ['object', 'array', 'string', 'number', 'integer', 'boolean', 'null'].includes(type)
}

function valueMatchesType(value: unknown, type: string): boolean {
	switch (type) {
		case 'object':
			return typeof value === 'object' && value !== null && !Array.isArray(value)
		case 'array':
			return Array.isArray(value)
		case 'string':
			return typeof value === 'string'
		case 'number':
			return typeof value === 'number' && Number.isFinite(value)
		case 'integer':
			return typeof value === 'number' && Number.isInteger(value)
		case 'boolean':
			return typeof value === 'boolean'
		case 'null':
			return value === null
		default:
			return true
	}
}

function validateJsonSchema(
	value: unknown,
	schema: JsonSchema,
	path: (string | number)[] = []
): string[] {
	if (!schema || schema.$ref) {
		return []
	}

	if (value === null || value === undefined) {
		return schema.nullable || value === undefined ? [] : [`${formatPath(path)} must not be null`]
	}

	if (schema.allOf) {
		return schema.allOf.flatMap((inner) => validateJsonSchema(value, inner, path))
	}

	if (schema.anyOf || schema.oneOf) {
		const candidates = schema.anyOf ?? schema.oneOf ?? []
		const errors = candidates.map((inner) => validateJsonSchema(value, inner, path))
		return errors.some((candidateErrors) => candidateErrors.length === 0) ? [] : (errors[0] ?? [])
	}

	const rawTypes = Array.isArray(schema.type) ? schema.type : schema.type ? [schema.type] : []
	const standardTypes = rawTypes.filter(isStandardJsonSchemaType)
	if (standardTypes.length > 0 && !standardTypes.some((type) => valueMatchesType(value, type))) {
		return [`${formatPath(path)} must be ${standardTypes.join(' or ')}`]
	}

	if (schema.enum && !schema.enum.includes(value)) {
		return [
			`${formatPath(path)} must be one of ${schema.enum.map((v) => JSON.stringify(v)).join(', ')}`
		]
	}

	if (schema.type === 'object' && schema.properties) {
		const record = value as Record<string, unknown>
		const errors: string[] = []
		for (const required of schema.required ?? []) {
			if (record[required] === undefined) {
				errors.push(`${formatPath([...path, required])} is required`)
			}
		}
		for (const [key, propertySchema] of Object.entries(schema.properties)) {
			if (record[key] !== undefined) {
				errors.push(...validateJsonSchema(record[key], propertySchema, [...path, key]))
			}
		}
		return errors
	}

	if (schema.type === 'array' && schema.items && Array.isArray(value)) {
		return value.flatMap((item, index) => validateJsonSchema(item, schema.items!, [...path, index]))
	}

	return []
}

function assertValid(value: unknown, schema: JsonSchema, label: string): void {
	const errors = validateJsonSchema(value, schema)
	if (errors.length > 0) {
		throw new Error(`${label} is invalid: ${errors.slice(0, 8).join('; ')}`)
	}
}

function rejectModelSuppliedInferredFields(args: Record<string, unknown>, toolName: string): void {
	for (const field of INFERRED_FIELDS) {
		if (args[field] !== undefined) {
			throw new Error(
				`${toolName} does not accept ${field}. It is inferred automatically from the current script or flow.`
			)
		}
	}
}

function inferCurrentRunnable(helpers: any): CurrentRunnable {
	if (typeof helpers?.getCurrentRunnableForAiTools === 'function') {
		return helpers.getCurrentRunnableForAiTools()
	}
	if (typeof helpers?.getScriptOptions === 'function') {
		return {
			path: helpers.getScriptOptions().path,
			isFlow: false,
			label: 'script'
		}
	}
	if (typeof helpers?.getFlowAndSelectedId === 'function') {
		const { flow } = helpers.getFlowAndSelectedId()
		return {
			path: (flow as any)?.path,
			isFlow: true,
			label: 'flow'
		}
	}
	throw new Error('Cannot infer the current script or flow path in this chat mode.')
}

function requireRunnablePath(helpers: any, action: string): CurrentRunnable & { path: string } {
	const runnable = inferCurrentRunnable(helpers)
	if (!runnable.path?.trim()) {
		throw new Error(
			`Cannot ${action} because the current ${runnable.label} has no saved path. Ask the user to save the ${runnable.label} first.`
		)
	}
	return { ...runnable, path: runnable.path }
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

const createScheduleBodySchema = getEndpointBodySchema('createSchedule')
const createScheduleParametersSchema = omitPropertiesFromSchema(createScheduleBodySchema, [
	'script_path',
	'is_flow'
])
createScheduleParametersSchema.required =
	createScheduleParametersSchema.required?.filter((field) => field !== 'args') ?? []

const createScheduleToolDef: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		name: 'create_schedule',
		description:
			'Create a schedule for the current script or flow. Do not provide script_path or is_flow; they are inferred from the current editor.',
		strict: false,
		parameters: createScheduleParametersSchema as Record<string, unknown>
	}
}

const triggerConfigs: Record<
	TriggerKind,
	{
		label: string
		schema: JsonSchema
		create: (data: { workspace: string; requestBody: any }) => Promise<string>
		defaults?: Record<string, unknown>
	}
> = {
	http: {
		label: 'HTTP trigger',
		schema: parseYamlSchema(httpTriggerSchemaYaml),
		create: (data) => HttpTriggerService.createHttpTrigger(data),
		defaults: {
			authentication_method: 'none',
			request_type: 'sync',
			is_static_website: false,
			workspaced_route: false,
			wrap_body: false,
			raw_string: false
		}
	},
	websocket: {
		label: 'WebSocket trigger',
		schema: parseYamlSchema(websocketTriggerSchemaYaml),
		create: (data) => WebsocketTriggerService.createWebsocketTrigger(data),
		defaults: {
			filters: [],
			filter_logic: 'and',
			can_return_message: false,
			can_return_error_result: false
		}
	},
	kafka: {
		label: 'Kafka trigger',
		schema: parseYamlSchema(kafkaTriggerSchemaYaml),
		create: (data) => KafkaTriggerService.createKafkaTrigger(data),
		defaults: { filters: [], filter_logic: 'and', auto_offset_reset: 'latest', auto_commit: true }
	},
	nats: {
		label: 'NATS trigger',
		schema: parseYamlSchema(natsTriggerSchemaYaml),
		create: (data) => NatsTriggerService.createNatsTrigger(data),
		defaults: { use_jetstream: false }
	},
	postgres: {
		label: 'Postgres trigger',
		schema: parseYamlSchema(postgresTriggerSchemaYaml),
		create: (data) => PostgresTriggerService.createPostgresTrigger(data)
	},
	mqtt: {
		label: 'MQTT trigger',
		schema: parseYamlSchema(mqttTriggerSchemaYaml),
		create: (data) => MqttTriggerService.createMqttTrigger(data)
	},
	sqs: {
		label: 'SQS trigger',
		schema: parseYamlSchema(sqsTriggerSchemaYaml),
		create: (data) => SqsTriggerService.createSqsTrigger(data),
		defaults: { aws_auth_resource_type: 'credentials' }
	},
	gcp: {
		label: 'GCP Pub/Sub trigger',
		schema: parseYamlSchema(gcpTriggerSchemaYaml),
		create: (data) => GcpTriggerService.createGcpTrigger(data),
		defaults: { auto_acknowledge_msg: true }
	},
	azure: {
		label: 'Azure Event Grid trigger',
		schema: parseYamlSchema(azureTriggerSchemaYaml),
		create: (data) => AzureTriggerService.createAzureTrigger(data)
	}
}

const triggerKinds = Object.keys(triggerConfigs) as TriggerKind[]

const createTriggerToolDef: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		name: 'create_trigger',
		description:
			'Create a trigger for the current script or flow. Do not provide script_path or is_flow; they are inferred from the current editor. Supported kinds: http, websocket, kafka, nats, postgres, mqtt, sqs, gcp, azure.',
		strict: false,
		parameters: {
			type: 'object',
			properties: {
				kind: {
					type: 'string',
					enum: triggerKinds,
					description: 'Trigger kind to create.'
				},
				path: {
					type: 'string',
					description: "The new trigger's Windmill path, for example f/folder/my_trigger."
				},
				config: {
					type: 'object',
					description:
						'Trigger-specific configuration from the generated trigger schema. Omit path, script_path, and is_flow. Common defaults are applied for HTTP, Kafka, WebSocket, NATS, SQS, and GCP.'
				}
			},
			required: ['kind', 'path', 'config']
		}
	}
}

const createScheduleTool: Tool<any> = {
	def: createScheduleToolDef,
	requiresConfirmation: true,
	confirmationMessage: 'Create schedule',
	showDetails: true,
	fn: async ({ args, workspace, helpers, toolCallbacks, toolId }) => {
		rejectModelSuppliedInferredFields(args, 'create_schedule')
		const runnable = requireRunnablePath(helpers, 'create a schedule')
		const requestBody = {
			enabled: true,
			args: {},
			...args,
			script_path: runnable.path,
			is_flow: runnable.isFlow
		}

		assertValid(requestBody, createScheduleBodySchema, 'Schedule')

		toolCallbacks.setToolStatus(toolId, {
			content: `Validating schedule "${requestBody.path}"...`
		})
		try {
			await ScheduleService.previewSchedule({
				requestBody: {
					schedule: requestBody.schedule,
					timezone: requestBody.timezone,
					cron_version: requestBody.cron_version
				}
			})
		} catch (error) {
			throw new Error(`Invalid schedule or timezone: ${formatApiError(error)}`)
		}

		toolCallbacks.setToolStatus(toolId, {
			content: `Creating schedule "${requestBody.path}" for current ${runnable.label}...`
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
				target_path: runnable.path,
				target_kind: runnable.label,
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
	fn: async ({ args, workspace, helpers, toolCallbacks, toolId }) => {
		const kind = args.kind as TriggerKind
		if (!triggerKinds.includes(kind)) {
			throw new Error(
				`Unsupported trigger kind "${args.kind}". Supported trigger kinds are: ${triggerKinds.join(', ')}.`
			)
		}
		if (!args.path || typeof args.path !== 'string') {
			throw new Error('create_trigger requires a string path for the new trigger.')
		}
		const config = args.config
		if (!config || typeof config !== 'object' || Array.isArray(config)) {
			throw new Error('create_trigger requires config to be an object.')
		}

		rejectModelSuppliedInferredFields(config, 'create_trigger config')
		if ((config as Record<string, unknown>).path !== undefined) {
			throw new Error(
				'create_trigger config does not accept path. Pass path as the top-level path argument.'
			)
		}

		const runnable = requireRunnablePath(helpers, 'create a trigger')
		const triggerConfig = triggerConfigs[kind]
		const requestBody = {
			mode: 'enabled',
			...(triggerConfig.defaults ?? {}),
			...(config as Record<string, unknown>),
			path: args.path,
			script_path: runnable.path,
			is_flow: runnable.isFlow
		}
		const validationSchema = cloneSchema(triggerConfig.schema)
		validationSchema.required =
			validationSchema.required?.filter(
				(field) => !TRIGGER_REQUIRED_OPTIONAL_AT_RUNTIME.includes(field)
			) ?? []

		assertValid(requestBody, validationSchema, triggerConfig.label)

		toolCallbacks.setToolStatus(toolId, {
			content: `Creating ${triggerConfig.label} "${args.path}" for current ${runnable.label}...`
		})
		try {
			const result = await triggerConfig.create({ workspace, requestBody })
			toolCallbacks.setToolStatus(toolId, {
				content: `Created ${triggerConfig.label} "${args.path}"`,
				result
			})
			return JSON.stringify({
				success: true,
				kind,
				path: args.path,
				target_path: runnable.path,
				target_kind: runnable.label,
				result
			})
		} catch (error) {
			throw new Error(
				`Failed to create ${triggerConfig.label} "${args.path}": ${formatApiError(error)}`
			)
		}
	}
}

const workspaceMutationTools = [createScheduleTool, createTriggerTool]

export function createWorkspaceMutationTools<T>(): Tool<T>[] {
	return workspaceMutationTools as Tool<T>[]
}
