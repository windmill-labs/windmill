import {
	AppService,
	AzureTriggerService,
	FlowService,
	FolderService,
	GcpTriggerService,
	HttpTriggerService,
	JobService,
	KafkaTriggerService,
	MqttTriggerService,
	AmqpTriggerService,
	NatsTriggerService,
	PostgresTriggerService,
	ResourceService,
	ScheduleService,
	ScriptService,
	SqsTriggerService,
	VariableService,
	WebsocketTriggerService,
	WorkspaceService
} from '$lib/gen'
import { createTwoFilesPatch } from 'diff'
import { $ScriptLang } from '$lib/gen/schemas.gen'
import type {
	AppWithLastVersion,
	CreateResource,
	CreateVariable,
	Flow,
	FlowValue,
	Job,
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
import { appSourceToDraftValue } from '$lib/components/raw_apps/rawAppDraftValue'
import type { RawAppDomQuery } from '$lib/components/raw_apps/rawAppDom'
import { dataUrlToImagePart, normalizeImageDataUrl, type AttachedImage } from '../imageUtils'
import { sanitizeAttachmentName, textLineCount, type AttachedTextFile } from '../textFileUtils'
import { modelSupportsVision } from '../../modelConfig'
import { tryGetCurrentModel } from '$lib/aiStore'
import { isChromiumBrowser } from '$lib/utils'
import {
	applyEditableFlowJsonToFlow,
	buildEditableFlowJson,
	type EditableFlowJson,
	validateEditableFlowJson
} from '../flow/editableFlowJson'
import { createInlineScriptSession } from '../flow/inlineScriptsUtils'
import { searchNpmPackagesTool } from '../script/core'
import {
	getDatatableSdkReference,
	getFlowPrompt,
	getPipelinePrompt,
	getRawAppPrompt,
	getResourcePrompt,
	getScriptPrompt
} from '$system_prompts'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import { z } from 'zod'
import {
	createToolDef,
	createSearchHubScriptsTool,
	executeFlowStepTestRun,
	executeTestRun,
	findAndReplace,
	type CreatedResourceTriggerKind,
	type Tool,
	type ToolCallbacks,
	type ToolDisplayAction
} from '../shared'
import { searchDocsTool, readDocsPageTool } from '../docs/core'
import { createDbSchemaTool } from '../script/core'
import type { ContextElement } from '../context'
import { getDatatableTools } from '../datatableTools'
import { fileTools } from '../files/fileTools'
import type { AttachedFilesStore } from '../files/attachedFiles.svelte'
import { artifactTools } from '../artifacts/artifactTools'
import type { SessionArtifactsStore } from '../artifacts/artifactsState.svelte'
import { UserDraft } from '$lib/userDraft.svelte'
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
import {
	userStore,
	superadmin,
	enterpriseLicense,
	userWorkspaces,
	workspaceStore
} from '$lib/stores'
import { get } from 'svelte/store'
import {
	canonicalDraftSideValue,
	deployDraft as deployDraftToWorkspace,
	getDraftDiffValues
} from '$lib/utils_draft_deploy'
import { changedLineIndices, draftDeployedPatch, windowPatch } from './draftDiff'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import { invalidateWorkspaceComparison } from '$lib/workspaceComparison'
import type { UserDraftItemKind } from '$lib/gen'
import { bundleRawAppDraft } from './rawAppBundlerBridge'
import {
	buildRunsUrl,
	buildSchedulesUrl,
	buildVariablesUrl,
	buildResourcesUrl,
	buildAssetsUrl,
	buildAuditLogsUrl,
	buildWorkspaceSettingsUrl,
	buildFoldersUrl,
	buildGroupsUrl,
	buildTriggersUrl,
	buildCompareUrl,
	WORKSPACE_SETTINGS_TABS
} from './pageNavigation'
import {
	COMPARE_ITEMS_PARAM,
	parseItemsMaskParam
} from '$lib/components/sessions/modifiedItemsMask'
import {
	pageHref,
	TRIGGER_PAGES,
	type TriggerKind as PageTriggerKind
} from '$lib/components/sessions/previewRouter'
import {
	clearEphemeralSecretVariableDraftValue,
	deleteGlobalDraft,
	flushGlobalDraftSaves,
	getEphemeralSecretVariableDraftValue,
	getGlobalDraft,
	getGlobalDraftStoragePath,
	itemKindFor,
	listGlobalDrafts,
	persistGlobalDraft,
	readGlobalDraftValue,
	readLocalDraftCellByKind,
	resolveGlobalDraftStoragePathByKind,
	saveGlobalAppDraft,
	setEphemeralSecretVariableDraftValue,
	type DraftPersistResult
} from './userDraftAdapter'
import {
	computeDiffParts,
	expireWorkspaceDiffList,
	getForkComparisonStatus,
	getForkDiffIndex,
	getForkParentWorkspaceId,
	getWorkspaceDiffIndex,
	maskVariableDiffSides,
	readForkDiffEntries,
	readWorkspaceDiffEntry,
	resolveWorkspaceDiffTarget,
	type DiffFileView,
	type ForkDiffEntryView,
	type WorkspaceDiffEntryView
} from './diffSnapshot'

const VARIABLE_MASKED_NOTE =
	'Note: variable values are never shown in chat — the diff marks whether the value changed without revealing it.\n\n'
const SECRET_UNCOMPARABLE_NOTE =
	'Note: this is a SECRET variable — its value is never shown and cannot be compared, so it may ALSO have changed beyond what this diff shows.\n\n'
import { apiCatalogTools } from './apiCatalogTools'
import { isSessionPipelinesEnabled, SESSION_PIPELINES_GATED_MESSAGE } from './pipelineGate'

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
// `datatable` is not a workspace item type, but the model can request the
// datatable SDK reference (the wmill.datatable() runnable API) the same way.
// `pipeline` likewise isn't an item type — a data pipeline is a set of
// annotated scripts in a folder, so it gets authoring guidance, not a CRUD type.
const INSTRUCTION_SUBJECTS_EXTRA = ['datatable', 'pipeline'] as const
const ALL_INSTRUCTION_SUBJECTS = [...INSTRUCTION_SUBJECTS, ...INSTRUCTION_SUBJECTS_EXTRA] as const
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
	/** Images attached to this message; delivered as image_url content parts. */
	images?: AttachedImage[]
	/** Text files attached to this message; listed by reference below — the model
	 * reads their content on demand via the file tools. */
	files?: AttachedTextFile[]
}

const itemTypeSchema = z.enum(ITEM_TYPES)
const instructionSubjectSchema = z.enum(ALL_INSTRUCTION_SUBJECTS)
const triggerKindSchema = z.enum(TRIGGER_KINDS)
const scriptLangSchema = z.enum($ScriptLang.enum)

const getInstructionsSchema = z.object({
	subject: instructionSubjectSchema.describe(
		'What to get authoring instructions for: a workspace item type (script, flow, resource, app), "pipeline" for building a data pipeline (a DAG of annotated scripts wired by storage assets — NOT a flow), or "datatable" for the wmill.datatable() SQL SDK used inside runnables. Schedules, triggers, and variables don\'t need instructions — their tool schemas describe everything.'
	),
	language: scriptLangSchema
		.optional()
		.describe(
			'The target language. Required when subject is script. For subject "datatable" it selects which SDK to return (e.g. "bun" for TypeScript, "python3" for Python) and defaults to TypeScript if omitted. Use the existing language when modifying, or the requested target language when creating. Other subjects ignore it.'
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
		.describe('Two to ten proposed answer strings.'),
	multiSelect: z
		.boolean()
		.optional()
		.describe(
			'When true, the user may select several answers; use only when the choices can genuinely co-apply (not mutually exclusive). Defaults to single-select.'
		)
})

// Matches the per-mode cap enforced by the prompt-settings UI (AIPromptsModal) and
// the backend workspace prompt (MAX_CUSTOM_PROMPT_LENGTH).
export const MAX_USER_INSTRUCTIONS_LENGTH = 5000

const updateUserInstructionsSchema = z.object({
	operation: z
		.enum(['append', 'replace'])
		.describe(
			'What to do with your personal Global instructions. \'append\' adds a new instruction to the end (use for "remember this" / "always do X"). \'replace\' performs an exact find-and-replace to edit or remove existing text (use for "change X" / "stop doing Y"); set new_string to "" to remove.'
		),
	text: z
		.string()
		.min(1)
		.optional()
		.describe(
			"Required when operation is 'append': the instruction to add. Ignored for 'replace'."
		),
	old_string: z
		.string()
		.min(1)
		.optional()
		.describe(
			"Required when operation is 'replace': exact text to find in your current personal instructions. Ignored for 'append'."
		),
	new_string: z
		.string()
		.optional()
		.describe(
			"Required when operation is 'replace': replacement text. Use an empty string to delete the matched text. Ignored for 'append'."
		),
	replace_all: z
		.boolean()
		.optional()
		.default(false)
		.describe(
			"For operation 'replace': when true, replace every exact match; when false, old_string must match exactly once."
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
		.describe(
			'Maximum items per item type per page (for triggers, per trigger kind). Defaults to 50 and is capped at 100.'
		),
	page: z
		.number()
		.int()
		.min(1)
		.optional()
		.describe(
			'Page number, starting at 1. Each item type pages independently: request the next page while any type still returns a full page. Drafts appear on page 1 only, capped at limit per type.'
		)
})

const readWorkspaceItemSchema = z.object({
	type: itemTypeSchema,
	path: z.string().describe('Workspace path of the item to read.'),
	trigger_kind: triggerKindSchema
		.optional()
		.describe('Required when type is trigger. Identifies which trigger service to call.'),
	version: z
		.enum(['deployed'])
		.optional()
		.describe(
			'Pass "deployed" to read the deployed workspace state even when a draft exists (e.g. to learn the deployed input schema before running the deployed version). Default reads your draft when one exists.'
		)
})

const draftOverrideField = z
	.boolean()
	.optional()
	.describe(
		'Overwrite the server draft even if it changed externally since you last read it (resolve a save conflict, your version wins).'
	)

const writeScriptSchema = z.object({
	path: z.string().describe('Workspace path of the script, e.g. f/folder/name or u/user/name.'),
	summary: z.string().optional().describe('Short human-readable summary.'),
	language: scriptLangSchema.describe('Script language.'),
	content: z.string().describe('Full script source code.'),
	override: draftOverrideField
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
	description: z
		.string()
		.optional()
		.describe(
			'Longer human-readable description of what the flow does. Top-level flow metadata, separate from the modules — not part of the compact value patched by patch_flow_json.'
		),
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
			'JSON string, array of semantic flow groups (call get_instructions subject:"flow" for the full field reference). color MUST be one of: yellow, blue, green, purple, pink, orange, red, cyan, lime, gray — never hex codes. Pass null to clear groups.'
		),
	notes: z
		.string()
		.optional()
		.nullable()
		.describe(
			'JSON string, array of free-floating sticky notes (type must be "free"; call get_instructions subject:"flow" for the full field reference). color MUST be one of: yellow, blue, green, purple, pink, orange, red, cyan, lime, gray — never hex codes. Pass null to clear notes.'
		),
	override: draftOverrideField
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
		groups: editable.groups ?? undefined,
		notes: editable.notes ?? undefined
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

const writeScheduleSchema = scheduleRequestSchema.extend({ override: draftOverrideField })

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
			triggerRequestSchemas.amqp,
			triggerRequestSchemas.sqs,
			triggerRequestSchemas.gcp,
			triggerRequestSchemas.azure
		])
		.describe(
			'Full trigger configuration. Must include path, script_path, is_flow plus the kind-specific fields.'
		),
	override: draftOverrideField
})

const writeResourceSchema = resourceRequestSchema.extend({ override: draftOverrideField })

const writeVariableSchema = variableRequestSchema.extend({ override: draftOverrideField })

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

const getJobLogsSchema = z.object({
	id: z.string().describe('The UUID of the job to fetch logs for.')
})

const cancelJobSchema = z.object({
	id: z.string().describe('The UUID of the job to cancel.')
})

const listRunsSchema = z.object({
	path: z.string().optional().describe('Filter to runs of this exact script or flow path.'),
	created_by: z.string().optional().describe('Filter by the username that started the run.'),
	label: z.string().optional().describe('Filter by job label.'),
	success: z
		.boolean()
		.optional()
		.describe('Only completed runs with this outcome (true = succeeded, false = failed).'),
	running: z.boolean().optional().describe('Only currently running runs.'),
	limit: z
		.number()
		.int()
		.min(1)
		.max(100)
		.optional()
		.describe('Max number of runs to return, most recent first. Defaults to 30.')
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
	path: z.string().describe('Workspace path of the draft to discard.'),
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
		.describe('Optional deployment message recorded with the change.'),
	force: z
		.boolean()
		.optional()
		.describe(
			'Deploy even if the draft was started from an older deployed version, overwriting the version deployed since. Defaults to false; prefer calling rebase_draft first to keep the newer changes.'
		)
})

const rebaseDraftSchema = z.object({
	type: itemTypeSchema,
	path: z
		.string()
		.describe('Workspace path of the draft to rebase onto the latest deployed version.')
})

const diffSchema = z.object({
	against: z
		.enum(['deployed', 'parent_workspace'])
		.optional()
		.describe(
			"What to compare against. 'deployed' (default): the current draft vs the deployed version. 'parent_workspace': the deployed fork vs its parent workspace (only in a fork; local drafts are flagged but not part of that comparison)."
		),
	type: itemTypeSchema
		.optional()
		.describe('With path: the item to diff. Omit both type and path for the workspace index.'),
	path: z
		.string()
		.optional()
		.describe(
			'Workspace path of the item to diff (draft vs deployed). Omit for the index of every draft in the workspace.'
		),
	trigger_kind: triggerKindSchema
		.optional()
		.describe('Required when type is trigger. Must match the draft trigger kind.'),
	file: z
		.string()
		.optional()
		.describe(
			'Item mode, multi-file apps only: read one file\'s diff inside the app (e.g. "src/App.tsx"). Omit for the per-file summary plus config changes.'
		),
	search: z
		.string()
		.optional()
		.describe(
			'Search mode: literal substring (case-insensitive, not a regex) matched against added/removed diff lines across every diff in the comparison. Ignores type/path.'
		),
	file_glob: z
		.string()
		.optional()
		.describe(
			'Search mode: optional glob filter on item paths and app file paths (e.g. "*.ts" matches file names, "f/dash/**" matches full paths).'
		),
	max_matches: z
		.number()
		.int()
		.min(1)
		.optional()
		.describe('Search mode: maximum matching diff lines returned (default 50, hard cap 200).'),
	types: z
		.array(itemTypeSchema)
		.optional()
		.describe('Index mode: only list drafts of these item types.'),
	path_prefix: z
		.string()
		.optional()
		.describe('Index mode: only list drafts under this path prefix, such as f/billing/.'),
	offset: z
		.number()
		.int()
		.min(0)
		.optional()
		.describe('Item mode: skip this many patch lines (paginate a large diff).'),
	limit: z
		.number()
		.int()
		.min(1)
		.optional()
		.describe(
			'Index mode: max items to list (default 50, capped at 100). Item mode: max patch lines to return (default 500).'
		)
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

const testRunArgsSchema = z
	.record(z.string(), z.any())
	.nullable()
	.optional()
	.describe('Arguments to pass to the runnable. Omit or pass null when no arguments are needed.')

const backgroundArgSchema = z
	.boolean()
	.optional()
	.describe(
		'Run in the background without waiting. Set true for jobs you expect to be long (deploys, backfills, big queries) — you will be notified when it finishes. Leave unset for normal runs, which wait briefly and only background automatically if slow.'
	)

const waitSecondsArgSchema = z
	.number()
	.optional()
	.describe(
		'How many seconds to wait for the job in-turn before it detaches into the background jobs tray. Defaults to 15. Raise it (capped at 120) for a job you expect to finish in, say, 30–60s and want the result in this same turn. Ignored when background is true. Do not use this to poll — larger values just hold the turn longer.'
	)

/** Translate the model's `wait_seconds` into executeTestRun's `detachAfterMs`
 * (the value is clamped to MAX_DETACH_AFTER_MS there). `undefined` keeps the
 * default inline budget; negatives are floored to 0 (immediate detach). */
function waitSecondsToDetachMs(waitSeconds: number | undefined): number | undefined {
	return waitSeconds == null ? undefined : Math.max(0, waitSeconds) * 1000
}

const testRunScriptSchema = z.object({
	path: z.string().describe('Workspace path of the script to test.'),
	args: testRunArgsSchema,
	background: backgroundArgSchema,
	wait_seconds: waitSecondsArgSchema
})

const testRunScriptToolDef = createToolDef(
	testRunScriptSchema,
	'test_run_script',
	'Execute a preview-style test run of a script by path, preferring draft content when it exists.',
	{ strict: false }
)

const testRunFlowSchema = z.object({
	path: z.string().describe('Workspace path of the flow to test.'),
	args: testRunArgsSchema,
	background: backgroundArgSchema,
	wait_seconds: waitSecondsArgSchema
})

const testRunFlowToolDef = createToolDef(
	testRunFlowSchema,
	'test_run_flow',
	'Execute a preview-style test run of a flow by path, preferring draft content when it exists.',
	{ strict: false }
)

const testRunStepSchema = z.object({
	path: z.string().describe('Workspace path of the flow containing the step to test.'),
	stepId: z.string().describe('The id of the step/module to test.'),
	args: testRunArgsSchema,
	background: backgroundArgSchema,
	wait_seconds: waitSecondsArgSchema
})

const testRunStepToolDef = createToolDef(
	testRunStepSchema,
	'test_run_step',
	'Execute a test run of one step in a flow by path, preferring draft flow/script content when it exists.',
	{ strict: false }
)

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
		),
	offset: z
		.number()
		.int()
		.min(1)
		.optional()
		.describe('1-based line number to start reading from. Use to page through a large file.'),
	limit: z
		.number()
		.int()
		.min(1)
		.optional()
		.describe(
			'Maximum number of lines to return. Large files are truncated by default; pass offset/limit to read a specific line range.'
		)
})

const searchAppSchema = z.object({
	path: z.string().describe('Workspace path of the app, e.g. f/folder/name.'),
	query: z
		.string()
		.describe(
			'Literal substring to find across all app files (case-insensitive) — not a regex, so spaces and operators match verbatim. Returns matching file:line rows, not file bodies. To find call sites and skip similarly-named helpers, include the call paren (e.g. "formatCurrency(" matches calls but not "formatCurrencyPrecise"). Then read_app_file to inspect the ranges.'
		),
	file_glob: z
		.string()
		.optional()
		.describe(
			'Optional path filter. A pattern without "/" matches the file name anywhere (e.g. "*.tsx"); a pattern with "/" matches the full path (e.g. "/components/*", "backend/**"). Supports * (any chars except /), ** (any chars), and ?.'
		),
	max_matches: z
		.number()
		.int()
		.min(1)
		.optional()
		.describe(
			'Maximum number of matching lines to return (default 100, hard cap 200); each is shown with a few lines of surrounding context, so the output has more rows than this.'
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

const openPreviewSchema = z.object({
	kind: z
		.enum(['script', 'flow', 'raw_app', 'pipeline'])
		.describe(
			'Item kind to preview. Use "raw_app" for code-based apps (created via init_app). Use "pipeline" to show the data-pipeline graph for a folder — here `path` is the folder name, not an item path. The legacy drag-and-drop app builder ("app") is not previewable in the session panel — don\'t pass it.'
		),
	path: z
		.string()
		.describe('Workspace path of the item to preview, or the folder name when kind is "pipeline".')
})

const getPreviewStatusSchema = z.object({})

const closePageSchema = z.object({
	all: z
		.boolean()
		.optional()
		.describe('Close every open preview tab, clearing the side panel. Ignores `match` when true.'),
	match: z
		.string()
		.optional()
		.describe(
			'Close the preview tab(s) whose page name or item path contains this text (case-insensitive), e.g. "runs", "schedules", or a script path. Call get_preview_status first if unsure what is open.'
		)
})

type SessionToolResult = {
	aiResult: string
	uiMessage: string
	toolResult: string
}

const getRuntimeLogsSchema = z.object({
	limit: z
		.number()
		.int()
		.min(1)
		.max(100)
		.optional()
		.describe('How many of the most recent runtime log lines to return. Defaults to 10.')
})

const listAppRunsSchema = z.object({
	limit: z
		.number()
		.int()
		.min(1)
		.max(100)
		.optional()
		.describe('How many of the most recent backend runs to return, newest first. Defaults to 20.')
})

const domSelectorField = z
	.string()
	.optional()
	.describe(
		'CSS selector for the element to inspect in the live raw app preview. Omit to target the whole page (<body>). Prefer a selector from a DOM element chip the user attached. If it matches several elements, the first is used.'
	)

const domAppPathField = z
	.string()
	.optional()
	.describe(
		"Raw-app path of the element, from its `app_path` in the SELECTED DOM ELEMENTS block. Pass it so the RIGHT app's preview is read even if another preview tab is now visible; omit to use the currently active preview. If that app's preview has been closed, the tool says so."
	)

const searchDomSchema = z.object({
	app_path: domAppPathField,
	selector: domSelectorField,
	pattern: z.string().describe('JavaScript regular expression to search the rendered HTML for.'),
	ignore_case: z.boolean().optional().describe('Case-insensitive matching. Defaults to false.')
})

const readDomSchema = z.object({
	app_path: domAppPathField,
	selector: domSelectorField,
	start_line: z
		.number()
		.int()
		.optional()
		.describe('1-based first line of the pretty-printed HTML to read. Defaults to 1.'),
	end_line: z
		.number()
		.int()
		.optional()
		.describe('1-based last line to read. The window is capped at 200 lines.')
})

const takeScreenshotSchema = z.object({})

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
		)
})

// Mirrors the backend VALID_FOLDER_NAME check so an invalid name fails before the
// network round-trip (the server enforces the same rule).
const VALID_FOLDER_NAME = /^[a-zA-Z_0-9-]+$/

const createFolderSchema = z.object({
	name: z
		.string()
		.describe(
			'Folder name — letters, digits, underscores or hyphens only. The folder becomes addressable as `f/<name>/<item>`.'
		),
	summary: z.string().optional().describe('Optional human-readable description of the folder.')
})

type FolderPromptContext = { folders: string[]; foldersRead: string[]; isAdmin: boolean }

// Renders the folders the current user can act on into the system prompt so the
// model can pick an `f/<folder>/...` path without a discovery round-trip (there
// is no folder-listing tool). For a non-admin, `folders` (from whoami) is exactly
// the writable set, so read-only folders are listed separately as off-limits.
// Admins bypass folder ACLs and can write anywhere, but `folders` only carries
// their explicitly-permissioned subset, so for admins it is offered as a
// non-exhaustive hint alongside permission-agnostic guidance (the complete set
// needs a folder-listing tool — follow-up).
// Capped so a folder-heavy workspace can't dominate the prompt.
function buildFolderGuidance(username: string, ctx?: FolderPromptContext): string {
	if (!ctx) return ''
	const MAX = 40
	const writable = ctx.folders ?? []
	const fmt = (names: string[]) => {
		const shown = names
			.slice(0, MAX)
			.map((n) => `\`f/${n}\``)
			.join(', ')
		return names.length > MAX ? `${shown} (+${names.length - MAX} more)` : shown
	}
	if (ctx.isAdmin) {
		const known =
			writable.length > 0
				? ` Folders here include ${fmt(writable)} (you can also write to others not listed).`
				: ''
		return `- As a workspace admin you can write to any existing folder.${known} If the user names a folder, use it; if they explicitly ask for a new folder, create it with \`create_folder\`; otherwise ask them which folder to use rather than guessing or creating one unprompted.`
	}
	const readOnly = (ctx.foldersRead ?? []).filter((f) => !writable.includes(f))
	const lines: string[] = []
	if (writable.length > 0) {
		lines.push(
			`- Folders you can write to in this workspace: ${fmt(writable)}. For shared/team work, pick the one whose purpose matches the request; if none clearly fits, ask which folder to use (askUserQuestion) rather than inventing a path. Use \`create_folder\` only when the user explicitly asks for a new folder.`
		)
	} else {
		lines.push(
			`- You have no shared folders you can write to in this workspace, so use \`u/${username}/<name>\`. If the user explicitly asks for a shared folder, create one with \`create_folder\` (you become an owner); otherwise ask before placing shared work rather than inventing an \`f/<folder>/...\` path.`
		)
	}
	if (readOnly.length > 0) {
		lines.push(
			`- You can see but NOT write to: ${fmt(readOnly)} — never create or deploy items there.`
		)
	}
	return lines.join('\n')
}

