import {
	AzureTriggerService,
	FlowService,
	GcpTriggerService,
	HttpTriggerService,
	KafkaTriggerService,
	MqttTriggerService,
	NatsTriggerService,
	PostgresTriggerService,
	ResourceService,
	ScheduleService,
	ScriptService,
	SqsTriggerService,
	VariableService,
	WebsocketTriggerService
} from '$lib/gen'
import { $ScriptLang } from '$lib/gen/schemas.gen'
import type {
	Flow,
	ListableResource,
	ListableVariable,
	Schedule,
	Script,
	ScriptLang
} from '$lib/gen/types.gen'
import { getFlowPrompt, getResourcePrompt, getScriptPrompt } from '$system_prompts'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import { z } from 'zod'
import {
	createToolDef,
	type CreatedResourceTriggerKind,
	type Tool,
	type ToolCallbacks,
	type ToolDisplayAction
} from '../shared'
import { flowModuleSchema, flowModulesSchema } from '../flow/openFlowZod.gen'
import {
	resourceRequestSchema,
	scheduleRequestSchema,
	triggerRequestSchemas,
	variableRequestSchema
} from '../workspaceToolsZod.gen'
import {
	getWorkspaceItemKey,
	globalDraftStore,
	TRIGGER_KINDS,
	type TriggerKind,
	type WorkspaceItem,
	type WorkspaceItemType
} from './draftStore.svelte'

const ITEM_TYPES = [
	'script',
	'flow',
	'schedule',
	'trigger',
	'resource',
	'variable'
] as const satisfies readonly WorkspaceItemType[]
const INSTRUCTION_SUBJECTS = [
	'script',
	'flow',
	'resource'
] as const satisfies readonly WorkspaceItemType[]
const MAX_LIST_LIMIT = 100

const itemTypeSchema = z.enum(ITEM_TYPES)
const instructionSubjectSchema = z.enum(INSTRUCTION_SUBJECTS)
const triggerKindSchema = z.enum(TRIGGER_KINDS)
const scriptLangSchema = z.enum($ScriptLang.enum)

const getInstructionsSchema = z.object({
	subject: instructionSubjectSchema.describe(
		"The workspace item type to get authoring instructions for. Schedules and triggers don't need instructions — their tool schemas describe everything."
	),
	language: scriptLangSchema
		.optional()
		.describe(
			'Required when subject is script. Use the existing script language when modifying, or the requested target language when creating.'
		)
})

const listWorkspaceItemsSchema = z.object({
	types: z
		.array(itemTypeSchema)
		.optional()
		.describe(
			'Optional item types to list. Defaults to scripts and flows. Pass schedule or trigger explicitly when needed (listing triggers spans 9 kinds and is heavier).'
		),
	query: z.string().optional().describe('Optional case-insensitive path or summary search string.'),
	path_prefix: z
		.string()
		.optional()
		.describe('Optional path prefix filter, such as f/ or u/user/.'),
	limit: z
		.number()
		.int()
		.min(1)
		.max(MAX_LIST_LIMIT)
		.optional()
		.describe('Maximum number of items to return. Defaults to 50 and is capped at 100.')
})

const readWorkspaceItemSchema = z.object({
	type: itemTypeSchema,
	path: z.string().describe('Workspace path of the item to read.'),
	trigger_kind: triggerKindSchema
		.optional()
		.describe('Required when type is trigger. Identifies which trigger service to call.')
})

const writeScriptSchema = z.object({
	path: z
		.string()
		.describe('Workspace path of the script, e.g. f/folder/name or u/user/name.'),
	summary: z.string().optional().describe('Short human-readable summary.'),
	language: scriptLangSchema.describe('Script language.'),
	content: z.string().describe('Full script source code.')
})

const flowValueSchema = z
	.looseObject({
		modules: flowModulesSchema.describe('Sequential flow modules.'),
		preprocessor_module: flowModuleSchema
			.nullable()
			.optional()
			.describe(
				"Optional preprocessor module with id 'preprocessor'. Runs before normal modules; cannot reference results.*."
			),
		failure_module: flowModuleSchema
			.nullable()
			.optional()
			.describe("Optional failure handler module with id 'failure'.")
	})
	.describe('OpenFlow value: modules plus optional preprocessor_module and failure_module.')

const writeFlowSchema = z.object({
	path: z
		.string()
		.describe('Workspace path of the flow, e.g. f/folder/name or u/user/name.'),
	summary: z.string().optional().describe('Short human-readable summary.'),
	value: flowValueSchema
})

const writeScheduleSchema = scheduleRequestSchema

const writeTriggerSchema = z.object({
	kind: triggerKindSchema.describe('Trigger kind. Determines which fields are valid in config.'),
	config: z
		.union([
			triggerRequestSchemas.http,
			triggerRequestSchemas.websocket,
			triggerRequestSchemas.kafka,
			triggerRequestSchemas.nats,
			triggerRequestSchemas.postgres,
			triggerRequestSchemas.mqtt,
			triggerRequestSchemas.sqs,
			triggerRequestSchemas.gcp,
			triggerRequestSchemas.azure
		])
		.describe(
			'Full trigger configuration. Must include path, script_path, is_flow plus the kind-specific fields.'
		)
})

const writeResourceSchema = resourceRequestSchema

const writeVariableSchema = variableRequestSchema

const searchResourceTypesSchema = z.object({
	query: z.string().describe('Substring to match against resource type names.'),
	limit: z
		.number()
		.int()
		.min(1)
		.max(20)
		.optional()
		.describe('Max number of resource types to return. Defaults to 5.')
})

