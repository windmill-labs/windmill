import {
	AppService,
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
	AppWithLastVersion,
	CreateResource,
	CreateVariable,
	Flow,
	FlowValue,
	ListableApp,
	ListableResource,
	ListableVariable,
	NewSchedule,
	NewScript,
	Resource,
	Schedule,
	Script,
	ScriptLang
} from '$lib/gen/types.gen'
import { updateRawAppPolicy } from '$lib/components/raw_apps/rawAppPolicy'
import {
	FRAMEWORK_TEMPLATES,
	STARTER_RUNNABLE,
	STARTER_RUNNABLE_KEY,
	type FrameworkKey
} from '$lib/components/raw_apps/templates'
import { DEFAULT_DATA as DEFAULT_RAW_APP_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
import {
	applyEditableFlowJsonToFlow,
	buildEditableFlowJson,
	type EditableFlowJson,
	validateEditableFlowJson
} from '../flow/editableFlowJson'
import { createInlineScriptSession } from '../flow/inlineScriptsUtils'
import { getFlowPrompt, getRawAppPrompt, getResourcePrompt, getScriptPrompt } from '$system_prompts'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import { z } from 'zod'
import {
	createToolDef,
	findAndReplace,
	type CreatedResourceTriggerKind,
	type Tool,
	type ToolCallbacks,
	type ToolDisplayAction
} from '../shared'
import type { ContextElement } from '../context'
import { UserDraft, type UserDraftMeta } from '$lib/userDraft.svelte'
import { emptySchema } from '$lib/utils'
import { inferArgs } from '$lib/infer'
import {
	resourceRequestSchema,
	scheduleRequestSchema,
	triggerRequestSchemas,
	variableRequestSchema
} from '../workspaceToolsZod.gen'
import {
	getWorkspaceItemKey,
	TRIGGER_KINDS,
	type AppDraftValue,
	type FlowDraftValue,
	type ResourceDraftState,
	type TriggerKind,
	type TriggerRequestBody,
	type VariableDraftState,
	type WorkspaceItem,
	type WorkspaceItemType
} from './workspaceItems'
import { buildFlowDeployRequestBody, buildScriptDeployRequestBody } from './deployRequests'
import { bundleRawAppDraft } from './rawAppBundlerBridge'
import {
	clearEphemeralSecretVariableDraftValue,
	deleteGlobalDraft,
	getEphemeralSecretVariableDraftValue,
	getGlobalDraft,
	getGlobalDraftStoragePath,
	listGlobalDrafts,
	saveGlobalAppDraft,
	setEphemeralSecretVariableDraftValue,
	triggerKindToUserDraftKind
} from './userDraftAdapter'

const ITEM_TYPES = [
	'script',
	'flow',
	'schedule',
	'trigger',
	'resource',
	'variable',
	'app'
] as const satisfies readonly WorkspaceItemType[]
const INSTRUCTION_SUBJECTS = [
	'script',
	'flow',
	'resource',
	'app'
] as const satisfies readonly WorkspaceItemType[]
const MAX_LIST_LIMIT = 100
type ActiveGlobalEditorType = Extract<WorkspaceItemType, 'script' | 'flow' | 'app'>
type LiveEditorDraftKind = Parameters<typeof UserDraft.getLiveEditorDraft>[0]

const ACTIVE_GLOBAL_EDITOR_DRAFTS: readonly {
	itemKind: LiveEditorDraftKind
	type: ActiveGlobalEditorType
}[] = [
	{ itemKind: 'script', type: 'script' },
	{ itemKind: 'flow', type: 'flow' },
	{ itemKind: 'raw_app', type: 'app' }
]

export type GlobalActiveEditorContext = {
	type: ActiveGlobalEditorType
	path: string
	isLiveDraft: true
}

export type GlobalUserMessageOptions = {
	workspace?: string
	activeEditor?: GlobalActiveEditorContext
}

const itemTypeSchema = z.enum(ITEM_TYPES)
const instructionSubjectSchema = z.enum(INSTRUCTION_SUBJECTS)
const triggerKindSchema = z.enum(TRIGGER_KINDS)
const scriptLangSchema = z.enum($ScriptLang.enum)

const getInstructionsSchema = z.object({
	subject: instructionSubjectSchema.describe(
		"The workspace item type to get authoring instructions for (script, flow, resource, app). Schedules, triggers, and variables don't need instructions — their tool schemas describe everything."
	),
	language: scriptLangSchema
		.optional()
		.describe(
			'Required when subject is script. Use the existing script language when modifying, or the requested target language when creating.'
		)
})

const askUserQuestionSchema = z.object({
	question: z
		.string()
		.min(1)
		.describe('The concise question to show to the user before continuing.'),
	choices: z
		.array(z.string().min(1).describe('Proposed answer text shown to the user and returned as-is.'))
		.min(2)
		.max(10)
		.describe('Two to ten mutually exclusive proposed answer strings.')
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
	path: z.string().describe('Workspace path of the script, e.g. f/folder/name or u/user/name.'),
	summary: z.string().optional().describe('Short human-readable summary.'),
	language: scriptLangSchema.describe('Script language.'),
	content: z.string().describe('Full script source code.')
})

const readFlowModuleCodeSchema = z.object({
	path: z.string().describe('Workspace path of the flow.'),
	module_id: z
		.string()
		.describe(
			'Module id whose inline rawscript content to read. Must reference a module whose value.type is "rawscript".'
		)
})

const setFlowModuleCodeSchema = z.object({
	path: z.string().describe('Workspace path of the flow.'),
	module_id: z
		.string()
		.describe(
			'Module id whose inline rawscript content to overwrite. Must reference a module whose value.type is "rawscript". Use patch_flow_json for structural changes.'
		),
	code: z.string().describe("New script source. Replaces the module's value.content entirely.")
})

// Flow structure fields are taken as JSON strings rather than typed objects
// because the underlying flow module schema is recursive (modules can contain
// modules), which makes z.toJSONSchema emit $defs/$ref. Gemini's tools API
// rejects those keywords ("Unknown name $ref/$defs"). Same trick as
// set_flow_json in chat/flow/core.ts.
const writeFlowSchema = z.object({
	path: z.string().describe('Workspace path of the flow, e.g. f/folder/name or u/user/name.'),
	summary: z.string().optional().describe('Short human-readable summary.'),
	modules: z.string().describe('JSON string containing the complete flow modules array.'),
	schema: z
		.string()
		.optional()
		.nullable()
		.describe('JSON string containing the flow input schema.'),
	preprocessor_module: z
		.string()
		.optional()
		.nullable()
		.describe('JSON string containing the optional preprocessor module.'),
	failure_module: z
		.string()
		.optional()
		.nullable()
		.describe('JSON string containing the optional failure module.'),
	groups: z
		.string()
		.optional()
		.nullable()
		.describe(
			'JSON string containing the optional array of semantic flow groups. Pass null to clear groups.'
		)
})

function parseOptionalJsonArg(value: unknown, field: string): unknown {
	if (value === undefined || value === null) {
		return value
	}

	try {
		return typeof value === 'string' ? JSON.parse(value) : value
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error)
		throw new Error(`Invalid JSON for ${field}: ${message}`)
	}
}

function editableFlowToDraftValue(editable: EditableFlowJson): FlowDraftValue {
	const value: FlowValue = {
		modules: editable.modules,
		preprocessor_module: editable.preprocessor_module ?? undefined,
		failure_module: editable.failure_module ?? undefined,
		groups: editable.groups ?? undefined
	}
	return {
		value,
		schema: editable.schema,
		groups: editable.groups
	}
}

function flowDraftAsEditableInput(flowDraft: FlowDraftValue): {
	value: FlowValue
	schema?: Record<string, any> | null | undefined
} {
	return {
		value:
			flowDraft.groups === undefined
				? flowDraft.value
				: { ...flowDraft.value, groups: flowDraft.groups ?? undefined },
		schema: flowDraft.schema
	}
}

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