const buildGlobalSystemPrompt = (
	username: string,
	previewTools: boolean,
	folderCtx?: FolderPromptContext,
	skills: AiSkillListItem[] = []
) => {
	const folderGuidance = buildFolderGuidance(username, folderCtx)
	const folderGuidanceBlock = folderGuidance ? `\n${folderGuidance}` : ''
	// `previewTools` doubles as "this is a session chat" — sessions are the only
	// chats that get the preview tool set, and the only surface the gate covers.
	const pipelinesGated = previewTools && !isSessionPipelinesEnabled()
	const pipelineBullet = pipelinesGated
		? `- Data pipelines are in alpha and NOT yet available in this chat. A "data pipeline" is a DAG of independent annotated scripts wired by storage assets — it is not a flow. If the user asks for a data pipeline (or to ingest/transform/materialize data across steps in one), tell them that data pipelines are in alpha and will be handled by the chat soon — do not build one (no annotated pipeline scripts, no get_instructions with subject "pipeline"), and do not build a flow as a substitute.`
		: `- A "data pipeline" is NOT a flow: it is a DAG of independent scripts in one folder, wired by storage assets (DuckLake/data tables/S3) and triggers via top-of-file \`pipeline\` / \`on <ref>\` annotation comments written in each script's comment syntax (\`--\` for SQL, \`#\` for Python/Bash, \`//\` for TS — a \`//\` line in a SQL node is a syntax error). When the user asks for a data pipeline (or to ingest/transform/materialize data across steps), call get_instructions with subject "pipeline" and build annotated script drafts — do not build a flow.`
	return `You are Windmill's global workspace assistant.

The current user's workspace username is "${username}".

Use tools to inspect workspace items and create per-user drafts (saved server-side, visible only to this user — not deployed) for scripts, flows, schedules, triggers, resources, variables, and raw apps.

Path conventions:
- A workspace path starts with one of two namespaces; its trailing <name> may itself contain "/", so a path has three or more segments:
  - \`u/${username}/<name>\` — your personal scope. Default for ad-hoc, exploratory, or scratch work.
  - \`f/<folder>/<name>\` — a shared folder scope; the <folder> must already exist (a bare \`f/<name>\` with no folder segment is INVALID and will fail).
- If the user supplies a fully qualified \`f/<folder>/...\` path, use that exact path; they have already chosen the folder. Do not ask for folder confirmation or substitute a \`u/${username}/...\` path unless a tool rejects it.
- Default a bare name with no namespace prefix (e.g. "create a flow called myflow") to \`u/${username}/<name>\`. Never invent an \`f/<folder>/...\` path for a folder that does not exist; create one with \`create_folder\` only when the user explicitly asks for a new folder.${folderGuidanceBlock}

Rules:
- Draft tools create or update drafts only; they do not deploy or mutate deployed workspace items.
- Use list_workspace_items to find items and read_workspace_item before changing an existing item. For triggers, pass trigger_kind.
- If the user message includes an ACTIVE EDITOR section, treat it as the currently open item and use it for references like "this", "current", or "open editor".
- Use deploy_workspace_item only after the user explicitly asks to deploy. It persists a draft to the workspace.
- Use discard_local_draft to remove a draft, including the matching open editor draft. Use delete_workspace_item only to delete a deployed workspace item.
- Use diff to review changes — before deploying, or when the user asks what changed. It is read-only: without arguments it lists every draft in the workspace with its change status; with type+path it returns that item's unified diff (for multi-file apps, pass file to read one file's diff). In a fork, pass against="parent_workspace" to compare the deployed fork with its parent workspace instead. Pass search to grep changed lines across all diffs.
- Variable values are never readable. For secrets, create a secret variable and reference it from resources as "$var:path/to/variable".
- Use search_resource_types before write_resource.
- When script or raw app code needs an external npm package you are not fully familiar with, use search_npm_packages to find it and get its documentation and type definitions. Link the package documentation in your answer when you rely on it.
- Use get_db_schema with a database resource path to fetch its tables and columns before writing SQL (or a script querying that database).
- Use get_instructions before writing scripts, flows, resources, or apps. For scripts, pass the target language.
${pipelineBullet}
- After creating or editing a script or flow draft, run test_run_script, test_run_flow, or test_run_step with representative args before reporting that it works. These tools prefer drafts, so testing does not require deployment.
- Use list_runs to find recent runs (optionally filtered by path, creator, label, or status), then get_job_logs with a returned id to inspect a specific run's logs — without starting a new test run.
- Use open_page to show a workspace page with filters applied — Runs, Schedules, Variables, Resources, Assets, Audit logs, or Workspace settings on a specific tab (e.g. "open the failed runs of f/foo/bar", "open the schedule for X", "open the git sync settings"). Only the pages listed for this user in the tool are available; don't offer pages that aren't listed. Don't use it as a substitute for list_runs when you just need the data yourself.
- When the user is happy with the changes and wants to review or deploy them, use open_page with page "compare" — it opens the Compare & Deploy review page.${
		previewTools
			? ' By default it preselects the items this chat modified; pass items ("<kind>:<path>" entries) to control the selection'
			: ' Pass items ("<kind>:<path>" entries naming the items you changed) so the review is scoped to them — omitting items preselects every pending change in the workspace'
	}, or mode ("draft" or "fork") to force which comparison is shown. Prefer offering this review page over calling deploy_workspace_item directly when several items changed.
- For a Windmill operation no other tool covers (workers, queue state, a run's result or args, ...), use search_api_endpoints to find a REST endpoint, then call_api_get for reads or call_api_endpoint for mutations (the user is asked to confirm those). Always prefer a dedicated tool when one exists; endpoints for authoring or deleting scripts, flows, apps, schedules, resources, or variables are not available through the API catalog tools — use the draft tools and delete_workspace_item instead.
- runScriptByPath / runFlowByPath from the API catalog run the DEPLOYED version of an item. Use them only when the user explicitly asks to run the deployed version, and read the item with read_workspace_item version: "deployed" first so the arguments match the deployed input schema (a draft may have different inputs). To test something you are editing or just wrote, always use test_run_script, test_run_flow, or test_run_step — they run the draft.
- When a required decision is ambiguous, use askUserQuestion with two to ten clear proposed answer strings instead of guessing. The user can also type a custom answer when none of the proposed answers fit. Set multiSelect: true only when the answers can genuinely co-apply and the user may pick several (not mutually exclusive).
- When the user asks you to remember a lasting preference, always/never do something, or change/stop a behavior going forward, call update_user_instructions to persist it. It edits only the USER INSTRUCTIONS block (not WORKSPACE INSTRUCTIONS). Keep each instruction concise; do not use it for one-off requests scoped to the current task.
- Keep context targeted.${
		previewTools
			? `
- After writing or substantially editing a script / flow / app draft, show it via open_preview(kind, path) so the user sees the editor and live preview right next to the chat. First check whether it is already shown: if unsure, call get_preview_status. Only call open_preview (or offer to) when no preview is open or it is showing a different item — don't re-open a preview already showing the item you just edited.${
					pipelinesGated
						? ''
						: `
- Building a data pipeline: call open_preview(kind="pipeline", path="<folder>") as the FIRST step, before creating any node — this opens the pipeline editor the user reviews in. path is the folder, not an item; an empty or not-yet-created folder is fine (create_folder first if needed, then open it). Opening it registers build_pipeline_node / edit_pipeline_node — use ONLY those to add or change pipeline nodes, never write_script for a pipeline node — they apply directly as unsaved drafts on the canvas (no separate accept/reject step) that the user reviews and deploys. Do not write pipeline scripts without first opening the editor.`
				}
- When debugging a running raw app, call get_app_runtime_logs to read the live preview's browser console output. It needs the raw app preview open (open_preview kind="raw_app").
- To inspect what actually rendered in a running raw app (verify an edit landed on screen, diagnose a blank/empty or wrong view, answer "what's showing"), use search_dom (regex over the live HTML) and read_dom (a line-numbered window). Pass a \`selector\` to scope to an element — prefer the selector from a DOM element chip the user attached — or omit it for the whole page. When a chip lists an \`app_path\`, pass it too so the RIGHT app is read (several previews can be open; a query without \`app_path\` hits the visible one). The DOM is read live and is never in context; no match means the element isn't rendered. Both need the raw app preview open.
- get_app_runtime_logs only shows the app's browser console. For the server-side logs of a backend runnable the app invoked (a backend.<id> call), call list_app_runs to get that run's job_id from the live preview, then get_job_logs with it. Use this when a backend call errors or returns something unexpected.
${
	isChromiumBrowser()
		? `- When the user raises how a raw app looks (something is off, or they want the design or layout improved), call take_screenshot to see what they are looking at before changing anything. Reach for it when the request is about appearance, not to review your own edits, which you can read back from the code. It needs the raw app preview open (open_preview kind="raw_app").`
		: `- When the user raises how a raw app looks (something is off, or they want the design or layout improved) and their description alone isn't specific enough to pinpoint the problem, ask them to paste or drop a screenshot of it into the chat before changing anything.`
}
- open_page opens its page as a tab in the side-panel preview next to the chat — the only way to show one of these pages there (open_preview only handles editable items). Changing filters on a page already open updates that same tab; only pass new_tab when the user explicitly asks for a separate tab.
- create_artifact saves a persistent markdown document (a planning doc, design write-up, spec, or other longer structured output) shown in the session preview panel. Prefer it over a long inline reply for content the user will revisit; keep brief answers inline. To revise one, call list_artifacts then read_artifact for the current content, then update_artifact to overwrite it — never create a second artifact for the same document.`
			: ''
	}

Documentation:
- Use search_docs to look up how a Windmill feature works in the official documentation (a flag, concept, function, or "does Windmill support X") instead of guessing about product behavior. It returns matching doc snippets with their Source URL; call read_docs_page with a Source URL to read the full page (or a section, if it returns headings). Cite the Source URL when you rely on it.
- Complete your response with precisions about how it works based on the documentation. Also drop a link to the relevant documentation if possible.
- If the user asks about something that you are unsure about, say that you are not sure about the answer and suggest to ask the question to the windmill team.
- If the first search returns nothing useful, retry with different or broader keywords before giving up.
- If the documentation does not cover the user's question, say so clearly rather than inventing an answer, and suggest asking the Windmill team.

Flows:
- read_workspace_item returns compact flow JSON. Inline script bodies appear as "inline_script.<moduleId>".
- Use read_flow_module_code and set_flow_module_code for inline script bodies.
- Use patch_flow_json for structural flow edits and write_flow for full flow rewrites.

Raw apps:
- read_workspace_item returns app metadata only. Use read_app_file for file and inline runnable contents.
- Use write_app_file, patch_app_file, and delete_app_file for frontend files.
- Use write_app_runnable and delete_app_runnable for backend runnables.
- Use init_app only after confirming framework, path, and summary with the user.
- Use deploy_workspace_item after explicit user deploy intent; raw app deploy bundles JS/CSS before saving.

Data Tables:
- Datatables are workspace-scoped managed PostgreSQL databases, shared across the workspace (not owned by any single app). They must be configured by the user in their workspace settings (Workspace settings → Data Tables); they cannot be created via SQL.
- Use list_datatables to discover the available datatables and their tables. Reuse an existing table rather than creating a duplicate. If list_datatables reports none, this is a blocking prerequisite — tell the user to set up a datatable in their workspace settings and stop; do not assume a "main" datatable exists or call exec_datatable_sql.
- Use get_datatable_table_schema only when you need a table's column names/types; list_datatables is enough for table-list or availability summaries.
- Use exec_datatable_sql to explore data, run queries, mutate rows, or change schema (CREATE/ALTER/DROP). Creating a table is a normal CREATE TABLE statement — it appears in list_datatables afterward, with no registration step.
- When writing runnable code (inline app runnables, scripts, flow modules) that reads or writes datatable data at runtime, it accesses a datatable via wmill.datatable(). Default to TypeScript (bun) unless the user asked for another language. Call get_instructions with subject "datatable" and language "bun" for the TypeScript SQL SDK reference (or language "python3" for Python) — it returns only that language so you get just what you need.${
		skills.length > 0
			? `

Skills:
- Skills are reusable instruction sets curated for this workspace, each covering a specific kind of task. The available skills are listed below by name and description.
- When a user's request matches a skill's description, call read_skill with its exact name to load the full instructions BEFORE acting, then follow them.
${skills.map((s) => `- ${s.name}: ${s.description}`).join('\n')}`
			: ''
	}`
}

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

