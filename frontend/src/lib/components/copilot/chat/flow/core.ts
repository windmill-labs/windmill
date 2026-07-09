import { type FlowModule, type InputTransform, type RawScript } from '$lib/gen'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import { z } from 'zod'
import {
	createDbSchemaTool,
	getFormattedResourceTypes,
	getLangContext,
	SUPPORTED_CHAT_SCRIPT_LANGUAGES
} from '../script/core'
import {
	createSearchHubScriptsTool,
	createToolDef,
	findAndReplace,
	type Tool,
	executeTestRun,
	buildSchemaForTool,
	buildTestRunArgs,
	buildContextString,
	applyCodePiecesToFlowModules,
	SPECIAL_MODULE_IDS,
	formatScriptLintResult,
	type ScriptLintResult,
	createSearchWorkspaceTool,
	createGetRunnableDetailsTool,
	executeFlowStepTestRun
} from '../shared'
import { createWorkspaceMutationTools } from '../workspaceTools'
import type { ContextElement } from '../context'
import type { ExtendedOpenFlow } from '$lib/components/flows/types'
import { findModuleInFlow, findModuleInModules } from '$lib/components/flows/flowTree'
import { createInlineScriptSession, type InlineScriptSession } from './inlineScriptsUtils'
import {
	validateFlowGroups,
	validateFlowNotes,
	type FlowGroup,
	type FlowNote,
	type FlowJsonUpdateResult
} from './helperUtils'
import { flowModuleSchema } from './openFlowZod.gen'
import { collectAllFlowModuleIdsFromModules } from '$lib/components/flows/flowTree'
import {
	buildEditableFlowJson as buildEditableFlowJsonBase,
	validateEditableFlowJson,
	validateFlowModules,
	validateFlowSchema,
	type EditableFlowJson
} from './editableFlowJson'
import { FLOW_CHAT_SPECIAL_MODULES, getFlowPrompt } from '$system_prompts'

type FlowJsonUpdate = {
	modules?: FlowModule[]
	schema?: Record<string, any> | null
	preprocessorModule?: FlowModule | null
	failureModule?: FlowModule | null
	groups?: FlowGroup[] | null
	notes?: FlowNote[] | null
}

function formatEmptyInlineScriptWarning({
	emptyInlineScriptModuleIds
}: FlowJsonUpdateResult): string {
	if (emptyInlineScriptModuleIds.length === 0) {
		return ''
	}
	const moduleList = emptyInlineScriptModuleIds.map((id) => `'${id}'`).join(', ')
	return ` Warning: inline scripts ${moduleList} are empty for now. Use set_module_code to fill them in.`
}

/**
 * Build the agent-facing compact view of a flow, with editor-only post-processing
 * (insert `[#START]` / `[#END]` markers around code pieces selected as context).
 * Delegates to the shared `buildEditableFlowJson` for the rest.
 */
function buildEditableFlowJson(
	flow: ExtendedOpenFlow,
	inlineScriptSession?: InlineScriptSession,
	selectedContext: ContextElement[] = []
): EditableFlowJson {
	const json = buildEditableFlowJsonBase(
		{ value: flow.value, schema: flow.schema },
		inlineScriptSession
	)
	const codePieces = selectedContext.filter((c) => c.type === 'flow_module_code_piece')
	if (codePieces.length === 0) {
		return json
	}
	return { ...json, modules: applyCodePiecesToFlowModules(codePieces, json.modules) }
}

/**
 * Helper interface for AI chat flow operations
 *
 * Note: AI chat is only responsible for setting the beforeFlow snapshot when making changes.
 * Accept/reject operations are exposed here but implemented via flowDiffManager.
 */
export interface FlowAIChatHelpers {
	// flow context
	getFlowAndSelectedId: () => { flow: ExtendedOpenFlow; selectedId: string }
	getRootModules: () => FlowModule[]
	inlineScriptSession: InlineScriptSession

	// snapshot management (AI sets this when making changes)
	/** Set the before flow snapshot */
	setSnapshot: (snapshot: ExtendedOpenFlow) => void
	/** Revert the entire flow to a previous snapshot */
	revertToSnapshot: (snapshot?: ExtendedOpenFlow) => void

	// ai chat tools
	setCode: (id: string, code: string) => Promise<void>
	setFlowJson: (update: FlowJsonUpdate) => Promise<FlowJsonUpdateResult>
	getFlowInputsSchema: () => Promise<Record<string, any>>
	/** Update exprsToSet store for InputTransformForm components (only if module is selected) */
	updateExprsToSet: (id: string, inputTransforms: Record<string, InputTransform>) => void

	// accept/reject operations (via flowDiffManager)
	/** Accept all pending module changes */
	acceptAllModuleActions: () => void
	/** Reject all pending module changes */
	rejectAllModuleActions: () => void
	/** Check if there are pending changes requiring user approval */
	hasPendingChanges: () => boolean
	/** Select a step in the flow */
	selectStep: (id: string) => void

	/** Run a test of the current flow using the UI's preview mechanism */
	testFlow: (args?: Record<string, any>, conversationId?: string) => Promise<string | undefined>

	/** Get lint errors from a specific module (focuses it first, waits for Monaco to analyze) */
	getLintErrors: (moduleId: string) => Promise<ScriptLintResult>
}

const langSchema = z.enum(
	SUPPORTED_CHAT_SCRIPT_LANGUAGES as [RawScript['language'], ...RawScript['language'][]]
)