const deleteWorkspaceItemSchema = z.object({
	type: itemTypeSchema,
	path: z.string().describe('Workspace path of the item to delete.'),
	trigger_kind: triggerKindSchema
		.optional()
		.describe('Required when type is trigger. Identifies which trigger service to call.')
})

const deployWorkspaceItemSchema = z.object({
	type: itemTypeSchema,
	path: z.string().describe('Workspace path of the draft to deploy.'),
	trigger_kind: triggerKindSchema
		.optional()
		.describe('Required when type is trigger. Must match the draft trigger kind.'),
	deployment_message: z
		.string()
		.optional()
		.describe('Optional deployment message recorded with the change.')
})

const editScriptSchema = z.object({
	path: z.string().describe('Workspace path of the script to edit.'),
	old_string: z.string().min(1).describe('Exact text to find in the script source.'),
	new_string: z.string().describe('Replacement text.'),
	replace_all: z
		.boolean()
		.optional()
		.default(false)
		.describe(
			'When true, replace every exact match. When false, old_string must match exactly once.'
		)
})

const patchFlowJsonSchema = z.object({
	path: z.string().describe('Workspace path of the flow to edit.'),
	old_string: z
		.string()
		.min(1)
		.describe('Exact text to find in the flow value, serialized as compact JSON (no indent).'),
	new_string: z.string().describe('Replacement JSON text.'),
	replace_all: z
		.boolean()
		.optional()
		.default(false)
		.describe(
			'When true, replace every exact match. When false, old_string must match exactly once.'
		)
})

const GLOBAL_SYSTEM_PROMPT = `You are Windmill's global workspace assistant.

You can inspect workspace scripts, flows, schedules, triggers, resources, and variables, then create draft changes in the frontend AI draft store.

Important rules:
- write_{script,flow,schedule,trigger,resource,variable} create or overwrite drafts. They do not save, deploy, or mutate workspace items.
- edit_script and patch_flow_json apply small exact-text edits and save the result as a draft. Prefer them for localized changes; use write_* for large rewrites.
- deploy_workspace_item persists a draft to the workspace via the real backend create/update API and removes the draft. Requires user confirmation. Only call after the user has reviewed the draft and explicitly asked to deploy.
- delete_workspace_item permanently removes a workspace item (and any matching draft). Irreversible. Requires user confirmation. Only call when the user has explicitly asked to delete.
- Use list_workspace_items before broad reads.
- Use read_workspace_item before overwriting an existing item, unless the user already provided the complete current item. For triggers, pass trigger_kind.
- Variable values are NEVER returned by read_workspace_item or list_workspace_items — only metadata (path, description, is_secret). The model cannot read secret values, by design.
- For resources that need secrets, write a Variable first (with is_secret: true), then in the resource value reference it as "$var:path/to/variable". When deploying both, deploy the variable before the resource.
- Use search_resource_types before write_resource to discover the resource_type name and the JSON Schema its value must match.
- Use get_instructions before writing a script, flow, or resource. For scripts, pass the target language; when modifying, use the language from the item you read.
- Schedules, triggers, and variables do not need get_instructions — their tool schemas describe every field.
- A workspace item is { type, path, summary?, language?, triggerKind?, value, isDraft }. For scripts, value is the source code string. For flows, value is the OpenFlow value object. For schedules/triggers/resources/variables, value is the full request body for that type.
- Keep context targeted. Do not read unrelated items.
- Be explicit with the user when you create or update a draft.`

const DEFAULT_LIST_TYPES = ['script', 'flow'] as const satisfies readonly WorkspaceItemType[]

function getRequestedTypes(types: WorkspaceItemType[] | undefined): WorkspaceItemType[] {
	return types && types.length > 0 ? types : [...DEFAULT_LIST_TYPES]
}

function countExactMatches(content: string, search: string): number {
	if (search.length === 0) return 0
	let count = 0
	let index = 0
	while ((index = content.indexOf(search, index)) !== -1) {
		count++
		index += search.length
	}
	return count
}

function applyExactReplace(
	content: string,
	oldString: string,
	newString: string,
	replaceAll: boolean
): string {
	if (replaceAll) return content.split(oldString).join(newString)
	const index = content.indexOf(oldString)
	if (index === -1) return content
	return content.slice(0, index) + newString + content.slice(index + oldString.length)
}

function itemMatches(
	item: Pick<WorkspaceItem, 'path' | 'summary'>,
	query: string | undefined
): boolean {
	const normalized = query?.trim().toLowerCase()
	if (!normalized) return true
	return (
		item.path.toLowerCase().includes(normalized) ||
		(item.summary?.toLowerCase().includes(normalized) ?? false)
	)
}

function scriptToItem(script: Script, includeValue: boolean): WorkspaceItem {
	return {
		type: 'script',
		path: script.path,
		summary: script.summary,
		language: script.language,
		value: includeValue ? script.content : undefined,
		isDraft: false
	}
}

function flowToItem(flow: Flow, includeValue: boolean): WorkspaceItem {
	return {
		type: 'flow',
		path: flow.path,
		summary: flow.summary,
		value: includeValue ? flow.value : undefined,
		isDraft: false
	}
}

function scheduleToItem(schedule: Schedule, includeValue: boolean): WorkspaceItem {
	return {
		type: 'schedule',
		path: schedule.path,
		summary: schedule.summary ?? undefined,
		value: includeValue ? (schedule as unknown as WorkspaceItem['value']) : undefined,
		isDraft: false
	}
}