function scriptToItem(script: Script | NewScript, includeValue: boolean): WorkspaceItem {
	return {
		type: 'script',
		path: script.path,
		summary: script.summary,
		language: script.language,
		value: includeValue ? script.content : undefined,
		schema: includeValue ? (script as Script).schema : undefined,
		// Listings with includeDraftOnly synthesize rows for editor drafts that
		// have no deployed counterpart — label those honestly.
		isDraft: (script as Script).draft_only ?? false
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
		isDraft: (flow as Flow & { draft_only?: boolean }).draft_only ?? false
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

	if (
		item.type === 'app' &&
		item.value &&
		typeof item.value === 'object' &&
		'files' in item.value
	) {
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

// Compact metadata for one run. The raw Job carries args/result/logs/raw_code
// which can be huge — list_runs returns only what's needed to identify a run.
function summarizeRun(job: Job): Record<string, unknown> {
	const base = {
		id: job.id,
		job_kind: job.job_kind,
		path: job.script_path,
		created_by: job.created_by,
		created_at: job.created_at,
		started_at: job.started_at,
		schedule_path: job.schedule_path,
		is_flow_step: job.is_flow_step,
		tag: job.tag,
		worker: job.worker
	}
	if ('success' in job) {
		// CompletedJob
		return {
			...base,
			status: job.canceled ? 'canceled' : job.success ? 'success' : 'failure',
			duration_ms: job.duration_ms
		}
	}
	// QueuedJob (running or still waiting in the queue)
	return {
		...base,
		status: job.running ? 'running' : 'queued'
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

async function loadAppValueForRead(path: string, workspace: string): Promise<AppDraftValue> {
	const draft = await getGlobalDraft(workspace, 'app', path)
	if (draft && draft.value && typeof draft.value === 'object' && 'files' in draft.value) {
		return draft.value as AppDraftValue
	}

	const app = await AppService.getAppByPath({ workspace, path })
	return appSourceToDraftValue(app, app)
}

async function loadAppDraftValue(path: string, workspace: string): Promise<LoadedAppDraftValue> {
	const draft = await getGlobalDraft(workspace, 'app', path)
	if (draft && draft.value && typeof draft.value === 'object' && 'files' in draft.value) {
		return { value: draft.value as AppDraftValue }
	}

	const app = await AppService.getAppByPath({ workspace, path })
	return { value: appSourceToDraftValue(app, app) }
}

async function saveAppDraft(
	workspace: string,
	path: string,
	value: AppDraftValue
): Promise<DraftPersistResult> {
	return saveGlobalAppDraft(workspace, path, value)
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
		page?: number
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
	amqp: {
		exists: (a) => AmqpTriggerService.existsAmqpTrigger(a),
		get: (a) => AmqpTriggerService.getAmqpTrigger(a),
		list: (a) => AmqpTriggerService.listAmqpTriggers(a),
		create: (a) => AmqpTriggerService.createAmqpTrigger(a),
		update: (a) => AmqpTriggerService.updateAmqpTrigger(a),
		delete: (a) => AmqpTriggerService.deleteAmqpTrigger(a)
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
	triggerKind?: TriggerKind,
	deployedOnly = false
): Promise<WorkspaceItem> {
	switch (type) {
		case 'script': {
			// Prefer the DB draft (newer than the deployed version) when one exists,
			// unless the caller explicitly asked for the deployed state.
			const script = await ScriptService.getScriptByPath({
				workspace,
				path,
				getDraft: !deployedOnly
			})
			const draft = deployedOnly ? undefined : (script.draft as Script | undefined)
			return scriptToItem(draft ?? script, true)
		}
		case 'flow': {
			// Prefer the DB draft (newer than the deployed version) when one exists,
			// unless the caller explicitly asked for the deployed state.
			const flow = await FlowService.getFlowByPath({ workspace, path, getDraft: !deployedOnly })
			const draft = deployedOnly ? undefined : (flow.draft as Flow | undefined)
			return flowToItem(draft ?? flow, true)
		}
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
			return resourceToItem(await ResourceService.getResource({ workspace, path }), true)
		case 'variable':
			// Never expose the value, even when read directly. Pass decryptSecret=false
			// to avoid materializing secret values server-side.
			return variableToItem(
				await VariableService.getVariable({ workspace, path, decryptSecret: false })
			)
		case 'app': {
			// Returns lightweight metadata only — file/runnable contents come via read_app_file.
			const app = await AppService.getAppByPath({ workspace, path })
			const value = appSourceToDraftValue(app)
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
	perPage: number,
	page?: number
): Promise<WorkspaceItem[]> {
	const items: WorkspaceItem[] = []

	if (types.includes('script')) {
		const scripts = await ScriptService.listScripts({
			workspace,
			pathStart: pathPrefix,
			perPage,
			page,
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
			page,
			includeDraftOnly: true,
			withoutDescription: true
		})
		for (const flow of flows) items.push(flowToItem(flow, false))
	}

	if (types.includes('schedule')) {
		const schedules = await ScheduleService.listSchedules({
			workspace,
			pathStart: pathPrefix,
			perPage,
			page
		})
		for (const schedule of schedules) items.push(scheduleToItem(schedule, false))
	}

	if (types.includes('trigger')) {
		for (const kind of TRIGGER_KINDS) {
			const triggers = await triggerServices[kind].list({
				workspace,
				pathStart: pathPrefix,
				perPage,
				page
			})
			for (const trigger of triggers) items.push(triggerToItem(kind, trigger, false))
		}
	}

	if (types.includes('resource')) {
		const resources = await ResourceService.listResource({
			workspace,
			pathStart: pathPrefix,
			perPage,
			page
		})
		for (const resource of resources) items.push(resourceToItem(resource, false))
	}

	if (types.includes('variable')) {
		const variables = await VariableService.listVariable({
			workspace,
			pathStart: pathPrefix,
			perPage,
			page
		})
		for (const variable of variables) items.push(variableToItem(variable))
	}

	if (types.includes('app')) {
		const apps = await AppService.listApps({
			workspace,
			pathStart: pathPrefix,
			perPage,
			page
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
- Paths follow the conventions in the system prompt: default to \`u/<current-user>/<name>\` when the user gave a bare name; only use \`f/<folder>/<name>\` when the folder is known to exist. Preserve the current path/language when modifying unless the user asked to change them.
- Use \`edit_script\` for small localized changes (provide \`old_string\`/\`new_string\`); use \`write_script\` for full rewrites.${note}

# Windmill script authoring reference (${selected})

${getScriptPrompt(selected)}`
}

function getFlowInstructions(): string {
	return `# Global draft flow instructions

- Global mode writes complete draft payloads only; it does not save, deploy, run, scaffold local files, or generate metadata.
- Paths follow the conventions in the system prompt: default to \`u/<current-user>/<name>\` when the user gave a bare name; only use \`f/<folder>/<name>\` when the folder is known to exist. Never invent a folder.
- \`write_flow\` mirrors flow mode's \`set_flow_json\`: pass \`path\`, optional \`summary\`, optional \`description\`, required \`modules\`, and optional \`schema\`, \`preprocessor_module\`, \`failure_module\`, \`groups\`, and \`notes\`. \`summary\` and \`description\` are top-level flow metadata (not part of the compact value \`patch_flow_json\` edits); the flow-structure arguments are JSON strings, matching the tool schema descriptions.
- \`read_workspace_item\` returns a compact flow \`value\` object with \`modules\`, \`schema\`, \`preprocessor_module\`, \`failure_module\`, \`groups\`, and \`notes\`.
- \`modules\` contains normal sequential modules. Use top-level \`preprocessor_module\` and \`failure_module\` for special modules; do not put \`preprocessor\` or \`failure\` in \`modules\`.
- Every module needs a stable unique \`id\` and a useful \`summary\` when the schema supports it.
- Prefer path/script/flow modules when composing existing workspace logic. Use rawscript modules only when new inline code is needed.
- When writing rawscript module code, call \`get_instructions\` with \`subject: "script"\` and the rawscript language first.

## Organizing flows: groups and notes

- \`groups\`: Array of semantic groups for organizing modules in the editor (optional, but **strongly recommended** — proactively segment any non-trivial flow into groups so it reads clearly; don't wait to be asked). Each group has \`summary\` (display name), \`note\` (markdown description shown below the group header — attached directly to the group, not a separate sticky note), \`autocollapse\`, \`start_id\`, \`end_id\`, and \`color\`. \`start_id\` and \`end_id\` must reference existing module IDs in the flow (not \`preprocessor\` or \`failure\`). \`color\` MUST be one of these exact names: \`yellow\`, \`blue\`, \`green\`, \`purple\`, \`pink\`, \`orange\`, \`red\`, \`cyan\`, \`lime\`, \`gray\` — do NOT use hex codes, CSS colors, or any other strings. Omit \`color\` entirely if no preference and the editor will assign one automatically. Groups do not affect execution — they provide naming and collapsibility in the editor. Pass \`null\` to clear existing groups.
- \`notes\`: Array of free-floating sticky notes shown in the editor (optional). Each note has \`id\` (unique string), \`text\` (markdown content), \`color\` (same palette as groups: \`yellow\`, \`blue\`, \`green\`, \`purple\`, \`pink\`, \`orange\`, \`red\`, \`cyan\`, \`lime\`, \`gray\` — never hex codes), and optional \`position\` {x, y} / \`size\` {width, height} (omit both — the editor auto-places and sizes the note). Always set \`type\` to \`free\`. The \`group\` note type is **deprecated** — do not create group notes; use the \`groups\` field to segment a flow instead. Notes are documentation only and do not affect execution. Pass \`null\` to clear existing notes.

### When to use notes vs groups

**Strongly prefer \`groups\` to organize flows.** Groups are the primary way to make a flow readable: whenever a flow has more than a couple of steps, or any time consecutive steps form a logical stage (e.g. "fetch", "transform", "notify"), segment them into \`groups\`. Each group spans a range of steps (\`start_id\`..\`end_id\`), carries its own \`summary\`, \`note\` (markdown under the group header), and \`color\`, and can be collapsed. Proactively add or update groups when building or restructuring a flow — do not wait to be asked. Aim for every meaningful step to belong to a semantic group.

- **\`groups\` (default, use liberally):** segment a flow into labelled semantic sections. This is the main organizational tool — reach for it on essentially any non-trivial flow, not just "complex" ones.
- **\`notes\` (free sticky notes, use sparingly):** reserve for important flow-wide information that does not belong to a specific span of steps — overall purpose, key assumptions, warnings, or TODOs. Usually a single note is enough; do not use notes to label sequences of steps (that is what \`groups\` are for).
- Do **not** use \`group\`-type notes (deprecated) — \`groups\` is the supported way to group steps.
- With \`patch_flow_json\`, edit \`groups\` and \`notes\` the same way as any other field — they appear as top-level keys in the compact flow value.

## Compact view: how rawscript bodies surface in tool I/O

- \`read_workspace_item\` and \`patch_flow_json\` operate on a **compact view** of the flow: every rawscript module's \`value.content\` is replaced with the placeholder \`"inline_script.<moduleId>"\` so inline script bodies don't bloat tool I/O. Schema, groups, notes, preprocessor_module and failure_module are all shown in this view.
- Inline rawscript content is **not** part of the JSON \`patch_flow_json\` sees. Edits to inline bodies happen via dedicated tools:
  - \`read_flow_module_code(path, module_id)\` — returns the raw inline script content for one module.
  - \`set_flow_module_code(path, module_id, code)\` — overwrites that module's inline script content; saves to the draft.
- Use \`patch_flow_json\` for *structural* edits: module ids, paths, input_transforms, branch arrangement, summaries, preprocessor/failure swaps, schema/groups/notes. Use \`set_flow_module_code\` for changes inside a specific rawscript body.
- \`write_flow\` is for full overwrites / create-from-scratch. Its \`modules\`, \`preprocessor_module\`, and \`failure_module\` arguments use **non-compact** flow modules (rawscript content is the actual code, not a placeholder).

# Windmill flow authoring reference

${getFlowPrompt()}`
}

type InstructionSubject = (typeof ALL_INSTRUCTION_SUBJECTS)[number]

function getAppInstructions(): string {
	return `# Global draft app instructions

- Global mode edits raw app drafts only; it does not save or deploy unless the user explicitly asks to deploy.
- App drafts are addressed by workspace path. Follow the path conventions in the system prompt: default to \`u/<current-user>/<name>\` for bare names; only use \`f/<folder>/<name>\` when the folder is known to exist. The first write tool snapshots the workspace app onto the draft, and subsequent writes accumulate.
- To create a new app, use \`init_app\` with a path, optional summary, and a framework (\`react19\` / \`react18\` / \`svelte5\` / \`vue\`). Confirm framework + path + summary with the user before calling — do not silently default to \`react19\` even though it is the recommended choice. \`init_app\` errors if an app already exists at the path or a draft is already in flight; in that case, edit the existing one rather than re-initializing.
- \`init_app\` seeds a starter inline runnable named \`a\` (bun, \`main(x: string) => string\`) so the React/Svelte demo button works on first render. Replace or remove it once you start building real backend runnables.
- Frontend file paths start with \`/\` (e.g. \`/index.tsx\`, \`/App.tsx\`, \`/styles.css\`). Use \`write_app_file\` / \`patch_app_file\` / \`delete_app_file\`.
- Backend inline runnables are addressed as \`backend/<key>/main.{ts|py}\` from the file tools, but you create or update them via \`write_app_runnable\` / \`delete_app_runnable\` (which take the runnable shape directly: \`{ name, type, inlineScript?, path?, staticInputs? }\`).
- \`/wmill.d.ts\` (or \`wmill.ts\`) is generated automatically from the backend runnables — never write it directly.
- Inline runnables only support \`bun\` or \`python3\` in chat. Path runnables (\`script\`/\`flow\`/\`hubscript\`) reference an existing item.
- Use \`deploy_workspace_item\` after explicit user deploy intent. The deploy tool bundles JS/CSS before saving the raw app.
- Use \`read_workspace_item\` with \`type: 'app'\` for a metadata summary (file paths and runnable list, no contents). Use \`read_app_file\` to read an individual file; large files are truncated to a head slice, so pass \`offset\`/\`limit\` to page through the rest rather than re-reading the whole file.
- To find where a symbol or string lives across the app, call \`search_app\` (greps every frontend file and inline runnable, returns matching \`file:line\` rows) instead of reading files one by one — then \`read_app_file\` only the ranges you need. The loop is list (\`read_workspace_item\`) → locate (\`search_app\`) → inspect (\`read_app_file\` with \`offset\`/\`limit\`).
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

function getDatatableInstructions(language?: ScriptLang): string {
	// Default to the TypeScript SDK so we return only what's needed, not both.
	const lang = language ?? 'bun'
	return `# Datatable SQL SDK reference

Datatables are workspace-scoped managed PostgreSQL databases. In chat, explore and shape them with the \`list_datatables\`, \`get_datatable_table_schema\`, and \`exec_datatable_sql\` tools. The reference below is for code you author inside runnables (inline app runnables, scripts, or flow rawscript modules) that reads or writes datatable data at runtime.

- A runnable accesses a datatable via \`wmill.datatable()\` (the default "main") or \`wmill.datatable('<name>')\`, referencing tables as \`schema.table\`.
- Use parameterized queries (the tagged template in TypeScript, \`$1\`/\`$2\` placeholders in Python) — never interpolate untrusted values into SQL strings.

${getDatatableSdkReference(lang)}`
}

function getPipelineInstructions(): string {
	return getPipelinePrompt()
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
		case 'datatable':
			return getDatatableInstructions(language)
		case 'pipeline':
			return getPipelineInstructions()
	}
}

export type AiSkillListItem = { name: string; description: string }

/** Live session facts appended to the GLOBAL system prompt for session chats.
 * Provided by the session runtime as a resolver (copilot must not import the
 * sessions modules) and re-read on every system-message rebuild — the fork
 * commits at first send, and the user can re-point the session's workspace. */
export type SessionPromptContext = {
	/** Operating workspace (undefined while the session is an unsent draft with
	 * no pick). Only slug-validated workspace IDs belong here — free-form
	 * metadata like display names is user-controlled text that must not be
	 * interpolated into the system prompt. */
	workspaceId?: string
	/** Set when the operating workspace is a fork of this workspace (staged
	 * session fork or persistent dev workspace — `isDevWorkspace` splits them). */
	parentWorkspaceId?: string
	/** The operating workspace is a persistent dev workspace, not an ephemeral
	 * staged fork. Same promote-to-parent deploy flow; different lifecycle. */
	isDevWorkspace?: boolean
	/** Committed workspace missing from the user's workspace list (access lost /
	 * stale store): still a fork per `isForkSession`, but the parent is unknown —
	 * must not be presented as the live workspace. */
	forkParentUnknown?: boolean
	/** Pre-send intent: a staged fork of this workspace is created at first send. */
	pendingForkOf?: string
}

/** Session-state guidance appended to the global system prompt so the model
 * knows where its work lands (staged fork vs the live workspace). */
export function getSessionContextPromptSection(ctx: SessionPromptContext): string {
	const lines = [
		'',
		'',
		'Session state:',
		'- This chat is a Windmill AI session with its own operating workspace: every tool call (reads, drafts, test runs, deploys) targets that workspace.'
	]
	if (ctx.pendingForkOf) {
		lines.push(
			`- No workspace is committed yet: a staged fork of workspace "${ctx.pendingForkOf}" is created automatically when the first message is sent, and all work lands in that fork.`
		)
	} else if (ctx.parentWorkspaceId && ctx.isDevWorkspace) {
		lines.push(
			`- Operating workspace: "${ctx.workspaceId}" — the user's persistent DEV WORKSPACE, forked from workspace "${ctx.parentWorkspaceId}". deploy_workspace_item publishes into the dev workspace only; the user reviews & promotes changes into "${ctx.parentWorkspaceId}" from the session's deploy panel. Never present a change as live in "${ctx.parentWorkspaceId}".`
		)
	} else if (ctx.parentWorkspaceId) {
		lines.push(
			`- Operating workspace: "${ctx.workspaceId}" — an ephemeral STAGED FORK of workspace "${ctx.parentWorkspaceId}", created for session work. deploy_workspace_item publishes into the fork only, and the user reviews & promotes fork changes into "${ctx.parentWorkspaceId}" from the session's deploy panel. Never present a change as live in "${ctx.parentWorkspaceId}".`
		)
	} else if (ctx.forkParentUnknown) {
		lines.push(
			`- Operating workspace: "${ctx.workspaceId}" — a fork whose parent workspace is not currently visible to this user. deploy_workspace_item publishes into the fork only; the user promotes changes from the session's deploy panel. Never present a change as live in any other workspace.`
		)
	} else if (ctx.workspaceId) {
		lines.push(
			`- Operating workspace: "${ctx.workspaceId}" — the live workspace itself, not a fork. deploy_workspace_item publishes directly to everyone in it.`
		)
	} else {
		lines.push(
			'- No operating workspace is set yet; the user picks one (or a new staged fork) before the first message is sent.'
		)
	}
	return lines.join('\n')
}

/** `/` picker entry: a workspace skill or a built-in session action. The kind
 * drives the picker's category grouping; entries without one are ungrouped. */
export type ChatCommandItem = AiSkillListItem & { kind?: 'action' | 'skill' }

/** Fetch the workspace's AI skills (name + description) for the global system prompt. */
export async function loadWorkspaceSkills(workspace: string): Promise<AiSkillListItem[]> {
	if (!workspace) return []
	try {
		return await WorkspaceService.listAiSkills({ workspace })
	} catch (e) {
		console.error('Failed to load AI skills', e)
		return []
	}
}

const readSkillSchema = z.object({
	name: z
		.string()
		.describe('The exact skill name as listed in the Skills section of the system prompt.')
})

export const readSkillTool: Tool<{}> = {
	def: createToolDef(
		readSkillSchema,
		'read_skill',
		'Load the full instructions for a workspace AI skill by name. Skills are listed in the system prompt under "Skills"; call this before acting on a task a skill covers, then follow its instructions.'
	),
	fn: async ({ args, workspace, toolId, toolCallbacks }) => {
		const parsed = readSkillSchema.parse(args)
		toolCallbacks.setToolStatus(toolId, { content: `Reading skill "${parsed.name}"...` })
		try {
			const skill = await WorkspaceService.getAiSkill({ workspace, name: parsed.name })
			toolCallbacks.setToolStatus(toolId, { content: `Read skill "${parsed.name}"` })
			return `Skill: ${skill.name}\nDescription: ${skill.description}\n\nInstructions:\n${skill.instructions}`
		} catch (e) {
			const msg = e instanceof Error ? e.message : String(e)
			toolCallbacks.setToolStatus(toolId, {
				content: `Error reading skill "${parsed.name}"`,
				error: msg
			})
			return `Failed to read skill "${parsed.name}": ${msg}. Check the name against the Skills list in the system prompt.`
		}
	}
}

const OPEN_PAGE_NAMES = [
	'runs',
	'schedules',
	'variables',
	'resources',
	'assets',
	'audit_logs',
	'folders',
	'groups',
	'triggers',
	'workspace_settings',
	'compare'
] as const
type OpenPageName = (typeof OPEN_PAGE_NAMES)[number]

const OPEN_PAGE_LABELS: Record<OpenPageName, string> = {
	runs: 'Runs',
	schedules: 'Schedules',
	variables: 'Variables',
	resources: 'Resources',
	assets: 'Assets',
	audit_logs: 'Audit logs',
	folders: 'Folders',
	groups: 'Groups',
	triggers: 'Triggers',
	workspace_settings: 'Workspace settings',
	compare: 'Compare & Deploy'
}

// Trigger kinds available given the workspace's license — the EE-gated kinds
// (kafka/nats/sqs/gcp/azure) are only offered with an enterprise license.
function allowedTriggerKinds(): PageTriggerKind[] {
	const ee = !!get(enterpriseLicense)
	return (Object.keys(TRIGGER_PAGES) as PageTriggerKind[]).filter((k) => ee || !TRIGGER_PAGES[k].ee)
}

// Which pages the current user can actually reach — mirrors the sidebar's gating.
// Operators see exactly the pages enabled in the operating workspace's operator_settings
// (the same source OperatorMenu gates on); a missing/false flag means no access, and
// operators are never admins so workspace_settings is always excluded. Non-operators
// get every page except workspace_settings, which is admin/superadmin only. `workspaceId`
// is the chat's operating workspace (a session targets its own, possibly forked workspace);
// it defaults to the navigation workspace for the global side-panel chat. The tool only
// ever advertises, and only ever acts on, pages in this set.
function allowedOpenPages(workspaceId: string | undefined = get(workspaceStore)): OpenPageName[] {
	const u = get(userStore)
	const isAdmin = !!u?.is_admin || !!get(superadmin)
	if (u?.operator) {
		const settings = get(userWorkspaces).find((w) => w.id === workspaceId)?.operator_settings
		return OPEN_PAGE_NAMES.filter(
			(p) =>
				p !== 'workspace_settings' && settings?.[p as keyof NonNullable<typeof settings>] === true
		)
	}
	const allowed = new Set<OpenPageName>([
		'runs',
		'assets',
		'schedules',
		'variables',
		'resources',
		'audit_logs',
		'folders',
		'groups',
		'triggers',
		'compare'
	])
	if (isAdmin) allowed.add('workspace_settings')
	return OPEN_PAGE_NAMES.filter((p) => allowed.has(p))
}

// The advertised `items` description must match this chat's surface: only chats that
// track their modified items (AI sessions) can honor "omitted = this chat's edits" —
// on an untracked chat (the global side panel) an omitted mask falls through to the
// page's select-all default, so the model is told to pass the items explicitly there.
const COMPARE_ITEMS_DESCRIPTIONS = {
	tracked:
		"Compare: preselect exactly these changed items, each as '<kind>:<path>' where kind is script, flow, raw_app, app, resource, variable, or a trigger kind like trigger_schedule / trigger_http (e.g. 'script:f/foo/bar'). Omit to preselect the items modified in this chat (everything when this chat modified nothing).",
	untracked:
		"Compare: preselect exactly these changed items, each as '<kind>:<path>' where kind is script, flow, raw_app, app, resource, variable, or a trigger kind like trigger_schedule / trigger_http (e.g. 'script:f/foo/bar'). If omitted, the page preselects EVERY pending change in the workspace, not just this chat's — when you changed specific items, pass them so the review is scoped to them."
} as const

// One flat object (not a discriminated union): `page` selects the target and the
// per-page fields are optional. Top-level `type: object` is what Anthropic's
// input_schema requires; a top-level oneOf would be rejected. Each per-page URL builder
// drops any key that isn't one of its page's real query params, so a field that doesn't
// apply to the chosen page is harmless. This full schema is used to PARSE tool args; the
// advertised schema (what the model sees) is narrowed per-user in `setSchema`.
const openPageFullSchema = z.object({
	page: z.enum(OPEN_PAGE_NAMES).describe('Which page to open'),
	path: z
		.string()
		.optional()
		.describe(
			'Runs/Schedules/Variables/Resources/Assets: the script, flow or item path to filter by'
		),
	status: z
		.enum(['running', 'success', 'failure', 'canceled', 'waiting', 'suspended'])
		.optional()
		.describe('Runs: filter by job execution status'),
	schedule_path: z
		.string()
		.optional()
		.describe(
			'Runs: only runs triggered by this schedule path. Schedules: filter by this exact schedule path.'
		),
	job_kinds: z
		.enum(['all', 'runs', 'dependencies', 'previews', 'deploymentcallbacks'])
		.optional()
		.describe('Runs: filter by job category (defaults to top-level runs)'),
	user: z.string().optional().describe('Runs: filter by the user who created the job'),
	open: z
		.string()
		.optional()
		.describe(
			'Schedules/Triggers: exact schedule or trigger path to open in the edit drawer, e.g. f/foo/my_schedule'
		),
	summary: z
		.string()
		.optional()
		.describe('Schedules: search text matched against schedule summaries'),
	trigger_kind: z
		.enum([...(Object.keys(TRIGGER_PAGES) as [PageTriggerKind, ...PageTriggerKind[]])])
		.optional()
		.describe('Triggers: which trigger kind page to open (http, websocket, postgres, kafka, ...)'),
	resource_type: z
		.string()
		.optional()
		.describe('Resources: filter by resource type, e.g. postgres'),
	owner: z
		.string()
		.optional()
		.describe('Variables/Resources: filter by owner, e.g. u/alice or f/team'),
	username: z.string().optional().describe('Audit logs: filter by the acting username'),
	operation: z.string().optional().describe('Audit logs: filter by operation, e.g. jobs.run'),
	resource: z.string().optional().describe('Audit logs: filter by the affected resource path'),
	tab: z
		.enum([...WORKSPACE_SETTINGS_TABS] as [string, ...string[]])
		.optional()
		.describe('Workspace settings: which settings tab to open'),
	mode: z
		.enum(['draft', 'fork'])
		.optional()
		.describe(
			"Compare: which comparison to show — 'draft' (deployed items vs their pending drafts) or 'fork' (this forked workspace vs its parent). Omit to auto-pick: the view containing the preselected items (draft whenever any of them is a pending draft); with nothing preselected, fork on a forked workspace and draft otherwise."
		),
	items: z.array(z.string()).min(1).optional().describe(COMPARE_ITEMS_DESCRIPTIONS.tracked),
	new_tab: z
		.boolean()
		.optional()
		.describe(
			'Open in a NEW preview tab instead of updating the tab already showing this page. Only set true when the user explicitly asks for a new or separate tab; by default changing filters reuses the existing tab.'
		)
})

type OpenPageArgs = z.infer<typeof openPageFullSchema>

// Which pages each optional field applies to — drives narrowing the advertised schema
// so the model isn't shown fields for pages it can't open.
const OPEN_PAGE_FIELD_PAGES: Record<string, OpenPageName[]> = {
	path: ['runs', 'schedules', 'variables', 'resources', 'assets'],
	status: ['runs'],
	schedule_path: ['runs', 'schedules'],
	job_kinds: ['runs'],
	user: ['runs'],
	open: ['schedules', 'triggers'],
	summary: ['schedules'],
	trigger_kind: ['triggers'],
	resource_type: ['resources'],
	owner: ['variables', 'resources'],
	username: ['audit_logs'],
	operation: ['audit_logs'],
	resource: ['audit_logs'],
	tab: ['workspace_settings'],
	mode: ['compare'],
	items: ['compare']
}

// The model-facing schema for the given allowed pages: the `page` enum plus only the
// fields relevant to those pages (reusing the full schema's field definitions). The
// `trigger_kind` enum is narrowed to the license-available kinds.
function buildOpenPageDefSchema(
	pages: readonly OpenPageName[],
	triggerKinds: readonly PageTriggerKind[],
	chatEditsTracked: boolean
): z.ZodTypeAny {
	const full = openPageFullSchema.shape as Record<string, z.ZodTypeAny>
	// z.enum() rejects an empty list, and a user with no reachable pages (e.g. an operator
	// with every operator_settings flag off) yields exactly that. Fall back to a plain
	// string so schema-building can't throw; the handler still fails closed on every page.
	const shape: Record<string, z.ZodTypeAny> = {
		page: pages.length
			? z.enum([...pages] as [OpenPageName, ...OpenPageName[]]).describe('Which page to open')
			: z.string().describe('No pages are available to you in this workspace')
	}
	for (const [field, fieldPages] of Object.entries(OPEN_PAGE_FIELD_PAGES)) {
		if (!fieldPages.some((p) => pages.includes(p))) continue
		shape[field] =
			field === 'trigger_kind'
				? z
						.enum([...triggerKinds] as [string, ...string[]])
						.optional()
						.describe('Triggers: which trigger kind page to open')
				: field === 'items'
					? z
							.array(z.string())
							.min(1)
							.optional()
							.describe(COMPARE_ITEMS_DESCRIPTIONS[chatEditsTracked ? 'tracked' : 'untracked'])
					: full[field]
	}
	shape.new_tab = full.new_tab
	return z.object(shape)
}

const OPEN_PAGE_DESCRIPTION =
	'Open a Windmill page with filters applied — Runs, Schedules, Variables, Resources, Assets, Audit logs, Folders, Groups, Triggers (by kind), Workspace settings (on a specific tab), or the Compare & Deploy review page. Inside an AI session it opens as a tab in the side-panel preview next to the chat; elsewhere it offers a clickable link. Use after surfacing something the user likely wants to inspect (e.g. "show me the failed runs of X", "open the schedule for Y", "open the git sync settings", "open the kafka triggers"), and use page "compare" when the user wants to review and deploy pending changes (the items field controls which changes are preselected). This is the only way to show one of these pages in the session preview — open_preview only handles editable items (scripts, flows, raw apps, pipelines). Only pages listed for this user are available; do not offer others.'

// Non-arg inputs the URL builder needs: the chat's operating workspace (the compare
// page cannot fall back to its own store default inside a session preview) and the
// live modified-items mask backing the compare page's default preselection.
type OpenPageUrlCtx = { workspaceId: string; chatItems?: readonly string[] }

export function buildOpenPageUrl(page: OpenPageName, a: OpenPageArgs, ctx: OpenPageUrlCtx): string {
	switch (page) {
		case 'runs':
			return buildRunsUrl({
				status: a.status,
				path: a.path,
				schedule_path: a.schedule_path,
				job_kinds: a.job_kinds,
				user: a.user
			})
		case 'schedules':
			return buildSchedulesUrl({
				open: a.open,
				filters: { path: a.path, schedule_path: a.schedule_path, summary: a.summary }
			})
		case 'variables':
			return buildVariablesUrl({ path: a.path, owner: a.owner })
		case 'resources':
			return buildResourcesUrl({ path: a.path, resource_type: a.resource_type, owner: a.owner })
		case 'assets':
			return buildAssetsUrl({ path: a.path })
		case 'audit_logs':
			return buildAuditLogsUrl({
				username: a.username,
				operation: a.operation,
				resource: a.resource
			})
		case 'folders':
			return buildFoldersUrl()
		case 'groups':
			return buildGroupsUrl()
		case 'triggers':
			return buildTriggersUrl({
				// Default to HTTP routes when the model names no kind.
				trigger_kind: (a.trigger_kind as PageTriggerKind | undefined) ?? 'http',
				open: a.open
			})
		case 'workspace_settings':
			return buildWorkspaceSettingsUrl({ tab: a.tab })
		case 'compare':
			// Explicit `items` wins; otherwise preselect this chat's modified items. An
			// empty mask (chat modified nothing) passes no items so the page keeps its
			// select-all default instead of preselecting nothing.
			return buildCompareUrl({
				workspace_id: ctx.workspaceId,
				mode: a.mode,
				items: a.items ?? (ctx.chatItems?.length ? ctx.chatItems : undefined)
			})
	}
}

// Human-readable one-liner for the tool status/chip: the applied query params (and any
// hash target), or "all <page>" when unfiltered.
function summarizeOpenPage(url: string, page: OpenPageName): string {
	const u = new URL(url, 'http://x')
	if (page === 'compare') {
		// The raw params (workspace_id + a possibly long items list) are noise here —
		// summarize the selection instead.
		const parts: string[] = []
		const mode = u.searchParams.get('mode')
		if (mode) parts.push(`mode=${mode}`)
		const items = u.searchParams.get(COMPARE_ITEMS_PARAM)
		if (items) {
			const n = parseItemsMaskParam(items).size
			parts.push(`${n} item${n === 1 ? '' : 's'} preselected`)
		}
		return parts.length ? parts.join(', ') : 'all pending changes'
	}
	const parts: string[] = []
	u.searchParams.forEach((v, k) => parts.push(`${k}=${v}`))
	if (u.hash) parts.push(u.hash.slice(1))
	return parts.length ? parts.join(', ') : `all ${OPEN_PAGE_LABELS[page].toLowerCase()}`
}

export const openPageTool: Tool<{}> = {
	// The initial def assumes an untracked chat; setSchema below rebuilds it with the
	// caller's real surface before each iteration.
	def: createToolDef(
		buildOpenPageDefSchema(allowedOpenPages(), allowedTriggerKinds(), false),
		'open_page',
		OPEN_PAGE_DESCRIPTION
	),
	// Keep the row expanded so the link chip (attached below as an action) is visible
	// without the user having to expand the tool call.
	showDetails: true,
	autoCollapseDetails: false,
	// Re-narrow the advertised `page` enum to this user's permissions each iteration, so
	// the model never sees (or suggests) a page the user can't reach. Gates on the chat's
	// operating workspace (the session's, when different from the navigation workspace).
	setSchema: async function (helpers) {
		this.def = createToolDef(
			buildOpenPageDefSchema(
				allowedOpenPages(operatingWorkspaceFromHelpers(helpers)),
				allowedTriggerKinds(),
				(helpers as GlobalToolHelpers | undefined)?.getModifiedItems?.() !== undefined
			),
			'open_page',
			OPEN_PAGE_DESCRIPTION
		)
	},
	fn: async (ctx) => {
		const { args, toolId, toolCallbacks } = ctx
		const parsed = openPageFullSchema.parse(args)
		const page = parsed.page as OpenPageName
		const workspaceId = operatingWorkspaceFromHelpers(ctx.helpers)
		// Defense in depth: never act on a page the user can't reach, even if the model
		// requests one outside the advertised enum.
		if (!allowedOpenPages(workspaceId).includes(page)) {
			return `You don't have access to the ${OPEN_PAGE_LABELS[page] ?? page} page in this workspace.`
		}
		// Same fail-closed check for trigger_kind, which is narrowed to license-available
		// kinds in the advertised schema: a model ignoring that narrowing must not get an
		// EE-only trigger route built on a non-EE instance.
		const triggerKind = parsed.trigger_kind as PageTriggerKind | undefined
		if (page === 'triggers' && triggerKind && !allowedTriggerKinds().includes(triggerKind)) {
			return `${TRIGGER_PAGES[triggerKind].label} aren't available on this instance.`
		}
		const urlWorkspace = workspaceId ?? get(workspaceStore)
		if (!urlWorkspace) {
			return 'Error: no workspace is selected, so no page can be opened.'
		}
		const url = buildOpenPageUrl(page, parsed, {
			workspaceId: urlWorkspace,
			chatItems: (ctx.helpers as GlobalToolHelpers | undefined)?.getModifiedItems?.()
		})
		const pageLabel = OPEN_PAGE_LABELS[page]
		const summary = summarizeOpenPage(url, page)

		// In a session, show the page as a preview tab alongside the chat (the primary
		// UX). By default a filter change reuses the tab already showing this page;
		// new_tab forces a separate tab. openPagePreview returns undefined when there
		// is no active session, in which case we offer a link chip instead.
		const previewResult = openPagePreview({
			sessionId: sessionIdFromCtx(ctx),
			href: pageHref(url),
			label: pageLabel,
			newTab: parsed.new_tab ?? false
		})
		if (previewResult) {
			toolCallbacks.setToolStatus(toolId, { content: `Opened ${pageLabel} preview: ${summary}` })
			return previewResult
		}

		// Outside a session there is no preview panel — offer a clickable link the user
		// controls. (We deliberately don't navigate in place: a same-route goto would
		// not re-sync the page's URL-driven filter state, so the change wouldn't land.)
		toolCallbacks.setToolStatus(toolId, {
			content: `Prepared a link to ${pageLabel}: ${summary}`,
			actions: [{ id: `open-page:${page}:${url}`, type: 'navigate', label: summary, url, page }]
		})
		return `Offered the user a link to the ${pageLabel} page (${summary}). They can click it to open.`
	}
}

export const globalTools: Tool<{}>[] = [
	readSkillTool,
	openPageTool,
	{
		def: createToolDef(
			getInstructionsSchema,
			'get_instructions',
			'Get authoring guidance for scripts, flows, data pipelines, resources, apps, or the datatable SQL SDK (wmill.datatable()) used inside runnables.'
		),
		fn: async (ctx) => {
			const { args, toolId, toolCallbacks } = ctx
			const parsed = getInstructionsSchema.parse(args)
			// Session chats (explicit isSessionChat helper) don't get pipeline
			// authoring while pipelines are gated; the standalone global chat keeps it.
			if (
				parsed.subject === 'pipeline' &&
				isSessionChatFromCtx(ctx) &&
				!isSessionPipelinesEnabled()
			) {
				toolCallbacks.setToolStatus(toolId, { content: 'Data pipelines are in alpha' })
				return SESSION_PIPELINES_GATED_MESSAGE
			}
			const label =
				parsed.subject === 'script' && parsed.language
					? `${parsed.subject} (${parsed.language})`
					: parsed.subject
			toolCallbacks.setToolStatus(toolId, { content: `Loaded ${label} instructions` })
			return getInstructions(parsed.subject, parsed.language)
		}
	},
	createSearchHubScriptsTool(false),
	searchNpmPackagesTool,
	searchDocsTool,
	readDocsPageTool,
	{
		def: createToolDef(
			askUserQuestionSchema,
			'askUserQuestion',
			'Ask the user a question with proposed answers and wait for their selected or custom answer before continuing.'
		),
		streamingLabel: 'Asking the user a question...',
		fn: async ({ args, toolId, toolCallbacks }) => {
			const parsed = askUserQuestionSchema.parse(args)
			const userQuestion = {
				question: parsed.question,
				choices: parsed.choices,
				multiSelect: parsed.multiSelect
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Asking user: ${parsed.question}`,
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

			const selected = await toolCallbacks.requestUserQuestion(toolId, userQuestion)
			if (!selected?.length) {
				const message = 'Question cancelled by user'
				toolCallbacks.setToolStatus(toolId, {
					content: `Asked: ${parsed.question} — cancelled by user`,
					userQuestion: { ...userQuestion, canceled: true },
					isLoading: false,
					error: message
				})
				return JSON.stringify({ success: false, error: message })
			}

			// Model-facing answer: bare string for one pick (preserves the single-select
			// contract, even when multiSelect was set), newline-bulleted list for several.
			// Comma-joining is avoided here so a choice that itself contains a comma
			// ("Yes, immediately") stays unambiguous to the model reading it back.
			const answerText =
				selected.length === 1 ? selected[0] : selected.map((c) => `- ${c}`).join('\n')
			// The collapsed tool-header is a human glance, not model input, so it carries
			// the question plus the picks as a compact comma list (not a bullet list) —
			// it is the only place the exchange stays readable in the transcript.
			const answerSummary = selected.join(', ')

			toolCallbacks.setToolStatus(toolId, {
				content: `Asked: ${parsed.question} — ${answerSummary}`,
				userQuestion: {
					...userQuestion,
					selectedChoices: selected
				},
				result: answerText,
				isLoading: false
			})
			return answerText
		}
	},
	{
		def: createToolDef(
			updateUserInstructionsSchema,
			'update_user_instructions',
			'Modify your own persistent personal instructions for the Global assistant. Use when the user asks you to remember a preference, always/never do something, or change/stop a behavior. Edits only the user-level instructions (the USER INSTRUCTIONS block, not workspace-level); changes persist in this browser and take effect on your next message.'
		),
		showDetails: true,
		fn: async ({ args, toolId, toolCallbacks, helpers }) => {
			const parsed = updateUserInstructionsSchema.parse(args)
			const h = helpers as GlobalToolHelpers
			if (!h?.getUserInstructions || !h?.setUserInstructions) {
				const message = 'This chat context cannot modify user instructions.'
				toolCallbacks.setToolStatus(toolId, { content: message, error: message })
				return message
			}

			const current = h.getUserInstructions()
			let next: string
			if (parsed.operation === 'append') {
				const text = parsed.text?.trim()
				if (!text) {
					const message = "operation 'append' requires a non-empty text."
					toolCallbacks.setToolStatus(toolId, { content: message, error: message })
					return message
				}
				next = current.trim() ? `${current.trim()}\n\n${text}` : text
			} else {
				if (parsed.old_string === undefined || parsed.new_string === undefined) {
					const message = "operation 'replace' requires old_string and new_string."
					toolCallbacks.setToolStatus(toolId, { content: message, error: message })
					return message
				}
				try {
					next = findAndReplace(
						current,
						parsed.old_string,
						parsed.new_string,
						parsed.replace_all ?? false,
						'personal instructions'
					).trim()
				} catch (e) {
					const detail = e instanceof Error ? e.message : String(e)
					// Echo the current text on this rare recovery path so the model can rebuild a
					// correct old_string. The system prompt's USER INSTRUCTIONS block can be stale
					// within a turn that already made a successful edit, but `current` is always live.
					const hint = current.trim()
						? `Your current personal instructions are:\n${current.trim()}`
						: "You have no personal instructions yet; use operation 'append' to add one."
					const message = `${detail} ${hint}`
					toolCallbacks.setToolStatus(toolId, { content: detail, error: detail })
					return message
				}
			}

			if (next.length > MAX_USER_INSTRUCTIONS_LENGTH) {
				const message = `Resulting instructions would be ${next.length} characters, over the ${MAX_USER_INSTRUCTIONS_LENGTH} limit. Make them more concise.`
				toolCallbacks.setToolStatus(toolId, { content: message, error: message })
				return message
			}

			h.setUserInstructions(next)

			const summary = next
				? parsed.operation === 'append'
					? 'Added a personal instruction'
					: 'Updated your personal instructions'
				: 'Cleared your personal instructions'
			toolCallbacks.setToolStatus(toolId, { content: summary, result: summary })
			// Return only a short confirmation. The updated text is re-injected into the system
			// prompt on the next iteration, so echoing it back here would waste context.
			return `${summary}. It takes effect from your next message and is editable in the Global chat settings.`
		}
	},
	{
		def: createToolDef(
			listWorkspaceItemsSchema,
			'list_workspace_items',
			'List workspace items and drafts. Returns metadata only, up to limit items per item type per page (default 50); pass page to continue past a full page.'
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
				Math.min(limit, MAX_LIST_LIMIT),
				parsed.page
			)
			for (const item of workspaceItems) {
				byKey.set(getWorkspaceItemKey(item.type, item.path, item.triggerKind), item)
			}

			// Drafts are not paginated server-side; overlay them on page 1 only,
			// capped at `limit` per type, so results stay bounded and later pages
			// never repeat a page-1 item as its draft twin. Chat draft counts are
			// small — past the cap, a narrower path_prefix still finds any draft
			// (it filters before the cap; query filters after).
			if ((parsed.page ?? 1) === 1) {
				const draftCountByType = new Map<string, number>()
				for (const draft of await listGlobalDrafts(workspace)) {
					if (!types.includes(draft.type)) continue
					if (parsed.path_prefix && !draft.path.startsWith(parsed.path_prefix)) continue
					const count = draftCountByType.get(draft.type) ?? 0
					if (count >= limit) continue
					draftCountByType.set(draft.type, count + 1)
					byKey.set(getWorkspaceItemKey(draft.type, draft.path, draft.triggerKind), {
						...draft,
						value: undefined
					})
				}
			}

			// No cross-type truncation: each type is already capped at `limit` rows by
			// its own list call, and slicing the concatenation would silently drop the
			// later types' rows while their next page skips past them.
			const results = Array.from(byKey.values()).filter((item) => itemMatches(item, parsed.query))

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
			'Read one workspace item or draft. Prefers your draft when one exists; pass version: "deployed" to read the deployed state instead.'
		),
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsed = readWorkspaceItemSchema.parse(args)
			if (parsed.type === 'trigger' && !parsed.trigger_kind) {
				const message = 'trigger_kind is required when type is trigger.'
				toolCallbacks.setToolStatus(toolId, { content: message, error: message })
				return JSON.stringify({ success: false, error: message })
			}
			const draft =
				parsed.version === 'deployed'
					? null
					: await getGlobalDraft(workspace, parsed.type, parsed.path, parsed.trigger_kind)
			if (draft) {
				toolCallbacks.setToolStatus(toolId, {
					content: `Read draft ${parsed.type} "${parsed.path}"`
				})
				return JSON.stringify(serializeWorkspaceItemForRead(draft), null, 2)
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Reading ${parsed.type} "${parsed.path}"...`
			})
			const item = await readWorkspaceItem(
				parsed.type,
				parsed.path,
				workspace,
				parsed.trigger_kind,
				parsed.version === 'deployed'
			)
			toolCallbacks.setToolStatus(toolId, { content: `Read ${parsed.type} "${parsed.path}"` })
			return JSON.stringify(serializeWorkspaceItemForRead(item), null, 2)
		}
	},
	{
		def: createToolDef(
			createFolderSchema,
			'create_folder',
			'Create a new shared folder (addressable as f/<name>/) in the workspace. The current user is added as an owner.'
		),
		requiresConfirmation: true,
		confirmationMessage: 'Create folder',
		showDetails: true,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsed = createFolderSchema.parse(args)
			if (!VALID_FOLDER_NAME.test(parsed.name)) {
				const error =
					'Folder name can only contain alphanumeric characters, underscores, and hyphens.'
				toolCallbacks.setToolStatus(toolId, { content: error, error })
				return JSON.stringify({ success: false, error })
			}
			toolCallbacks.setToolStatus(toolId, { content: `Creating folder \`f/${parsed.name}\`...` })
			try {
				await FolderService.createFolder({
					workspace,
					requestBody: { name: parsed.name, summary: parsed.summary }
				})
				// Reflect the new folder in the path-convention context for the rest of this
				// session, matching FolderPicker's local update (avoids userStore.set()).
				const user = get(userStore)
				if (user) {
					if (!user.folders) user.folders = []
					if (!user.folders.includes(parsed.name)) user.folders.push(parsed.name)
				}
				const message = `Created folder \`f/${parsed.name}\`. You can now write items to \`f/${parsed.name}/<name>\`.`
				toolCallbacks.setToolStatus(toolId, { content: message })
				return JSON.stringify({ success: true, message })
			} catch (e) {
				const error = e instanceof Error ? e.message : String(e)
				toolCallbacks.setToolStatus(toolId, { content: `Error: ${error}`, error })
				return JSON.stringify({ success: false, error })
			}
		}
	},
	{
		def: createToolDef(writeScriptSchema, 'write_script', 'Create or overwrite a draft script.'),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = writeScriptSchema.parse(ctx.args)
			return writeScriptDraft(parsed, ctx)
		}
	},
	{
		def: createToolDef(writeFlowSchema, 'write_flow', 'Create or overwrite a draft flow.'),
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
				groups: parseOptionalJsonArg(parsed.groups, 'groups'),
				notes: parseOptionalJsonArg(parsed.notes, 'notes')
			})
			return writeFlowDraft(
				{
					path: parsed.path,
					summary: parsed.summary,
					description: parsed.description,
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
			'Create or overwrite a draft schedule.',
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
			'Create or overwrite a draft trigger.',
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
			'Find/replace exact text in a script and save a draft.'
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
			'Find/replace exact text in compact flow JSON and save a draft.'
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
		def: testRunScriptToolDef,
		fn: async (ctx) => {
			const parsed = testRunScriptSchema.parse(ctx.args)
			return testRunScriptByPath(parsed, ctx)
		},
		requiresConfirmation: true,
		confirmationMessage: (args) => `Run a test of ${pathLeaf(args?.path, 'the script')}`,
		showDetails: true,
		autoCollapseDetails: false
	},
	{
		def: testRunFlowToolDef,
		fn: async (ctx) => {
			const parsed = testRunFlowSchema.parse(ctx.args)
			return testRunFlowByPath(parsed, ctx)
		},
		requiresConfirmation: true,
		confirmationMessage: (args) => `Run a test of ${pathLeaf(args?.path, 'the flow')}`,
		showDetails: true,
		autoCollapseDetails: false
	},
	{
		def: testRunStepToolDef,
		fn: async (ctx) => {
			const parsed = testRunStepSchema.parse(ctx.args)
			return testRunFlowStepByPath(parsed, ctx)
		},
		requiresConfirmation: true,
		confirmationMessage: (args) =>
			`Run a test of step "${args?.stepId ?? ''}" in ${pathLeaf(args?.path, 'the flow')}`,
		showDetails: true,
		autoCollapseDetails: false
	},
	{
		def: createToolDef(
			listRunsSchema,
			'list_runs',
			"List recent runs (jobs), most recent first. Optionally filter by path, creator, label, or status. Returns compact metadata only — use get_job_logs with a returned id to read a run's logs."
		),
		showDetails: true,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsed = listRunsSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, { content: 'Listing runs...' })
			const jobs = await JobService.listJobs({
				workspace,
				scriptPathExact: parsed.path,
				createdBy: parsed.created_by,
				label: parsed.label,
				success: parsed.success,
				running: parsed.running,
				perPage: parsed.limit ?? 30
			})
			const runs = jobs.map(summarizeRun)
			const result = JSON.stringify(runs, null, 2)
			toolCallbacks.setToolStatus(toolId, {
				content: `Listed ${runs.length} run(s)`,
				result
			})
			return result
		}
	},
	{
		def: createToolDef(
			getJobLogsSchema,
			'get_job_logs',
			'Fetch the logs of a job by its id. Use this to inspect the output of an existing run.'
		),
		showDetails: true,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsed = getJobLogsSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, {
				content: `Fetching logs for job ${parsed.id}...`
			})
			const logs = await JobService.getJobLogs({
				workspace,
				id: parsed.id,
				// Always suppress the "to remove ansi colors, use: sed ..." hint the
				// backend otherwise prepends — it is noise for the model and is not
				// actual ANSI stripping (the raw logs are returned either way).
				removeAnsiWarnings: true
			})
			const hasLogs = typeof logs === 'string' && logs.trim().length > 0
			const result = hasLogs ? logs : 'No logs available for this job.'
			toolCallbacks.setToolStatus(toolId, {
				content: hasLogs
					? `Fetched logs for job ${parsed.id}`
					: `No logs available for job ${parsed.id}`,
				result
			})
			return result
		}
	},
	{
		def: createToolDef(
			cancelJobSchema,
			'cancel_job',
			'Cancel a running or queued job by its id (e.g. a background test run you started that is no longer needed).'
		),
		showDetails: true,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsed = cancelJobSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, { content: `Canceling job ${parsed.id}...` })
			try {
				await JobService.cancelQueuedJob({ workspace, id: parsed.id, requestBody: {} })
			} catch (e) {
				const msg = e instanceof Error ? e.message : 'Unknown error'
				toolCallbacks.setToolStatus(toolId, {
					content: `Could not cancel job ${parsed.id}`,
					error: msg
				})
				return `Failed to cancel job ${parsed.id}: ${msg}. It may have already finished.`
			}
			// The tray's background poller will pick up the canceled state (updating
			// its status + Job snapshot together); a bare status patch here would leave
			// the badge stale and stop the poller. Just report to the model.
			toolCallbacks.setToolStatus(toolId, { content: `Canceled job ${parsed.id}` })
			return `Job ${parsed.id} was canceled.`
		}
	},
	{
		def: createToolDef(
			deployWorkspaceItemSchema,
			'deploy_workspace_item',
			'Deploy a draft to the workspace. Mutates the workspace.',
			{ strict: false }
		),
		showDetails: true,
		showFade: true,
		requiresConfirmation: true,
		confirmationMessage: 'Deploy draft to workspace',
		fn: async (ctx) => {
			const parsed = deployWorkspaceItemSchema.parse(ctx.args)
			return deployDraft(parsed, { ...ctx, sessionId: sessionIdFromCtx(ctx) })
		}
	},
	{
		def: createToolDef(
			rebaseDraftSchema,
			'rebase_draft',
			'Discard a stale script, flow, or app draft and return your changes as a diff to re-apply on the latest deployed version. Use when deploy_workspace_item reports the draft was started from an older deployed version.',
			{ strict: false }
		),
		showDetails: true,
		showFade: true,
		fn: async (ctx) => {
			const parsed = rebaseDraftSchema.parse(ctx.args)
			return rebaseDraft(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			diffSchema,
			'diff',
			"Diff workspace changes. Read-only. Default: drafts vs deployed versions (index without type/path, one item's unified diff with them; file=<name> for one file inside an app). against='parent_workspace': deployed fork vs its parent workspace. search=<text> greps changed lines across all diffs."
		),
		showDetails: true,
		fn: async (ctx) => {
			const parsed = diffSchema.parse(ctx.args)
			if (parsed.search !== undefined) {
				return diffSearch(parsed, ctx)
			}
			if (parsed.against === 'parent_workspace') {
				return parsed.path !== undefined ? diffForkItem(parsed, ctx) : diffForkIndex(parsed, ctx)
			}
			return parsed.path !== undefined
				? diffWorkspaceItem(parsed, ctx)
				: diffWorkspaceIndex(parsed, ctx)
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
			'Discard a draft only. Does not mutate deployed workspace items, but clears the matching open editor draft if one is mounted.'
		),
		showDetails: true,
		showFade: true,
		requiresConfirmation: true,
		confirmationMessage: 'Discard draft',
		fn: async (ctx) => {
			const parsed = discardLocalDraftSchema.parse(ctx.args)
			return discardLocalDraft(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			writeResourceSchema,
			'write_resource',
			'Create or overwrite a draft resource.',
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
			'Create or overwrite a draft variable.',
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
	createDbSchemaTool<{}>({
		description:
			'Fetch the schema (tables and columns) of a database resource by its path. Supports postgresql, mysql, ms_sql_server, snowflake and bigquery resources.',
		updateEditorCache: false
	}),
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
			'Overwrite inline script code in one flow module and save a draft.'
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
			'Initialize a draft raw app from a framework template.',
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
			'Read one raw app frontend file or inline backend runnable. Large files are truncated to a head slice; pass offset/limit to page through the rest.'
		),
		fn: async (ctx) => {
			const parsed = readAppFileSchema.parse(ctx.args)
			return readAppFile(parsed, ctx)
		}
	},
	{
		def: createToolDef(
			searchAppSchema,
			'search_app',
			"Grep across all of a raw app's frontend files and inline backend runnables in one call. Returns matching file:line rows (capped), not file bodies — use it to locate a symbol or string before read_app_file instead of reading whole files one by one."
		),
		fn: async (ctx) => {
			const parsed = searchAppSchema.parse(ctx.args)
			return searchApp(parsed, ctx)
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
			'Find/replace exact text in a raw app file and save a draft.'
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
	},
	...artifactTools,
	{
		def: createToolDef(
			openPreviewSchema,
			'open_preview',
			'Open the live preview / editor for a workspace item in the side panel next to the chat. ONLY works inside an AI session — call this after writing or editing a script, flow, or raw app to let the user see and interact with it. The path you pass is the path of the item; for code-based apps use kind="raw_app" (legacy drag-and-drop apps are not previewable). Returns an error if there is no active session.'
		),
		fn: async (ctx) => {
			const parsed = openPreviewSchema.parse(ctx.args)
			return openSessionPreview(parsed, sessionIdFromCtx(ctx))
		}
	},
	{
		def: createToolDef(
			getPreviewStatusSchema,
			'get_preview_status',
			'Check whether the side-panel preview is open in this AI session and which item (kind + path) it is showing. Call this before offering or calling open_preview so you do not re-open a preview that is already showing the item you just edited. Only meaningful inside a session.'
		),
		fn: async (ctx) => getSessionPreviewStatus(sessionIdFromCtx(ctx))
	},
	{
		def: createToolDef(
			closePageSchema,
			'close_page',
			'Close one or more preview tabs in the side panel of this AI session. Pass `match` to close the tab(s) whose page name or item path contains that text, or `all: true` to clear the panel. Use this when the user asks to close/dismiss a tab they no longer need. Only works inside a session.'
		),
		fn: async (ctx) => {
			const parsed = closePageSchema.parse(ctx.args)
			return closeSessionPreviewTabs(parsed, sessionIdFromCtx(ctx))
		}
	},
	{
		def: createToolDef(
			getRuntimeLogsSchema,
			'get_app_runtime_logs',
			'Fetch the most recent browser console logs (and uncaught errors) from the raw app preview currently open in this AI session.'
		),
		showDetails: true,
		autoCollapseDetails: false,
		fn: async (ctx) => {
			const parsed = getRuntimeLogsSchema.parse(ctx.args)
			ctx.toolCallbacks.setToolStatus(ctx.toolId, { content: 'Reading app runtime logs...' })
			const result = await getSessionRuntimeLogs(parsed.limit ?? 10, sessionIdFromCtx(ctx))
			ctx.toolCallbacks.setToolStatus(ctx.toolId, {
				content: result.uiMessage,
				result: result.toolResult
			})
			return result.aiResult
		}
	},
	{
		def: createToolDef(
			listAppRunsSchema,
			'list_app_runs',
			'List the backend runnable executions (jobs) the raw app preview currently open in this AI session has triggered, newest first.'
		),
		showDetails: true,
		fn: async (ctx) => {
			const parsed = listAppRunsSchema.parse(ctx.args)
			ctx.toolCallbacks.setToolStatus(ctx.toolId, { content: 'Listing app runs...' })
			const result = await getSessionAppRuns(parsed.limit ?? 20, sessionIdFromCtx(ctx))
			ctx.toolCallbacks.setToolStatus(ctx.toolId, {
				content: result.uiMessage,
				result: result.toolResult
			})
			return result.aiResult
		}
	},
	{
		def: createToolDef(
			searchDomSchema,
			'search_dom',
			'Search the live rendered HTML of the raw app preview open in this AI session with a regex, returning matching lines with their line numbers. Use it to check what actually rendered (verify an edit landed, diagnose a blank/empty view). Scope to an element with `selector`, or omit it for the whole page. The DOM is read live, so it reflects the current state.'
		),
		showDetails: true,
		fn: async (ctx) => {
			const parsed = searchDomSchema.parse(ctx.args)
			ctx.toolCallbacks.setToolStatus(ctx.toolId, { content: 'Searching app DOM...' })
			const result = await getSessionDom(
				{
					mode: 'search',
					appPath: parsed.app_path,
					selector: parsed.selector,
					pattern: parsed.pattern,
					ignoreCase: parsed.ignore_case
				},
				sessionIdFromCtx(ctx)
			)
			ctx.toolCallbacks.setToolStatus(ctx.toolId, {
				content: result.uiMessage,
				result: result.toolResult
			})
			return result.aiResult
		}
	},
	{
		def: createToolDef(
			readDomSchema,
			'read_dom',
			'Read a bounded window of the live rendered HTML of the raw app preview open in this AI session, pretty-printed and line-numbered. Scope to an element with `selector`, or omit it for the whole page. Use search_dom first to locate content, then read_dom to see a specific region. The DOM is read live.'
		),
		showDetails: true,
		fn: async (ctx) => {
			const parsed = readDomSchema.parse(ctx.args)
			ctx.toolCallbacks.setToolStatus(ctx.toolId, { content: 'Reading app DOM...' })
			const result = await getSessionDom(
				{
					mode: 'read',
					appPath: parsed.app_path,
					selector: parsed.selector,
					startLine: parsed.start_line,
					endLine: parsed.end_line
				},
				sessionIdFromCtx(ctx)
			)
			ctx.toolCallbacks.setToolStatus(ctx.toolId, {
				content: result.uiMessage,
				result: result.toolResult
			})
			return result.aiResult
		}
	},
	{
		def: createToolDef(
			takeScreenshotSchema,
			'take_screenshot',
			// Keep this short: every global session iteration re-sends it. How to read
			// the result belongs on the result, where only a real capture pays for it.
			'Capture a screenshot of the raw app preview currently open in this AI session and attach it as an image so you can see the rendered UI. Use it when the user raises how the app looks, whether reporting a problem or asking for the design improved, rather than to check your own edits. The image is attached in the following message. Requires the raw app preview open (open_preview kind="raw_app").'
		),
		showDetails: true,
		fn: async (ctx) => {
			// A known text-only model would reject the follow-up image message and fail
			// the turn, so refuse before capturing rather than buffer an image it can
			// never read. The model is re-read here because it can change between turns.
			const model = tryGetCurrentModel()
			if (model && !modelSupportsVision(model.provider, model.model)) {
				const cannotSee = `${model.model} cannot read images, so a screenshot would be discarded. Ask the user to describe what looks wrong, or to switch to a model that supports images.`
				ctx.toolCallbacks.setToolStatus(ctx.toolId, {
					content: `${model.model} cannot read images`,
					error: cannotSee
				})
				return cannotSee
			}
			ctx.toolCallbacks.setToolStatus(ctx.toolId, { content: 'Capturing screenshot...' })
			const result = await getSessionScreenshot(sessionIdFromCtx(ctx))
			if (!result.dataUrl) {
				ctx.toolCallbacks.setToolStatus(ctx.toolId, {
					content: result.uiMessage ?? 'Screenshot unavailable',
					error: result.error
				})
				return result.error ?? 'Could not capture the app preview.'
			}
			// Normalize (downscale + png/jpeg) so history/context never carry a full-res blob;
			// buffered here and flushed as a follow-up user image message once the tool batch
			// completes (see appendPendingToolImages).
			const image = await normalizeImageDataUrl(result.dataUrl)
			ctx.toolCallbacks.attachToolImage?.(ctx.toolId, image)
			// The card shows the same copy the model gets; sharing the exact data URL
			// lets the history's blob store persist one copy for both.
			ctx.toolCallbacks.setToolStatus(ctx.toolId, {
				content: 'Screenshot captured',
				imageUrl: image.dataUrl
			})
			return (
				'Screenshot captured; the image is attached in the following message.\n\n' +
				'It is rebuilt from the DOM rather than captured from the screen, so it can differ from what the user sees, and it differs by browser. Treat what you see as real and fix it. Before dismissing anything as a capture artifact, read the source for that element and name the specific cause; if you cannot, it is a real bug. If you are still unsure, say what looks wrong and ask the user to screenshot it themselves and drag the image into the chat rather than guessing.'
			)
		}
	},
	// Workspace-scoped datatable tools (unrestricted: no whitelist, no creation policy)
	...getDatatableTools(),
	// Read-only tools over files the user attached to the conversation
	...fileTools,
	// Search + call access to the backend API endpoint catalog, for operations
	// no dedicated tool covers
	...apiCatalogTools
]

// Tools that only make sense inside an AI session (they drive the session's
// side-panel preview). The regular global side-panel chat shouldn't even be
// offered them — see `globalToolsFor`.
export const SESSION_PREVIEW_TOOL_NAMES = new Set([
	'open_preview',
	'get_preview_status',
	'close_page',
	'get_app_runtime_logs',
	'list_app_runs',
	'search_dom',
	'read_dom',
	'take_screenshot',
	'create_artifact',
	'update_artifact',
	'list_artifacts',
	'read_artifact'
])

/**
 * The global tool set for a given chat: the full `globalTools` for a session
 * chat, or `globalTools` minus the session-only preview tools for the regular
 * global side-panel chat.
 */
export function globalToolsFor({ sessionPreview }: { sessionPreview: boolean }): Tool<{}>[] {
	const tools = sessionPreview
		? globalTools
		: globalTools.filter((t) => !SESSION_PREVIEW_TOOL_NAMES.has(t.def.function.name))
	// DOM capture re-renders the app through the engine's SVG-image path, which is
	// only faithful on Blink — Gecko/WebKit shift text spacing and wrapping (font
	// fallback, sub-pixel rounding). Elsewhere the tool is withheld entirely and
	// the system prompt tells the agent to ask the user for a screenshot instead.
	return isChromiumBrowser()
		? tools
		: tools.filter((t) => t.def.function.name !== 'take_screenshot')
}

type WriteDraftCtx = {
	workspace: string
	toolId: string
	toolCallbacks: ToolCallbacks
	// Calling session id (session chats only), threaded through so a deploy
	// reloads the preview of the session that issued the deploy — not the
	// UI-active one. Undefined for the global side-panel chat.
	sessionId?: string
}

// Sessions are the only context where `open_preview` makes sense — the global
// singleton chat in the right side panel has nowhere to mount an editor pane.
// The session runtime registers a handler at construction time so the tool
// has somewhere to dispatch. When no session is active the handler is
// undefined and the tool returns a polite error.
// Per-manager tool helpers for a session chat. Each session's AIChatManager
// sets `helpers = { sessionId }`, so a tool call carries the *calling* session's
// id even when a different session is the UI-active one. Without this the
// handlers below would route a backgrounded session's tool call to whatever
// session the user happens to be viewing.
export type SessionToolHelpers = { sessionId?: string }

export type GlobalToolHelpers = SessionToolHelpers & {
	testActiveFlow?: (args?: Record<string, any>) => Promise<string | undefined>
	attachedFiles?: AttachedFilesStore
	// Read/write the user-level Global instructions. `setUserInstructions` persists the
	// value and rebuilds the system message so the change applies on the next chat-loop
	// iteration. Backed by the update_user_instructions tool.
	getUserInstructions?: () => string
	setUserInstructions?: (instructions: string) => void
	// The workspace this chat actually operates on — a session chat targets its own
	// (possibly forked) workspace while $workspaceStore stays on the navigation workspace,
	// so permission gating (open_page) must read this, not the global store.
	operatingWorkspace?: string
	// Wired only for session chats (see AIChatManager): the artifact tools are session-gated.
	artifacts?: SessionArtifactsStore
	getChatId?: () => string | undefined
	// Live snapshot of the items this chat modified (`kind:path` mask keys, see
	// modifiedItemsMask.ts); undefined when the chat doesn't track them (the global
	// side-panel chat). Backs open_page's compare-page default preselection.
	getModifiedItems?: () => string[] | undefined
	openArtifact?: (artifactId: string, name: string) => void
	// Explicit "this chat is an AI session" marker for session-scoped gating
	// (the pipeline gate). Do NOT infer it from `sessionId`: the eval harness
	// passes a sessionId to its standalone (non-session) chats too.
	isSessionChat?: boolean
}

function sessionIdFromCtx(ctx: { helpers?: unknown }): string | undefined {
	return (ctx.helpers as GlobalToolHelpers | undefined)?.sessionId
}

function isSessionChatFromCtx(ctx: { helpers?: unknown }): boolean {
	return (ctx.helpers as GlobalToolHelpers | undefined)?.isSessionChat === true
}

function operatingWorkspaceFromHelpers(helpers: unknown): string | undefined {
	return (helpers as GlobalToolHelpers | undefined)?.operatingWorkspace
}

function activeFlowTestFromCtx(
	ctx: { workspace: string; helpers?: unknown },
	path: string
): GlobalToolHelpers['testActiveFlow'] | undefined {
	const activeEditor = getActiveGlobalEditorContext(ctx.workspace)
	if (activeEditor?.type !== 'flow' || activeEditor.path !== path) {
		return undefined
	}
	return (ctx.helpers as GlobalToolHelpers | undefined)?.testActiveFlow
}

export type OpenPreviewHandler = (req: {
	sessionId: string | undefined
	kind: 'script' | 'flow' | 'raw_app' | 'pipeline'
	path: string
}) => string

let openPreviewHandler: OpenPreviewHandler | undefined

export function setOpenPreviewHandler(handler: OpenPreviewHandler | undefined): void {
	openPreviewHandler = handler
}

function openSessionPreview(
	args: { kind: 'script' | 'flow' | 'raw_app' | 'pipeline'; path: string },
	sessionId: string | undefined
) {
	if (!openPreviewHandler) {
		return 'Error: open_preview is only available inside an AI session. Tell the user to switch to a session to view the preview, or describe the item textually.'
	}
	// open_preview only exists in sessions, so no sessionId check is needed here.
	if (args.kind === 'pipeline' && !isSessionPipelinesEnabled()) {
		return SESSION_PIPELINES_GATED_MESSAGE
	}
	return openPreviewHandler({ ...args, sessionId })
}

// Opens a workspace *page* (Runs, Schedules, …) as a page tab in the session's
// side-panel preview, so open_page can show it next to the chat instead of
// navigating the whole browser. Registered by the session runtime. Returns a
// status string when opened in a session, or undefined when there is no active
// session — signalling open_page to fall back to a link chip / direct navigation.
export type OpenPagePreviewHandler = (req: {
	sessionId: string | undefined
	href: string
	label: string
	// Force a separate tab instead of reusing the tab already showing this page.
	newTab: boolean
}) => string | undefined

let openPagePreviewHandler: OpenPagePreviewHandler | undefined

export function setOpenPagePreviewHandler(handler: OpenPagePreviewHandler | undefined): void {
	openPagePreviewHandler = handler
}

function openPagePreview(req: {
	sessionId: string | undefined
	href: string
	label: string
	newTab: boolean
}): string | undefined {
	return openPagePreviewHandler?.(req)
}

// Companion to `open_preview`: lets the assistant query the current preview
// state (open? which item?) so it can avoid re-opening a preview already
// showing the item it just edited. Registered by the session runtime
// alongside the open-preview handler.
export type GetPreviewStatusHandler = (sessionId: string | undefined) => string

let getPreviewStatusHandler: GetPreviewStatusHandler | undefined

export function setGetPreviewStatusHandler(handler: GetPreviewStatusHandler | undefined): void {
	getPreviewStatusHandler = handler
}

function getSessionPreviewStatus(sessionId: string | undefined): string {
	if (!getPreviewStatusHandler) {
		return 'Error: get_preview_status is only available inside an AI session.'
	}
	return getPreviewStatusHandler(sessionId)
}

// Closes preview tabs in the calling session's side panel. Registered by the
// session runtime, which owns the tab model. Returns a status string the model
// relays to the user. `all` clears the panel; otherwise `match` is a
// case-insensitive substring tested against each tab's page label / item path.
export type ClosePreviewTabsHandler = (req: {
	sessionId: string | undefined
	all: boolean
	match: string | undefined
}) => string

let closePreviewTabsHandler: ClosePreviewTabsHandler | undefined

export function setClosePreviewTabsHandler(handler: ClosePreviewTabsHandler | undefined): void {
	closePreviewTabsHandler = handler
}

function closeSessionPreviewTabs(
	args: { all?: boolean; match?: string },
	sessionId: string | undefined
): string {
	if (!closePreviewTabsHandler) {
		return 'Error: close_page is only available inside an AI session.'
	}
	if (!args.all && !args.match?.trim()) {
		return 'Nothing to close: pass `match` with the tab to close, or `all: true` to close every tab.'
	}
	return closePreviewTabsHandler({ sessionId, all: args.all ?? false, match: args.match })
}

export type GetRuntimeLogsHandler = (req: {
	sessionId: string | undefined
	limit: number
}) => Promise<SessionToolResult>

let getRuntimeLogsHandler: GetRuntimeLogsHandler | undefined

export function setGetRuntimeLogsHandler(handler: GetRuntimeLogsHandler | undefined): void {
	getRuntimeLogsHandler = handler
}

function getSessionRuntimeLogs(
	limit: number,
	sessionId: string | undefined
): Promise<SessionToolResult> {
	if (!getRuntimeLogsHandler) {
		return Promise.resolve({
			aiResult:
				'Error: get_app_runtime_logs is only available inside an AI session. Tell the user runtime logs can only be read from a session preview, or switch to a session and open the raw app preview.',
			uiMessage: 'Runtime logs unavailable',
			toolResult: 'Runtime logs unavailable'
		})
	}
	return getRuntimeLogsHandler({ sessionId, limit })
}

export type ListAppRunsHandler = (req: {
	sessionId: string | undefined
	limit: number
}) => SessionToolResult

let listAppRunsHandler: ListAppRunsHandler | undefined

export function setListAppRunsHandler(handler: ListAppRunsHandler | undefined): void {
	listAppRunsHandler = handler
}

function getSessionAppRuns(
	limit: number,
	sessionId: string | undefined
): Promise<SessionToolResult> {
	if (!listAppRunsHandler) {
		return Promise.resolve({
			aiResult:
				'Error: list_app_runs is only available inside an AI session. Tell the user app runs can only be read from a session preview, or switch to a session and open the raw app preview.',
			uiMessage: 'App runs unavailable',
			toolResult: 'App runs unavailable'
		})
	}
	return Promise.resolve(listAppRunsHandler({ sessionId, limit }))
}

export type GetDomHandler = (req: {
	sessionId: string | undefined
	query: RawAppDomQuery
}) => Promise<SessionToolResult>

let getDomHandler: GetDomHandler | undefined

export function setGetDomHandler(handler: GetDomHandler | undefined): void {
	getDomHandler = handler
}

function getSessionDom(
	query: RawAppDomQuery,
	sessionId: string | undefined
): Promise<SessionToolResult> {
	if (!getDomHandler) {
		return Promise.resolve({
			aiResult:
				'Error: search_dom and read_dom are only available inside an AI session. Tell the user the rendered DOM can only be read from a session preview, or switch to a session and open the raw app preview.',
			uiMessage: 'DOM unavailable',
			toolResult: 'DOM unavailable'
		})
	}
	return getDomHandler({ sessionId, query })
}

export type SessionScreenshotResult = { dataUrl?: string; error?: string; uiMessage?: string }
export type ScreenshotHandler = (req: {
	sessionId: string | undefined
}) => Promise<SessionScreenshotResult>

let screenshotHandler: ScreenshotHandler | undefined

export function setScreenshotHandler(handler: ScreenshotHandler | undefined): void {
	screenshotHandler = handler
}

function getSessionScreenshot(sessionId: string | undefined): Promise<SessionScreenshotResult> {
	if (!screenshotHandler) {
		return Promise.resolve({
			error:
				'Error: take_screenshot is only available inside an AI session with a raw app preview open. Ask the user to open the raw app preview (open_preview kind="raw_app"), then try again.',
			uiMessage: 'Screenshot unavailable'
		})
	}
	return screenshotHandler({ sessionId })
}

// Registered by the session runtime to reload the open preview after a chat
// deploy. Undefined outside a session.
export type DeployedInSessionHandler = (req: {
	sessionId: string | undefined
	kind: 'script' | 'flow' | 'raw_app'
	path: string
}) => void

let deployedInSessionHandler: DeployedInSessionHandler | undefined

export function setDeployedInSessionHandler(handler: DeployedInSessionHandler | undefined): void {
	deployedInSessionHandler = handler
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
			`Secret value for draft variable "${path}" is no longer available because secret draft values are kept only in memory. Run write_variable again before deploying this secret.`
		)
	}

	return { ...requestBody, value: secretValue }
}

function startDraftWrite(ctx: WriteDraftCtx, type: WorkspaceItemType, path: string): void {
	ctx.toolCallbacks.setToolStatus(ctx.toolId, {
		content: `Saving ${type} "${path}" as a draft…`
	})
}

// Conflict / save-failure handling shared by the kind write tools and the app
// write tools. Returns the JSON tool-result for a non-saved persist, or undefined
// when the save succeeded (the caller then emits its own success payload).
function draftWriteFailure(result: DraftPersistResult, ctx: WriteDraftCtx): string | undefined {
	const stored = result.item
	if (result.status === 'conflict') {
		ctx.toolCallbacks.setToolStatus(ctx.toolId, {
			content: `Draft ${stored.type} "${stored.path}" changed externally`,
			result: `Conflict`
		})
		return JSON.stringify(
			{
				success: false,
				conflict: true,
				message: `The ${stored.type} draft "${stored.path}" changed externally since you last read it. Re-run this tool to merge onto the latest version, or pass override:true to overwrite. If an editor for it is open, a conflict dialog is also shown there.`
			},
			null,
			2
		)
	}
	if (result.status === 'error') {
		ctx.toolCallbacks.setToolStatus(ctx.toolId, {
			content: `Failed to save ${stored.type} "${stored.path}"`,
			result: `Save failed`
		})
		return JSON.stringify(
			{
				success: false,
				error: true,
				message: `The ${stored.type} draft "${stored.path}" could NOT be saved (${result.message}). The change was not persisted — retry; do not assume it succeeded.`
			},
			null,
			2
		)
	}
	return undefined
}

// App write tools build varied success messages but share the same conflict /
// save-failure handling; `onSaved` supplies the per-tool status + message.
function finishAppDraftWrite(
	result: DraftPersistResult,
	ctx: WriteDraftCtx,
	onSaved: () => { content: string; message: string }
): string {
	const failure = draftWriteFailure(result, ctx)
	if (failure) return failure
	ctx.toolCallbacks.onItemModified?.(result.itemKind, result.storagePath)
	const { content, message } = onSaved()
	ctx.toolCallbacks.setToolStatus(ctx.toolId, { content, result: 'Saved as draft' })
	return JSON.stringify({ success: true, message }, null, 2)
}

function finishDraftWrite(
	result: DraftPersistResult,
	existed: boolean,
	ctx: WriteDraftCtx
): string {
	const failure = draftWriteFailure(result, ctx)
	if (failure) return failure
	ctx.toolCallbacks.onItemModified?.(result.itemKind, result.storagePath)
	const stored = result.item
	const verb = existed ? 'Updated' : 'Created'
	// Don't echo the flow value back: the model just sent it in the write call,
	// so reflecting the (large) compact flow JSON only burns tokens. Variables
	// echo a redacted item; everything else round-trips its small payload.
	const serializedItem =
		stored.type === 'flow'
			? undefined
			: stored.type === 'variable'
				? serializeWorkspaceItemForRead(stored)
				: stored

	ctx.toolCallbacks.setToolStatus(ctx.toolId, {
		content: `${verb} ${stored.type} "${stored.path}" as a draft`,
		result: `Saved as draft`
	})
	return JSON.stringify(
		{
			success: true,
			message: `${verb} ${stored.type} "${stored.path}" as a per-user draft (saved server-side, visible only to this user — not a deployed workspace item). It was not deployed.`,
			item: serializedItem
		},
		null,
		2
	)
}

// Per-draft-kind knowledge for the shared write skeleton. `fetchDeployed` returns
// the deployed item already shaped as a draft value (e.g. script with parent_hash)
// so `buildDraft` treats a draft base and a deployed base identically; a `base` of
// undefined is the create-from-scratch case. `beforePersist` is a kind-local side
// effect run after the value is built (only variable, for its in-memory secret).
type WriteSpec<T, A> = {
	probe: (workspace: string, path: string) => Promise<boolean>
	fetchDeployed: (workspace: string, path: string) => Promise<T>
	buildDraft: (base: T | undefined, args: A, path: string) => T | Promise<T>
	beforePersist?: (workspace: string, args: A) => void
}

async function writeDraft<T, A>(
	spec: WriteSpec<T, A>,
	type: WorkspaceItemType,
	path: string,
	args: A,
	ctx: WriteDraftCtx,
	opts: { triggerKind?: TriggerKind; override?: boolean } = {}
): Promise<string> {
	const { workspace } = ctx
	startDraftWrite(ctx, type, path)

	const existingDraft = await readGlobalDraftValue<T>(workspace, type, path, opts.triggerKind)
	let base = existingDraft
	let existed = existingDraft !== undefined
	if (base === undefined && (await spec.probe(workspace, path))) {
		base = await spec.fetchDeployed(workspace, path)
		existed = true
	}

	const draft = await spec.buildDraft(base, args, path)
	spec.beforePersist?.(workspace, args)

	const result = await persistGlobalDraft(workspace, type, path, draft, {
		triggerKind: opts.triggerKind,
		force: opts.override
	})
	return finishDraftWrite(result, existed, ctx)
}

type ScriptDraftArgs = {
	path: string
	summary?: string
	language: ScriptLang
	content: string
	override?: boolean
}

const SCRIPT_SPEC: WriteSpec<NewScript, ScriptDraftArgs> = {
	probe: (workspace, path) => ScriptService.existsScriptByPath({ workspace, path }),
	fetchDeployed: async (workspace, path) => {
		const existing = await ScriptService.getScriptByPath({ workspace, path })
		return { ...(existing as unknown as NewScript), parent_hash: existing.hash }
	},
	buildDraft: async (base, args, path) => {
		const draft: NewScript = base
			? {
					...structuredClone(base),
					path,
					summary: args.summary ?? base.summary,
					content: args.content,
					language: args.language
				}
			: {
					path,
					summary: args.summary ?? '',
					description: '',
					content: args.content,
					schema: emptySchema(),
					is_template: false,
					language: args.language,
					kind: 'script'
				}
		// Infer the arg schema from the content at save time, like the editor does,
		// so the persisted draft is the single source of truth at deploy. Keep the
		// previous schema (or empty) on failure rather than blanking it.
		try {
			const schema = emptySchema()
			await inferArgs(draft.language, draft.content, schema)
			draft.schema = schema
		} catch (e) {
			console.error('Failed to infer script schema before saving draft', e)
		}
		return draft
	}
}

function writeScriptDraft(args: ScriptDraftArgs, ctx: WriteDraftCtx): Promise<string> {
	return writeDraft(SCRIPT_SPEC, 'script', args.path, args, ctx, { override: args.override })
}

type FlowDraftArgs = {
	path: string
	summary?: string
	description?: string
	flow: FlowDraftValue
	override?: boolean
}

const FLOW_SPEC: WriteSpec<Flow, FlowDraftArgs> = {
	probe: (workspace, path) => FlowService.existsFlowByPath({ workspace, path }),
	fetchDeployed: (workspace, path) => FlowService.getFlowByPath({ workspace, path }),
	buildDraft: (base, args, path) => {
		const value = structuredClone(args.flow.value)
		if (args.flow.groups !== undefined && args.flow.groups !== null) {
			value.groups = structuredClone(args.flow.groups)
		}
		return base
			? {
					...structuredClone(base),
					path,
					summary: args.summary ?? base.summary,
					description: args.description ?? base.description,
					value,
					schema: args.flow.schema ?? base.schema
				}
			: {
					path,
					summary: args.summary ?? '',
					description: args.description ?? '',
					value,
					schema: args.flow.schema ?? emptySchema(),
					edited_by: '',
					edited_at: '',
					archived: false,
					extra_perms: {}
				}
	}
}

function writeFlowDraft(args: FlowDraftArgs, ctx: WriteDraftCtx): Promise<string> {
	return writeDraft(FLOW_SPEC, 'flow', args.path, args, ctx, { override: args.override })
}

const SCHEDULE_SPEC: WriteSpec<ScheduleDraftConfig, NewSchedule & { override?: boolean }> = {
	probe: (workspace, path) => ScheduleService.existsSchedule({ workspace, path }),
	fetchDeployed: async (workspace, path) =>
		(await ScheduleService.getSchedule({ workspace, path })) as ScheduleDraftConfig,
	buildDraft: (base, args, path) => {
		// `override` is a tool-only conflict-resolution flag, not schedule config —
		// strip it so mergeDraftConfig doesn't clone it into the persisted draft.
		const { override: _override, ...config } = args
		return mergeDraftConfig<ScheduleDraftConfig>(base, config as DraftConfig, path)
	}
}

function writeScheduleDraft(
	args: NewSchedule & { override?: boolean },
	ctx: WriteDraftCtx
): Promise<string> {
	return writeDraft(SCHEDULE_SPEC, 'schedule', args.path, args, ctx, { override: args.override })
}

function triggerWriteSpec(kind: TriggerKind): WriteSpec<TriggerDraftConfig, TriggerDraftConfig> {
	const service = triggerServices[kind]
	return {
		probe: (workspace, path) => service.exists({ workspace, path }),
		fetchDeployed: async (workspace, path) =>
			(await service.get({ workspace, path })) as TriggerDraftConfig,
		buildDraft: (base, config, path) => mergeDraftConfig<TriggerDraftConfig>(base, config, path)
	}
}

function writeTriggerDraft(
	args: { kind: TriggerKind; config: unknown; override?: boolean },
	ctx: WriteDraftCtx
): Promise<string> {
	const config = args.config as TriggerDraftConfig
	return writeDraft(triggerWriteSpec(args.kind), 'trigger', config.path, config, ctx, {
		triggerKind: args.kind,
		override: args.override
	})
}

const RESOURCE_SPEC: WriteSpec<ResourceDraftState, CreateResource & { override?: boolean }> = {
	probe: (workspace, path) => ResourceService.existsResource({ workspace, path }),
	fetchDeployed: async (workspace, path) =>
		resourceToDraftState(await ResourceService.getResource({ workspace, path })),
	buildDraft: (base, args) => createResourceToDraftState(args, base)
}

function writeResourceDraft(
	args: CreateResource & { override?: boolean },
	ctx: WriteDraftCtx
): Promise<string> {
	return writeDraft(RESOURCE_SPEC, 'resource', args.path, args, ctx, { override: args.override })
}

const VARIABLE_SPEC: WriteSpec<VariableDraftState, CreateVariable & { override?: boolean }> = {
	probe: (workspace, path) => VariableService.existsVariable({ workspace, path }),
	fetchDeployed: async (workspace, path) =>
		variableToDraftState(
			await VariableService.getVariable({ workspace, path, decryptSecret: false })
		),
	buildDraft: (base, args) => createVariableToDraftState(args, base),
	beforePersist: (workspace, args) => syncEphemeralSecretVariableDraftValue(workspace, args)
}

function writeVariableDraft(
	args: CreateVariable & { override?: boolean },
	ctx: WriteDraftCtx
): Promise<string> {
	return writeDraft(VARIABLE_SPEC, 'variable', args.path, args, ctx, { override: args.override })
}

async function loadScriptForEdit(
	path: string,
	workspace: string
): Promise<{ content: string; language: ScriptLang; summary?: string }> {
	const draft = await getGlobalDraft(workspace, 'script', path)
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
	const draft = await getGlobalDraft(workspace, 'flow', path)
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

function normalizeTestRunArgs(args: Record<string, any> | null | undefined): Record<string, any> {
	return args ?? {}
}

function flowDraftValueForPreview(flowDraft: FlowDraftValue): FlowValue {
	return flowDraftAsEditableInput(flowDraft).value
}

async function loadScriptForFlowStep(
	moduleValue: { path: string; hash?: string },
	workspace: string
): Promise<{ content: string; language: ScriptLang }> {
	const draft = await getGlobalDraft(workspace, 'script', moduleValue.path)
	if (draft) {
		if (typeof draft.value !== 'string' || !draft.language) {
			throw new Error(`Draft script "${moduleValue.path}" is missing content or language.`)
		}
		return { content: draft.value, language: draft.language }
	}

	const script = moduleValue.hash
		? await ScriptService.getScriptByHash({ workspace, hash: moduleValue.hash })
		: await ScriptService.getScriptByPath({ workspace, path: moduleValue.path })
	return { content: script.content, language: script.language }
}

async function loadDraftFlowPreviewValue(
	path: string,
	workspace: string
): Promise<FlowValue | undefined> {
	if (!(await getGlobalDraft(workspace, 'flow', path))) {
		return undefined
	}
	const nestedFlow = await loadFlowDraftValue(path, workspace)
	return flowDraftValueForPreview(nestedFlow.flow)
}

// Leaf of a workspace path (last segment), for human-readable confirmation
// prompts. Falls back to the full path, then a generic noun.
function pathLeaf(path: unknown, fallback: string): string {
	const p = typeof path === 'string' ? path : ''
	return p.split('/').filter(Boolean).pop() || p || fallback
}

async function testRunScriptByPath(
	args: z.infer<typeof testRunScriptSchema>,
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const script = await loadScriptForEdit(args.path, workspace)
	const testArgs = normalizeTestRunArgs(args.args)

	return executeTestRun({
		jobStarter: () =>
			JobService.runScriptPreview({
				workspace,
				requestBody: {
					path: args.path,
					content: script.content,
					args: testArgs,
					language: script.language
				}
			}),
		workspace,
		toolCallbacks,
		toolId,
		startMessage: `Running test for script "${args.path}"...`,
		contextName: 'script',
		background: args.background,
		detachAfterMs: waitSecondsToDetachMs(args.wait_seconds),
		label: args.path
	})
}

async function testRunFlowByPath(
	args: z.infer<typeof testRunFlowSchema>,
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const testArgs = normalizeTestRunArgs(args.args)
	const testActiveFlow = activeFlowTestFromCtx(ctx, args.path)

	if (testActiveFlow) {
		return executeTestRun({
			jobStarter: async () => {
				const jobId = await testActiveFlow(testArgs)
				if (jobId) {
					return jobId
				}

				const flow = await loadFlowDraftValue(args.path, workspace)
				return JobService.runFlowPreview({
					workspace,
					requestBody: {
						path: args.path,
						value: flowDraftValueForPreview(flow.flow),
						args: testArgs
					}
				})
			},
			workspace,
			toolCallbacks,
			toolId,
			startMessage: `Starting flow test run for "${args.path}"...`,
			contextName: 'flow',
			background: args.background,
			detachAfterMs: waitSecondsToDetachMs(args.wait_seconds),
			label: args.path
		})
	}

	const flow = await loadFlowDraftValue(args.path, workspace)

	return executeTestRun({
		jobStarter: () =>
			JobService.runFlowPreview({
				workspace,
				requestBody: {
					path: args.path,
					value: flowDraftValueForPreview(flow.flow),
					args: testArgs
				}
			}),
		workspace,
		toolCallbacks,
		toolId,
		startMessage: `Starting flow test run for "${args.path}"...`,
		contextName: 'flow',
		background: args.background,
		detachAfterMs: waitSecondsToDetachMs(args.wait_seconds),
		label: args.path
	})
}

async function testRunFlowStepByPath(
	args: z.infer<typeof testRunStepSchema>,
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const flow = await loadFlowDraftValue(args.path, workspace)
	const flowValue = flowDraftValueForPreview(flow.flow)
	const testArgs = normalizeTestRunArgs(args.args)

	return executeFlowStepTestRun({
		flowValue,
		stepId: args.stepId,
		args: testArgs,
		workspace,
		toolCallbacks,
		toolId,
		background: args.background,
		detachAfterMs: waitSecondsToDetachMs(args.wait_seconds),
		loadScript: loadScriptForFlowStep,
		loadFlowPreviewValue: loadDraftFlowPreviewValue
	})
}

async function initApp(
	args: {
		path: string
		summary?: string
		framework: FrameworkKey
	},
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const { path, summary, framework } = args

	if (await getGlobalDraft(workspace, 'app', path)) {
		throw new Error(
			`A draft for app "${path}" already exists. Use write_app_file / write_app_runnable to modify it, or delete the existing draft first.`
		)
	}
	if (await AppService.existsApp({ workspace, path })) {
		throw new Error(
			`An app already exists at "${path}". Use read_workspace_item + write_app_file / write_app_runnable to modify it.`
		)
	}

	toolCallbacks.setToolStatus(toolId, {
		content: `Saving app "${path}" draft (${framework} template)…`
	})

	const template = FRAMEWORK_TEMPLATES[framework]
	const value: AppDraftValue = {
		summary,
		files: { ...template },
		runnables: { [STARTER_RUNNABLE_KEY]: { ...STARTER_RUNNABLE } }
	}
	await recomputeAppPolicy(value)
	const result = await saveAppDraft(workspace, path, value)
	return finishAppDraftWrite(result, ctx, () => ({
		content: `Saved app "${path}" draft (${framework})`,
		message: `Initialized a per-user draft app "${path}" from the ${framework} template with a starter runnable "${STARTER_RUNNABLE_KEY}" (saved server-side, not a deployed workspace item). Use write_app_file / write_app_runnable to evolve it.`
	}))
}

// read_app_file caps: a large frontend file or inline runnable would otherwise
// enter context in full and persist for the rest of the session. Default to a
// head slice with a pointer to page further; the model widens with offset/limit.
// The char budget is a hard ceiling on a single read: a selected line window over
// it is truncated and the model is told to narrow the line range. There is no
// char-level paging, so a single line longer than the budget can't be read past —
// add paging here if minified/long-line files must be fully readable.
const READ_APP_FILE_DEFAULT_LINE_LIMIT = 1500
const READ_APP_FILE_CHAR_BUDGET = 50_000

type AppFileSlice = {
	body: string
	startLine: number
	endLine: number
	requestedStartLine: number
	totalLines: number
	lineWindowChars: number
	charTruncated: boolean
	truncated: boolean
}

function sliceAppFileForRead(content: string, offset?: number, limit?: number): AppFileSlice {
	const lines = content.split('\n')
	const totalLines = lines.length
	const requestedStartLine = offset ?? 1
	const start = Math.min(Math.max(requestedStartLine - 1, 0), totalLines)
	const lineLimit = limit ?? READ_APP_FILE_DEFAULT_LINE_LIMIT
	const end = Math.min(start + lineLimit, totalLines)
	const selectedBody = lines.slice(start, end).join('\n')
	const lineWindowChars = selectedBody.length
	const body = selectedBody.slice(0, READ_APP_FILE_CHAR_BUDGET)
	const charTruncated = lineWindowChars > READ_APP_FILE_CHAR_BUDGET

	return {
		body,
		startLine: start + 1,
		endLine: end,
		requestedStartLine,
		totalLines,
		lineWindowChars,
		charTruncated,
		truncated: start > 0 || end < totalLines || charTruncated
	}
}

function formatAppFileReadRangeLabel(slice: AppFileSlice): string {
	const lineRange = `lines ${slice.startLine}-${slice.endLine} of ${slice.totalLines}`
	if (!slice.charTruncated) {
		return lineRange
	}
	return `${lineRange}, truncated to the first ${READ_APP_FILE_CHAR_BUDGET} of ${slice.lineWindowChars} chars`
}

// Small files (returned whole, starting at line 1) keep the raw body so
// patch_app_file's exact-match stays trivial; truncated/windowed reads get a
// one-line annotation describing the range (not part of the file).
function formatAppFileReadResult(slice: AppFileSlice): string {
	// offset past the last line: report it plainly instead of a backwards range.
	if (slice.requestedStartLine > slice.totalLines) {
		return `offset ${slice.requestedStartLine} is past the end of the file (${slice.totalLines} lines).`
	}
	if (!slice.truncated && slice.startLine === 1) {
		return slice.body
	}

	let more = ''
	if (slice.charTruncated) {
		more = ` Reached the ${READ_APP_FILE_CHAR_BUDGET}-char limit; re-read with a smaller limit (fewer lines). If a single line exceeds the limit the file is likely minified and not readable this way.`
	} else if (slice.endLine < slice.totalLines) {
		more = ` Call read_app_file again with offset=${slice.endLine + 1} to continue.`
	}
	// No tool-name/path prefix: the model already has them from the call args. The
	// range line orients it; the body follows after a blank line.
	return `${formatAppFileReadRangeLabel(slice)}.${more}\n\n${slice.body}`
}

async function readAppFile(
	args: {
		path: string
		file_path: string
		offset?: number
		limit?: number
	},
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const target = resolveAppFileTarget(args.file_path)
	toolCallbacks.setToolStatus(toolId, {
		content: `Reading ${target.filePath} from app "${args.path}"...`
	})

	const value = await loadAppValueForRead(args.path, workspace)

	let content: string
	if (target.kind === 'frontend') {
		const frontend = value.files[target.filePath]
		if (frontend === undefined) {
			throw new Error(`Frontend file "${target.filePath}" not found in app "${args.path}".`)
		}
		content = frontend
	} else {
		content = getInlineRunnableContent(value, target, args.path).content
	}

	const slice = sliceAppFileForRead(content, args.offset, args.limit)

	toolCallbacks.setToolStatus(toolId, { content: `Read ${target.filePath}` })
	return formatAppFileReadResult(slice)
}

// search_app caps: a single query must stay sparse and cheap even when it hits a
// minified bundle or a 5k-line data module. Per-line and total-output caps bound
// the result the same way read_app_file's char budget bounds one file read; the
// match cap keeps a broad query from flooding context instead of locating it.
const SEARCH_APP_DEFAULT_MAX_MATCHES = 100
const SEARCH_APP_MAX_MATCHES_CEILING = 200
const SEARCH_APP_MAX_LINE_CHARS = 200
const SEARCH_APP_TOTAL_CHAR_BUDGET = 12_000
// Fixed surrounding-context window per match, kept off the tool schema to keep it
// lean. Bump if matches need more context than the line ± this.
const SEARCH_APP_CONTEXT_LINES = 2

type AppSearchableFile = { filePath: string; content: string }

// The files search_app scans: frontend files (minus generated ones) plus inline
// backend runnables, each addressed exactly as read_app_file expects so a match
// row's path can be passed straight back to read_app_file.
function collectSearchableAppFiles(value: AppDraftValue): AppSearchableFile[] {
	const files: AppSearchableFile[] = []
	for (const [filePath, content] of Object.entries(value.files)) {
		if (GENERATED_APP_FILE_PATHS.has(filePath)) continue
		if (typeof content === 'string') files.push({ filePath, content })
	}
	for (const [key, runnable] of Object.entries(value.runnables)) {
		const persisted = runnable as PersistedRunnable | undefined
		const content = persisted?.inlineScript?.content
		if (typeof content !== 'string') continue
		files.push({ filePath: `backend/${key}/main.${getInlineScriptExtension(persisted)}`, content })
	}
	return files
}

// Minimal glob: * = any chars except '/', ** = any chars, ? = single non-slash.
// A pattern without '/' matches the file name only (ripgrep-style), so "*.tsx"
// finds nested files; a pattern with '/' matches the full path.
function appFileMatchesGlob(filePath: string, glob: string): boolean {
	const hasSlash = glob.includes('/')
	const subject = hasSlash ? filePath : filePath.slice(filePath.lastIndexOf('/') + 1)
	const body = glob
		.replace(/[.+^${}()|[\]\\]/g, '\\$&')
		.replace(/\*\*/g, '\u0000')
		.replace(/\*/g, '[^/]*')
		.replace(/\u0000/g, '.*')
		.replace(/\?/g, '[^/]')
	try {
		return new RegExp(`^${body}$`).test(subject)
	} catch {
		return false
	}
}

type AppSearchMatch = { filePath: string; line: number; text: string }

async function searchApp(
	args: {
		path: string
		query: string
		file_glob?: string
		max_matches?: number
	},
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const query = args.query
	if (query.length === 0) {
		throw new Error('search_app requires a non-empty query.')
	}
	toolCallbacks.setToolStatus(toolId, {
		content: `Searching app "${args.path}" for "${query}"...`
	})

	const value = await loadAppValueForRead(args.path, workspace)
	const maxMatches = Math.min(
		args.max_matches ?? SEARCH_APP_DEFAULT_MAX_MATCHES,
		SEARCH_APP_MAX_MATCHES_CEILING
	)
	const contextLines = SEARCH_APP_CONTEXT_LINES
	const needle = query.toLowerCase()

	let files = collectSearchableAppFiles(value).sort((a, b) => a.filePath.localeCompare(b.filePath))
	if (args.file_glob) {
		files = files.filter((f) => appFileMatchesGlob(f.filePath, args.file_glob as string))
	}

	const matches: AppSearchMatch[] = []
	let totalMatchCount = 0
	// Cap on matching LINES, not pushed rows — each match expands to its context
	// window, so counting rows would make `max_matches`/"showing the first N" wrong.
	let renderedMatchCount = 0
	let fileCount = 0
	let truncated = false
	for (const file of files) {
		const lines = file.content.split('\n')
		let fileHadMatch = false
		for (let i = 0; i < lines.length; i++) {
			if (!lines[i].toLowerCase().includes(needle)) continue
			totalMatchCount++
			// Count the file on its first match, before the render cap, so the
			// "N matches in M files" header counts every file with the symbol — not
			// only the ones whose matches landed in the rendered slice (find-all-usages).
			fileHadMatch = true
			if (renderedMatchCount >= maxMatches) {
				truncated = true
				continue
			}
			renderedMatchCount++
			const lo = Math.max(0, i - contextLines)
			const hi = Math.min(lines.length - 1, i + contextLines)
			for (let j = lo; j <= hi; j++) {
				matches.push({ filePath: file.filePath, line: j + 1, text: lines[j] })
			}
		}
		if (fileHadMatch) fileCount++
	}

	if (totalMatchCount === 0) {
		toolCallbacks.setToolStatus(toolId, { content: `No matches for "${query}"` })
		return `No matches. Try a broader or differently-spelled term${
			args.file_glob ? ', or drop the file_glob' : ''
		}.`
	}

	// No tool-name/query prefix: the model already has them from the call args.
	const header = `${totalMatchCount} match${
		totalMatchCount === 1 ? '' : 'es'
	} in ${fileCount} file${fileCount === 1 ? '' : 's'}${
		truncated
			? ` (showing the first ${maxMatches}; narrow with file_glob or a more specific query)`
			: ''
	}`

	const out: string[] = [header]
	let currentFile = ''
	let budgetSpent = header.length
	let budgetHit = false
	const seen = new Set<string>()
	for (const m of matches) {
		// context windows of adjacent matches overlap — show each source line once.
		const dedupeKey = `${m.filePath}:${m.line}`
		if (seen.has(dedupeKey)) continue
		seen.add(dedupeKey)
		const text =
			m.text.length > SEARCH_APP_MAX_LINE_CHARS
				? `${m.text.slice(0, SEARCH_APP_MAX_LINE_CHARS)}… [line truncated]`
				: m.text
		const fileHeader = m.filePath === currentFile ? '' : `${m.filePath}\n`
		const row = `${fileHeader}  ${m.line}: ${text}`
		if (budgetSpent + row.length + 1 > SEARCH_APP_TOTAL_CHAR_BUDGET) {
			budgetHit = true
			break
		}
		if (fileHeader) currentFile = m.filePath
		out.push(row)
		budgetSpent += row.length + 1
	}
	if (budgetHit) {
		out.push(
			`… output truncated at the context budget — narrow with file_glob or a more specific query.`
		)
	}

	toolCallbacks.setToolStatus(toolId, {
		content: `Found ${totalMatchCount} match${totalMatchCount === 1 ? '' : 'es'} in ${fileCount} file${
			fileCount === 1 ? '' : 's'
		}`
	})
	return out.join('\n')
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

	const { value } = await loadAppDraftValue(args.path, workspace)
	value.files = { ...value.files, [target.filePath]: args.content }
	const result = await saveAppDraft(workspace, args.path, value)
	return finishAppDraftWrite(result, ctx, () => ({
		content: `Updated ${target.filePath} in app "${args.path}"`,
		message: `Updated draft app "${args.path}" with frontend file "${target.filePath}".`
	}))
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

	const { value } = await loadAppDraftValue(args.path, workspace)
	if (!(target.filePath in value.files)) {
		throw new Error(`Frontend file "${target.filePath}" not found in app "${args.path}".`)
	}
	const { [target.filePath]: _removed, ...remaining } = value.files
	value.files = remaining
	const result = await saveAppDraft(workspace, args.path, value)
	return finishAppDraftWrite(result, ctx, () => ({
		content: `Removed ${target.filePath} from app "${args.path}"`,
		message: `Removed "${target.filePath}" from draft app "${args.path}".`
	}))
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

	const { value } = await loadAppDraftValue(path, workspace)
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

	const result = await saveAppDraft(workspace, path, value)
	return finishAppDraftWrite(result, ctx, () => ({
		content: `Patched ${target.filePath} in app "${path}"`,
		message: `Patched "${target.filePath}" in draft app "${path}".`
	}))
}

async function recomputeAppPolicy(value: AppDraftValue): Promise<void> {
	const policy = (await updateRawAppPolicy(
		value.runnables as any,
		value.policy as any
	)) as NonNullable<AppDraftValue['policy']>
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

	const { value } = await loadAppDraftValue(path, workspace)
	const existing = value.runnables[key] as PersistedRunnable | undefined
	const persisted = buildPersistedRunnable(input, existing)
	value.runnables = { ...value.runnables, [key]: persisted }
	await recomputeAppPolicy(value)
	const result = await saveAppDraft(workspace, path, value)
	return finishAppDraftWrite(result, ctx, () => ({
		content: `Updated runnable "${key}" in app "${path}"`,
		message: `Updated draft app "${path}" with runnable "${key}".`
	}))
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

	const { value } = await loadAppDraftValue(path, workspace)
	if (!(key in value.runnables)) {
		throw new Error(`Backend runnable "${key}" not found in app "${path}".`)
	}
	const { [key]: _removed, ...remaining } = value.runnables
	value.runnables = remaining
	await recomputeAppPolicy(value)
	const result = await saveAppDraft(workspace, path, value)
	return finishAppDraftWrite(result, ctx, () => ({
		content: `Removed runnable "${key}" from app "${path}"`,
		message: `Removed runnable "${key}" from draft app "${path}".`
	}))
}

const triggerLabels: Record<TriggerKind, string> = {
	http: 'HTTP trigger',
	websocket: 'WebSocket trigger',
	kafka: 'Kafka trigger',
	nats: 'NATS trigger',
	postgres: 'Postgres trigger',
	mqtt: 'MQTT trigger',
	amqp: 'AMQP trigger',
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

	const draft = await getGlobalDraft(workspace, type, path, triggerKind)
	if (!draft) {
		throw new Error(`No draft found for ${type} "${path}".`)
	}

	await deleteGlobalDraft(workspace, type, path, triggerKind)

	// The chat's touch on the item is undone — drop it from the mask so a
	// pre-existing deployed item doesn't keep reading as this chat's edit.
	const discardedKind = itemKindFor(type, triggerKind)
	if (discardedKind) {
		toolCallbacks.onItemDiscarded?.(
			discardedKind,
			getGlobalDraftStoragePath(workspace, type, path, triggerKind)
		)
	}

	toolCallbacks.setToolStatus(toolId, {
		content: `Discarded ${type} "${path}" draft`,
		result: 'Draft discarded'
	})
	return JSON.stringify(
		{
			success: true,
			message: `Discarded the draft for ${type} "${path}". The deployed workspace item was not changed.`,
			type,
			path,
			triggerKind
		},
		null,
		2
	)
}

// A draft started from an older deploy would silently overwrite whatever was
// deployed since. Block the deploy and point the model at rebase_draft, unless it
// explicitly forces the overwrite. `base`/`head` undefined ⇒ can't tell ⇒ allow.
function assertDraftBasedOnLatest(
	type: WorkspaceItemType,
	path: string,
	base: string | number | undefined,
	head: string | number | undefined,
	force: boolean | undefined
): void {
	if (force || base == null || head == null || base === head) return
	throw new Error(
		`This ${type} draft "${path}" was started from an older deployed version (forked from ${base}, ` +
			`latest is ${head}). Deploying now would overwrite the version deployed since. Call rebase_draft to ` +
			`discard the stale draft and get your changes back as a diff, then re-apply them (the new draft ` +
			`re-bases onto the latest version) and deploy. To deploy as-is and replace the newer version, call ` +
			`deploy_workspace_item again with force: true.`
	)
}

// Discard a stale draft and return its own changes (vs the fork base) as a diff so
// the model can re-apply them on the latest deploy. Discarding rather than resetting
// keeps the base pointer honest (the next write re-bases on the current head) and
// fails safe: a premature deploy hits "no draft" instead of silently shipping the
// latest unchanged.
async function rebaseDraft(
	args: { type: WorkspaceItemType; path: string },
	ctx: WriteDraftCtx
): Promise<string> {
	switch (args.type) {
		case 'script':
			return rebaseScriptDraft(args.path, ctx)
		case 'flow':
			return rebaseFlowDraft(args.path, ctx)
		case 'app':
			return rebaseAppDraft(args.path, ctx)
		default:
			throw new Error('rebase_draft currently supports scripts, flows, and apps.')
	}
}

async function rebaseScriptDraft(path: string, ctx: WriteDraftCtx): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx

	const draft = await getGlobalDraft(workspace, 'script', path)
	if (!draft || typeof draft.value !== 'string' || !draft.language) {
		throw new Error(`No script draft found for "${path}".`)
	}
	if (!(await ScriptService.existsScriptByPath({ workspace, path }))) {
		throw new Error(`Script "${path}" is not deployed; there is no newer version to rebase onto.`)
	}

	const latest = await ScriptService.getScriptByPath({ workspace, path })
	const baseHash = draft.parentHash
	if (baseHash && baseHash === latest.hash) {
		const message = `Draft "${path}" is already based on the latest deployed version (${latest.hash}).`
		toolCallbacks.setToolStatus(toolId, { content: message })
		return JSON.stringify({ success: true, alreadyLatest: true, latest_hash: latest.hash, message })
	}

	toolCallbacks.setToolStatus(toolId, { content: `Rebasing draft "${path}" onto latest...` })

	// Capture the draft's own changes (vs its fork base) BEFORE discarding — this
	// diff is the only clean record of what to replay. Best-effort: if the base
	// version is gone, diff against empty so the full draft is surfaced.
	let baseContent = ''
	if (baseHash) {
		try {
			baseContent = (await ScriptService.getScriptByHash({ workspace, hash: baseHash })).content
		} catch (e) {
			console.error(`rebase_draft: could not fetch base version ${baseHash} for "${path}"`, e)
		}
	}
	const yourChanges = createTwoFilesPatch(
		'fork-base',
		'your-draft',
		baseContent,
		draft.value,
		'',
		''
	)

	// Discard the stale draft rather than resetting it to latest: the next write
	// re-bases on the current head, and a premature deploy fails cleanly ("no
	// draft") instead of silently shipping the latest unchanged and losing the work.
	await deleteGlobalDraft(workspace, 'script', path)

	toolCallbacks.setToolStatus(toolId, {
		content: `Discarded stale draft "${path}"`,
		result: 'Rebased'
	})
	return JSON.stringify(
		{
			success: true,
			message:
				`Discarded the stale draft for "${path}". Your changes are in "your_changes" (a diff against the ` +
				`version you forked from). Re-apply them with edit_script / write_script — the new draft will be ` +
				`based on the latest deployed version (hash ${latest.hash}) — then deploy.`,
			latest_hash: latest.hash,
			your_changes: yourChanges
		},
		null,
		2
	)
}

async function rebaseFlowDraft(path: string, ctx: WriteDraftCtx): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx

	const draft = await getGlobalDraft(workspace, 'flow', path)
	if (!draft || draft.value === undefined || typeof draft.value === 'string') {
		throw new Error(`No flow draft found for "${path}".`)
	}
	if (!(await FlowService.existsFlowByPath({ workspace, path }))) {
		throw new Error(`Flow "${path}" is not deployed; there is no newer version to rebase onto.`)
	}

	const latest = await FlowService.getFlowByPath({ workspace, path })
	const baseVersion = draft.parentVersionId
	if (baseVersion != null && baseVersion === latest.version_id) {
		const message = `Draft "${path}" is already based on the latest deployed version (${latest.version_id}).`
		toolCallbacks.setToolStatus(toolId, { content: message })
		return JSON.stringify({
			success: true,
			alreadyLatest: true,
			latest_version: latest.version_id,
			message
		})
	}

	toolCallbacks.setToolStatus(toolId, { content: `Rebasing draft "${path}" onto latest...` })

	// The draft's own changes vs its fork-base flow value, as a JSON diff for the
	// model to replay. Best-effort: skip if the base version can't be fetched.
	let baseValue: unknown = {}
	if (baseVersion != null) {
		try {
			baseValue = (await FlowService.getFlowVersion({ workspace, version: baseVersion })).value
		} catch (e) {
			console.error(
				`rebase_draft: could not fetch base flow version ${baseVersion} for "${path}"`,
				e
			)
		}
	}
	const oursValue = (draft.value as FlowDraftValue).value
	const yourChanges = createTwoFilesPatch(
		'fork-base',
		'your-draft',
		JSON.stringify(baseValue, null, 2),
		JSON.stringify(oursValue, null, 2),
		'',
		''
	)

	// Discard the stale draft (see rebaseScriptDraft): the next write re-bases on
	// the current head, and a premature deploy fails cleanly instead of shipping
	// the latest unchanged.
	await deleteGlobalDraft(workspace, 'flow', path)

	toolCallbacks.setToolStatus(toolId, {
		content: `Discarded stale draft "${path}"`,
		result: 'Rebased'
	})
	return JSON.stringify(
		{
			success: true,
			message:
				`Discarded the stale draft for "${path}". Your changes are in "your_changes" (a JSON diff against ` +
				`the version you forked from). Re-apply them with the flow edit tools — the new draft will be ` +
				`based on the latest deployed version (version ${latest.version_id}) — then deploy.`,
			latest_version: latest.version_id,
			your_changes: yourChanges
		},
		null,
		2
	)
}

async function rebaseAppDraft(path: string, ctx: WriteDraftCtx): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx

	const draft = await getGlobalDraft(workspace, 'app', path)
	if (!draft || !draft.value || typeof draft.value === 'string' || !('files' in draft.value)) {
		throw new Error(`No app draft found for "${path}".`)
	}
	if (!(await AppService.existsApp({ workspace, path }))) {
		throw new Error(`App "${path}" is not deployed; there is no newer version to rebase onto.`)
	}

	const deployed = await AppService.getAppByPath({ workspace, path })
	const headVersion = deployed.versions?.[deployed.versions.length - 1]
	const baseVersion = draft.parentVersionId
	if (baseVersion != null && baseVersion === headVersion) {
		const message = `Draft "${path}" is already based on the latest deployed version (${headVersion}).`
		toolCallbacks.setToolStatus(toolId, { content: message })
		return JSON.stringify({
			success: true,
			alreadyLatest: true,
			latest_version: headVersion,
			message
		})
	}

	toolCallbacks.setToolStatus(toolId, { content: `Rebasing draft "${path}" onto latest...` })

	// The draft's own changes vs its fork-base app source, as a JSON diff for the
	// model to replay. Best-effort: skip if the base version can't be fetched.
	const oursValue = draft.value as AppDraftValue
	let baseSource: Pick<AppDraftValue, 'files' | 'runnables' | 'data'> = {
		files: {},
		runnables: {},
		data: undefined
	}
	if (baseVersion != null) {
		try {
			const baseApp = await AppService.getAppByVersion({ workspace, id: baseVersion })
			const base = appSourceToDraftValue(baseApp, baseApp)
			baseSource = { files: base.files, runnables: base.runnables, data: base.data }
		} catch (e) {
			console.error(
				`rebase_draft: could not fetch base app version ${baseVersion} for "${path}"`,
				e
			)
		}
	}
	const yourChanges = createTwoFilesPatch(
		'fork-base',
		'your-draft',
		JSON.stringify(baseSource, null, 2),
		JSON.stringify(
			{ files: oursValue.files, runnables: oursValue.runnables, data: oursValue.data },
			null,
			2
		),
		'',
		''
	)

	// Discard the stale draft (see rebaseScriptDraft): the next write re-projects
	// the deployed app into a fresh draft (re-pinning parent_version to the head),
	// and a premature deploy fails cleanly instead of shipping the latest unchanged.
	await deleteGlobalDraft(workspace, 'app', path)

	toolCallbacks.setToolStatus(toolId, {
		content: `Discarded stale draft "${path}"`,
		result: 'Rebased'
	})
	return JSON.stringify(
		{
			success: true,
			message:
				`Discarded the stale draft for "${path}". Your changes are in "your_changes" (a JSON diff against ` +
				`the version you forked from). Re-apply them with the app edit tools — the new draft will be based ` +
				`on the latest deployed version (version ${headVersion}) — then deploy.`,
			latest_version: headVersion,
			your_changes: yourChanges
		},
		null,
		2
	)
}