const discardLocalDraftSchema = z.object({
	type: itemTypeSchema,
	path: z.string().describe('Workspace path of the local draft to discard.'),
	trigger_kind: triggerKindSchema
		.optional()
		.describe('Required when type is trigger. Must match the draft trigger kind.')
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

// ============= App tools (raw apps) =============

const backendRunnableSchema = z
	.object({
		name: z.string().describe('Short summary/description of what the runnable does.'),
		type: z
			.enum(['inline', 'script', 'flow', 'hubscript'])
			.describe(
				'Runnable kind: "inline" for custom code stored on the app; "script"/"flow" for a workspace runnable; "hubscript" for a hub script.'
			),
		staticInputs: z
			.record(z.string(), z.any())
			.optional()
			.describe(
				'Static inputs that are not overridable by the frontend caller. Useful for workspace/hub scripts to pre-fill arguments.'
			),
		inlineScript: z
			.object({
				language: z.enum(['bun', 'python3']).describe('Inline script language.'),
				content: z.string().describe('Inline script source. Must export a main function.')
			})
			.optional()
			.describe('Required when type is "inline".'),
		path: z
			.string()
			.optional()
			.describe(
				'Required when type is "script", "flow", or "hubscript". Workspace path of the runnable.'
			)
	})
	.describe(
		'Backend runnable shape: same as in app mode. Inline runnables carry their script content; path runnables reference an existing workspace/hub item.'
	)

const readAppFileSchema = z.object({
	path: z.string().describe('Workspace path of the app, e.g. f/folder/name.'),
	file_path: z
		.string()
		.describe(
			'Frontend file path like /index.tsx, or backend inline runnable path like backend/<key>/main.ts (or main.py).'
		)
})

const writeAppFileSchema = z.object({
	path: z.string().describe('Workspace path of the app.'),
	file_path: z
		.string()
		.describe(
			'Frontend file path (must start with /). Use write_app_runnable to set inline backend script content.'
		),
	content: z.string().describe('Full file content.')
})

const deleteAppFileSchema = z.object({
	path: z.string().describe('Workspace path of the app.'),
	file_path: z
		.string()
		.describe(
			'Frontend file path to remove from the draft. Use delete_app_runnable for backend runnables.'
		)
})

const patchAppFileSchema = z.object({
	path: z.string().describe('Workspace path of the app.'),
	file_path: z
		.string()
		.describe(
			'Frontend file path like /index.tsx, or backend inline runnable path like backend/<key>/main.ts.'
		),
	old_string: z.string().min(1).describe('Exact text to find.'),
	new_string: z.string().describe('Replacement text.'),
	replace_all: z
		.boolean()
		.optional()
		.default(false)
		.describe(
			'When true, replace every exact match. When false, old_string must match exactly once.'
		)
})

const writeAppRunnableSchema = z.object({
	path: z.string().describe('Workspace path of the app.'),
	key: z
		.string()
		.describe(
			'Unique key for the backend runnable (called from frontend as backend.<key>()). Becomes the file id at backend/<key>/main.{ts|py}.'
		),
	runnable: backendRunnableSchema
})

const deleteAppRunnableSchema = z.object({
	path: z.string().describe('Workspace path of the app.'),
	key: z.string().describe('Key of the backend runnable to remove.')
})

const FRAMEWORK_KEYS = [
	'react19',
	'react18',
	'svelte5',
	'vue'
] as const satisfies readonly FrameworkKey[]

const initAppSchema = z.object({
	path: z
		.string()
		.describe(
			'Workspace path for the new app, e.g. f/folder/my_app or u/username/my_app. Errors if an app already exists at this path or a draft is already in flight.'
		),
	summary: z.string().optional().describe('Short human-readable summary of the app.'),
	framework: z
		.enum(FRAMEWORK_KEYS)
		.describe(
			'Frontend framework template. Confirm with the user before calling — never default silently. react19 is recommended for new apps.'
		),
	data: z
		.object({
			datatable: z.string().optional().describe('Default datatable name (e.g. "main").'),
			schema: z.string().optional().describe('Default schema (PostgreSQL schema, optional).'),
			tables: z
				.array(z.string())
				.optional()
				.describe(
					'Initially-whitelisted tables, in the format "<datatable>/<table>" or "<datatable>/<schema>:<table>".'
				)
		})
		.optional()
		.describe('Optional datatable configuration. Omit unless the user asked to wire one up.')
})

const GLOBAL_SYSTEM_PROMPT = `You are Windmill's global workspace assistant.

Use tools to inspect workspace items and create local drafts for scripts, flows, schedules, triggers, resources, variables, and raw apps.

Rules:
- Draft tools create or update local drafts only; they do not deploy or mutate deployed workspace items.
- Use list_workspace_items to find items and read_workspace_item before changing an existing item. For triggers, pass trigger_kind.
- If the user message includes an ACTIVE EDITOR section, treat it as the currently open item and use it for references like "this", "current", or "open editor".
- Use deploy_workspace_item only after the user explicitly asks to deploy. It persists a local draft to the workspace.
- Use discard_local_draft to remove an unsaved local draft, including the matching open editor draft. Use delete_workspace_item only to delete a deployed workspace item.
- Variable values are never readable. For secrets, create a secret variable and reference it from resources as "$var:path/to/variable".
- Use search_resource_types before write_resource.
- Use get_instructions before writing scripts, flows, resources, or apps. For scripts, pass the target language.
- When a required decision is ambiguous, use askUserQuestion with two to ten clear proposed answer strings instead of guessing. The user can also type a custom answer when none of the proposed answers fit.
- Keep context targeted.

Flows:
- read_workspace_item returns compact flow JSON. Inline script bodies appear as "inline_script.<moduleId>".
- Use read_flow_module_code and set_flow_module_code for inline script bodies.
- Use patch_flow_json for structural flow edits and write_flow for full flow rewrites.

Raw apps:
- read_workspace_item returns app metadata only. Use read_app_file for file and inline runnable contents.
- Use write_app_file, patch_app_file, and delete_app_file for frontend files.
- Use write_app_runnable and delete_app_runnable for backend runnables.
- Use init_app only after confirming framework, path, and summary with the user.
- Use deploy_workspace_item after explicit user deploy intent; raw app deploy bundles JS/CSS before saving.`

const DEFAULT_LIST_TYPES = ['script', 'flow'] as const satisfies readonly WorkspaceItemType[]

function getRequestedTypes(types: WorkspaceItemType[] | undefined): WorkspaceItemType[] {
	return types && types.length > 0 ? types : [...DEFAULT_LIST_TYPES]
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
		value: includeValue
			? { value: flow.value, schema: flow.schema, groups: flow.value.groups ?? null }
			: undefined,
		isDraft: false
	}
}

/**
 * Turn a flow workspace item into the compact response we send to the model:
 * rawscript content is replaced with `inline_script.<moduleId>` placeholders.
 * The model retrieves real content via `read_flow_module_code` and edits it via
 * `set_flow_module_code`. `patch_flow_json` operates on this compact view too,
 * so structural edits never have to traverse inline script bodies.
 */
function serializeWorkspaceItemForRead(item: WorkspaceItem): unknown {
	if (item.type === 'variable') {
		return {
			type: 'variable',
			path: item.path,
			summary: item.summary,
			isDraft: item.isDraft
		}
	}

	if (item.type === 'app' && item.value && typeof item.value === 'object' && 'files' in item.value) {
		return {
			type: 'app',
			path: item.path,
			summary: item.summary,
			value: summarizeAppValue(item.value as AppDraftValue),
			isDraft: item.isDraft
		}
	}

	if (item.type !== 'flow' || !item.value) return item
	const flowDraft = item.value as FlowDraftValue
	const session = createInlineScriptSession()
	const editable = buildEditableFlowJson(flowDraftAsEditableInput(flowDraft), session)
	return {
		type: 'flow',
		path: item.path,
		summary: item.summary,
		value: editable,
		isDraft: item.isDraft
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

// ============= App helpers =============

type BackendRunnableInput = z.infer<typeof backendRunnableSchema>

type PersistedRunnable = {
	name: string
	type: 'inline' | 'path' | 'runnableByName' | 'runnableByPath'
	runType?: 'script' | 'flow' | 'hubscript'
	path?: string
	inlineScript?: { language: string; content: string }
	fields?: Record<string, any>
	schema?: any
	[key: string]: any
}

function convertPersistedToBackendRunnable(
	persisted: PersistedRunnable | undefined,
	key: string
): BackendRunnableInput | undefined {
	if (!persisted) return undefined

	const out: BackendRunnableInput = {
		name: persisted.name ?? key,
		type: 'inline'
	}

	if (persisted.type === 'inline' || persisted.type === 'runnableByName') {
		out.type = 'inline'
		if (persisted.inlineScript) {
			let language = persisted.inlineScript.language
			if (language === 'nativets' || language === 'deno') language = 'bun'
			out.inlineScript = {
				language: language as 'bun' | 'python3',
				content: persisted.inlineScript.content ?? ''
			}
		}
	} else if (persisted.type === 'path' || persisted.type === 'runnableByPath') {
		if (persisted.runType === 'flow') out.type = 'flow'
		else if (persisted.runType === 'hubscript') out.type = 'hubscript'
		else out.type = 'script'
		out.path = persisted.path
	}

	if (persisted.fields) {
		const staticInputs: Record<string, any> = {}
		for (const [k, v] of Object.entries(persisted.fields)) {
			if (v && typeof v === 'object' && (v as any).type === 'static') {
				staticInputs[k] = (v as any).value
			}
		}
		if (Object.keys(staticInputs).length > 0) out.staticInputs = staticInputs
	}

	return out
}

function buildPersistedRunnable(
	input: BackendRunnableInput,
	existing: PersistedRunnable | undefined
): PersistedRunnable {
	const fields = input.staticInputs
		? Object.fromEntries(
				Object.entries(input.staticInputs).map(([k, v]) => [
					k,
					{ type: 'static', value: v, fieldType: 'object' }
				])
			)
		: (existing?.fields ?? {})

	if (input.type === 'inline') {
		if (!input.inlineScript) {
			throw new Error('inlineScript is required when runnable type is "inline".')
		}
		return {
			...(existing ?? {}),
			name: input.name,
			type: 'inline',
			inlineScript: {
				content: input.inlineScript.content,
				language: input.inlineScript.language
			},
			fields
		}
	}

	if (!input.path) {
		throw new Error('path is required when runnable type is "script", "flow", or "hubscript".')
	}
	return {
		...(existing ?? {}),
		name: input.name,
		type: 'path',
		runType: input.type,
		path: input.path,
		fields,
		schema: existing?.schema ?? {}
	}
}

type AppFrontendFileMetadata = {
	path: string
	size: number
}

type AppBackendRunnableMetadata = {
	key: string
	name: string
	type: BackendRunnableInput['type']
	path?: string
	language?: 'bun' | 'python3'
	contentSize?: number
	staticInputKeys?: string[]
}

type AppMetadata = {
	frontend: AppFrontendFileMetadata[]
	backend: AppBackendRunnableMetadata[]
	data?: any
}

type LoadedAppDraftValue = {
	value: AppDraftValue
	meta?: UserDraftMeta
}

function summarizeAppValue(value: AppDraftValue): AppMetadata {
	const frontend: AppFrontendFileMetadata[] = Object.entries(value.files).map(
		([path, content]) => ({
			path,
			size: typeof content === 'string' ? content.length : 0
		})
	)
	const backend: AppBackendRunnableMetadata[] = Object.entries(value.runnables).map(
		([key, runnable]) => {
			const converted = convertPersistedToBackendRunnable(runnable as PersistedRunnable, key)
			const staticInputKeys = converted?.staticInputs
				? Object.keys(converted.staticInputs)
				: undefined
			return {
				key,
				name: converted?.name ?? key,
				type: converted?.type ?? 'inline',
				...(converted?.path && { path: converted.path }),
				...(converted?.inlineScript && {
					language: converted.inlineScript.language,
					contentSize: converted.inlineScript.content.length
				}),
				...(staticInputKeys && staticInputKeys.length > 0 && { staticInputKeys })
			}
		}
	)
	return {
		frontend,
		backend,
		...(value.data && { data: value.data })
	}
}

function appToItem(app: ListableApp | AppWithLastVersion, includeValue: boolean): WorkspaceItem {
	return {
		type: 'app',
		path: app.path,
		summary: app.summary,
		value: includeValue ? ((app as AppWithLastVersion).value as AppDraftValue) : undefined,
		isDraft: false
	}
}

const GENERATED_APP_FILE_PATHS = new Set(['/wmill.d.ts'])

function assertNotGeneratedAppFile(filePath: string): void {
	if (GENERATED_APP_FILE_PATHS.has(filePath)) {
		throw new Error(
			`"${filePath}" is generated automatically from backend runnables and cannot be modified directly.`
		)
	}
}

type AppFileTarget =
	| { kind: 'frontend'; filePath: string }
	| { kind: 'backend'; filePath: string; key: string; extension: 'ts' | 'py' }

function resolveAppFileTarget(rawPath: string): AppFileTarget {
	const trimmed = rawPath.trim()
	const backendMatch = trimmed.match(/^backend\/([^/]+)\/main\.(ts|py)$/)
	if (backendMatch) {
		return {
			kind: 'backend',
			filePath: trimmed,
			key: backendMatch[1],
			extension: backendMatch[2] as 'ts' | 'py'
		}
	}
	return {
		kind: 'frontend',
		filePath: trimmed.startsWith('/') ? trimmed : `/${trimmed}`
	}
}

function getInlineScriptExtension(runnable: PersistedRunnable | undefined): 'ts' | 'py' {
	return runnable?.inlineScript?.language === 'python3' ? 'py' : 'ts'
}

/**
 * Resolve a backend file target to its inline script body, validating that the
 * runnable exists, is inline, and matches the requested file extension. Throws
 * with a clear message otherwise.
 */
function getInlineRunnableContent(
	value: AppDraftValue,
	target: { kind: 'backend'; filePath: string; key: string; extension: 'ts' | 'py' },
	appPath: string
): { content: string; runnable: PersistedRunnable } {
	const runnable = value.runnables[target.key] as PersistedRunnable | undefined
	if (!runnable) {
		throw new Error(`Backend runnable "${target.key}" not found in app "${appPath}".`)
	}
	if (runnable.type !== 'inline' && runnable.type !== 'runnableByName') {
		throw new Error(
			`Runnable "${target.key}" is not inline. Use read_workspace_item on the referenced ${runnable.runType ?? 'item'} instead.`
		)
	}
	const expected = getInlineScriptExtension(runnable)
	if (target.extension !== expected) {
		throw new Error(
			`Runnable "${target.key}" language is ${expected}. Use backend/${target.key}/main.${expected}.`
		)
	}
	return { content: runnable.inlineScript?.content ?? '', runnable }
}

function normalizeRawAppData(value: Record<string, any>): AppDraftValue['data'] {
	if (value.data?.creation) {
		return {
			tables: value.data.tables ?? [],
			datatable: value.data.creation.datatable,
			schema: value.data.creation.schema
		}
	}
	if (value.data) {
		return value.data
	}
	if (value.datatables) {
		return { ...DEFAULT_RAW_APP_DATA, tables: value.datatables }
	}
	if (value.dataTableRefs) {
		return { ...DEFAULT_RAW_APP_DATA, tables: value.dataTableRefs }
	}
	return { ...DEFAULT_RAW_APP_DATA }
}

function appSourceToDraftValue(app: any, fallback?: any): AppDraftValue {
	const value = (app.value ?? {}) as Record<string, any>
	return {
		summary: app.summary ?? '',
		files: { ...(value.files ?? {}) },
		runnables: { ...(value.runnables ?? {}) },
		data: normalizeRawAppData(value),
		policy: app.policy ?? fallback?.policy,
		custom_path: app.custom_path ?? fallback?.custom_path
	}
}

function appDraftMeta(app: { versions?: number[]; draft_created_at?: string }): UserDraftMeta {
	return {
		remoteRev: app.versions ? app.versions[app.versions.length - 1] : undefined,
		remoteDraftRev: app.draft_created_at
	}
}

async function loadAppValueForRead(path: string, workspace: string): Promise<AppDraftValue> {
	const draft = getGlobalDraft(workspace, 'app', path)
	if (draft && draft.value && typeof draft.value === 'object' && 'files' in draft.value) {
		return draft.value as AppDraftValue
	}

	const app = await AppService.getAppByPathWithDraft({ workspace, path })
	return appSourceToDraftValue(app.draft ?? app, app)
}

async function loadAppDraftValue(path: string, workspace: string): Promise<LoadedAppDraftValue> {
	const draft = getGlobalDraft(workspace, 'app', path)
	if (draft && draft.value && typeof draft.value === 'object' && 'files' in draft.value) {
		return { value: draft.value as AppDraftValue }
	}

	const app = await AppService.getAppByPathWithDraft({ workspace, path })
	const value = appSourceToDraftValue(app.draft ?? app, app)
	return { value, meta: appDraftMeta(app) }
}

function saveAppDraft(
	workspace: string,
	path: string,
	value: AppDraftValue,
	meta?: UserDraftMeta
): WorkspaceItem {
	return saveGlobalAppDraft(workspace, path, value, meta)
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
	list(args: { workspace: string; pathStart?: string; perPage?: number }): Promise<TriggerLike[]>
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
				(await ResourceService.getResource({ workspace, path })) as ListableResource,
				true
			)
		case 'variable':
			// Never expose the value, even when read directly. Pass decryptSecret=false
			// to avoid materializing secret values server-side.
			return variableToItem(
				await VariableService.getVariable({ workspace, path, decryptSecret: false })
			)
		case 'app': {
			// Returns lightweight metadata only — file/runnable contents come via read_app_file.
			const app = await AppService.getAppByPathWithDraft({ workspace, path })
			const value = appSourceToDraftValue(app.draft ?? app)
			const metadata = summarizeAppValue(value)
			return {
				type: 'app',
				path: app.path,
				summary: value.summary,
				value: metadata as unknown as AppDraftValue,
				isDraft: false
			}
		}
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

	if (types.includes('app')) {
		const apps = await AppService.listApps({
			workspace,
			pathStart: pathPrefix,
			perPage
		})
		for (const app of apps) items.push(appToItem(app, false))
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
- \`write_flow\` mirrors flow mode's \`set_flow_json\`: pass \`path\`, optional \`summary\`, required \`modules\`, and optional \`schema\`, \`preprocessor_module\`, \`failure_module\`, and \`groups\`. The flow-structure arguments are JSON strings, matching the tool schema descriptions.
- \`read_workspace_item\` returns a compact flow \`value\` object with \`modules\`, \`schema\`, \`preprocessor_module\`, \`failure_module\`, and \`groups\`.
- \`modules\` contains normal sequential modules. Use top-level \`preprocessor_module\` and \`failure_module\` for special modules; do not put \`preprocessor\` or \`failure\` in \`modules\`.
- Every module needs a stable unique \`id\` and a useful \`summary\` when the schema supports it.
- Prefer path/script/flow modules when composing existing workspace logic. Use rawscript modules only when new inline code is needed.
- When writing rawscript module code, call \`get_instructions\` with \`subject: "script"\` and the rawscript language first.

## Compact view: how rawscript bodies surface in tool I/O

- \`read_workspace_item\` and \`patch_flow_json\` operate on a **compact view** of the flow: every rawscript module's \`value.content\` is replaced with the placeholder \`"inline_script.<moduleId>"\` so inline script bodies don't bloat tool I/O. Schema, groups, preprocessor_module and failure_module are all shown in this view.
- Inline rawscript content is **not** part of the JSON \`patch_flow_json\` sees. Edits to inline bodies happen via dedicated tools:
  - \`read_flow_module_code(path, module_id)\` — returns the raw inline script content for one module.
  - \`set_flow_module_code(path, module_id, code)\` — overwrites that module's inline script content; saves to the local draft.
- Use \`patch_flow_json\` for *structural* edits: module ids, paths, input_transforms, branch arrangement, summaries, preprocessor/failure swaps, schema/groups. Use \`set_flow_module_code\` for changes inside a specific rawscript body.
- \`write_flow\` is for full overwrites / create-from-scratch. Its \`modules\`, \`preprocessor_module\`, and \`failure_module\` arguments use **non-compact** flow modules (rawscript content is the actual code, not a placeholder).

# Windmill flow authoring reference

${getFlowPrompt()}`
}

type InstructionSubject = (typeof INSTRUCTION_SUBJECTS)[number]

function getAppInstructions(): string {
	return `# Global draft app instructions

- Global mode edits raw app drafts only; it does not save or deploy unless the user explicitly asks to deploy.
- App drafts are addressed by workspace path (e.g. \`f/folder/my_app\`). The first write tool snapshots the workspace app onto the draft, and subsequent writes accumulate.
- To create a new app, use \`init_app\` with a path, optional summary, and a framework (\`react19\` / \`react18\` / \`svelte5\` / \`vue\`). Confirm framework + path + summary with the user before calling — do not silently default to \`react19\` even though it is the recommended choice. \`init_app\` errors if an app already exists at the path or a draft is already in flight; in that case, edit the existing one rather than re-initializing.
- \`init_app\` seeds a starter inline runnable named \`a\` (bun, \`main(x: string) => string\`) so the React/Svelte demo button works on first render. Replace or remove it once you start building real backend runnables.
- Frontend file paths start with \`/\` (e.g. \`/index.tsx\`, \`/App.tsx\`, \`/styles.css\`). Use \`write_app_file\` / \`patch_app_file\` / \`delete_app_file\`.
- Backend inline runnables are addressed as \`backend/<key>/main.{ts|py}\` from the file tools, but you create or update them via \`write_app_runnable\` / \`delete_app_runnable\` (which take the runnable shape directly: \`{ name, type, inlineScript?, path?, staticInputs? }\`).
- \`/wmill.d.ts\` (or \`wmill.ts\`) is generated automatically from the backend runnables — never write it directly.
- Inline runnables only support \`bun\` or \`python3\` in chat. Path runnables (\`script\`/\`flow\`/\`hubscript\`) reference an existing item.
- Use \`deploy_workspace_item\` after explicit user deploy intent. The deploy tool bundles JS/CSS before saving the raw app.
- Use \`read_workspace_item\` with \`type: 'app'\` for a metadata summary (file paths and runnable list, no contents). Use \`read_app_file\` to read an individual file.
- Note: the authoring reference below mentions the CLI on-disk layout (\`backend/<id>.<ext>\`, \`raw_app.yaml\`, \`sql_to_apply/\`). That layout is only relevant for the terminal workflow — in chat, apps are addressed via the tool surface above.

# Windmill raw app authoring reference

${getRawAppPrompt()}`
}

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
		case 'app':
			return getAppInstructions()
	}
}

export const globalTools: Tool<{}>[] = [
	{
		def: createToolDef(
			getInstructionsSchema,
			'get_instructions',
			'Get authoring guidance for scripts, flows, resources, or apps.'
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
			askUserQuestionSchema,
			'askUserQuestion',
			'Ask the user a question with proposed answers and wait for their selected or custom answer before continuing.'
		),
		fn: async ({ args, toolId, toolCallbacks }) => {
			const parsed = askUserQuestionSchema.parse(args)
			const userQuestion = {
				question: parsed.question,
				choices: parsed.choices
			}

			toolCallbacks.setToolStatus(toolId, {
				content: parsed.question,
				userQuestion,
				isLoading: true
			})

			if (!toolCallbacks.requestUserQuestion) {
				const message = 'This chat context cannot ask interactive questions.'
				toolCallbacks.setToolStatus(toolId, {
					content: message,
					userQuestion: { ...userQuestion, canceled: true },
					isLoading: false,
					error: message
				})
				return JSON.stringify({ success: false, error: message })
			}

			const selectedChoice = await toolCallbacks.requestUserQuestion(toolId, userQuestion)
			if (!selectedChoice) {
				const message = 'Question cancelled by user'
				toolCallbacks.setToolStatus(toolId, {
					content: message,
					userQuestion: { ...userQuestion, canceled: true },
					isLoading: false,
					error: message
				})
				return JSON.stringify({ success: false, error: message })
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `User answered question: ${selectedChoice}`,
				userQuestion: {
					...userQuestion,
					selectedChoice
				},
				result: selectedChoice,
				isLoading: false
			})
			return selectedChoice
		}
	},
	{
		def: createToolDef(
			listWorkspaceItemsSchema,
			'list_workspace_items',
			'List workspace items and local drafts. Returns metadata only.'
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

			for (const draft of listGlobalDrafts(workspace)) {
				if (!types.includes(draft.type)) continue
				if (parsed.path_prefix && !draft.path.startsWith(parsed.path_prefix)) continue
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
			'Read one workspace item or local draft.'
		),
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsed = readWorkspaceItemSchema.parse(args)
			if (parsed.type === 'trigger' && !parsed.trigger_kind) {
				const message = 'trigger_kind is required when type is trigger.'
				toolCallbacks.setToolStatus(toolId, { content: message, error: message })
				return JSON.stringify({ success: false, error: message })
			}
			const draft = getGlobalDraft(workspace, parsed.type, parsed.path, parsed.trigger_kind)
			if (draft) {
				toolCallbacks.setToolStatus(toolId, {
					content: `Read local draft ${parsed.type} "${parsed.path}"`
				})
				return JSON.stringify(serializeWorkspaceItemForRead(draft), null, 2)
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Reading ${parsed.type} "${parsed.path}"...`
			})
			const item = await readWorkspaceItem(parsed.type, parsed.path, workspace, parsed.trigger_kind)
			toolCallbacks.setToolStatus(toolId, { content: `Read ${parsed.type} "${parsed.path}"` })
			return JSON.stringify(serializeWorkspaceItemForRead(item), null, 2)
		}
	},
	{
		def: createToolDef(
			writeScriptSchema,
			'write_script',
			'Create or overwrite a local draft script.'
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = writeScriptSchema.parse(ctx.args)
			return writeScriptDraft(parsed, ctx)
		}
	},
	{
		def: createToolDef(writeFlowSchema, 'write_flow', 'Create or overwrite a local draft flow.'),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = writeFlowSchema.parse(ctx.args)
			const editable = validateEditableFlowJson({
				modules: parseOptionalJsonArg(parsed.modules, 'modules'),
				schema: parseOptionalJsonArg(parsed.schema, 'schema'),
				preprocessor_module: parseOptionalJsonArg(
					parsed.preprocessor_module,
					'preprocessor_module'
				),
				failure_module: parseOptionalJsonArg(parsed.failure_module, 'failure_module'),
				groups: parseOptionalJsonArg(parsed.groups, 'groups')
			})
			return writeFlowDraft(
				{
					path: parsed.path,
					summary: parsed.summary,
					flow: editableFlowToDraftValue(editable)
				},
				ctx
			)
		}
	},
	{
		def: createToolDef(
			writeScheduleSchema,
			'write_schedule',
			'Create or overwrite a local draft schedule.',
			{ strict: false }
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = writeScheduleSchema.parse(ctx.args)
			return writeScheduleDraft(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			writeTriggerSchema,
			'write_trigger',
			'Create or overwrite a local draft trigger.',
			{ strict: false }
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = writeTriggerSchema.parse(ctx.args)
			return writeTriggerDraft(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			editScriptSchema,
			'edit_script',
			'Find/replace exact text in a script and save a local draft.'
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
			'Find/replace exact text in compact flow JSON and save a local draft.'
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
			'Deploy a local draft to the workspace. Mutates the workspace.',
			{ strict: false }
		),
		showDetails: true,
		showFade: true,
		requiresConfirmation: true,
		confirmationMessage: 'Deploy local draft to workspace',
		fn: async (ctx) => {
			const parsed = deployWorkspaceItemSchema.parse(ctx.args)
			return deployDraft(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			deleteWorkspaceItemSchema,
			'delete_workspace_item',
			'Delete a deployed workspace item. Mutates the workspace.'
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
			discardLocalDraftSchema,
			'discard_local_draft',
			'Discard a local draft only. Does not mutate deployed workspace items, but clears the matching open editor draft if one is mounted.'
		),
		showDetails: true,
		showFade: true,
		requiresConfirmation: true,
		confirmationMessage: 'Discard local draft',
		fn: async (ctx) => {
			const parsed = discardLocalDraftSchema.parse(ctx.args)
			return discardLocalDraft(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			writeResourceSchema,
			'write_resource',
			'Create or overwrite a local draft resource.',
			{ strict: false }
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = writeResourceSchema.parse(ctx.args)
			return writeResourceDraft(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			writeVariableSchema,
			'write_variable',
			'Create or overwrite a local draft variable.',
			{ strict: false }
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = writeVariableSchema.parse(ctx.args)
			return writeVariableDraft(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			searchResourceTypesSchema,
			'search_resource_types',
			'Search workspace resource types and schemas.'
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
	},
	{
		def: createToolDef(
			readFlowModuleCodeSchema,
			'read_flow_module_code',
			'Read inline script code from one flow module.'
		),
		fn: async (ctx) => {
			const parsed = readFlowModuleCodeSchema.parse(ctx.args)
			return readFlowModuleCode(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			setFlowModuleCodeSchema,
			'set_flow_module_code',
			'Overwrite inline script code in one flow module and save a local draft.'
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = setFlowModuleCodeSchema.parse(ctx.args)
			return setFlowModuleCode(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			initAppSchema,
			'init_app',
			'Initialize a local draft raw app from a framework template.',
			{ strict: false }
		),
		showDetails: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = initAppSchema.parse(ctx.args)
			return initApp(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			readAppFileSchema,
			'read_app_file',
			'Read one raw app frontend file or inline backend runnable.'
		),
		fn: async (ctx) => {
			const parsed = readAppFileSchema.parse(ctx.args)
			return readAppFile(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			writeAppFileSchema,
			'write_app_file',
			'Create or overwrite a frontend file in a local app draft.'
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = writeAppFileSchema.parse(ctx.args)
			return writeAppFile(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			deleteAppFileSchema,
			'delete_app_file',
			'Remove a frontend file from a local app draft.'
		),
		fn: async (ctx) => {
			const parsed = deleteAppFileSchema.parse(ctx.args)
			return deleteAppFile(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			patchAppFileSchema,
			'patch_app_file',
			'Find/replace exact text in a raw app file and save a local draft.'
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = patchAppFileSchema.parse(ctx.args)
			return patchAppFile(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			writeAppRunnableSchema,
			'write_app_runnable',
			'Create or overwrite a backend runnable in a local app draft.',
			{ strict: false }
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = writeAppRunnableSchema.parse(ctx.args)
			return writeAppRunnable(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			deleteAppRunnableSchema,
			'delete_app_runnable',
			'Remove a backend runnable from a local app draft.'
		),
		fn: async (ctx) => {
			const parsed = deleteAppRunnableSchema.parse(ctx.args)
			return deleteAppRunnable(parsed, ctx)
		}
	}
]

type WriteDraftCtx = {
	workspace: string
	toolId: string
	toolCallbacks: ToolCallbacks
}

type DraftConfig = Record<string, any>
type ScheduleDraftConfig = NewSchedule & DraftConfig
type TriggerDraftConfig = TriggerRequestBody & DraftConfig & { path: string }

function stripBackendMetadata<T extends DraftConfig>(value: T): T {
	const draft = structuredClone(value)
	delete draft.workspace_id
	delete draft.edited_by
	delete draft.edited_at
	delete draft.email
	delete draft.error
	return draft
}

function mergeDraftConfig<T extends DraftConfig>(
	base: T | undefined,
	overrides: DraftConfig,
	path: string
): T {
	return {
		...(base ? stripBackendMetadata(base) : {}),
		...structuredClone(overrides),
		path
	} as unknown as T
}

function resourceToDraftState(resource: Resource): ResourceDraftState {
	return {
		path: resource.path,
		description: resource.description ?? '',
		args: structuredClone((resource.value ?? {}) as Record<string, any>),
		labels: resource.labels ?? undefined,
		wsSpecific: resource.ws_specific ?? false,
		resource_type: resource.resource_type
	}
}

function createResourceToDraftState(
	args: CreateResource,
	base?: ResourceDraftState
): ResourceDraftState {
	return {
		...base,
		path: args.path,
		description: args.description ?? base?.description ?? '',
		args: structuredClone((args.value ?? base?.args ?? {}) as Record<string, any>),
		labels: args.labels ?? base?.labels,
		wsSpecific: args.ws_specific ?? base?.wsSpecific ?? false,
		resource_type: args.resource_type ?? base?.resource_type
	}
}

function variableToDraftState(variable: ListableVariable): VariableDraftState {
	return {
		path: variable.path,
		variable: {
			value: variable.value ?? '',
			is_secret: variable.is_secret,
			description: variable.description ?? ''
		},
		labels: variable.labels ?? undefined,
		wsSpecific: variable.ws_specific ?? false,
		account: variable.account,
		is_oauth: variable.is_oauth,
		expires_at: variable.expires_at
	}
}

function createVariableToDraftState(
	args: CreateVariable,
	base?: VariableDraftState
): VariableDraftState {
	return {
		...base,
		path: args.path,
		variable: {
			value: args.is_secret ? '' : args.value,
			is_secret: args.is_secret,
			description: args.description
		},
		labels: args.labels ?? base?.labels,
		wsSpecific: args.ws_specific ?? base?.wsSpecific ?? false,
		account: args.account ?? base?.account,
		is_oauth: args.is_oauth ?? base?.is_oauth,
		expires_at: args.expires_at ?? base?.expires_at
	}
}

function syncEphemeralSecretVariableDraftValue(workspace: string, args: CreateVariable): void {
	const storagePath = getGlobalDraftStoragePath(workspace, 'variable', args.path)
	if (args.is_secret) {
		setEphemeralSecretVariableDraftValue(workspace, storagePath, args.value)
	} else {
		clearEphemeralSecretVariableDraftValue(workspace, storagePath)
	}
}

function buildVariableDeployRequestBody(
	workspace: string,
	path: string,
	draftValue: CreateVariable
): CreateVariable {
	const requestBody = structuredClone(draftValue)
	if (!requestBody.is_secret) return requestBody

	const storagePath = getGlobalDraftStoragePath(workspace, 'variable', path)
	const secretValue = getEphemeralSecretVariableDraftValue(workspace, storagePath)
	if (secretValue === undefined) {
		throw new Error(
			`Secret value for local draft variable "${path}" is no longer available because secret draft values are kept only in memory. Run write_variable again before deploying this secret.`
		)
	}

	return { ...requestBody, value: secretValue }
}

function startDraftWrite(ctx: WriteDraftCtx, type: WorkspaceItemType, path: string): void {
	ctx.toolCallbacks.setToolStatus(ctx.toolId, {
		content: `Writing draft ${type} "${path}"...`
	})
}

function getRequiredGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): WorkspaceItem {
	const draft = getGlobalDraft(workspace, type, path, triggerKind)
	if (!draft) {
		throw new Error(`Could not read written draft ${type} "${path}".`)
	}
	return draft
}

function finishDraftWrite(stored: WorkspaceItem, existed: boolean, ctx: WriteDraftCtx): string {
	const verb = existed ? 'Updated' : 'Created'
	const serializedItem =
		stored.type === 'variable' || stored.type === 'flow'
			? serializeWorkspaceItemForRead(stored)
			: stored

	ctx.toolCallbacks.setToolStatus(ctx.toolId, {
		content: `${verb} local draft ${stored.type} "${stored.path}"`,
		result: `Draft ${verb.toLowerCase()}`
	})
	return JSON.stringify(
		{
			success: true,
			message: `${verb} local draft ${stored.type} "${stored.path}". The workspace was not saved or deployed.`,
			item: serializedItem
		},
		null,
		2
	)
}

async function writeScriptDraft(
	args: { path: string; summary?: string; language: ScriptLang; content: string },
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace } = ctx
	startDraftWrite(ctx, 'script', args.path)
	const storagePath = getGlobalDraftStoragePath(workspace, 'script', args.path)

	const existingDraft = UserDraft.get<NewScript>('script', storagePath, { workspace })
	const backendExists = existingDraft
		? false
		: await ScriptService.existsScriptByPath({ workspace, path: args.path })

	if (existingDraft) {
		const draft: NewScript = {
			...structuredClone(existingDraft),
			path: args.path,
			summary: args.summary ?? existingDraft.summary,
			content: args.content,
			language: args.language
		}
		UserDraft.save('script', storagePath, draft, { workspace })
	} else if (backendExists) {
		const existing = await ScriptService.getScriptByPathWithDraft({
			workspace,
			path: args.path
		})
		const base = (existing.draft ?? existing) as NewScript
		const draft: NewScript = {
			...structuredClone(base),
			parent_hash: existing.hash,
			path: args.path,
			summary: args.summary ?? base.summary,
			content: args.content,
			language: args.language
		}
		UserDraft.setDraftAndMeta(
			'script',
			storagePath,
			draft,
			{ remoteRev: existing.hash, remoteDraftRev: existing.draft_created_at },
			{ workspace }
		)
	} else {
		const draft: NewScript = {
			path: args.path,
			summary: args.summary ?? '',
			description: '',
			content: args.content,
			schema: emptySchema(),
			is_template: false,
			language: args.language,
			kind: 'script'
		}
		UserDraft.save('script', storagePath, draft, { workspace })
	}

	return finishDraftWrite(
		getRequiredGlobalDraft(workspace, 'script', args.path),
		existingDraft !== undefined || backendExists,
		ctx
	)
}

async function writeFlowDraft(
	args: { path: string; summary?: string; flow: FlowDraftValue },
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace } = ctx
	startDraftWrite(ctx, 'flow', args.path)
	const storagePath = getGlobalDraftStoragePath(workspace, 'flow', args.path)

	const draftValue = args.flow
	const value = structuredClone(draftValue.value)
	if (draftValue.groups !== undefined && draftValue.groups !== null) {
		value.groups = structuredClone(draftValue.groups)
	}

	const existingDraft = UserDraft.get<Flow>('flow', storagePath, { workspace })
	const backendExists = existingDraft
		? false
		: await FlowService.existsFlowByPath({ workspace, path: args.path })

	if (existingDraft) {
		const draft: Flow = {
			...structuredClone(existingDraft),
			path: args.path,
			summary: args.summary ?? existingDraft.summary,
			value,
			schema: draftValue.schema ?? existingDraft.schema
		}
		UserDraft.save('flow', storagePath, draft, { workspace })
	} else if (backendExists) {
		const [existing, latestVersion] = await Promise.all([
			FlowService.getFlowByPathWithDraft({ workspace, path: args.path }),
			FlowService.getFlowLatestVersion({ workspace, path: args.path })
		])
		const base = (existing.draft ?? existing) as Flow
		const draft: Flow = {
			...structuredClone(base),
			path: args.path,
			summary: args.summary ?? base.summary,
			value,
			schema: draftValue.schema ?? base.schema
		}
		UserDraft.setDraftAndMeta(
			'flow',
			storagePath,
			draft,
			{ remoteRev: latestVersion.id, remoteDraftRev: existing.draft_created_at },
			{ workspace }
		)
	} else {
		const draft: Flow = {
			path: args.path,
			summary: args.summary ?? '',
			value,
			schema: draftValue.schema ?? emptySchema(),
			edited_by: '',
			edited_at: '',
			archived: false,
			extra_perms: {}
		}
		UserDraft.save('flow', storagePath, draft, { workspace })
	}

	return finishDraftWrite(
		getRequiredGlobalDraft(workspace, 'flow', args.path),
		existingDraft !== undefined || backendExists,
		ctx
	)
}

async function writeScheduleDraft(args: NewSchedule, ctx: WriteDraftCtx): Promise<string> {
	const { workspace } = ctx
	startDraftWrite(ctx, 'schedule', args.path)

	const existingDraft = UserDraft.get<ScheduleDraftConfig>('trigger_schedule', args.path, {
		workspace
	})
	const backendExists = existingDraft
		? false
		: await ScheduleService.existsSchedule({ workspace, path: args.path })

	const base = existingDraft
		? existingDraft
		: backendExists
			? ((await ScheduleService.getSchedule({
					workspace,
					path: args.path
				})) as ScheduleDraftConfig)
			: undefined
	const draft = mergeDraftConfig<ScheduleDraftConfig>(base, args as DraftConfig, args.path)

	UserDraft.save('trigger_schedule', args.path, draft, { workspace })

	return finishDraftWrite(
		getRequiredGlobalDraft(workspace, 'schedule', args.path),
		existingDraft !== undefined || backendExists,
		ctx
	)
}

async function writeTriggerDraft(
	args: { kind: TriggerKind; config: unknown },
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace } = ctx
	const config = args.config as TriggerDraftConfig
	const path = config.path
	const itemKind = triggerKindToUserDraftKind(args.kind)
	startDraftWrite(ctx, 'trigger', path)

	const existingDraft = UserDraft.get<TriggerDraftConfig>(itemKind, path, { workspace })
	const backendExists = existingDraft
		? false
		: await triggerServices[args.kind].exists({ workspace, path })

	const base = existingDraft
		? existingDraft
		: backendExists
			? ((await triggerServices[args.kind].get({ workspace, path })) as TriggerDraftConfig)
			: undefined
	const draft = mergeDraftConfig<TriggerDraftConfig>(base, config, path)

	UserDraft.save(itemKind, path, draft, { workspace })

	return finishDraftWrite(
		getRequiredGlobalDraft(workspace, 'trigger', path, args.kind),
		existingDraft !== undefined || backendExists,
		ctx
	)
}

async function writeResourceDraft(args: CreateResource, ctx: WriteDraftCtx): Promise<string> {
	const { workspace } = ctx
	startDraftWrite(ctx, 'resource', args.path)

	const existingDraft = UserDraft.get<ResourceDraftState>('resource', args.path, { workspace })
	const backendExists = existingDraft
		? false
		: await ResourceService.existsResource({ workspace, path: args.path })

	if (existingDraft) {
		UserDraft.save('resource', args.path, createResourceToDraftState(args, existingDraft), {
			workspace
		})
	} else if (backendExists) {
		const existing = await ResourceService.getResource({ workspace, path: args.path })
		UserDraft.setDraftAndMeta(
			'resource',
			args.path,
			createResourceToDraftState(args, resourceToDraftState(existing)),
			{ remoteRev: existing.edited_at },
			{ workspace }
		)
	} else {
		UserDraft.save('resource', args.path, createResourceToDraftState(args), { workspace })
	}

	return finishDraftWrite(
		getRequiredGlobalDraft(workspace, 'resource', args.path),
		existingDraft !== undefined || backendExists,
		ctx
	)
}

async function writeVariableDraft(args: CreateVariable, ctx: WriteDraftCtx): Promise<string> {
	const { workspace } = ctx
	startDraftWrite(ctx, 'variable', args.path)

	const existingDraft = UserDraft.get<VariableDraftState>('variable', args.path, { workspace })
	const backendExists = existingDraft
		? false
		: await VariableService.existsVariable({ workspace, path: args.path })

	if (existingDraft) {
		UserDraft.save('variable', args.path, createVariableToDraftState(args, existingDraft), {
			workspace
		})
	} else if (backendExists) {
		const existing = await VariableService.getVariable({
			workspace,
			path: args.path,
			decryptSecret: false
		})
		UserDraft.setDraftAndMeta(
			'variable',
			args.path,
			createVariableToDraftState(args, variableToDraftState(existing)),
			{ remoteRev: existing.edited_at },
			{ workspace }
		)
	} else {
		UserDraft.save('variable', args.path, createVariableToDraftState(args), { workspace })
	}
	syncEphemeralSecretVariableDraftValue(workspace, args)

	return finishDraftWrite(
		getRequiredGlobalDraft(workspace, 'variable', args.path),
		existingDraft !== undefined || backendExists,
		ctx
	)
}

async function loadScriptForEdit(
	path: string,
	workspace: string
): Promise<{ content: string; language: ScriptLang; summary?: string }> {
	const draft = getGlobalDraft(workspace, 'script', path)
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
	const updated = findAndReplace(base.content, oldString, newString, replaceAll, 'script source')
	return writeScriptDraft(
		{
			path,
			summary: base.summary,
			language: base.language,
			content: updated
		},
		ctx
	)
}

async function loadFlowDraftValue(
	path: string,
	workspace: string
): Promise<{ flow: FlowDraftValue; summary?: string }> {
	const draft = getGlobalDraft(workspace, 'flow', path)
	if (draft) {
		if (draft.value === undefined || typeof draft.value === 'string') {
			throw new Error(`Draft flow "${path}" has no value.`)
		}
		return { flow: draft.value as FlowDraftValue, summary: draft.summary }
	}
	const flow = await FlowService.getFlowByPath({ workspace, path })
	return {
		flow: { value: flow.value, schema: flow.schema, groups: flow.value.groups ?? null },
		summary: flow.summary
	}
}

async function patchFlowJson(
	args: { path: string; old_string: string; new_string: string; replace_all: boolean },
	ctx: WriteDraftCtx
): Promise<string> {
	const { path, old_string: oldString, new_string: newString, replace_all: replaceAll } = args
	ctx.toolCallbacks.setToolStatus(ctx.toolId, { content: `Patching flow "${path}"...` })

	// Operate on the compact (placeholders-for-rawscript) view so inline script
	// bodies don't appear in the JSON the model sees or patches. Real script
	// content is preserved through the patch via the InlineScriptSession; the
	// model uses set_flow_module_code to change inline script bodies.
	const base = await loadFlowDraftValue(path, ctx.workspace)
	const session = createInlineScriptSession()
	const editable = buildEditableFlowJson(flowDraftAsEditableInput(base.flow), session)
	const currentJson = JSON.stringify(editable)
	const updatedJson = findAndReplace(
		currentJson,
		oldString,
		newString,
		replaceAll,
		'compact flow JSON'
	)
	let parsedValue: unknown
	try {
		parsedValue = JSON.parse(updatedJson)
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error)
		throw new Error(`Invalid JSON after replacement: ${message}`)
	}

	const patchedEditable = validateEditableFlowJson(parsedValue)
	const newFlowValue = applyEditableFlowJsonToFlow(base.flow.value, patchedEditable, session)

	return writeFlowDraft(
		{
			path,
			summary: base.summary,
			flow: {
				...base.flow,
				value: newFlowValue,
				schema: patchedEditable.schema,
				groups: patchedEditable.groups
			}
		},
		ctx
	)
}

async function readFlowModuleCode(
	args: { path: string; module_id: string },
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	toolCallbacks.setToolStatus(toolId, {
		content: `Reading inline script for module "${args.module_id}" from flow "${args.path}"...`
	})
	const base = await loadFlowDraftValue(args.path, workspace)
	const session = createInlineScriptSession()
	buildEditableFlowJson(flowDraftAsEditableInput(base.flow), session)
	const content = session.get(args.module_id)
	if (content === undefined) {
		throw new Error(
			`Module "${args.module_id}" is not an inline rawscript in flow "${args.path}". (Path runnables, branches, and loops have no inline script content; use patch_flow_json to inspect them.)`
		)
	}
	toolCallbacks.setToolStatus(toolId, {
		content: `Read inline script for "${args.module_id}"`
	})
	return content
}

async function setFlowModuleCode(
	args: { path: string; module_id: string; code: string },
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	toolCallbacks.setToolStatus(toolId, {
		content: `Updating inline script for module "${args.module_id}" in flow "${args.path}"...`
	})
	const base = await loadFlowDraftValue(args.path, workspace)
	const session = createInlineScriptSession()
	const editable = buildEditableFlowJson(flowDraftAsEditableInput(base.flow), session)
	if (!session.has(args.module_id)) {
		throw new Error(
			`Module "${args.module_id}" is not an inline rawscript in flow "${args.path}". Use patch_flow_json or write_flow for structural changes.`
		)
	}
	session.set(args.module_id, args.code)
	const newFlowValue = applyEditableFlowJsonToFlow(base.flow.value, editable, session)
	return writeFlowDraft(
		{
			path: args.path,
			summary: base.summary,
			flow: { ...base.flow, value: newFlowValue }
		},
		ctx
	)
}

async function initApp(
	args: {
		path: string
		summary?: string
		framework: FrameworkKey
		data?: { datatable?: string; schema?: string; tables?: string[] }
	},
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const { path, summary, framework, data } = args

	if (getGlobalDraft(workspace, 'app', path)) {
		throw new Error(
			`A local draft for app "${path}" already exists. Use write_app_file / write_app_runnable to modify it, or delete the existing draft first.`
		)
	}
	if (await AppService.existsApp({ workspace, path })) {
		throw new Error(
			`An app already exists at "${path}". Use read_workspace_item + write_app_file / write_app_runnable to modify it.`
		)
	}

	toolCallbacks.setToolStatus(toolId, {
		content: `Initializing app draft "${path}" with ${framework} template...`
	})

	const template = FRAMEWORK_TEMPLATES[framework]
	const value: AppDraftValue = {
		summary,
		files: { ...template },
		runnables: { [STARTER_RUNNABLE_KEY]: { ...STARTER_RUNNABLE } },
		...(data && {
			data: {
				tables: data.tables ?? [],
				datatable: data.datatable,
				schema: data.schema
			}
		})
	}
	await recomputeAppPolicy(value)
	const stored = saveAppDraft(workspace, path, value)

	toolCallbacks.setToolStatus(toolId, {
		content: `Initialized app draft "${path}" (${framework})`,
		result: 'Draft initialized'
	})
	return JSON.stringify(
		{
			success: true,
			message: `Initialized local draft app "${path}" from the ${framework} template with a starter runnable "${STARTER_RUNNABLE_KEY}". Use write_app_file / write_app_runnable to evolve the draft.`,
			item: stored
		},
		null,
		2
	)
}

async function readAppFile(
	args: { path: string; file_path: string },
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const target = resolveAppFileTarget(args.file_path)
	toolCallbacks.setToolStatus(toolId, {
		content: `Reading ${target.filePath} from app "${args.path}"...`
	})

	const value = await loadAppValueForRead(args.path, workspace)

	if (target.kind === 'frontend') {
		const content = value.files[target.filePath]
		if (content === undefined) {
			throw new Error(`Frontend file "${target.filePath}" not found in app "${args.path}".`)
		}
		toolCallbacks.setToolStatus(toolId, { content: `Read ${target.filePath}` })
		return content
	}

	const { content } = getInlineRunnableContent(value, target, args.path)
	toolCallbacks.setToolStatus(toolId, { content: `Read ${target.filePath}` })
	return content
}

async function writeAppFile(
	args: { path: string; file_path: string; content: string },
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const target = resolveAppFileTarget(args.file_path)
	if (target.kind !== 'frontend') {
		throw new Error(
			`write_app_file only writes frontend files. Use write_app_runnable to set inline backend script content.`
		)
	}
	assertNotGeneratedAppFile(target.filePath)

	toolCallbacks.setToolStatus(toolId, {
		content: `Writing ${target.filePath} to app "${args.path}"...`
	})

	const { value, meta } = await loadAppDraftValue(args.path, workspace)
	value.files = { ...value.files, [target.filePath]: args.content }
	const stored = saveAppDraft(workspace, args.path, value, meta)

	toolCallbacks.setToolStatus(toolId, {
		content: `Updated ${target.filePath} in app "${args.path}"`,
		result: 'Draft updated'
	})
	return JSON.stringify(
		{
			success: true,
			message: `Updated local draft app "${args.path}" with frontend file "${target.filePath}".`,
			item: stored
		},
		null,
		2
	)
}

async function deleteAppFile(
	args: { path: string; file_path: string },
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const target = resolveAppFileTarget(args.file_path)
	if (target.kind !== 'frontend') {
		throw new Error(
			`delete_app_file only deletes frontend files. Use delete_app_runnable for backend runnables.`
		)
	}
	assertNotGeneratedAppFile(target.filePath)

	toolCallbacks.setToolStatus(toolId, {
		content: `Deleting ${target.filePath} from app "${args.path}"...`
	})

	const { value, meta } = await loadAppDraftValue(args.path, workspace)
	if (!(target.filePath in value.files)) {
		throw new Error(`Frontend file "${target.filePath}" not found in app "${args.path}".`)
	}
	const { [target.filePath]: _removed, ...remaining } = value.files
	value.files = remaining
	const stored = saveAppDraft(workspace, args.path, value, meta)

	toolCallbacks.setToolStatus(toolId, {
		content: `Removed ${target.filePath} from app "${args.path}"`,
		result: 'Draft updated'
	})
	return JSON.stringify(
		{
			success: true,
			message: `Removed "${target.filePath}" from local draft app "${args.path}".`,
			item: stored
		},
		null,
		2
	)
}

async function patchAppFile(
	args: {
		path: string
		file_path: string
		old_string: string
		new_string: string
		replace_all: boolean
	},
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const {
		path,
		file_path: filePath,
		old_string: oldString,
		new_string: newString,
		replace_all: replaceAll
	} = args
	const target = resolveAppFileTarget(filePath)
	if (target.kind === 'frontend') {
		assertNotGeneratedAppFile(target.filePath)
	}

	toolCallbacks.setToolStatus(toolId, {
		content: `Patching ${target.filePath} in app "${path}"...`
	})

	const { value, meta } = await loadAppDraftValue(path, workspace)
	let currentContent: string
	let runnable: PersistedRunnable | undefined

	if (target.kind === 'frontend') {
		const existing = value.files[target.filePath]
		if (existing === undefined) {
			throw new Error(`Frontend file "${target.filePath}" not found in app "${path}".`)
		}
		currentContent = existing
	} else {
		const resolved = getInlineRunnableContent(value, target, path)
		currentContent = resolved.content
		runnable = resolved.runnable
	}

	const updated = findAndReplace(
		currentContent,
		oldString,
		newString,
		replaceAll,
		`${target.filePath} content`
	)

	if (target.kind === 'frontend') {
		value.files = { ...value.files, [target.filePath]: updated }
	} else {
		value.runnables = {
			...value.runnables,
			[target.key]: {
				...runnable!,
				inlineScript: {
					language:
						runnable!.inlineScript?.language ?? (target.extension === 'py' ? 'python3' : 'bun'),
					content: updated
				}
			}
		}
	}

	const stored = saveAppDraft(workspace, path, value, meta)
	toolCallbacks.setToolStatus(toolId, {
		content: `Patched ${target.filePath} in app "${path}"`,
		result: 'Draft updated'
	})
	return JSON.stringify(
		{
			success: true,
			message: `Patched "${target.filePath}" in local draft app "${path}".`,
			item: stored
		},
		null,
		2
	)
}

async function recomputeAppPolicy(value: AppDraftValue): Promise<void> {
	const policy = (await updateRawAppPolicy(value.runnables as any, value.policy as any)) as NonNullable<
		AppDraftValue['policy']
	>
	if (!policy.execution_mode) {
		policy.execution_mode = 'publisher'
	}
	value.policy = policy
}

async function writeAppRunnable(
	args: { path: string; key: string; runnable: BackendRunnableInput },
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const { path, key, runnable: input } = args
	toolCallbacks.setToolStatus(toolId, {
		content: `Writing runnable "${key}" to app "${path}"...`
	})

	const { value, meta } = await loadAppDraftValue(path, workspace)
	const existing = value.runnables[key] as PersistedRunnable | undefined
	const persisted = buildPersistedRunnable(input, existing)
	value.runnables = { ...value.runnables, [key]: persisted }
	await recomputeAppPolicy(value)
	const stored = saveAppDraft(workspace, path, value, meta)

	toolCallbacks.setToolStatus(toolId, {
		content: `Updated runnable "${key}" in app "${path}"`,
		result: 'Draft updated'
	})
	return JSON.stringify(
		{
			success: true,
			message: `Updated local draft app "${path}" with runnable "${key}".`,
			item: stored
		},
		null,
		2
	)
}

async function deleteAppRunnable(
	args: { path: string; key: string },
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const { path, key } = args
	toolCallbacks.setToolStatus(toolId, {
		content: `Removing runnable "${key}" from app "${path}"...`
	})

	const { value, meta } = await loadAppDraftValue(path, workspace)
	if (!(key in value.runnables)) {
		throw new Error(`Backend runnable "${key}" not found in app "${path}".`)
	}
	const { [key]: _removed, ...remaining } = value.runnables
	value.runnables = remaining
	await recomputeAppPolicy(value)
	const stored = saveAppDraft(workspace, path, value, meta)

	toolCallbacks.setToolStatus(toolId, {
		content: `Removed runnable "${key}" from app "${path}"`,
		result: 'Draft updated'
	})
	return JSON.stringify(
		{
			success: true,
			message: `Removed runnable "${key}" from local draft app "${path}".`,
			item: stored
		},
		null,
		2
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

function createOpenScheduleAction(path: string, targetKind: 'script' | 'flow'): ToolDisplayAction {
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

function createOpenResourceAction(path: string): ToolDisplayAction {
	return {
		id: `open-deployed-resource:${path}`,
		type: 'open_created_resource',
		label: 'Open resource',
		resource: 'resource',
		path
	}
}

function createOpenVariableAction(path: string): ToolDisplayAction {
	return {
		id: `open-deployed-variable:${path}`,
		type: 'open_created_resource',
		label: 'Open variable',
		resource: 'variable',
		path
	}
}

async function discardLocalDraft(
	args: { type: WorkspaceItemType; path: string; trigger_kind?: TriggerKind },
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const { type, path, trigger_kind: triggerKind } = args

	if (type === 'trigger' && !triggerKind) {
		throw new Error('trigger_kind is required when discarding a trigger draft.')
	}

	const draft = getGlobalDraft(workspace, type, path, triggerKind)
	if (!draft) {
		throw new Error(`No local draft found for ${type} "${path}".`)
	}

	deleteGlobalDraft(workspace, type, path, triggerKind)

	toolCallbacks.setToolStatus(toolId, {
		content: `Discarded local draft ${type} "${path}"`,
		result: 'Draft discarded'
	})
	return JSON.stringify(
		{
			success: true,
			message: `Discarded local draft ${type} "${path}". The deployed workspace item was not changed.`,
			type,
			path,
			triggerKind
		},
		null,
		2
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

	const draft = getGlobalDraft(workspace, type, path, triggerKind)
	if (!draft) {
		throw new Error(`No local draft found for ${type} "${path}".`)
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
			const existing = (await ScriptService.existsScriptByPath({ workspace, path }))
				? await ScriptService.getScriptByPath({ workspace, path })
				: undefined
			const requestBody = buildScriptDeployRequestBody(path, draft, existing, deploymentMessage)
			// Infer the arg schema from the content so it matches the code, like the editor does.
			try {
				const schema = emptySchema()
				await inferArgs(requestBody.language, requestBody.content, schema)
				requestBody.schema = schema
			} catch (e) {
				console.error('Failed to infer script schema before deploy', e)
			}
			await ScriptService.createScript({ workspace, requestBody })
			break
		}
		case 'flow': {
			const flowDraft = draft.value as FlowDraftValue
			const existing = (await FlowService.existsFlowByPath({ workspace, path }))
				? await FlowService.getFlowByPath({ workspace, path })
				: undefined
			const requestBody = buildFlowDeployRequestBody(
				path,
				draft.summary,
				flowDraft,
				existing,
				deploymentMessage
			)
			if (existing) {
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
			actions = [createOpenResourceAction(path)]
			break
		}
		case 'variable': {
			const requestBody = buildVariableDeployRequestBody(
				workspace,
				path,
				draft.value as CreateVariable
			)
			if (await VariableService.existsVariable({ workspace, path })) {
				await VariableService.updateVariable({ workspace, path, requestBody })
			} else {
				await VariableService.createVariable({ workspace, requestBody })
			}
			actions = [createOpenVariableAction(path)]
			break
		}
		case 'app': {
			const appDraft = draft.value as AppDraftValue
			const appValue: AppDraftValue = {
				...appDraft,
				files: { ...(appDraft.files ?? {}) },
				runnables: { ...(appDraft.runnables ?? {}) },
				data: appDraft.data ?? { ...DEFAULT_RAW_APP_DATA }
			}
			await recomputeAppPolicy(appValue)
			const policy = appValue.policy
			if (!policy) {
				throw new Error(`Draft app "${path}" has no policy to deploy.`)
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Bundling app "${path}"...`
			})
			const bundle = await bundleRawAppDraft({
				workspace,
				files: appValue.files,
				onLog: (delta) => {
					const lines = delta
						.split('\n')
						.map((line) => line.trim())
						.filter(Boolean)
					const latest = lines[lines.length - 1]
					if (latest) {
						toolCallbacks.setToolStatus(toolId, {
							content: `Bundling app "${path}"... ${latest}`
						})
					}
				}
			})

			toolCallbacks.setToolStatus(toolId, {
				content: `Deploying app "${path}"...`
			})
			const rawAppValue = {
				files: appValue.files,
				runnables: appValue.runnables,
				data: appValue.data ?? { ...DEFAULT_RAW_APP_DATA }
			}
			const summary = appValue.summary ?? draft.summary ?? ''
			if (await AppService.existsApp({ workspace, path })) {
				// Omit custom_path on update for now. The backend preserves it when absent, while
				// sending it requires admin privileges; this chat deploy path does not yet mirror
				// the raw app editor's user/admin-specific custom_path handling.
				await AppService.updateAppRaw({
					workspace,
					path,
					formData: {
						app: {
							path,
							value: rawAppValue,
							summary,
							policy,
							deployment_message: deploymentMessage
						},
						js: bundle.js,
						css: bundle.css
					}
				})
			} else {
				await AppService.createAppRaw({
					workspace,
					formData: {
						app: {
							path,
							value: rawAppValue,
							summary,
							policy,
							deployment_message: deploymentMessage,
							custom_path: appValue.custom_path
						},
						js: bundle.js,
						css: bundle.css
					}
				})
			}
			break
		}
	}

	deleteGlobalDraft(workspace, type, path, triggerKind, { preserveLiveDraft: true })

	toolCallbacks.setToolStatus(toolId, {
		content: `Deployed ${type} "${path}"`,
		result: 'Deployed',
		actions
	})
	return JSON.stringify(
		{
			success: true,
			message: `Deployed local draft ${type} "${path}" to the workspace. Draft removed from the local draft system.`,
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
		case 'app':
			await AppService.deleteApp({ workspace, path })
			break
	}

	deleteGlobalDraft(workspace, type, path, triggerKind)

	toolCallbacks.setToolStatus(toolId, {
		content: `Deleted ${type} "${path}"`,
		result: 'Deleted'
	})
	return JSON.stringify(
		{
			success: true,
			message: `Deleted ${type} "${path}" from the workspace. Any matching local draft was also cleared.`,
			type,
			path,
			triggerKind
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

export function getActiveGlobalEditorContext(
	workspace: string
): GlobalActiveEditorContext | undefined {
	for (const { itemKind, type } of ACTIVE_GLOBAL_EDITOR_DRAFTS) {
		const liveDraft = UserDraft.getLiveEditorDraft(itemKind, { workspace })
		const path = liveDraft?.effectivePath || liveDraft?.storagePath
		if (!path) continue
		return { type, path, isLiveDraft: true }
	}
}

export function prepareGlobalUserMessage(
	instructions: string,
	selectedContext: ContextElement[] = [],
	options: GlobalUserMessageOptions = {}
): ChatCompletionUserMessageParam {
	const selectedWorkspaceItems = selectedContext.filter(
		(context) => context.type === 'workspace_script' || context.type === 'workspace_flow'
	)
	const activeEditor =
		options.activeEditor ?? (options.workspace ? getActiveGlobalEditorContext(options.workspace) : undefined)
	let content = ''

	if (activeEditor) {
		content += '## ACTIVE EDITOR\n'
		content += `type: ${activeEditor.type}\n`
		content += `path: ${activeEditor.path}\n`
		content += `isLiveDraft: true\n\n`
	}

	if (selectedWorkspaceItems.length > 0) {
		content += '## SELECTED CONTEXT\n'
		for (const context of selectedWorkspaceItems) {
			content += `- type: ${context.type === 'workspace_script' ? 'script' : 'flow'}, path: ${context.path}\n`
		}
		content += '\n'
	}

	content += `## INSTRUCTIONS:\n${instructions}`

	return {
		role: 'user',
		content
	}
}