function resourceToItem(resource: ListableResource, includeValue: boolean): WorkspaceItem {
	return {
		type: 'resource',
		path: resource.path,
		summary: resource.description,
		value: includeValue ? (resource as unknown as WorkspaceItem['value']) : undefined,
		isDraft: false
	}
}

function variableToItem(variable: ListableVariable): WorkspaceItem {
	// Variables NEVER expose value (secret risk). Returns metadata only.
	return {
		type: 'variable',
		path: variable.path,
		summary: variable.description,
		isDraft: false
	}
}

type TriggerLike = { path: string; summary?: string | null }

function triggerToItem(
	kind: TriggerKind,
	trigger: TriggerLike,
	includeValue: boolean
): WorkspaceItem {
	return {
		type: 'trigger',
		triggerKind: kind,
		path: trigger.path,
		summary: trigger.summary ?? undefined,
		value: includeValue ? (trigger as unknown as WorkspaceItem['value']) : undefined,
		isDraft: false
	}
}

type TriggerService = {
	exists(args: { workspace: string; path: string }): Promise<boolean>
	get(args: { workspace: string; path: string }): Promise<TriggerLike>
	list(args: {
		workspace: string
		pathStart?: string
		perPage?: number
	}): Promise<TriggerLike[]>
	create(args: { workspace: string; requestBody: any }): Promise<string>
	update(args: { workspace: string; path: string; requestBody: any }): Promise<string>
	delete(args: { workspace: string; path: string }): Promise<string>
}

const triggerServices: Record<TriggerKind, TriggerService> = {
	http: {
		exists: (a) => HttpTriggerService.existsHttpTrigger(a),
		get: (a) => HttpTriggerService.getHttpTrigger(a),
		list: (a) => HttpTriggerService.listHttpTriggers(a),
		create: (a) => HttpTriggerService.createHttpTrigger(a),
		update: (a) => HttpTriggerService.updateHttpTrigger(a),
		delete: (a) => HttpTriggerService.deleteHttpTrigger(a)
	},
	websocket: {
		exists: (a) => WebsocketTriggerService.existsWebsocketTrigger(a),
		get: (a) => WebsocketTriggerService.getWebsocketTrigger(a),
		list: (a) => WebsocketTriggerService.listWebsocketTriggers(a),
		create: (a) => WebsocketTriggerService.createWebsocketTrigger(a),
		update: (a) => WebsocketTriggerService.updateWebsocketTrigger(a),
		delete: (a) => WebsocketTriggerService.deleteWebsocketTrigger(a)
	},
	kafka: {
		exists: (a) => KafkaTriggerService.existsKafkaTrigger(a),
		get: (a) => KafkaTriggerService.getKafkaTrigger(a),
		list: (a) => KafkaTriggerService.listKafkaTriggers(a),
		create: (a) => KafkaTriggerService.createKafkaTrigger(a),
		update: (a) => KafkaTriggerService.updateKafkaTrigger(a),
		delete: (a) => KafkaTriggerService.deleteKafkaTrigger(a)
	},
	nats: {
		exists: (a) => NatsTriggerService.existsNatsTrigger(a),
		get: (a) => NatsTriggerService.getNatsTrigger(a),
		list: (a) => NatsTriggerService.listNatsTriggers(a),
		create: (a) => NatsTriggerService.createNatsTrigger(a),
		update: (a) => NatsTriggerService.updateNatsTrigger(a),
		delete: (a) => NatsTriggerService.deleteNatsTrigger(a)
	},
	postgres: {
		exists: (a) => PostgresTriggerService.existsPostgresTrigger(a),
		get: (a) => PostgresTriggerService.getPostgresTrigger(a),
		list: (a) => PostgresTriggerService.listPostgresTriggers(a),
		create: (a) => PostgresTriggerService.createPostgresTrigger(a),
		update: (a) => PostgresTriggerService.updatePostgresTrigger(a),
		delete: (a) => PostgresTriggerService.deletePostgresTrigger(a)
	},
	mqtt: {
		exists: (a) => MqttTriggerService.existsMqttTrigger(a),
		get: (a) => MqttTriggerService.getMqttTrigger(a),
		list: (a) => MqttTriggerService.listMqttTriggers(a),
		create: (a) => MqttTriggerService.createMqttTrigger(a),
		update: (a) => MqttTriggerService.updateMqttTrigger(a),
		delete: (a) => MqttTriggerService.deleteMqttTrigger(a)
	},
	sqs: {
		exists: (a) => SqsTriggerService.existsSqsTrigger(a),
		get: (a) => SqsTriggerService.getSqsTrigger(a),
		list: (a) => SqsTriggerService.listSqsTriggers(a),
		create: (a) => SqsTriggerService.createSqsTrigger(a),
		update: (a) => SqsTriggerService.updateSqsTrigger(a),
		delete: (a) => SqsTriggerService.deleteSqsTrigger(a)
	},
	gcp: {
		exists: (a) => GcpTriggerService.existsGcpTrigger(a),
		get: (a) => GcpTriggerService.getGcpTrigger(a),
		list: (a) => GcpTriggerService.listGcpTriggers(a),
		create: (a) => GcpTriggerService.createGcpTrigger(a),
		update: (a) => GcpTriggerService.updateGcpTrigger(a),
		delete: (a) => GcpTriggerService.deleteGcpTrigger(a)
	},
	azure: {
		exists: (a) => AzureTriggerService.existsAzureTrigger(a),
		get: (a) => AzureTriggerService.getAzureTrigger(a),
		list: (a) => AzureTriggerService.listAzureTriggers(a),
		create: (a) => AzureTriggerService.createAzureTrigger(a),
		update: (a) => AzureTriggerService.updateAzureTrigger(a),
		delete: (a) => AzureTriggerService.deleteAzureTrigger(a)
	}
}