const MAX_DIFF_PATCH_CHARS = 50_000

function windowPatchBody(patch: string, offset: number, limit: number): string {
	return windowPatch(patch, offset, limit, MAX_DIFF_PATCH_CHARS)
}

const DIFF_READ_DEFAULT_LINES = 500
const DIFF_INDEX_DEFAULT_ITEMS = 50
const DIFF_INDEX_MAX_ITEMS = 100

// Read-only draft-vs-deployed diff for one draftable item, served from the
// workspace diff snapshot (fetched once, shared with the index), with a direct
// computation fallback when no draft row is listed.
async function diffWorkspaceItem(
	args: {
		type?: WorkspaceItemType
		path?: string
		trigger_kind?: TriggerKind
		file?: string
		offset?: number
		limit?: number
	},
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const { type, path, trigger_kind: triggerKind } = args
	if (!type || !path) {
		throw new Error('type is required when path is provided.')
	}
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) {
		throw new Error('trigger_kind is required when type is trigger.')
	}
	toolCallbacks.setToolStatus(toolId, {
		content: `Comparing draft vs deployed for "${path}"...`
	})

	// Address the draft by its storage path (a draft_only item lives at a
	// synthetic `u/{user}/draft_{uuid}` key) and flush any parked editor autosave
	// first so the server overlay reflects the latest edit — the same resolution
	// the deploy path uses. Unlike deploy: a flush conflict/failure doesn't abort
	// a read-only diff (surfaced as a caveat), and the auto-save toggle is
	// honored — with auto-save off, a read-only tool must not persist edits the
	// user chose to keep local. The just-flushed row must not be served from the
	// throttled listing cache, so expire it.
	// The chat `app` type spans two draft kinds: raw apps AND classic apps —
	// the classic editor parks its cell under `app`, so both keys must be
	// flushed and probed or classic edits silently go stale. Each kind resolves
	// its own storage path (live-editor mapping), and the listing resolves a
	// friendly/renamed path to the row that owns it — a renamed classic app's
	// cell lives at its ORIGINAL storage path, which only the listing knows.
	const draftKinds: UserDraftItemKind[] = type === 'app' ? ['raw_app', 'app'] : [itemKind]
	const flushQueries = draftKinds.map((kind) => ({
		workspace,
		itemKind: kind,
		path: resolveGlobalDraftStoragePathByKind(workspace, kind, path)
	}))
	const listedTarget = await resolveWorkspaceDiffTarget(workspace, draftKinds, path)
	if (
		listedTarget &&
		!flushQueries.some(
			(q) => q.itemKind === listedTarget.kind && q.path === listedTarget.storagePath
		)
	) {
		flushQueries.push({ workspace, itemKind: listedTarget.kind, path: listedTarget.storagePath })
	}
	const storagePath = listedTarget?.storagePath ?? flushQueries[0].path
	for (const query of flushQueries) {
		await UserDraftDbSyncer.flush(query, { honorAutosaveToggle: true })
	}
	expireWorkspaceDiffList(workspace)
	const hasConflict = flushQueries.some((query) => UserDraftDbSyncer.getConflict(query).conflict)

	// When the latest edits never reached the server (auto-save off, the save
	// failed, or it conflicted with a newer server version), the persisted
	// state is stale: diff against the in-memory editor value instead —
	// read-only, nothing gets persisted — and bypass the snapshot cache,
	// which must only ever hold persisted state.
	let flushSkipped = false
	let localValue: unknown
	let localKind: UserDraftItemKind = itemKind
	let localPath = storagePath
	for (const query of flushQueries) {
		const skipped =
			UserDraftDbSyncer.hasUnsavedDisabledChanges(query) ||
			UserDraftDbSyncer.getState(query).state === 'failed' ||
			UserDraftDbSyncer.getConflict(query).conflict
		if (!skipped) continue
		flushSkipped = true
		const cell = readLocalDraftCellByKind(workspace, query.itemKind, query.path)
		if (cell !== undefined) {
			localValue = cell
			localKind = query.itemKind
			localPath = query.path
			break
		}
	}

	let flushCaveat = hasConflict
		? localValue !== undefined
			? "Warning: the draft conflicts with a newer server version; this diff shows YOUR local editor value, not the server's. Resolve the conflict in the editor before deploying.\n\n"
			: 'Warning: the draft has a conflicting newer version on the server; this diff shows the persisted draft, which may not include the latest editor edits.\n\n'
		: ''
	let patch: string
	let noDeployed: boolean
	let files: Record<string, DiffFileView> | undefined
	let valueUncomparable = false
	if (localValue !== undefined) {
		let deployedSide: unknown
		try {
			const values = await getDraftDiffValues(localKind, localPath, workspace)
			noDeployed = values.noDeployed
			deployedSide = noDeployed ? undefined : values.deployed
		} catch (e) {
			if ((e as { status?: number } | null | undefined)?.status !== 404) throw e
			// Editor-only draft that was never persisted at all.
			noDeployed = true
		}
		let beforeSide = deployedSide
		let afterSide = canonicalDraftSideValue(localKind, localValue)
		// App sides carry `path` (staged renames diff); mirror it onto the local
		// canonical value, which only knows a draft_path.
		if (localKind === 'app' || localKind === 'raw_app') {
			const deployedPath = (deployedSide as { path?: string } | undefined)?.path ?? localPath
			const stagedPath = (localValue as { draft_path?: string } | null)?.draft_path
			afterSide = { ...(afterSide as Record<string, unknown>), path: stagedPath ?? deployedPath }
			if (deployedSide !== undefined) {
				beforeSide = { ...(deployedSide as Record<string, unknown>), path: deployedPath }
			}
		}
		if (itemKind === 'variable') {
			;({
				before: beforeSide,
				after: afterSide,
				valueUncomparable
			} = maskVariableDiffSides(beforeSide, afterSide))
			flushCaveat += valueUncomparable ? SECRET_UNCOMPARABLE_NOTE : VARIABLE_MASKED_NOTE
		}
		const parts = computeDiffParts(beforeSide, afterSide, 'deployed', 'draft')
		patch = parts.patch
		files = parts.files
		flushCaveat +=
			'Note: this diff includes unsaved editor changes that are NOT saved to the server draft yet (auto-save is off or the last save failed).\n\n'
	} else if (flushSkipped) {
		throw new Error(
			`The latest editor changes for ${type} "${path}" could not be saved and are not readable; retry once the editor saves.`
		)
	} else {
		const entry = await readWorkspaceDiffEntry(workspace, itemKind, storagePath)
		if (entry) {
			if (entry.status === 'error') {
				throw new Error(`Could not diff ${type} "${path}": ${entry.errorMessage}`)
			}
			patch = entry.patch ?? ''
			noDeployed = entry.noDeployed === true
			files = entry.files
			valueUncomparable = entry.valueUncomparable === true
			if (entry.valueMasked) {
				flushCaveat += valueUncomparable ? SECRET_UNCOMPARABLE_NOTE : VARIABLE_MASKED_NOTE
			}
		} else {
			// Not in the draft listing — either no draft at all (deployed is current),
			// nothing at the path, or a listing/overlay disagreement; ask the overlay.
			let values: Awaited<ReturnType<typeof getDraftDiffValues>>
			try {
				values = await getDraftDiffValues(itemKind, storagePath, workspace)
			} catch (e) {
				if ((e as { status?: number } | null | undefined)?.status === 404) {
					throw new Error(
						`No ${type} found at "${path}" — it has neither a deployed version nor a draft.`
					)
				}
				throw e
			}
			const { deployed, draft, hasDraft, noDeployed: fetchedNoDeployed } = values
			if (!fetchedNoDeployed && !hasDraft) {
				const message = `No draft exists for ${type} "${path}" — the deployed version is current.`
				toolCallbacks.setToolStatus(toolId, { content: message })
				return message
			}
			// A never-deployed item diffs against nothing: the whole draft reads as added.
			noDeployed = fetchedNoDeployed
			let beforeSide: unknown = noDeployed ? undefined : deployed
			let afterSide: unknown = draft
			if (itemKind === 'variable') {
				;({
					before: beforeSide,
					after: afterSide,
					valueUncomparable
				} = maskVariableDiffSides(beforeSide, afterSide))
				flushCaveat += valueUncomparable ? SECRET_UNCOMPARABLE_NOTE : VARIABLE_MASKED_NOTE
			}
			patch = draftDeployedPatch(beforeSide, afterSide)
		}
	}

	const changedFileCount = files ? Object.keys(files).length : 0
	if (!patch && changedFileCount === 0) {
		// A secret's sides are masked on both ends — an empty patch cannot prove
		// the value is unchanged.
		const message = valueUncomparable
			? `No visible changes for ${type} "${path}" — but a secret's value cannot be compared and may have been updated in the draft.`
			: `Draft matches the deployed version of ${type} "${path}" — no changes.`
		toolCallbacks.setToolStatus(toolId, { content: message, result: message })
		return flushCaveat + message
	}

	const header = noDeployed
		? `${type} "${path}" has no deployed version yet — the entire draft is new.\n\n`
		: `Draft changes vs deployed for ${type} "${path}":\n\n`
	if (args.file !== undefined && !files) {
		throw new Error(
			`file only applies to multi-file apps; ${type} "${path}" diffs as a single document — call again without file.`
		)
	}
	const body = files
		? renderEntryFiles(files, patch, args)
		: windowPatchBody(patch, args.offset ?? 0, args.limit ?? DIFF_READ_DEFAULT_LINES)
	const result = flushCaveat + header + body
	toolCallbacks.setToolStatus(toolId, {
		content: `Draft vs deployed diff for "${path}"`,
		result
	})
	return result
}