const resourceTypeToolSchema = z.object({
	query: z.string().describe('The query to search for, e.g. stripe, google, etc..'),
	language: langSchema.describe(
		'The programming language the code using the resource type will be written in'
	)
})

const resourceTypeToolDef = createToolDef(
	resourceTypeToolSchema,
	'resource_type',
	'Search for resource types (e.g., postgresql, stripe). Returns formatted resource type definitions with usage examples.'
)

const getInstructionsForCodeGenerationToolSchema = z.object({
	language: langSchema.describe('The programming language the code will be written in')
})

const getInstructionsForCodeGenerationToolDef = createToolDef(
	getInstructionsForCodeGenerationToolSchema,
	'get_instructions_for_code_generation',
	'Get instructions for code generation for a raw script step'
)

const specialModuleToolArgSchema = z
	.string()
	.nullable()
	.describe(
		'JSON string containing the special module object. Use null to remove the special module.'
	)

// Using string for modules and schema because Gemini-2.5-flash performs better with strings (MALFORMED_FUNCTION_CALL errors happens more often with objects)
const setFlowJsonToolSchema = z.object({
	modules: z.string().optional().nullable().describe('JSON string containing the flow modules'),
	schema: z.string().optional().nullable().describe('JSON string containing the flow input schema'),
	preprocessor_module: z
		.string()
		.optional()
		.nullable()
		.describe('JSON string containing the optional preprocessor module'),
	failure_module: z
		.string()
		.optional()
		.nullable()
		.describe('JSON string containing the optional failure module'),
	groups: z
		.string()
		.optional()
		.nullable()
		.describe(
			'JSON string containing the optional array of semantic flow groups. Each group has summary, note, autocollapse, start_id, end_id, color. color MUST be one of: yellow, blue, green, purple, pink, orange, red, cyan, lime, gray — never hex codes or other strings. Pass null to clear groups.'
		),
	notes: z
		.string()
		.optional()
		.nullable()
		.describe(
			'JSON string containing the optional array of free-floating sticky notes attached to the flow. Use notes to surface important flow-wide information (what the flow does, key assumptions, warnings, TODOs). Each note has id (unique string), text (markdown), color (one of: yellow, blue, green, purple, pink, orange, red, cyan, lime, gray — never hex codes), and optional position {x,y} and size {width,height} — omit both and the editor places and sizes the note automatically. Always use type "free". The "group" note type is DEPRECATED — to segment a complex flow into labelled, colored sections use the `groups` field instead (each group carries its own note and color). Pass null to clear notes.'
		)
})

const setFlowJsonToolDef = createToolDef(
	setFlowJsonToolSchema,
	'set_flow_json',
	'Set the complete flow modules array and optionally the flow input schema, preprocessor module, failure module, and semantic groups.',
	{ strict: false }
)

const setPreprocessorModuleToolSchema = z.object({
	module: specialModuleToolArgSchema
})

const setPreprocessorModuleToolDef = createToolDef(
	setPreprocessorModuleToolSchema,
	'set_preprocessor_module',
	'Set or replace the flow preprocessor module. Use this when the flow needs logic that runs before the main modules.'
)

const setFailureModuleToolSchema = z.object({
	module: specialModuleToolArgSchema
})

const setFailureModuleToolDef = createToolDef(
	setFailureModuleToolSchema,
	'set_failure_module',
	'Set or replace the flow failure module. Use this when the flow needs a dedicated error handler.'
)

const specialFlowModuleFields = {
	preprocessor_module: SPECIAL_MODULE_IDS.PREPROCESSOR,
	failure_module: SPECIAL_MODULE_IDS.FAILURE
} as const

type SpecialFlowModuleField = keyof typeof specialFlowModuleFields

function parseOptionalJsonArg(value: unknown, field: string): unknown {
	if (value === undefined || value === null) {
		return value
	}

	try {
		return typeof value === 'string' ? JSON.parse(value) : value
	} catch (e) {
		const errorMessage = e instanceof Error ? e.message : String(e)
		throw new Error(`Invalid JSON for ${field}: ${errorMessage}`)
	}
}

function validateSpecialFlowModule(
	module: unknown,
	field: SpecialFlowModuleField
): FlowModule | null | undefined {
	if (module === undefined || module === null) {
		return module
	}

	const result = flowModuleSchema.safeParse(module)
	if (!result.success) {
		const errors = result.error.issues.slice(0, 5).map((issue) => {
			const path = issue.path.length > 0 ? issue.path.join('.') : field
			return `${path}: ${issue.message}`
		})
		throw new Error(`Invalid ${field}:\n${errors.join('\n')}`)
	}

	const parsedModule = result.data
	const expectedId = specialFlowModuleFields[field]
	if (parsedModule.id !== expectedId) {
		throw new Error(`Invalid ${field}: id must be "${expectedId}"`)
	}

	if (parsedModule.value.type !== 'rawscript' && parsedModule.value.type !== 'script') {
		throw new Error(`Invalid ${field}: only "rawscript" and "script" modules are supported`)
	}

	return parsedModule
}

const patchFlowJsonSchema = z.object({
	old_string: z.string().min(1).describe('Exact text to find in the current compact flow JSON'),
	new_string: z.string().describe('Replacement JSON text'),
	replace_all: z
		.boolean()
		.optional()
		.default(false)
		.describe(
			'When true, replace every exact match. When false, the search text must match exactly once.'
		)
})