async function workspaceItemExists(
	type: WorkspaceItemType,
	path: string,
	workspace: string,
	triggerKind?: TriggerKind
): Promise<boolean> {
	switch (type) {
		case 'script':
			return ScriptService.existsScriptByPath({ workspace, path })
		case 'flow':
			return FlowService.existsFlowByPath({ workspace, path })
		case 'schedule':
			return ScheduleService.existsSchedule({ workspace, path })
		case 'trigger':
			if (!triggerKind) return false
			return triggerServices[triggerKind].exists({ workspace, path })
		case 'resource':
			return ResourceService.existsResource({ workspace, path })
		case 'variable':
			return VariableService.existsVariable({ workspace, path })
	}
}

async function readWorkspaceItem(
	type: WorkspaceItemType,
	path: string,
	workspace: string,
	triggerKind?: TriggerKind
): Promise<WorkspaceItem> {
	switch (type) {
		case 'script':
			return scriptToItem(await ScriptService.getScriptByPath({ workspace, path }), true)
		case 'flow':
			return flowToItem(await FlowService.getFlowByPath({ workspace, path }), true)
		case 'schedule':
			return scheduleToItem(await ScheduleService.getSchedule({ workspace, path }), true)
		case 'trigger':
			if (!triggerKind) {
				throw new Error('trigger_kind is required when type is trigger.')
			}
			return triggerToItem(
				triggerKind,
				await triggerServices[triggerKind].get({ workspace, path }),
				true
			)
		case 'resource':
			return resourceToItem(
				await ResourceService.getResource({ workspace, path }) as ListableResource,
				true
			)
		case 'variable':
			// Never expose the value, even when read directly. Pass decryptSecret=false
			// to avoid materializing secret values server-side.
			return variableToItem(
				await VariableService.getVariable({ workspace, path, decryptSecret: false })
			)
	}
}

async function listWorkspaceItems(
	types: WorkspaceItemType[],
	workspace: string,
	pathPrefix: string | undefined,
	perPage: number
): Promise<WorkspaceItem[]> {
	const items: WorkspaceItem[] = []

	if (types.includes('script')) {
		const scripts = await ScriptService.listScripts({
			workspace,
			pathStart: pathPrefix,
			perPage,
			includeDraftOnly: true,
			withoutDescription: true
		})
		for (const script of scripts) items.push(scriptToItem(script, false))
	}

	if (types.includes('flow')) {
		const flows = await FlowService.listFlows({
			workspace,
			pathStart: pathPrefix,
			perPage,
			includeDraftOnly: true,
			withoutDescription: true
		})
		for (const flow of flows) items.push(flowToItem(flow, false))
	}

	if (types.includes('schedule')) {
		const schedules = await ScheduleService.listSchedules({
			workspace,
			pathStart: pathPrefix,
			perPage
		})
		for (const schedule of schedules) items.push(scheduleToItem(schedule, false))
	}

	if (types.includes('trigger')) {
		for (const kind of TRIGGER_KINDS) {
			const triggers = await triggerServices[kind].list({
				workspace,
				pathStart: pathPrefix,
				perPage
			})
			for (const trigger of triggers) items.push(triggerToItem(kind, trigger, false))
		}
	}

	if (types.includes('resource')) {
		const resources = await ResourceService.listResource({
			workspace,
			pathStart: pathPrefix,
			perPage
		})
		for (const resource of resources) items.push(resourceToItem(resource, false))
	}

	if (types.includes('variable')) {
		const variables = await VariableService.listVariable({
			workspace,
			pathStart: pathPrefix,
			perPage
		})
		for (const variable of variables) items.push(variableToItem(variable))
	}

	return items
}

function getScriptInstructions(language: ScriptLang | undefined): string {
	const selected = language ?? 'bun'
	const note = language
		? ''
		: `\n- No script language was provided. Default to \`bun\` only for new TypeScript scripts; if the user requested another language or you read an existing script, call get_instructions again with that language.`

	return `# Global draft script instructions

- Global mode writes complete draft payloads only; it does not save, deploy, run, or generate metadata.
- A script draft is a workspace item: \`{ type: 'script', path, summary?, language, value, isDraft }\` where \`value\` is the source code string.
- Use workspace paths such as \`f/folder/name\` or \`u/username/name\`. Preserve the current path/language when modifying unless the user asked to change them.
- Use \`edit_script\` for small localized changes (provide \`old_string\`/\`new_string\`); use \`write_script\` for full rewrites.${note}

# Windmill script authoring reference (${selected})

${getScriptPrompt(selected)}`
}

function getFlowInstructions(): string {
	return `# Global draft flow instructions

- Global mode writes complete draft payloads only; it does not save, deploy, run, scaffold local files, or generate metadata.
- A flow draft is a workspace item: \`{ type: 'flow', path, summary?, value, isDraft }\` where \`value\` is the OpenFlow value object.
- \`value.modules\` contains normal sequential modules. Use top-level \`value.preprocessor_module\` and \`value.failure_module\` for special modules; do not put \`preprocessor\` or \`failure\` in \`value.modules\`.
- Every module needs a stable unique \`id\` and a useful \`summary\` when the schema supports it.
- Prefer path/script/flow modules when composing existing workspace logic. Use rawscript modules only when new inline code is needed.
- When writing rawscript module code, call \`get_instructions\` with \`subject: "script"\` and the rawscript language first.
- Use \`patch_flow_json\` for small localized changes (operates on the value as compact JSON); use \`write_flow\` for full rewrites.

# Windmill flow authoring reference

${getFlowPrompt()}`
}