// Body of an item read for a multi-file app: one file's patch when `file` is
// given, otherwise the per-file summary plus config changes.
function renderEntryFiles(
	files: Record<string, DiffFileView>,
	configPatch: string,
	args: { file?: string; offset?: number; limit?: number }
): string {
	if (args.file !== undefined) {
		// App files are keyed with a leading slash ("/App.tsx") — accept the
		// slash-less spelling and a unique basename too.
		const names = Object.keys(files)
		const requested = args.file
		const resolved =
			names.find((n) => n === requested) ??
			names.find((n) => n === `/${requested}`) ??
			(names.filter((n) => n.endsWith(`/${requested.replace(/^\//, '')}`)).length === 1
				? names.find((n) => n.endsWith(`/${requested.replace(/^\//, '')}`))
				: undefined)
		if (resolved === undefined) {
			const changed = names.join(', ') || '(none)'
			throw new Error(`No changes in file "${requested}". Changed files: ${changed}.`)
		}
		const fileDiff = files[resolved]
		if (fileDiff.patch === '') {
			// Empty file added/deleted: the presence change IS the whole diff.
			return `File "${resolved}" was ${fileDiff.status} with empty content.`
		}
		return windowPatchBody(fileDiff.patch, args.offset ?? 0, args.limit ?? DIFF_READ_DEFAULT_LINES)
	}
	const sections: string[] = []
	const fileLines = Object.entries(files).map(
		([name, fileDiff]) =>
			`- ${name} — ${fileDiff.status}${fileDiff.status === 'deleted' ? '' : fileDiff.lineCount === 0 ? ' (empty file)' : ` (${fileDiff.lineCount} diff lines)`}`
	)
	sections.push(
		fileLines.length > 0
			? `${fileLines.length} file(s) changed:\n${fileLines.join('\n')}\nRead one with file="<name>".`
			: 'No file contents changed.'
	)
	if (configPatch) {
		sections.push(
			'Config changes:\n' +
				windowPatchBody(configPatch, args.offset ?? 0, args.limit ?? DIFF_READ_DEFAULT_LINES)
		)
	}
	return sections.join('\n\n')
}

