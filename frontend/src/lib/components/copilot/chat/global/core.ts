import {
	AzureTriggerService,
	FlowService,
	GcpTriggerService,
	HttpTriggerService,
	KafkaTriggerService,
	MqttTriggerService,
	NatsTriggerService,
	PostgresTriggerService,
	ScheduleService,
	ScriptService,
	SqsTriggerService,
	WebsocketTriggerService
} from '$lib/gen'
import { $ScriptLang } from '$lib/gen/schemas.gen'
import type { Flow, Schedule, Script, ScriptLang } from '$lib/gen/types.gen'
import { getFlowPrompt, getScriptPrompt } from '$system_prompts'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import { z } from 'zod'
import { createToolDef, type Tool, type ToolCallbacks } from '../shared'
import { flowModuleSchema, flowModulesSchema } from '../flow/openFlowZod.gen'
import { scheduleRequestSchema, triggerRequestSchemas } from '../workspaceToolsZod.gen'
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
	'trigger'
] as const satisfies readonly WorkspaceItemType[]
const INSTRUCTION_SUBJECTS = ['script', 'flow'] as const satisfies readonly WorkspaceItemType[]
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

You can inspect workspace scripts, flows, schedules, and triggers, then create draft changes in the frontend AI draft store.

Important rules:
- write_script, write_flow, write_schedule, and write_trigger create or overwrite drafts. They do not save, deploy, or mutate workspace items.
- edit_script and patch_flow_json apply small exact-text edits and save the result as a draft. Prefer them for localized changes; use write_* for large rewrites.
- deploy_workspace_item is the only tool that mutates the workspace. It calls the real backend create/update API for one draft and removes it from the draft store. The user must confirm. Only call deploy after the user has reviewed the draft and explicitly asked to deploy.
- Use list_workspace_items before broad reads.
- Use read_workspace_item before overwriting an existing item, unless the user already provided the complete current item. For triggers, pass trigger_kind.
- Use get_instructions before writing a script or flow. For scripts, pass the target language; when modifying, use the language from the item you read.
- Schedules and triggers do not need get_instructions — their tool schemas describe every field.
- A workspace item is { type, path, summary?, language?, triggerKind?, value, isDraft }. For scripts, value is the source code string. For flows, value is the OpenFlow value object. For schedules, value is the full schedule object. For triggers, value is the kind-specific trigger config.
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
}

const triggerServices: Record<TriggerKind, TriggerService> = {
	http: {
		exists: (a) => HttpTriggerService.existsHttpTrigger(a),
		get: (a) => HttpTriggerService.getHttpTrigger(a),
		list: (a) => HttpTriggerService.listHttpTriggers(a),
		create: (a) => HttpTriggerService.createHttpTrigger(a),
		update: (a) => HttpTriggerService.updateHttpTrigger(a)
	},
	websocket: {
		exists: (a) => WebsocketTriggerService.existsWebsocketTrigger(a),
		get: (a) => WebsocketTriggerService.getWebsocketTrigger(a),
		list: (a) => WebsocketTriggerService.listWebsocketTriggers(a),
		create: (a) => WebsocketTriggerService.createWebsocketTrigger(a),
		update: (a) => WebsocketTriggerService.updateWebsocketTrigger(a)
	},
	kafka: {
		exists: (a) => KafkaTriggerService.existsKafkaTrigger(a),
		get: (a) => KafkaTriggerService.getKafkaTrigger(a),
		list: (a) => KafkaTriggerService.listKafkaTriggers(a),
		create: (a) => KafkaTriggerService.createKafkaTrigger(a),
		update: (a) => KafkaTriggerService.updateKafkaTrigger(a)
	},
	nats: {
		exists: (a) => NatsTriggerService.existsNatsTrigger(a),
		get: (a) => NatsTriggerService.getNatsTrigger(a),
		list: (a) => NatsTriggerService.listNatsTriggers(a),
		create: (a) => NatsTriggerService.createNatsTrigger(a),
		update: (a) => NatsTriggerService.updateNatsTrigger(a)
	},
	postgres: {
		exists: (a) => PostgresTriggerService.existsPostgresTrigger(a),
		get: (a) => PostgresTriggerService.getPostgresTrigger(a),
		list: (a) => PostgresTriggerService.listPostgresTriggers(a),
		create: (a) => PostgresTriggerService.createPostgresTrigger(a),
		update: (a) => PostgresTriggerService.updatePostgresTrigger(a)
	},
	mqtt: {
		exists: (a) => MqttTriggerService.existsMqttTrigger(a),
		get: (a) => MqttTriggerService.getMqttTrigger(a),
		list: (a) => MqttTriggerService.listMqttTriggers(a),
		create: (a) => MqttTriggerService.createMqttTrigger(a),
		update: (a) => MqttTriggerService.updateMqttTrigger(a)
	},
	sqs: {
		exists: (a) => SqsTriggerService.existsSqsTrigger(a),
		get: (a) => SqsTriggerService.getSqsTrigger(a),
		list: (a) => SqsTriggerService.listSqsTriggers(a),
		create: (a) => SqsTriggerService.createSqsTrigger(a),
		update: (a) => SqsTriggerService.updateSqsTrigger(a)
	},
	gcp: {
		exists: (a) => GcpTriggerService.existsGcpTrigger(a),
		get: (a) => GcpTriggerService.getGcpTrigger(a),
		list: (a) => GcpTriggerService.listGcpTriggers(a),
		create: (a) => GcpTriggerService.createGcpTrigger(a),
		update: (a) => GcpTriggerService.updateGcpTrigger(a)
	},
	azure: {
		exists: (a) => AzureTriggerService.existsAzureTrigger(a),
		get: (a) => AzureTriggerService.getAzureTrigger(a),
		list: (a) => AzureTriggerService.listAzureTriggers(a),
		create: (a) => AzureTriggerService.createAzureTrigger(a),
		update: (a) => AzureTriggerService.updateAzureTrigger(a)
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

function getInstructions(subject: InstructionSubject, language?: ScriptLang): string {
	switch (subject) {
		case 'script':
			return getScriptInstructions(language)
		case 'flow':
			return getFlowInstructions()
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
			break
		}
		case 'trigger': {
			const service = triggerServices[triggerKind!]
			const requestBody = draft.value
			if (await service.exists({ workspace, path })) {
				await service.update({ workspace, path, requestBody })
			} else {
				await service.create({ workspace, requestBody })
			}
			break
		}
	}

	globalDraftStore.deleteDraft(type, path, triggerKind)

	toolCallbacks.setToolStatus(toolId, {
		content: `Deployed ${type} "${path}"`,
		result: 'Deployed'
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