type InstructionSubject = (typeof INSTRUCTION_SUBJECTS)[number]

function getResourceInstructions(): string {
	return `# Global draft resource & variable instructions

- Global mode writes complete draft payloads only; it does not save, deploy, run, scaffold local files, or generate metadata.
- A resource draft is a workspace item: \`{ type: 'resource', path, summary?, value, isDraft }\`. \`value\` is a CreateResource body: \`{ path, value, description?, resource_type, labels? }\` where the inner \`value\` is the resource type's data shape.
- A variable draft is a workspace item: \`{ type: 'variable', path, summary?, value, isDraft }\`. \`value\` is a CreateVariable body: \`{ path, value, is_secret, description, account?, is_oauth?, expires_at?, labels? }\`.
- For secret fields in a resource value, do NOT inline the raw secret. Create a Variable first with \`is_secret: true\`, then in the resource value reference it as \`"$var:path/to/variable"\`.
- Reference formats inside resource values: \`$var:g/all/name\` (global), \`$var:u/user/name\` (user), \`$var:f/folder/name\` (folder). Reference another resource with \`$res:path/to/resource\`.
- When deploying drafts that depend on each other (e.g., a resource and the variables it references), deploy the variables first.
- Use \`search_resource_types\` to discover valid \`resource_type\` names and their JSON Schemas. Match the resource value to that schema.
- For OAuth resources, the \`is_oauth: true\` flag is managed by Windmill's OAuth flow; global mode generally creates manual resources, not OAuth ones.

# Windmill resource & variable reference

${getResourcePrompt()}`
}

function getInstructions(subject: InstructionSubject, language?: ScriptLang): string {
	switch (subject) {
		case 'script':
			return getScriptInstructions(language)
		case 'flow':
			return getFlowInstructions()
		case 'resource':
			return getResourceInstructions()
	}
}