// Indented per-file child rows under a multi-file app's index line.
const DIFF_INDEX_MAX_FILE_CHILDREN = 20
function fileChildrenLines(e: { files?: Record<string, DiffFileView>; patch?: string }): string[] {
	if (!e.files) return []
	const names = Object.keys(e.files)
	if (names.length === 0 && !e.patch) return []
	const lines = names.slice(0, DIFF_INDEX_MAX_FILE_CHILDREN).map((name) => {
		const fileDiff = e.files![name]
		return `    · ${name} — ${fileDiff.status}${
			fileDiff.status === 'deleted'
				? ''
				: fileDiff.lineCount === 0
					? ' (empty file)'
					: ` (${fileDiff.lineCount} lines)`
		}`
	})
	if (names.length > DIFF_INDEX_MAX_FILE_CHILDREN) {
		lines.push(`    · … ${names.length - DIFF_INDEX_MAX_FILE_CHILDREN} more files`)
	}
	if (e.patch) {
		lines.push(`    · (config) — modified (${e.patch.split('\n').length} lines)`)
	}
	return lines
}

function formatDiffIndexEntry(e: WorkspaceDiffEntryView): string {
	const label = e.type === 'trigger' ? `${e.triggerKind} trigger` : (e.type ?? e.kind)
	const name = `${label} "${e.path}"`
	switch (e.status) {
		case 'new':
			return `- ${name} — new, never deployed (${e.patchLineCount} lines)`
		case 'modified':
			return e.valueUncomparable
				? `- ${name} — modified (${e.patchLineCount} diff lines; secret value may also differ)`
				: `- ${name} — modified (${e.patchLineCount} diff lines)`
		case 'unchanged':
			return e.valueUncomparable
				? `- ${name} — no visible changes (secret value cannot be compared; may differ)`
				: `- ${name} — draft matches deployed`
		case 'pending':
			return `- ${name} — draft present (diff not computed yet; read it with type+path)`
		case 'error':
			return `- ${name} — diff failed: ${e.errorMessage}`
		case 'not_diffable':
			return `- ${e.kind} draft "${e.path}" — not addressable in this chat`
	}
}