const patchFlowJsonToolDef = createToolDef(
	patchFlowJsonSchema,
	'patch_flow_json',
	'Make a quick exact text edit in the current compact flow JSON. Prefer this for small localized changes; use set_flow_json for larger structural rewrites.'
)
// Will be overridden by setSchema
const testRunFlowSchema = z.object({
	args: z
		.object({})
		.describe('Arguments to pass to the flow (optional, uses default flow inputs if not provided)')
})

const testRunFlowToolDef = createToolDef(
	testRunFlowSchema,
	'test_run_flow',
	'Execute a test run of the current flow'
)

const testRunStepSchema = z.object({
	stepId: z.string().describe('The id of the step to test'),
	args: z
		.object({})
		.nullable()
		.optional()
		.describe('Arguments to pass to the step (optional, uses default step inputs if not provided)')
})

const testRunStepToolDef = createToolDef(
	testRunStepSchema,
	'test_run_step',
	'Execute a test run of a specific step in the flow',
	{ strict: false }
)

const inspectInlineScriptSchema = z.object({
	moduleId: z
		.string()
		.describe('The ID of the module whose inline script content you want to inspect')
})

const inspectInlineScriptToolDef = createToolDef(
	inspectInlineScriptSchema,
	'inspect_inline_script',
	'Inspect the full content of an inline script. Use this to view the actual script code before making changes with set_module_code.'
)

const setModuleCodeSchema = z.object({
	moduleId: z.string().describe('The ID of the module to set code for'),
	code: z.string().describe('The full script code content')
})

const setModuleCodeToolDef = createToolDef(
	setModuleCodeSchema,
	'set_module_code',
	'Set or modify the code for an existing inline script module. Use this for quick code-only changes. The module must already exist in the flow.'
)

const getLintErrorsSchema = z.object({
	module_id: z.string().describe('The ID of the module to get lint errors for.')
})

const getLintErrorsToolDef = createToolDef(
	getLintErrorsSchema,
	'get_lint_errors',
	'Get lint errors and warnings from a rawscript module. Pass module_id to focus a specific module and check its errors. ALWAYS call this for EACH module where you modified inline script code.'
)