export const globalTools: Tool<{}>[] = [
	{
		def: createToolDef(
			getInstructionsSchema,
			'get_instructions',
			'Get Windmill authoring instructions for scripts or flows. For scripts, pass the target language.'
		),
		fn: async ({ args, toolId, toolCallbacks }) => {
			const parsed = getInstructionsSchema.parse(args)
			const label =
				parsed.subject === 'script' && parsed.language
					? `${parsed.subject} (${parsed.language})`
					: parsed.subject
			toolCallbacks.setToolStatus(toolId, { content: `Loaded ${label} instructions` })
			return getInstructions(parsed.subject, parsed.language)
		}
	},
	{
		def: createToolDef(
			listWorkspaceItemsSchema,
			'list_workspace_items',
			'List workspace items (scripts, flows, schedules, triggers) and AI drafts. Returns metadata only (no value). Defaults to scripts and flows.'
		),
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsed = listWorkspaceItemsSchema.parse(args)
			const types = getRequestedTypes(parsed.types)
			const limit = parsed.limit ?? 50
			toolCallbacks.setToolStatus(toolId, { content: 'Listing workspace items...' })

			const byKey = new Map<string, WorkspaceItem>()
			const workspaceItems = await listWorkspaceItems(
				types,
				workspace,
				parsed.path_prefix,
				Math.min(limit, MAX_LIST_LIMIT)
			)
			for (const item of workspaceItems) {
				byKey.set(getWorkspaceItemKey(item.type, item.path, item.triggerKind), item)
			}

			for (const draft of globalDraftStore.listDrafts()) {
				if (!types.includes(draft.type)) continue
				byKey.set(getWorkspaceItemKey(draft.type, draft.path, draft.triggerKind), {
					...draft,
					value: undefined
				})
			}

			const results = Array.from(byKey.values())
				.filter((item) => itemMatches(item, parsed.query))
				.slice(0, limit)

			toolCallbacks.setToolStatus(toolId, {
				content: `Listed ${results.length} workspace item(s)`
			})
			return JSON.stringify(results, null, 2)
		}
	},
	{
		def: createToolDef(
			readWorkspaceItemSchema,
			'read_workspace_item',
			'Read one workspace item or AI draft by type and path. Returns the full workspace item including value.'
		),
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsed = readWorkspaceItemSchema.parse(args)
			if (parsed.type === 'trigger' && !parsed.trigger_kind) {
				const message = 'trigger_kind is required when type is trigger.'
				toolCallbacks.setToolStatus(toolId, { content: message, error: message })
				return JSON.stringify({ success: false, error: message })
			}
			const draft = globalDraftStore.getDraft(parsed.type, parsed.path, parsed.trigger_kind)
			if (draft) {
				toolCallbacks.setToolStatus(toolId, {
					content: `Read AI draft ${parsed.type} "${parsed.path}"`
				})
				return JSON.stringify(draft, null, 2)
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Reading ${parsed.type} "${parsed.path}"...`
			})
			const item = await readWorkspaceItem(
				parsed.type,
				parsed.path,
				workspace,
				parsed.trigger_kind
			)
			toolCallbacks.setToolStatus(toolId, { content: `Read ${parsed.type} "${parsed.path}"` })
			return JSON.stringify(item, null, 2)
		}
	},
	{
		def: createToolDef(
			writeScriptSchema,
			'write_script',
			'Create or overwrite an AI draft script. Does not save or deploy. Read the existing script first when overwriting.'
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = writeScriptSchema.parse(ctx.args)
			return writeDraft(
				{
					type: 'script',
					path: parsed.path,
					summary: parsed.summary,
					language: parsed.language,
					value: parsed.content,
					isDraft: true
				},
				ctx
			)
		}
	},
	{
		def: createToolDef(
			writeFlowSchema,
			'write_flow',
			'Create or overwrite an AI draft flow. Does not save or deploy. Read the existing flow first when overwriting.'
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = writeFlowSchema.parse(ctx.args)
			return writeDraft(
				{
					type: 'flow',
					path: parsed.path,
					summary: parsed.summary,
					value: parsed.value,
					isDraft: true
				},
				ctx
			)
		}
	},
	{
		def: createToolDef(
			writeScheduleSchema,
			'write_schedule',
			'Create or overwrite an AI draft schedule. Does not save or deploy. Provide script_path and is_flow to point to the runnable.',
			{ strict: false }
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = writeScheduleSchema.parse(ctx.args)
			return writeDraft(
				{
					type: 'schedule',
					path: parsed.path,
					summary: parsed.summary ?? undefined,
					value: parsed,
					isDraft: true
				},
				ctx
			)
		}
	},
	{
		def: createToolDef(
			writeTriggerSchema,
			'write_trigger',
			'Create or overwrite an AI draft trigger. Does not save or deploy. Provide kind plus the kind-specific config (including path, script_path, is_flow).',
			{ strict: false }
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = writeTriggerSchema.parse(ctx.args)
			const config = parsed.config as { path: string; summary?: string | null }
			return writeDraft(
				{
					type: 'trigger',
					triggerKind: parsed.kind,
					path: config.path,
					summary: config.summary ?? undefined,
					value: parsed.config,
					isDraft: true
				},
				ctx
			)
		}
	},
	{
		def: createToolDef(
			editScriptSchema,
			'edit_script',
			'Find/replace exact text in a script. Edits the existing draft if one exists, otherwise reads the workspace script and saves the result as a new draft.'
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = editScriptSchema.parse(ctx.args)
			return editScript(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			patchFlowJsonSchema,
			'patch_flow_json',
			'Find/replace exact text in a flow value (compact JSON). Edits the existing draft if one exists, otherwise reads the workspace flow and saves the result as a new draft. Use write_flow for larger structural rewrites.'
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = patchFlowJsonSchema.parse(ctx.args)
			return patchFlowJson(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			deployWorkspaceItemSchema,
			'deploy_workspace_item',
			'Persist an AI draft to the workspace by calling the real backend create/update API. This MUTATES the workspace. Requires user confirmation.',
			{ strict: false }
		),
		showDetails: true,
		showFade: true,
		requiresConfirmation: true,
		confirmationMessage: 'Deploy AI draft to workspace',
		fn: async (ctx) => {
			const parsed = deployWorkspaceItemSchema.parse(ctx.args)
			return deployDraft(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			deleteWorkspaceItemSchema,
			'delete_workspace_item',
			'Permanently delete a workspace item by path. This MUTATES the workspace and is irreversible. Also clears any matching AI draft. Requires user confirmation.'
		),
		showDetails: true,
		showFade: true,
		requiresConfirmation: true,
		confirmationMessage: 'Delete workspace item',
		fn: async (ctx) => {
			const parsed = deleteWorkspaceItemSchema.parse(ctx.args)
			return deleteWorkspaceItem(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			writeResourceSchema,
			'write_resource',
			'Create or overwrite an AI draft resource. Does not save or deploy. Reference secret values via $var:path/to/variable; create the variable separately with write_variable.',
			{ strict: false }
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = writeResourceSchema.parse(ctx.args)
			return writeDraft(
				{
					type: 'resource',
					path: parsed.path,
					summary: parsed.description,
					value: parsed,
					isDraft: true
				},
				ctx
			)
		}
	},
	{
		def: createToolDef(
			writeVariableSchema,
			'write_variable',
			'Create or overwrite an AI draft variable. Does not save or deploy. Use is_secret: true for secret values. After deploy, reference from a resource as $var:path/to/variable.',
			{ strict: false }
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = writeVariableSchema.parse(ctx.args)
			return writeDraft(
				{
					type: 'variable',
					path: parsed.path,
					summary: parsed.description,
					value: parsed,
					isDraft: true
				},
				ctx
			)
		}
	},
	{
		def: createToolDef(
			searchResourceTypesSchema,
			'search_resource_types',
			'Search for resource types in the workspace by substring. Returns names, descriptions, and JSON Schemas — use this before write_resource to know what shape value should have.'
		),
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsed = searchResourceTypesSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, {
				content: `Searching resource types for "${parsed.query}"...`
			})
			const results = await ResourceService.queryResourceTypes({
				workspace,
				text: parsed.query,
				limit: parsed.limit ?? 5
			})
			toolCallbacks.setToolStatus(toolId, {
				content: `Found ${results.length} resource type(s) for "${parsed.query}"`
			})
			return JSON.stringify(
				results.map((rt) => ({
					name: rt.name,
					schema: rt.schema
				})),
				null,
				2
			)
		}
	}
]

type WriteDraftCtx = {
	workspace: string
	toolId: string
	toolCallbacks: ToolCallbacks
}

async function loadScriptForEdit(
	path: string,
	workspace: string
): Promise<{ content: string; language: ScriptLang; summary?: string }> {
	const draft = globalDraftStore.getDraft('script', path)
	if (draft) {
		if (typeof draft.value !== 'string' || !draft.language) {
			throw new Error(`Draft script "${path}" is missing content or language.`)
		}
		return { content: draft.value, language: draft.language, summary: draft.summary }
	}
	const script = await ScriptService.getScriptByPath({ workspace, path })
	return { content: script.content, language: script.language, summary: script.summary }
}

async function editScript(
	args: { path: string; old_string: string; new_string: string; replace_all: boolean },
	ctx: WriteDraftCtx
): Promise<string> {
	const { path, old_string: oldString, new_string: newString, replace_all: replaceAll } = args
	ctx.toolCallbacks.setToolStatus(ctx.toolId, { content: `Editing script "${path}"...` })

	const base = await loadScriptForEdit(path, ctx.workspace)
	const matchCount = countExactMatches(base.content, oldString)
	if (matchCount === 0) {
		throw new Error('old_string was not found in the script source.')
	}
	if (!replaceAll && matchCount !== 1) {
		throw new Error(
			`old_string matched ${matchCount} locations. Make it more specific or set replace_all to true.`
		)
	}

	const updated = applyExactReplace(base.content, oldString, newString, replaceAll)
	return writeDraft(
		{
			type: 'script',
			path,
			summary: base.summary,
			language: base.language,
			value: updated,
			isDraft: true
		},
		ctx
	)
}

async function loadFlowValueForEdit(
	path: string,
	workspace: string
): Promise<{ value: unknown; summary?: string }> {
	const draft = globalDraftStore.getDraft('flow', path)
	if (draft) {
		if (draft.value === undefined) {
			throw new Error(`Draft flow "${path}" has no value.`)
		}
		return { value: draft.value, summary: draft.summary }
	}
	const flow = await FlowService.getFlowByPath({ workspace, path })
	return { value: flow.value, summary: flow.summary }
}

async function patchFlowJson(
	args: { path: string; old_string: string; new_string: string; replace_all: boolean },
	ctx: WriteDraftCtx
): Promise<string> {
	const { path, old_string: oldString, new_string: newString, replace_all: replaceAll } = args
	ctx.toolCallbacks.setToolStatus(ctx.toolId, { content: `Patching flow "${path}"...` })

	const base = await loadFlowValueForEdit(path, ctx.workspace)
	const currentJson = JSON.stringify(base.value)
	const matchCount = countExactMatches(currentJson, oldString)
	if (matchCount === 0) {
		throw new Error('old_string was not found in the flow value JSON.')
	}
	if (!replaceAll && matchCount !== 1) {
		throw new Error(
			`old_string matched ${matchCount} locations. Make it more specific or set replace_all to true.`
		)
	}

	const updatedJson = applyExactReplace(currentJson, oldString, newString, replaceAll)
	let parsedValue: unknown
	try {
		parsedValue = JSON.parse(updatedJson)
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error)
		throw new Error(`Invalid JSON after replacement: ${message}`)
	}

	const validated = flowValueSchema.safeParse(parsedValue)
	if (!validated.success) {
		throw new Error(
			`Replacement produced an invalid flow value: ${validated.error.issues
				.slice(0, 5)
				.map((i) => `${i.path.join('.')}: ${i.message}`)
				.join('; ')}`
		)
	}

	return writeDraft(
		{
			type: 'flow',
			path,
			summary: base.summary,
			value: validated.data,
			isDraft: true
		},
		ctx
	)
}

const triggerLabels: Record<TriggerKind, string> = {
	http: 'HTTP trigger',
	websocket: 'WebSocket trigger',
	kafka: 'Kafka trigger',
	nats: 'NATS trigger',
	postgres: 'Postgres trigger',
	mqtt: 'MQTT trigger',
	sqs: 'SQS trigger',
	gcp: 'GCP Pub/Sub trigger',
	azure: 'Azure Event Grid trigger'
}

function createOpenScheduleAction(
	path: string,
	targetKind: 'script' | 'flow'
): ToolDisplayAction {
	return {
		id: `open-deployed-schedule:${path}`,
		type: 'open_created_resource',
		label: 'Open schedule',
		resource: 'schedule',
		path,
		targetKind
	}
}

function createOpenTriggerAction(
	kind: TriggerKind,
	path: string,
	targetKind: 'script' | 'flow'
): ToolDisplayAction {
	return {
		id: `open-deployed-trigger:${kind}:${path}`,
		type: 'open_created_resource',
		label: `Open ${triggerLabels[kind]}`,
		resource: 'trigger',
		triggerKind: kind as CreatedResourceTriggerKind,
		path,
		targetKind
	}
}

async function deployDraft(
	args: {
		type: WorkspaceItemType
		path: string
		trigger_kind?: TriggerKind
		deployment_message?: string
	},
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const { type, path, trigger_kind: triggerKind, deployment_message: deploymentMessage } = args

	if (type === 'trigger' && !triggerKind) {
		throw new Error('trigger_kind is required when deploying a trigger.')
	}

	const draft = globalDraftStore.getDraft(type, path, triggerKind)
	if (!draft) {
		throw new Error(`No AI draft found for ${type} "${path}".`)
	}
	if (draft.value === undefined) {
		throw new Error(`Draft ${type} "${path}" has no value to deploy.`)
	}

	toolCallbacks.setToolStatus(toolId, {
		content: `Deploying ${type} "${path}"...`
	})

	let actions: ToolDisplayAction[] | undefined

	switch (type) {
		case 'script': {
			if (typeof draft.value !== 'string' || !draft.language) {
				throw new Error(`Draft script "${path}" is missing content or language.`)
			}
			const existing = (await ScriptService.existsScriptByPath({ workspace, path }))
				? await ScriptService.getScriptByPath({ workspace, path })
				: undefined
			await ScriptService.createScript({
				workspace,
				requestBody: {
					path,
					summary: draft.summary ?? '',
					content: draft.value,
					language: draft.language,
					parent_hash: existing?.hash,
					deployment_message: deploymentMessage
				}
			})
			break
		}
		case 'flow': {
			const value = draft.value as Flow['value']
			const requestBody = {
				path,
				summary: draft.summary ?? '',
				value,
				schema: {},
				deployment_message: deploymentMessage
			}
			if (await FlowService.existsFlowByPath({ workspace, path })) {
				await FlowService.updateFlow({ workspace, path, requestBody })
			} else {
				await FlowService.createFlow({ workspace, requestBody })
			}
			break
		}
		case 'schedule': {
			const requestBody = draft.value as any
			if (await ScheduleService.existsSchedule({ workspace, path })) {
				await ScheduleService.updateSchedule({ workspace, path, requestBody })
			} else {
				await ScheduleService.createSchedule({ workspace, requestBody })
			}
			actions = [createOpenScheduleAction(path, requestBody.is_flow ? 'flow' : 'script')]
			break
		}
		case 'trigger': {
			const service = triggerServices[triggerKind!]
			const requestBody = draft.value as { is_flow?: boolean }
			if (await service.exists({ workspace, path })) {
				await service.update({ workspace, path, requestBody })
			} else {
				await service.create({ workspace, requestBody })
			}
			actions = [
				createOpenTriggerAction(triggerKind!, path, requestBody.is_flow ? 'flow' : 'script')
			]
			break
		}
		case 'resource': {
			const requestBody = draft.value as any
			if (await ResourceService.existsResource({ workspace, path })) {
				await ResourceService.updateResource({ workspace, path, requestBody })
			} else {
				await ResourceService.createResource({ workspace, requestBody })
			}
			break
		}
		case 'variable': {
			const requestBody = draft.value as any
			if (await VariableService.existsVariable({ workspace, path })) {
				await VariableService.updateVariable({ workspace, path, requestBody })
			} else {
				await VariableService.createVariable({ workspace, requestBody })
			}
			break
		}
	}

	globalDraftStore.deleteDraft(type, path, triggerKind)

	toolCallbacks.setToolStatus(toolId, {
		content: `Deployed ${type} "${path}"`,
		result: 'Deployed',
		actions
	})
	return JSON.stringify(
		{
			success: true,
			message: `Deployed AI draft ${type} "${path}" to the workspace. Draft removed from the AI draft store.`,
			type,
			path,
			triggerKind
		},
		null,
		2
	)
}

async function deleteWorkspaceItem(
	args: { type: WorkspaceItemType; path: string; trigger_kind?: TriggerKind },
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const { type, path, trigger_kind: triggerKind } = args

	if (type === 'trigger' && !triggerKind) {
		throw new Error('trigger_kind is required when deleting a trigger.')
	}

	toolCallbacks.setToolStatus(toolId, {
		content: `Deleting ${type} "${path}"...`
	})

	switch (type) {
		case 'script':
			await ScriptService.deleteScriptByPath({ workspace, path })
			break
		case 'flow':
			await FlowService.deleteFlowByPath({ workspace, path })
			break
		case 'schedule':
			await ScheduleService.deleteSchedule({ workspace, path })
			break
		case 'trigger':
			await triggerServices[triggerKind!].delete({ workspace, path })
			break
		case 'resource':
			await ResourceService.deleteResource({ workspace, path })
			break
		case 'variable':
			await VariableService.deleteVariable({ workspace, path })
			break
	}

	globalDraftStore.deleteDraft(type, path, triggerKind)

	toolCallbacks.setToolStatus(toolId, {
		content: `Deleted ${type} "${path}"`,
		result: 'Deleted'
	})
	return JSON.stringify(
		{
			success: true,
			message: `Deleted ${type} "${path}" from the workspace. Any matching AI draft was also cleared.`,
			type,
			path,
			triggerKind
		},
		null,
		2
	)
}

async function writeDraft(item: WorkspaceItem, ctx: WriteDraftCtx): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	toolCallbacks.setToolStatus(toolId, {
		content: `Writing draft ${item.type} "${item.path}"...`
	})

	const exists =
		globalDraftStore.getDraft(item.type, item.path, item.triggerKind) !== undefined ||
		(await workspaceItemExists(item.type, item.path, workspace, item.triggerKind))

	const stored = globalDraftStore.setDraft(item)

	const verb = exists ? 'Updated' : 'Created'
	toolCallbacks.setToolStatus(toolId, {
		content: `${verb} AI draft ${item.type} "${item.path}"`,
		result: `Draft ${verb.toLowerCase()}`
	})
	return JSON.stringify(
		{
			success: true,
			message: `${verb} AI draft ${item.type} "${item.path}". The workspace was not saved or deployed.`,
			item: stored
		},
		null,
		2
	)
}

export function prepareGlobalSystemMessage(
	customPrompt?: string
): ChatCompletionSystemMessageParam {
	let content = GLOBAL_SYSTEM_PROMPT
	if (customPrompt?.trim()) {
		content = `${content}\n\nUSER GIVEN INSTRUCTIONS:\n${customPrompt.trim()}`
	}

	return {
		role: 'system',
		content
	}
}

export function prepareGlobalUserMessage(instructions: string): ChatCompletionUserMessageParam {
	return {
		role: 'user',
		content: instructions
	}
}