// Workspace index: every draft the current user has, with its change status
// from the materialized snapshot.
async function diffWorkspaceIndex(
	args: { types?: WorkspaceItemType[]; path_prefix?: string; limit?: number },
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	toolCallbacks.setToolStatus(toolId, { content: 'Computing workspace draft diff...' })
	// Parked editor autosaves may not have a server row yet (a brand-new draft
	// only appears in the listing after its first flush).
	const { unflushedPaths } = await flushGlobalDraftSaves(workspace)
	expireWorkspaceDiffList(workspace)
	const index = await getWorkspaceDiffIndex(workspace)
	let entries = index.entries
	if (args.types?.length) {
		entries = entries.filter((e) => e.type !== undefined && args.types!.includes(e.type))
	}
	if (args.path_prefix) {
		entries = entries.filter(
			(e) => e.path.startsWith(args.path_prefix!) || e.storagePath.startsWith(args.path_prefix!)
		)
	}
	const total = entries.length
	const shown = entries.slice(
		0,
		Math.min(args.limit ?? DIFF_INDEX_DEFAULT_ITEMS, DIFF_INDEX_MAX_ITEMS)
	)
	const lines = shown.flatMap((e) => [formatDiffIndexEntry(e), ...fileChildrenLines(e)])
	const notes: string[] = []
	if (total > shown.length) {
		notes.push(
			`Showing ${shown.length} of ${total} drafts — narrow with types/path_prefix or raise limit.`
		)
	}
	if (index.otherUsersDraftCount > 0) {
		notes.push(
			`${index.otherUsersDraftCount} draft(s) by other users exist in this workspace (not shown — drafts are per-user).`
		)
	}
	if (unflushedPaths.length > 0) {
		notes.push(
			`Warning: unsaved editor changes on ${unflushedPaths.join(', ')} are NOT reflected here (auto-save off, a save failed, or a conflict is unresolved). Read the item with type+path to include them.`
		)
	}
	const filtersActive = (args.types?.length ?? 0) > 0 || !!args.path_prefix
	const summaryLine =
		total === 0
			? filtersActive && index.entries.length > 0
				? `No drafts match your filters (${index.entries.length} draft(s) exist in the workspace).`
				: 'No drafts in this workspace — nothing differs from the deployed state.'
			: `${total} draft(s) vs deployed:`
	const result = [summaryLine, ...lines, ...notes].join('\n')
	toolCallbacks.setToolStatus(toolId, {
		content: `Workspace diff: ${total} draft(s)`,
		result
	})
	return result
}

function forkComparisonUnavailableMessage(parent: string): string {
	return `The comparison with parent workspace "${parent}" is unavailable for this fork (created before comparison tracking existed).`
}

function forkParentOrThrow(workspace: string): string {
	const parent = getForkParentWorkspaceId(workspace)
	if (!parent) {
		throw new Error(
			`Workspace "${workspace}" is not a fork — it has no parent workspace to compare against. Use diff without 'against' to compare drafts vs deployed versions.`
		)
	}
	return parent
}

function forkEntryLabel(e: ForkDiffEntryView): string {
	const label = e.type === 'trigger' ? `${e.triggerKind} trigger` : (e.type ?? e.kind)
	return `${label} "${e.path}"`
}

function formatForkIndexEntry(e: ForkDiffEntryView): string {
	const name = forkEntryLabel(e)
	const draftFlag = e.hasLocalDraft ? ' [+ local draft]' : ''
	const aheadBehind = [
		e.ahead > 0 ? `ahead ${e.ahead}` : undefined,
		e.behind > 0 ? `behind ${e.behind}` : undefined
	]
		.filter(Boolean)
		.join(', ')
	switch (e.status) {
		case 'only_in_fork':
			return `- ${name} — only in fork (${e.patchLineCount} lines)${draftFlag}`
		case 'deleted_in_fork':
			return `- ${name} — deleted in fork, still in parent${draftFlag}`
		case 'modified':
			return `- ${name} — differs (${aheadBehind}; ${e.patchLineCount} diff lines)${draftFlag}`
		case 'unchanged':
			// Folder display_name lives only in the DB (no API surface exposes
			// it), so an identical projection cannot prove folder parity.
			if (e.kind === 'folder') {
				const suffix = aheadBehind ? ` (${aheadBehind})` : ''
				return `- ${name} — no comparable differences; the folder display name is not exposed by the API and may differ${suffix}${draftFlag}`
			}
			return e.valueMasked
				? `- ${name} — value never shown in chat; may differ (${aheadBehind})${draftFlag}`
				: `- ${name} — content matches parent (version history differs: ${aheadBehind})${draftFlag}`
		case 'pending':
			return e.type !== undefined
				? `- ${name} — differs (${aheadBehind}; diff not computed yet, read it with type+path)${draftFlag}`
				: `- ${name} — differs (${aheadBehind}; diff not computed yet, read it by path alone)${draftFlag}`
		case 'error':
			return `- ${name} — diff failed: ${e.errorMessage}${draftFlag}`
	}
}

// Comparison kinds a chat (type, trigger_kind) pair addresses.
function forkKindsFor(type: WorkspaceItemType, triggerKind?: TriggerKind): string[] {
	switch (type) {
		case 'app':
			return ['app', 'raw_app']
		case 'trigger':
			return triggerKind ? [`${triggerKind}_trigger`] : []
		default:
			return [type]
	}
}

// Fork index: deployed fork vs deployed parent, same tally as the fork banner.
async function diffForkIndex(
	args: { types?: WorkspaceItemType[]; path_prefix?: string; limit?: number },
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const parent = forkParentOrThrow(workspace)
	toolCallbacks.setToolStatus(toolId, {
		content: `Comparing fork with parent workspace "${parent}"...`
	})
	const index = await getForkDiffIndex(workspace, parent)
	if (index.skippedComparison) {
		const message = forkComparisonUnavailableMessage(parent)
		toolCallbacks.setToolStatus(toolId, { content: message })
		return message
	}
	let entries = index.entries
	if (args.types?.length) {
		entries = entries.filter((e) => e.type !== undefined && args.types!.includes(e.type))
	}
	if (args.path_prefix) {
		entries = entries.filter((e) => e.path.startsWith(args.path_prefix!))
	}
	const total = entries.length
	const shown = entries.slice(
		0,
		Math.min(args.limit ?? DIFF_INDEX_DEFAULT_ITEMS, DIFF_INDEX_MAX_ITEMS)
	)
	const lines = shown.flatMap((e) => [formatForkIndexEntry(e), ...fileChildrenLines(e)])
	const notes: string[] = []
	if (total > shown.length) {
		notes.push(
			`Showing ${shown.length} of ${total} items — narrow with types/path_prefix or raise limit.`
		)
	}
	if (shown.some((e) => e.hasLocalDraft)) {
		notes.push(
			'[+ local draft]: you also have an undeployed draft there — not part of this deployed-vs-deployed comparison; use diff without against to see it.'
		)
	}
	const hasHidden = index.hiddenAheadCount > 0 || index.hiddenBehindCount > 0
	if (hasHidden) {
		notes.push(
			`Hidden items you lack permission to view also differ: ${index.hiddenAheadCount} ahead, ${index.hiddenBehindCount} behind (a conflicted item counts in both).`
		)
	}
	const forkFiltersActive = (args.types?.length ?? 0) > 0 || !!args.path_prefix
	const summaryLine =
		total === 0
			? forkFiltersActive && index.entries.length > 0
				? `No differing items match your filters (${index.entries.length} differing item(s) exist).`
				: hasHidden
					? `No differences visible to you between this fork and its parent "${parent}" — but hidden items differ (see below).`
					: `This fork matches its parent workspace "${parent}" — no differences.`
			: `${total} item(s) differ between this fork and its parent "${parent}":`
	const result = [summaryLine, ...lines, ...notes].join('\n')
	toolCallbacks.setToolStatus(toolId, {
		content: `Fork vs parent: ${total} differing item(s)`,
		result
	})
	return result
}