export const flowTools: Tool<FlowAIChatHelpers>[] = [
	createSearchHubScriptsTool(false),
	createDbSchemaTool<FlowAIChatHelpers>(),
	createSearchWorkspaceTool(),
	createGetRunnableDetailsTool(),
	...createWorkspaceMutationTools<FlowAIChatHelpers>(),
	{
		def: resourceTypeToolDef,
		fn: async ({ args, toolId, workspace, toolCallbacks }) => {
			const parsedArgs = resourceTypeToolSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, {
				content: 'Searching resource types for "' + parsedArgs.query + '"...'
			})
			const formattedResourceTypes = await getFormattedResourceTypes(
				parsedArgs.language,
				parsedArgs.query,
				workspace
			)
			toolCallbacks.setToolStatus(toolId, {
				content: 'Retrieved resource types for "' + parsedArgs.query + '"'
			})
			return formattedResourceTypes
		}
	},
	{
		def: getInstructionsForCodeGenerationToolDef,
		fn: async ({ args, toolId, toolCallbacks }) => {
			const parsedArgs = getInstructionsForCodeGenerationToolSchema.parse(args)
			const langContext = getLangContext(parsedArgs.language, {
				allowResourcesFetch: true
			})
			toolCallbacks.setToolStatus(toolId, {
				content: 'Retrieved instructions for code generation in ' + parsedArgs.language
			})
			return langContext
		}
	},
	{
		def: testRunFlowToolDef,
		fn: async function ({ args, workspace, helpers, toolCallbacks, toolId }) {
			const { flow } = helpers.getFlowAndSelectedId()

			if (!flow || !flow.value) {
				toolCallbacks.setToolStatus(toolId, {
					content: 'No flow available to test',
					error: 'No flow found in current context'
				})
				throw new Error(
					'No flow available to test. Please ensure you have a flow open in the editor.'
				)
			}

			const parsedArgs = await buildTestRunArgs(args, this.def)
			// Use the UI test mechanism - this opens the preview panel
			return executeTestRun({
				jobStarter: async () => {
					const jobId = await helpers.testFlow(parsedArgs)
					if (!jobId) {
						throw new Error('Failed to start test run - testFlow returned undefined')
					}
					return jobId
				},
				workspace,
				toolCallbacks,
				toolId,
				startMessage: 'Starting flow test run...',
				contextName: 'flow'
			})
		},
		setSchema: async function (helpers: FlowAIChatHelpers) {
			await buildSchemaForTool(this.def, async () => {
				const flowInputsSchema = await helpers.getFlowInputsSchema()
				return flowInputsSchema
			})
		},
		requiresConfirmation: true,
		confirmationMessage: 'Run a test of the current flow',
		showDetails: true,
		autoCollapseDetails: false
	},
	{
		// set strict to false to avoid issues with open ai models
		def: testRunStepToolDef,
		fn: async ({ args, workspace, helpers, toolCallbacks, toolId }) => {
			const { flow } = helpers.getFlowAndSelectedId()

			if (!flow || !flow.value) {
				toolCallbacks.setToolStatus(toolId, {
					content: 'No flow available to test step from',
					error: 'No flow found in current context'
				})
				throw new Error(
					'No flow available to test step from. Please ensure you have a flow open in the editor.'
				)
			}

			const stepId = args.stepId
			const stepArgs = args.args || {}

			return executeFlowStepTestRun({
				flowValue: flow.value,
				stepId,
				args: stepArgs,
				workspace,
				toolCallbacks,
				toolId
			})
		},
		requiresConfirmation: true,
		confirmationMessage: (args) => `Run a test of step "${args?.stepId ?? ''}"`,
		showDetails: true,
		autoCollapseDetails: false
	},
	{
		def: inspectInlineScriptToolDef,
		fn: async ({ args, helpers, toolCallbacks, toolId }) => {
			const parsedArgs = inspectInlineScriptSchema.parse(args)
			const moduleId = parsedArgs.moduleId

			toolCallbacks.setToolStatus(toolId, {
				content: `Retrieving inline script content for module '${moduleId}'...`
			})

			const content = helpers.inlineScriptSession.get(moduleId)

			if (content === undefined) {
				toolCallbacks.setToolStatus(toolId, {
					content: `Module '${moduleId}' not found in inline script store`,
					error: `No inline script found for module ID '${moduleId}'`
				})
				throw new Error(
					`Module '${moduleId}' not found. This module either doesn't exist, isn't a rawscript, or wasn't replaced with a reference.`
				)
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Retrieved inline script for module '${moduleId}'`
			})

			return JSON.stringify({
				moduleId,
				content,
				note: 'To modify this script, use the set_module_code tool with the new code'
			})
		}
	},
	{
		def: setModuleCodeToolDef,
		streamArguments: true,
		showDetails: true,
		showFade: true,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setModuleCodeSchema.parse(args)
			const { moduleId, code } = parsedArgs

			toolCallbacks.setToolStatus(toolId, { content: `Setting code for module '${moduleId}'...` })

			// Update the flow directly via helper
			await helpers.setCode(moduleId, code)

			toolCallbacks.setToolStatus(toolId, {
				content: `Code updated for module '${moduleId}'`,
				result: 'Success'
			})
			return `Code for module '${moduleId}' has been updated successfully.`
		}
	},
	{
		def: patchFlowJsonToolDef,
		streamArguments: true,
		showDetails: true,
		showFade: true,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = patchFlowJsonSchema.parse(args)
			const { old_string: oldString, new_string: newString, replace_all: replaceAll } = parsedArgs
			const { flow, selectedId } = helpers.getFlowAndSelectedId()
			// Snapshot the current flow with a fresh session so the compact JSON matches what the model saw,
			// then copy extracted inline scripts back into the helper session before applying the patch.
			const inlineScriptSession = createInlineScriptSession()
			const currentFlowJson = JSON.stringify(buildEditableFlowJson(flow, inlineScriptSession))

			toolCallbacks.setToolStatus(toolId, {
				content: 'Applying JSON patch...'
			})

			const updatedFlowJson = findAndReplace(
				currentFlowJson,
				oldString,
				newString,
				replaceAll,
				'current flow JSON'
			)

			let parsedFlow: EditableFlowJson
			try {
				parsedFlow = validateEditableFlowJson(JSON.parse(updatedFlowJson))
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error)
				throw new Error(`Invalid JSON after replacement: ${message}`)
			}

			for (const [moduleId, content] of Object.entries(inlineScriptSession.getAll())) {
				helpers.inlineScriptSession.set(moduleId, content)
			}

			const updateResult = await helpers.setFlowJson({
				modules: parsedFlow.modules,
				schema: parsedFlow.schema,
				preprocessorModule: parsedFlow.preprocessor_module,
				failureModule: parsedFlow.failure_module,
				groups: parsedFlow.groups,
				notes: parsedFlow.notes
			})
			const warning = formatEmptyInlineScriptWarning(updateResult)

			const selectedModule = findModuleInFlow(parsedFlow, selectedId) ?? undefined
			if (
				selectedModule &&
				'input_transforms' in selectedModule.value &&
				selectedModule.value.input_transforms
			) {
				helpers.updateExprsToSet(selectedId, selectedModule.value.input_transforms)
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Updated flow JSON`,
				result: 'Success'
			})

			return `Flow JSON updated.${warning}`
		}
	},
	{
		def: setPreprocessorModuleToolDef,
		streamArguments: true,
		showDetails: true,
		showFade: true,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setPreprocessorModuleToolSchema.parse(args)
			const parsedModule = validateSpecialFlowModule(
				parseOptionalJsonArg(parsedArgs.module, 'module'),
				'preprocessor_module'
			)

			toolCallbacks.setToolStatus(toolId, {
				content:
					parsedModule === null
						? 'Removing preprocessor module...'
						: 'Setting preprocessor module...'
			})
			const updateResult = await helpers.setFlowJson({ preprocessorModule: parsedModule })
			const warning = formatEmptyInlineScriptWarning(updateResult)

			if (
				parsedModule &&
				helpers.getFlowAndSelectedId().selectedId === SPECIAL_MODULE_IDS.PREPROCESSOR &&
				'input_transforms' in parsedModule.value &&
				parsedModule.value.input_transforms
			) {
				helpers.updateExprsToSet(parsedModule.id, parsedModule.value.input_transforms)
			}

			toolCallbacks.setToolStatus(toolId, {
				content:
					parsedModule === null ? 'Preprocessor module removed' : 'Preprocessor module updated',
				result: 'Success'
			})
			return parsedModule === null
				? 'Preprocessor module removed'
				: `Preprocessor module updated successfully.${warning}`
		}
	},
	{
		def: setFailureModuleToolDef,
		streamArguments: true,
		showDetails: true,
		showFade: true,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setFailureModuleToolSchema.parse(args)
			const parsedModule = validateSpecialFlowModule(
				parseOptionalJsonArg(parsedArgs.module, 'module'),
				'failure_module'
			)

			toolCallbacks.setToolStatus(toolId, {
				content: parsedModule === null ? 'Removing failure module...' : 'Setting failure module...'
			})
			const updateResult = await helpers.setFlowJson({ failureModule: parsedModule })
			const warning = formatEmptyInlineScriptWarning(updateResult)

			if (
				parsedModule &&
				helpers.getFlowAndSelectedId().selectedId === SPECIAL_MODULE_IDS.FAILURE &&
				'input_transforms' in parsedModule.value &&
				parsedModule.value.input_transforms
			) {
				helpers.updateExprsToSet(parsedModule.id, parsedModule.value.input_transforms)
			}

			toolCallbacks.setToolStatus(toolId, {
				content: parsedModule === null ? 'Failure module removed' : 'Failure module updated',
				result: 'Success'
			})
			return parsedModule === null
				? 'Failure module removed'
				: `Failure module updated successfully.${warning}`
		}
	},
	{
		def: setFlowJsonToolDef,
		streamArguments: true,
		showDetails: true,
		showFade: true,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const { modules, schema, preprocessor_module, failure_module, groups, notes } = args

			let parsedModules: FlowModule[] | null | undefined
			let parsedSchema: Record<string, any> | null | undefined
			let parsedPreprocessorModule: FlowModule | null | undefined
			let parsedFailureModule: FlowModule | null | undefined
			let parsedGroups: FlowGroup[] | null | undefined
			let parsedNotes: FlowNote[] | null | undefined

			// Parse JSON strings
			parsedModules = parseOptionalJsonArg(modules, 'modules') as FlowModule[] | null | undefined
			parsedSchema = parseOptionalJsonArg(schema, 'schema') as
				| Record<string, any>
				| null
				| undefined
			parsedPreprocessorModule = parseOptionalJsonArg(
				preprocessor_module,
				'preprocessor_module'
			) as FlowModule | null | undefined
			parsedFailureModule = parseOptionalJsonArg(failure_module, 'failure_module') as
				| FlowModule
				| null
				| undefined
			parsedGroups = parseOptionalJsonArg(groups, 'groups') as FlowGroup[] | null | undefined
			parsedNotes = parseOptionalJsonArg(notes, 'notes') as FlowNote[] | null | undefined
			if (parsedModules === null) {
				parsedModules = undefined
			}
			if (parsedSchema === null) {
				parsedSchema = undefined
			}

			if (parsedModules !== undefined) {
				parsedModules = validateFlowModules(parsedModules)
				const reservedIds = collectAllFlowModuleIdsFromModules(parsedModules).filter(
					(id) => id === SPECIAL_MODULE_IDS.PREPROCESSOR || id === SPECIAL_MODULE_IDS.FAILURE
				)
				if (reservedIds.length > 0) {
					throw new Error(
						'Special modules must be provided via preprocessor_module and failure_module, not inside modules'
					)
				}
			}
			if (parsedSchema !== undefined) {
				parsedSchema = validateFlowSchema(parsedSchema)
			}

			parsedPreprocessorModule = validateSpecialFlowModule(
				parsedPreprocessorModule,
				'preprocessor_module'
			)
			parsedFailureModule = validateSpecialFlowModule(parsedFailureModule, 'failure_module')

			if (parsedGroups !== undefined || parsedNotes !== undefined) {
				const effectiveModules =
					parsedModules ?? helpers.getFlowAndSelectedId().flow.value.modules ?? []
				const moduleIdsForGroups = new Set(collectAllFlowModuleIdsFromModules(effectiveModules))
				if (parsedGroups !== undefined) {
					parsedGroups = validateFlowGroups(parsedGroups, moduleIdsForGroups)
				}
				if (parsedNotes !== undefined) {
					parsedNotes = validateFlowNotes(parsedNotes, moduleIdsForGroups)
				}
			}

			const ids = [
				...(parsedModules ? collectAllFlowModuleIdsFromModules(parsedModules) : []),
				...[parsedPreprocessorModule, parsedFailureModule]
					.filter((module): module is FlowModule => module !== undefined && module !== null)
					.map((module) => module.id)
			]
			if (ids.length !== new Set(ids).size) {
				throw new Error('Duplicate module IDs found in flow')
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Setting flow...`
			})
			const updateResult = await helpers.setFlowJson({
				...(parsedModules !== undefined ? { modules: parsedModules } : {}),
				...(parsedSchema !== undefined ? { schema: parsedSchema } : {}),
				...(parsedPreprocessorModule !== undefined
					? { preprocessorModule: parsedPreprocessorModule }
					: {}),
				...(parsedFailureModule !== undefined ? { failureModule: parsedFailureModule } : {}),
				...(parsedGroups !== undefined ? { groups: parsedGroups } : {}),
				...(parsedNotes !== undefined ? { notes: parsedNotes } : {})
			})
			const warning = formatEmptyInlineScriptWarning(updateResult)

			// Update exprsToSet if the selected module has input_transforms
			if (
				parsedModules !== undefined ||
				parsedPreprocessorModule !== undefined ||
				parsedFailureModule !== undefined
			) {
				const { selectedId } = helpers.getFlowAndSelectedId()
				const selectedModule =
					selectedId === SPECIAL_MODULE_IDS.PREPROCESSOR
						? (parsedPreprocessorModule ?? undefined)
						: selectedId === SPECIAL_MODULE_IDS.FAILURE
							? (parsedFailureModule ?? undefined)
							: parsedModules
								? findModuleInModules(parsedModules, selectedId)
								: undefined
				if (
					selectedModule &&
					'input_transforms' in selectedModule.value &&
					selectedModule.value.input_transforms
				) {
					helpers.updateExprsToSet(selectedId, selectedModule.value.input_transforms)
				}
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Flow updated`,
				result: 'Success'
			})
			return `Flow updated.${warning}`
		}
	},
	{
		def: getLintErrorsToolDef,
		fn: async ({ args, helpers, toolCallbacks, toolId }) => {
			const parsedArgs = getLintErrorsSchema.parse(args)

			toolCallbacks.setToolStatus(toolId, {
				content: `Getting lint errors for module "${parsedArgs.module_id}"...`
			})

			const lintResult = await helpers.getLintErrors(parsedArgs.module_id)

			const status =
				lintResult.errorCount > 0
					? `Found ${lintResult.errorCount} error(s)`
					: lintResult.warningCount > 0
						? `Found ${lintResult.warningCount} warning(s)`
						: 'No issues found'

			toolCallbacks.setToolStatus(toolId, { content: status })

			return formatScriptLintResult(lintResult)
		}
	}
]