// One item's fork-vs-parent unified diff (deployed sides only).
async function diffForkItem(
	args: {
		type?: WorkspaceItemType
		path?: string
		trigger_kind?: TriggerKind
		file?: string
		offset?: number
		limit?: number
	},
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const { type, path, trigger_kind: triggerKind } = args
	if (!path) {
		throw new Error('path is required.')
	}
	const parent = forkParentOrThrow(workspace)
	// No type = path-only wildcard: comparison kinds outside the chat type enum
	// (folder, resource_type, …) are only reachable this way.
	const kinds = type ? forkKindsFor(type, triggerKind) : []
	if (type === 'trigger' && kinds.length === 0) {
		throw new Error('trigger_kind is required when type is trigger.')
	}
	toolCallbacks.setToolStatus(toolId, {
		content: `Comparing fork vs parent for "${path}"...`
	})
	if ((await getForkComparisonStatus(workspace, parent)).skippedComparison) {
		const message = forkComparisonUnavailableMessage(parent)
		toolCallbacks.setToolStatus(toolId, { content: message })
		return message
	}
	const entries = await readForkDiffEntries(workspace, parent, kinds, path)
	if (entries.length === 0) {
		const message = `${type ?? 'item'} "${path}" does not differ between this fork and its parent "${parent}" (or does not exist in either).`
		toolCallbacks.setToolStatus(toolId, { content: message })
		return message
	}
	// A wildcard can match several kinds at one path (nothing in the chat
	// schema could pick between them) — render each kind's section.
	const sections = entries.map((entry) => renderForkEntrySection(entry, path, parent, args))
	const result = sections.join('\n\n====\n\n')
	toolCallbacks.setToolStatus(toolId, {
		content: `Fork vs parent diff for "${path}"`,
		result
	})
	return result
}

function renderForkEntrySection(
	entry: ForkDiffEntryView,
	path: string,
	parent: string,
	args: { file?: string; offset?: number; limit?: number }
): string {
	if (entry.status === 'error') {
		throw new Error(
			`Could not diff ${entry.kind} "${path}" against the parent: ${entry.errorMessage}`
		)
	}
	let draftCaveat = entry.hasLocalDraft
		? 'Note: you also have an undeployed local draft on this item — it is NOT part of this deployed-vs-deployed comparison; use diff without against to see it.\n\n'
		: ''
	if (entry.valueMasked && entry.status === 'modified') {
		draftCaveat +=
			'Note: variable values are never compared in chat — the value may also differ beyond the changes shown.\n\n'
	}
	const changedFileCount = entry.files ? Object.keys(entry.files).length : 0
	if (entry.status === 'unchanged' || (!entry.patch && changedFileCount === 0)) {
		// A masked value can differ in content without producing a patch —
		// never report that as "same content".
		const message = entry.valueMasked
			? `${entry.kind} "${path}": no visible config differences vs parent "${parent}", but variable values are never shown in chat, so a value change cannot be displayed. The workspace comparison reports it as ${entry.ahead > 0 || entry.behind > 0 ? `differing (ahead ${entry.ahead}, behind ${entry.behind})` : 'in sync'}.`
			: entry.kind === 'folder'
				? `folder "${path}": no comparable differences vs parent "${parent}" — the folder display name is not exposed by the API and may be what differs (comparison reports ahead ${entry.ahead}, behind ${entry.behind}).`
				: `${entry.kind} "${path}" has the same content in the fork and its parent "${parent}" (only version history differs).`
		return draftCaveat + message
	}
	const header =
		entry.status === 'only_in_fork'
			? `${entry.kind} "${path}" exists only in the fork — not in parent "${parent}". Full content:\n\n`
			: entry.status === 'deleted_in_fork'
				? `${entry.kind} "${path}" was deleted in the fork but still exists in parent "${parent}". Removed content:\n\n`
				: `Fork changes vs parent "${parent}" for ${entry.kind} "${path}":\n\n`
	if (args.file !== undefined && !entry.files) {
		throw new Error(
			`file only applies to multi-file apps; ${entry.kind} "${path}" diffs as a single document — call again without file.`
		)
	}
	const body = entry.files
		? renderEntryFiles(entry.files, entry.patch ?? '', args)
		: windowPatchBody(entry.patch ?? '', args.offset ?? 0, args.limit ?? DIFF_READ_DEFAULT_LINES)
	return draftCaveat + header + body
}

const DIFF_SEARCH_DEFAULT_MAX_MATCHES = 50
const DIFF_SEARCH_MAX_MATCHES_CEILING = 200

interface DiffSearchUnit {
	/** Item path, or `${itemPath}/${fileName}` for a file inside an app. */
	subject: string
	patch: string
}

function collectDiffSearchUnits(
	entries: Array<{
		path: string
		status?: string
		patch?: string
		files?: Record<string, DiffFileView>
	}>,
	out: DiffSearchUnit[],
	failedPaths: string[]
): void {
	for (const e of entries) {
		// A failed materialization has no patch — claiming "no matches" for it
		// would present an incomplete search as a definitive one.
		if (e.status === 'error') {
			failedPaths.push(e.path)
			continue
		}
		if (e.files) {
			for (const [name, fileDiff] of Object.entries(e.files)) {
				// App file keys lead with '/'; a raw join would yield `f/x//file`,
				// which slash-anchored globs like `f/x/*.tsx` can never match.
				out.push({ subject: `${e.path}/${name.replace(/^\/+/, '')}`, patch: fileDiff.patch })
			}
			if (e.patch) out.push({ subject: e.path, patch: e.patch })
		} else if (e.patch) {
			out.push({ subject: e.path, patch: e.patch })
		}
	}
}

// Literal substring search over the changed lines of every diff in the
// comparison. Materializes all patches first (search cannot skip any), then
// scans in memory — same output conventions as search_app.
async function diffSearch(
	args: {
		against?: 'deployed' | 'parent_workspace'
		search?: string
		file_glob?: string
		max_matches?: number
	},
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks } = ctx
	const query = args.search ?? ''
	if (query.length === 0) {
		throw new Error('search requires a non-empty string.')
	}
	toolCallbacks.setToolStatus(toolId, { content: `Searching diffs for "${query}"...` })

	const units: DiffSearchUnit[] = []
	const failedPaths: string[] = []
	let unflushedNote = ''
	if (args.against === 'parent_workspace') {
		const parent = forkParentOrThrow(workspace)
		const index = await getForkDiffIndex(workspace, parent, { materializeAll: true })
		if (index.skippedComparison) {
			const message = forkComparisonUnavailableMessage(parent)
			toolCallbacks.setToolStatus(toolId, { content: message })
			return message
		}
		collectDiffSearchUnits(index.entries, units, failedPaths)
	} else {
		const { unflushedPaths } = await flushGlobalDraftSaves(workspace)
		expireWorkspaceDiffList(workspace)
		if (unflushedPaths.length > 0) {
			unflushedNote = `\nWarning: unsaved editor changes on ${unflushedPaths.join(', ')} were not searched (auto-save off, a save failed, or a conflict is unresolved).`
		}
		const index = await getWorkspaceDiffIndex(workspace, { materializeAll: true })
		collectDiffSearchUnits(index.entries, units, failedPaths)
	}
	if (failedPaths.length > 0) {
		unflushedNote += `\nWarning: ${failedPaths.length === 1 ? 'this diff' : 'these diffs'} could not be computed and ${failedPaths.length === 1 ? 'was' : 'were'} NOT searched (matches may be missing): ${failedPaths.join(', ')}. Retry, or read the item${failedPaths.length === 1 ? '' : 's'} directly for the error.`
	}
	const filtered = args.file_glob
		? units.filter((u) => appFileMatchesGlob(u.subject, args.file_glob as string))
		: units

	const needle = query.toLowerCase()
	const maxMatches = Math.min(
		args.max_matches ?? DIFF_SEARCH_DEFAULT_MAX_MATCHES,
		DIFF_SEARCH_MAX_MATCHES_CEILING
	)
	const matches: { subject: string; line: number; text: string }[] = []
	let totalMatchCount = 0
	let renderedMatchCount = 0
	let subjectCount = 0
	let truncated = false
	for (const unit of filtered.sort((a, b) => a.subject.localeCompare(b.subject))) {
		const lines = unit.patch.split('\n')
		const changed = new Set(changedLineIndices(unit.patch))
		let unitHadMatch = false
		for (let i = 0; i < lines.length; i++) {
			if (!changed.has(i) || !lines[i].toLowerCase().includes(needle)) continue
			totalMatchCount++
			unitHadMatch = true
			if (renderedMatchCount >= maxMatches) {
				truncated = true
				continue
			}
			renderedMatchCount++
			const lo = Math.max(0, i - SEARCH_APP_CONTEXT_LINES)
			const hi = Math.min(lines.length - 1, i + SEARCH_APP_CONTEXT_LINES)
			for (let j = lo; j <= hi; j++) {
				matches.push({ subject: unit.subject, line: j + 1, text: lines[j] })
			}
		}
		if (unitHadMatch) subjectCount++
	}

	if (totalMatchCount === 0) {
		toolCallbacks.setToolStatus(toolId, { content: `No diff matches for "${query}"` })
		return (
			`No changed lines match. Try a broader or differently-spelled term${
				args.file_glob ? ', or drop the file_glob' : ''
			}.` + unflushedNote
		)
	}

	const header = `${totalMatchCount} changed line${totalMatchCount === 1 ? '' : 's'} match in ${subjectCount} diff${
		subjectCount === 1 ? '' : 's'
	}${truncated ? ` (showing the first ${maxMatches}; narrow with file_glob or a more specific query)` : ''}`
	const out: string[] = [header]
	let currentSubject = ''
	let budgetSpent = header.length
	let budgetHit = false
	const seen = new Set<string>()
	for (const m of matches) {
		const dedupeKey = `${m.subject}:${m.line}`
		if (seen.has(dedupeKey)) continue
		seen.add(dedupeKey)
		const text =
			m.text.length > SEARCH_APP_MAX_LINE_CHARS
				? `${m.text.slice(0, SEARCH_APP_MAX_LINE_CHARS)}… [line truncated]`
				: m.text
		const subjectHeader = m.subject === currentSubject ? '' : `${m.subject}\n`
		const row = `${subjectHeader}  ${m.line}: ${text}`
		if (budgetSpent + row.length + 1 > SEARCH_APP_TOTAL_CHAR_BUDGET) {
			budgetHit = true
			break
		}
		if (subjectHeader) currentSubject = m.subject
		out.push(row)
		budgetSpent += row.length + 1
	}
	if (budgetHit) {
		out.push(
			`… output truncated at the context budget — narrow with file_glob or a more specific query.`
		)
	}

	toolCallbacks.setToolStatus(toolId, {
		content: `Found ${totalMatchCount} matching changed line${totalMatchCount === 1 ? '' : 's'}`
	})
	return out.join('\n') + unflushedNote
}

// Flush a draft's pending editor autosave, then verify it actually landed before
// the caller re-reads the persisted draft. `flush()` resolves even when the save
// recorded a conflict (server has a newer version) or failed (network/5xx) — it
// does not throw — so without this check a deploy could publish a stale/conflicting
// draft. Abort with a clear message instead.
async function flushDraftOrThrow(
	query: Parameters<typeof UserDraftDbSyncer.flush>[0],
	label: string
): Promise<void> {
	await UserDraftDbSyncer.flush(query)
	if (UserDraftDbSyncer.getConflict(query).conflict) {
		throw new Error(
			`Cannot deploy ${label}: the draft has a conflicting newer version on the server. Open it in the editor and resolve the conflict first.`
		)
	}
	const { state, failureMessage } = UserDraftDbSyncer.getState(query)
	if (state === 'failed') {
		throw new Error(
			`Cannot deploy ${label}: saving the latest draft failed (${failureMessage ?? 'unknown error'}). Retry once the draft saves.`
		)
	}
}

async function deployDraft(
	args: {
		type: WorkspaceItemType
		path: string
		trigger_kind?: TriggerKind
		deployment_message?: string
		force?: boolean
	},
	ctx: WriteDraftCtx
): Promise<string> {
	const { workspace, toolId, toolCallbacks, sessionId } = ctx
	const {
		type,
		path,
		trigger_kind: triggerKind,
		deployment_message: deploymentMessage,
		force
	} = args

	if (type === 'trigger' && !triggerKind) {
		throw new Error('trigger_kind is required when deploying a trigger.')
	}

	const draft = await getGlobalDraft(workspace, type, path, triggerKind)
	if (!draft) {
		throw new Error(`No draft found for ${type} "${path}".`)
	}
	if (draft.value === undefined) {
		throw new Error(`Draft ${type} "${path}" has no value to deploy.`)
	}

	toolCallbacks.setToolStatus(toolId, {
		content: `Deploying ${type} "${path}"...`
	})

	let actions: ToolDisplayAction[] | undefined
	// Where the deploy actually lands — the app branch can resolve a different
	// target from the draft's own path fields; the mask rename below must track it.
	let deployedPath = path

	if (type === 'script' || type === 'flow') {
		// Promote the full persisted draft via the shared deploy module — the same
		// "promote a draft to deployed" code the compare page's Review & Deploy uses.
		// It deploys every field of the draft; the previous local builders dropped
		// most config fields (tag, priority, schema, description, concurrency…),
		// reading them from the already-deployed version instead. The other kinds
		// below already deploy their draft value directly, so only script/flow need
		// this. Scripts always create (with parent_hash); a flow on a deployed item
		// updates, a draft-only flow (no flow row) is created.
		// Address the draft by its STORAGE path: a draft_only item created in the
		// editor lives at a synthetic `u/{user}/draft_{uuid}` key while its chosen
		// path is held in the draft value. The shared deployer reads the draft via
		// getScriptByPath/getFlowByPath at the path we pass (then deploys at the
		// draft's own `path`), so passing the display/chosen path would 404. For a
		// draft on a deployed item the storage path is just the item path.
		const storagePath = getGlobalDraftStoragePath(workspace, type, path, triggerKind)
		// The shared deployer re-reads the persisted DB draft, but an open editor's
		// edit may still be parked in a debounced/disabled autosave. Flush it first so
		// we deploy the latest value (not a stale persisted one) — and so the
		// post-deploy draft delete doesn't drop an unsaved edit. `flush` always saves
		// the parked value (like Ctrl/Cmd+S), since the user explicitly asked to deploy.
		await flushDraftOrThrow({ workspace, itemKind: type, path: storagePath }, `${type} "${path}"`)
		// Stale-draft guard: block when the draft was forked from an older deploy than
		// the current head (unless force), pointing the model at rebase_draft.
		if (type === 'script') {
			const existing = (await ScriptService.existsScriptByPath({ workspace, path }))
				? await ScriptService.getScriptByPath({ workspace, path })
				: undefined
			assertDraftBasedOnLatest('script', path, draft.parentHash, existing?.hash, force)
		} else {
			const existing = (await FlowService.existsFlowByPath({ workspace, path }))
				? await FlowService.getFlowByPath({ workspace, path })
				: undefined
			assertDraftBasedOnLatest('flow', path, draft.parentVersionId, existing?.version_id, force)
		}
		const draftOnly =
			type === 'flow'
				? !(await FlowService.existsFlowByPath({ workspace, path: storagePath }))
				: false
		const result = await deployDraftToWorkspace(type, storagePath, workspace, {
			draftOnly,
			deploymentMessage
		})
		if (!result.success) {
			throw new Error(result.error ?? `Failed to deploy ${type} "${path}".`)
		}
	} else {
		switch (type) {
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
				// The chat keeps secret draft values only in memory (the DB draft
				// stores `''`); buildVariableDeployRequestBody re-injects the ephemeral
				// secret, so this can't go through the DB-reading shared deployer.
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
				// Raw apps store a flat AppDraftValue (files/runnables at top level),
				// not the deployed app's nested `value` shape the shared raw-app
				// deployer reads, so they deploy through the chat's own bundle path.
				const appDraft = draft.value as AppDraftValue
				// Stale-draft guard: only fetch the deployed head when the draft records
				// a fork base to compare against (pre-feature drafts have none).
				if (draft.parentVersionId != null) {
					const deployedApp = (await AppService.existsApp({ workspace, path }))
						? await AppService.getAppByPath({ workspace, path })
						: undefined
					assertDraftBasedOnLatest(
						'app',
						path,
						draft.parentVersionId,
						deployedApp?.versions?.[deployedApp.versions.length - 1],
						force
					)
				}
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
				// Deploy at the draft's chosen path. A draft_only raw app created in the
				// editor lives at a synthetic `u/{user}/draft_{uuid}` storage key with its
				// chosen path in the raw_app draft's `draft_path`; the chat's AppDraftValue
				// doesn't carry it, so read it from the backend draft. For a chat-created app
				// (real path, no draft_path) or a draft on a deployed app, the storage path
				// is the deploy path. Same storage-path resolution as script/flow.
				const storagePath = getGlobalDraftStoragePath(workspace, 'app', path)
				// `draft_path` is read from the persisted backend draft below, but an
				// editor rename may still be parked in a debounced/disabled autosave.
				// Flush first (like script/flow) so we read the latest chosen path.
				await flushDraftOrThrow(
					{ workspace, itemKind: 'raw_app', path: storagePath },
					`app "${path}"`
				)
				let targetPath = storagePath
				try {
					const row = (await AppService.getAppByPath({
						workspace,
						path: storagePath,
						getDraft: true,
						rawApp: true
					})) as { draft?: { draft_path?: string; path?: string }; draft_path?: string }
					targetPath = row?.draft?.draft_path ?? row?.draft?.path ?? row?.draft_path ?? storagePath
				} catch (e) {
					// Only a missing item (404) justifies falling back to the storage path;
					// a real lookup failure (network/5xx) must abort rather than silently
					// deploy to the wrong path.
					if ((e as { status?: number } | null | undefined)?.status === 404) {
						targetPath = storagePath
					} else {
						throw e
					}
				}
				deployedPath = targetPath
				if (await AppService.existsApp({ workspace, path: targetPath })) {
					// Omit custom_path on update for now. The backend preserves it when absent, while
					// sending it requires admin privileges; this chat deploy path does not yet mirror
					// the raw app editor's user/admin-specific custom_path handling.
					await AppService.updateAppRaw({
						workspace,
						path: targetPath,
						formData: {
							app: {
								path: targetPath,
								value: rawAppValue,
								summary,
								policy,
								deployment_message: deploymentMessage,
								// Preserve the policy's on_behalf_of: this chat deploy path has no
								// on-behalf-of selector, so without the flag the backend resets it to
								// the deploying user (gated server-side by can_preserve_on_behalf_of).
								preserve_on_behalf_of: policy.on_behalf_of ? true : undefined
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
								path: targetPath,
								value: rawAppValue,
								summary,
								policy,
								deployment_message: deploymentMessage,
								custom_path: appValue.custom_path,
								// Preserve the policy's on_behalf_of (see update branch above).
								preserve_on_behalf_of: policy.on_behalf_of ? true : undefined
							},
							js: bundle.js,
							css: bundle.css
						}
					})
				}
				break
			}
		}
	}

	// Deployed state moved for EVERY branch above (some bypass
	// deployDraftToWorkspace, which invalidates on its own path) — evict cached
	// fork comparisons before the fallible draft cleanup below.
	invalidateWorkspaceComparison(workspace)

	await deleteGlobalDraft(workspace, type, path, triggerKind, { preserveLiveDraft: true })

	// Move the chat's mask entry to the deployed path: a draft-only item's
	// synthetic storage key never exists deployed, so the entry would otherwise
	// stop matching anything after the draft is gone.
	const deployedKind = itemKindFor(type, triggerKind)
	if (deployedKind) {
		toolCallbacks.onItemDeployed?.(
			deployedKind,
			getGlobalDraftStoragePath(workspace, type, path, triggerKind),
			deployedPath
		)
	}

	// Reload the session preview if it's open on the deployed item. Map the
	// deploy type to the preview kind — a raw app deploys under 'app' but the
	// preview addresses it as 'raw_app'; non-previewable types map to undefined.
	const previewKindByType: Partial<Record<WorkspaceItemType, 'script' | 'flow' | 'raw_app'>> = {
		script: 'script',
		flow: 'flow',
		app: 'raw_app'
	}
	const kind = previewKindByType[type]
	if (kind) deployedInSessionHandler?.({ sessionId, kind, path })

	toolCallbacks.setToolStatus(toolId, {
		content: `Deployed ${type} "${path}"`,
		result: 'Deployed',
		actions
	})
	return JSON.stringify(
		{
			success: true,
			message: `Deployed draft ${type} "${path}" to the workspace. Draft removed.`,
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

	// Deployed state changed — cached fork comparisons involving this workspace
	// are no longer trustworthy (same rule as deploy success). Before the
	// draft cleanup: a cleanup failure must not leave stale comparisons.
	invalidateWorkspaceComparison(workspace)
	await deleteGlobalDraft(workspace, type, path, triggerKind)

	// Record the deletion in the chat's modified-items mask. In a fork this leaves a
	// reviewable "removed" diff vs the parent that stays scoped to this chat. Keyed
	// by the same (itemKind, storagePath) as writes so it joins the draft/fork lists.
	const deletedKind = itemKindFor(type, triggerKind)
	if (deletedKind) {
		toolCallbacks.onItemModified?.(
			deletedKind,
			getGlobalDraftStoragePath(workspace, type, path, triggerKind)
		)
	}

	toolCallbacks.setToolStatus(toolId, {
		content: `Deleted ${type} "${path}"`,
		result: 'Deleted'
	})
	return JSON.stringify(
		{
			success: true,
			message: `Deleted ${type} "${path}" from the workspace. Any matching draft was also cleared.`,
			type,
			path,
			triggerKind
		},
		null,
		2
	)
}

export function prepareGlobalSystemMessage(
	instructions?: { workspace?: string; user?: string },
	opts?: {
		previewTools?: boolean
		// Identity the path-convention guidance is built from. Production omits it
		// (read from userStore); callers that must not touch the process-global
		// store (the eval harness) pass it explicitly instead.
		user?: { username: string; is_admin?: boolean; folders?: string[]; folders_read?: string[] }
		skills?: AiSkillListItem[]
	}
): ChatCompletionSystemMessageParam {
	const user = opts?.user ?? get(userStore)
	const username = user?.username ?? ''
	const folderCtx: FolderPromptContext | undefined = user
		? {
				folders: user.folders ?? [],
				foldersRead: user.folders_read ?? user.folders ?? [],
				isAdmin: user.is_admin ?? false
			}
		: undefined
	let content = buildGlobalSystemPrompt(
		username,
		opts?.previewTools ?? false,
		folderCtx,
		opts?.skills ?? []
	)
	if (instructions?.workspace?.trim()) {
		content = `${content}\n\nWORKSPACE INSTRUCTIONS (configured by a workspace admin, shared by everyone in this workspace — you cannot modify these):\n${instructions.workspace.trim()}`
	}
	if (instructions?.user?.trim()) {
		content = `${content}\n\nUSER INSTRUCTIONS (this user's personal instructions — update them with the update_user_instructions tool when the user asks you to remember, change, or stop something):\n${instructions.user.trim()}`
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
		(context) =>
			context.type === 'workspace_script' ||
			context.type === 'workspace_flow' ||
			context.type === 'workspace_app'
	)
	const activeEditor =
		options.activeEditor ??
		(options.workspace ? getActiveGlobalEditorContext(options.workspace) : undefined)
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
			const itemType =
				context.type === 'workspace_script'
					? 'script'
					: context.type === 'workspace_flow'
						? 'flow'
						: 'raw_app'
			content += `- type: ${itemType}, path: ${context.path}\n`
		}
		content += '\n'
	}

	const domSelectors = selectedContext.filter((c) => c.type === 'app_dom_selector')
	if (domSelectors.length > 0) {
		content += '## SELECTED DOM ELEMENTS\n'
		content +=
			"The user pointed at these elements in the live raw app preview. Their HTML is not included here — inspect it live with search_dom / read_dom, passing the element's `app_path` and `selector` so the right app's preview is read.\n"
		for (const el of domSelectors) {
			content += `- ${el.title} — app_path: ${el.appPath}, selector: ${el.selector}\n`
		}
		content += '\n'
	}

	const files = options.files ?? []
	if (files.length > 0) {
		content += '## ATTACHED FILES\n'
		content +=
			'The user attached these files to this message. Their content is NOT included here — read it with `read_file` (or scan it with `search_files`), passing the file id, before answering questions about it.\n'
		for (const f of files) {
			// textLineCount matches read_file's numbering — a mismatch would make the
			// model request line ranges past the end.
			const lines = textLineCount(f.content)
			// The id is the durable reference (names may repeat across messages);
			// absent only on legacy pre-id transcripts, where the name resolves.
			// Sanitized again here: legacy names predate attach-time sanitization.
			const name = sanitizeAttachmentName(f.name)
			const ref = f.id ? `${name} (file id: ${f.id})` : name
			content += `- ${ref} — ${lines} lines, ${f.content.length} chars\n`
		}
		content += '\n'
	}

	content += `## INSTRUCTIONS:\n${instructions}`

	const images = options.images ?? []
	if (images.length > 0) {
		// Multimodal message: the text block plus one image_url part per attachment.
		// The provider converters translate image_url for Anthropic/Responses; the
		// OpenAI-compatible path sends it as-is.
		return {
			role: 'user',
			content: [
				{ type: 'text', text: content },
				...images.map((img) => dataUrlToImagePart(img.dataUrl))
			]
		}
	}

	return {
		role: 'user',
		content
	}
}