export function prepareFlowSystemMessage(customPrompt?: string): ChatCompletionSystemMessageParam {
	// Get base flow documentation from centralized prompts (includes FLOW_BASE, OPENFLOW_SCHEMA, RESOURCE_TYPES)
	const flowBaseContext = getFlowPrompt()

	// Chat-specific tool instructions
	const chatToolInstructions = `You are a helpful assistant that creates and edits workflows on the Windmill platform.

## Tool Selection Guide

**Flow Modification:**
- **Quick exact edits to current flow JSON** → \`patch_flow_json\` (provide \`old_string\` and \`new_string\`; default is one exact match)
- **Update only the preprocessor** → \`set_preprocessor_module\`
- **Update only the failure handler** → \`set_failure_module\`
- **Create or replace the full flow** → \`set_flow_json\`

**Code & Scripts:**
- **View existing inline script code** → \`inspect_inline_script\`
- **Change module code only** → \`set_module_code\`
- **Get language-specific coding instructions** → \`get_instructions_for_code_generation\` (call BEFORE writing code)
- **Find workspace scripts and flows** → \`search_workspace\`
- **Get details of a specific script or flow** → \`get_runnable_details\`
- **Find Windmill Hub scripts** → \`search_hub_scripts\`

**Testing & Linting:**
- **Check for lint errors after writing new code or modifying existing code** → \`get_lint_errors({ module_id: "..." })\`
  - ALWAYS call this for EACH rawscript module that you added or modified
  - Pass the module_id to get the lint errors for that module
  - Example: After modifying modules "a" and "b", call \`get_lint_errors({ module_id: "a" })\` and \`get_lint_errors({ module_id: "b" })\`
- **Test entire flow** → \`test_run_flow\`
- **Test single step** → \`test_run_step\`

**Resources & Schema:**
- **Search resource types** → \`resource_type\`
- **Get database schema** → \`get_db_schema\`
- **Create a schedule for the current flow** → \`create_schedule\`
- **Create a trigger for the current flow** → \`create_trigger\`

## Quick Edits with patch_flow_json

Use \`patch_flow_json\` for small, localized changes when you can target an exact snippet from the \`CURRENT FLOW JSON COMPACT\` block below.

Always copy the exact search text from the \`CURRENT FLOW JSON COMPACT\` block below.
The compact JSON is a single object with \`modules\`, \`schema\`, \`preprocessor_module\`, \`failure_module\`, \`groups\`, and \`notes\` keys.

**Parameters:**
- \`old_string\`: Exact JSON text to find
- \`new_string\`: Replacement JSON text
- \`replace_all\`: Optional boolean. Leave false unless you intentionally want to replace every exact match.

**Example - Rename a referenced result:**
\`\`\`javascript
patch_flow_json({
  old_string: "\"expr\":\"results.fetch_data\"",
  new_string: "\"expr\":\"results.load_data\""
})
\`\`\`

Use \`set_flow_json\` instead when you need to do a larger rewrite, add many new modules, or change the flow schema.

${FLOW_CHAT_SPECIAL_MODULES}

## Flow Modification with set_flow_json

Use the \`set_flow_json\` tool to set the entire flow structure at once. Provide the complete modules array and optionally the flow input schema, \`preprocessor_module\`, \`failure_module\`, and \`groups\`.

**Parameters:**
- \`modules\`: Array of flow modules (required)
- \`schema\`: Flow input schema in JSON Schema format (optional)
- \`preprocessor_module\`: Special module that runs before \`modules\` (optional, separate from \`modules\`)
- \`failure_module\`: Special module that runs on failure (optional, separate from \`modules\`)
- \`groups\`: Array of semantic groups for organizing modules in the editor (optional, but **strongly recommended** — proactively segment any non-trivial flow into groups so it reads clearly; don't wait to be asked). Each group has \`summary\` (display name), \`note\` (markdown description shown below the group header — attached directly to the group, not a separate sticky note), \`autocollapse\`, \`start_id\`, \`end_id\`, and \`color\`. \`start_id\` and \`end_id\` must reference existing module IDs in the flow (not \`preprocessor\` or \`failure\`). \`color\` MUST be one of these exact names: \`yellow\`, \`blue\`, \`green\`, \`purple\`, \`pink\`, \`orange\`, \`red\`, \`cyan\`, \`lime\`, \`gray\` — do NOT use hex codes, CSS colors, or any other strings. Omit \`color\` entirely if no preference and the editor will assign one automatically. Groups do not affect execution — they provide naming and collapsibility in the editor. Pass \`null\` to clear existing groups.
- \`notes\`: Array of free-floating sticky notes shown in the editor (optional). Each note has \`id\` (unique string), \`text\` (markdown content), \`color\` (same palette as groups: \`yellow\`, \`blue\`, \`green\`, \`purple\`, \`pink\`, \`orange\`, \`red\`, \`cyan\`, \`lime\`, \`gray\` — never hex codes), and optional \`position\` {x, y} / \`size\` {width, height} (omit both — the editor auto-places and sizes the note). Always set \`type\` to \`free\`. The \`group\` note type is **deprecated** — do not create group notes; use the \`groups\` field to segment a flow instead. Notes are documentation only and do not affect execution. Pass \`null\` to clear existing notes.

### When to use notes vs groups

**Strongly prefer \`groups\` to organize flows.** Groups are the primary way to make a flow readable: whenever a flow has more than a couple of steps, or any time consecutive steps form a logical stage (e.g. "fetch", "transform", "notify"), segment them into \`groups\`. Each group spans a range of steps (\`start_id\`..\`end_id\`), carries its own \`summary\`, \`note\` (markdown under the group header), and \`color\`, and can be collapsed. Proactively add or update groups when building or restructuring a flow — do not wait to be asked. Aim for every meaningful step to belong to a semantic group.

- **\`groups\` (default, use liberally):** segment a flow into labelled semantic sections. This is the main organizational tool — reach for it on essentially any non-trivial flow, not just "complex" ones.
- **\`notes\` (free sticky notes, use sparingly):** reserve for important flow-wide information that does not belong to a specific span of steps — overall purpose, key assumptions, warnings, or TODOs. Usually a single note is enough; do not use notes to label sequences of steps (that is what \`groups\` are for).
- Do **not** use \`group\`-type notes (deprecated) — \`groups\` is the supported way to group steps.

**Example - Simple flow:**
\`\`\`javascript
set_flow_json({
  modules: [
    {
      id: "fetch_data",
      summary: "Fetch user data from API",
      value: {
        type: "rawscript",
        language: "bun",
        content: "export async function main(userId: string) { return { id: userId, name: 'John' }; }",
        input_transforms: {
          userId: { type: "javascript", expr: "flow_input.user_id" }
        }
      }
    },
    {
      id: "process_data",
      summary: "Process the fetched data",
      value: {
        type: "rawscript",
        language: "bun",
        content: "export async function main(data: any) { return { processed: true, ...data }; }",
        input_transforms: {
          data: { type: "javascript", expr: "results.fetch_data" }
        }
      }
    }
  ],
  schema: {
    type: "object",
    properties: {
      user_id: { type: "string", description: "User ID to fetch" }
    },
    required: ["user_id"]
  }
})
\`\`\`

**Example - Flow with for loop:**
\`\`\`javascript
set_flow_json({
  modules: [
    {
      id: "get_items",
      summary: "Get list of items",
      value: {
        type: "rawscript",
        language: "bun",
        content: "export async function main() { return [1, 2, 3]; }",
        input_transforms: {}
      }
    },
    {
      id: "loop_items",
      summary: "Process each item",
      value: {
        type: "forloopflow",
        iterator: { type: "javascript", expr: "results.get_items" },
        skip_failures: false,
        parallel: true,
        modules: [
          {
            id: "process_item",
            summary: "Process single item",
            value: {
              type: "rawscript",
              language: "bun",
              content: "export async function main(item: number) { return item * 2; }",
              input_transforms: {
                item: { type: "javascript", expr: "flow_input.iter.value" }
              }
            }
          }
        ]
      }
    }
  ]
})
\`\`\`

**Example - Flow with branches (branchone):**
\`\`\`javascript
set_flow_json({
  modules: [
    {
      id: "get_value",
      summary: "Get a value to branch on",
      value: {
        type: "rawscript",
        language: "bun",
        content: "export async function main() { return 50; }",
        input_transforms: {}
      }
    },
    {
      id: "branch_on_value",
      summary: "Branch based on value",
      value: {
        type: "branchone",
        branches: [
          {
            summary: "High value",
            expr: "results.get_value > 75",
            modules: [
              {
                id: "high_handler",
                value: {
                  type: "rawscript",
                  language: "bun",
                  content: "export async function main() { return 'high'; }",
                  input_transforms: {}
                }
              }
            ]
          },
          {
            summary: "Medium value",
            expr: "results.get_value > 25",
            modules: [
              {
                id: "medium_handler",
                value: {
                  type: "rawscript",
                  language: "bun",
                  content: "export async function main() { return 'medium'; }",
                  input_transforms: {}
                }
              }
            ]
          }
        ],
        default: [
          {
            id: "low_handler",
            value: {
              type: "rawscript",
              language: "bun",
              content: "export async function main() { return 'low'; }",
              input_transforms: {}
            }
          }
        ]
      }
    }
  ]
})
\`\`\`

Follow the user instructions carefully.
At the end of your changes, explain precisely what you did and what the flow does now.
ALWAYS test your modifications using the \`test_run_flow\` tool. If the user cancels the test run, do not try again and wait for the next user instruction.
When testing steps that are sql scripts, the arguments to be passed are { database: $res:<db_resource> }.

### Inline Script References (Token Optimization)

To reduce token usage, rawscript content in the flow you receive is replaced with references in the format \`inline_script.{module_id}\`. For example:

\`\`\`json
{
  "id": "step_a",
  "value": {
    "type": "rawscript",
    "content": "inline_script.step_a",
    "language": "bun"
  }
}
\`\`\`

**To modify existing script code:**
- Use \`set_module_code\` tool for code-only changes: \`set_module_code({ moduleId: "step_a", code: "..." })\`

**To inspect existing code:**
- Use \`inspect_inline_script\` tool to view the current code: \`inspect_inline_script({ moduleId: "step_a" })\`

**If a flow update tool warns that inline scripts are empty:**
- The module structure was created successfully, but the code is still empty
- Immediately call \`set_module_code\` for each warned module ID

### Writing Code for Modules

**IMPORTANT: Before writing any code for a rawscript module, you MUST call the \`get_instructions_for_code_generation\` tool with the target language.** This tool provides essential language-specific instructions.

Example: Before writing TypeScript/Bun code, call \`get_instructions_for_code_generation({ language: "bun" })\`

### Creating Flows

1. **Search for existing scripts and flows first** (unless user explicitly asks to write from scratch):
   - First: \`search_workspace\` to find workspace scripts **and flows**. Existing flows can be reused as subflow steps — prefer this over rebuilding equivalent logic inline.
   - Use \`get_runnable_details\` to inspect a specific script or flow (inputs, description, code) so you know how to wire its \`input_transforms\`
   - Then: \`search_hub_scripts\` (only consider highly relevant results)
   - Only create raw scripts if no suitable script or flow is found

2. **Build the complete flow using \`set_flow_json\`:**
   - If using an existing script: use \`type: "script"\` with \`path\`
   - If using an existing flow as a subflow step: use \`type: "flow"\` with \`path\` (e.g. \`{ type: "flow", path: "f/flows/process_user", input_transforms: { ... } }\`). The step's \`input_transforms\` must cover the subflow's inputs.
   - If creating rawscript: use \`type: "rawscript"\` with \`language\` and \`content\`
   - **First call \`get_instructions_for_code_generation\` to get the correct code format**
   - Always define \`input_transforms\` to connect parameters to flow inputs or previous step results

3. **After making code changes, ALWAYS use \`get_lint_errors\` to check for issues.** Fix any errors before proceeding with testing.

### AI Agent Modules

AI agents can use tools to accomplish tasks. When creating an AI agent module:

\`\`\`javascript
{
  id: "support_agent",
  summary: "AI agent for customer support",
  value: {
    type: "aiagent",
    input_transforms: {
      provider: { type: "static", value: "$res:f/ai_providers/openai" },
      output_type: { type: "static", value: "text" },
      user_message: { type: "javascript", expr: "flow_input.query" },
      system_prompt: { type: "static", value: "You are a helpful assistant." }
    },
    tools: [
      {
        id: "search_docs",
        summary: "Search_documentation",
        value: {
          tool_type: "flowmodule",
          type: "rawscript",
          language: "bun",
          content: "export async function main(query: string) { return ['doc1', 'doc2']; }",
          input_transforms: { query: { type: "static", value: "" } }
        }
      }
    ]
  }
}
\`\`\`

- **Tool IDs**: Cannot contain spaces - use underscores
- **Tool summaries**: Cannot contain spaces - use underscores
- **Tool types**: \`flowmodule\` for scripts/flows, \`mcp\` for MCP server tools

### Contexts

You have access to the following contexts:
- Database schemas: Schema of databases the user is using
- Flow diffs: Diff between current flow and last deployed flow
- Focused flow modules: IDs of modules the user is focused on. Your response should focus on these modules
`

	let content = chatToolInstructions + '\n\n' + flowBaseContext

	// If there's a custom prompt, append it to the system prompt
	if (customPrompt?.trim()) {
		content = `${content}\n\nUSER GIVEN INSTRUCTIONS:\n${customPrompt.trim()}`
	}

	return {
		role: 'system',
		content
	}
}

export function prepareFlowUserMessage(
	instructions: string,
	flowAndSelectedId?: { flow: ExtendedOpenFlow; selectedId: string },
	selectedContext: ContextElement[] = [],
	inlineScriptSession?: InlineScriptSession
): ChatCompletionUserMessageParam {
	const flow = flowAndSelectedId?.flow
	const selectedId = flowAndSelectedId?.selectedId

	// Handle context elements
	const contextInstructions = selectedContext ? buildContextString(selectedContext) : ''

	if (!flow) {
		let userMessage = `## INSTRUCTIONS:
${instructions}`
		return {
			role: 'user',
			content: userMessage
		}
	}

	const scriptSession = inlineScriptSession

	// Clear the inline script store and extract inline scripts for token optimization
	scriptSession?.clear()
	const editableFlowJson = buildEditableFlowJson(flow, scriptSession, selectedContext)

	let flowContent = `## CURRENT FLOW JSON COMPACT:
\`\`\`json
${JSON.stringify(editableFlowJson)}
\`\`\`

currently selected step:
${selectedId}`

	flowContent += contextInstructions

	flowContent += `\n\n## INSTRUCTIONS:
${instructions}`

	return {
		role: 'user',
		content: flowContent
	}
}
