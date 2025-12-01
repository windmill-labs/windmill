import { ScriptService, type FlowModule, type RawScript, type Script, JobService } from '$lib/gen'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import type { ChatCompletionTool as ChatCompletionFunctionTool } from 'openai/resources/chat/completions.mjs'
import { z } from 'zod'
import uFuzzy from '@leeoniya/ufuzzy'
import { emptyString } from '$lib/utils'
import {
	createDbSchemaTool,
	getFormattedResourceTypes,
	getLangContext,
	SUPPORTED_CHAT_SCRIPT_LANGUAGES
} from '../script/core'
import {
	createSearchHubScriptsTool,
	createToolDef,
	type Tool,
	executeTestRun,
	buildSchemaForTool,
	buildTestRunArgs,
	buildContextString,
	applyCodePiecesToFlowModules,
	findModuleById,
	SPECIAL_MODULE_IDS
} from '../shared'
import type { ContextElement } from '../context'
import type { ExtendedOpenFlow } from '$lib/components/flows/types'
import openFlowSchema from './openFlow.json'

/**
 * Action types for flow module changes during diff tracking
 * - added: Module was added to the flow
 * - modified: Module content was changed
 * - removed: Module was deleted from the flow
 * - shadowed: Module is shown as removed (visualization mode)
 */
export type AIModuleAction = 'added' | 'modified' | 'removed' | 'shadowed' | undefined

/**
 * Tracks the action performed on a module and whether it requires user approval
 */
export type ModuleActionInfo = {
	action: AIModuleAction
	/** Whether this change is pending user approval (accept/reject) */
	pending: boolean
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
	getModules: (id?: string) => FlowModule[]

	// snapshot management (AI sets this when making changes)
	/** Set the before flow snapshot */
	setSnapshot: (snapshot: ExtendedOpenFlow) => void
	/** Revert the entire flow to a previous snapshot */
	revertToSnapshot: (snapshot?: ExtendedOpenFlow) => void

	// ai chat tools
	setCode: (id: string, code: string) => Promise<void>
	setFlowJson: (json: string) => Promise<void>
	getFlowInputsSchema: () => Promise<Record<string, any>>

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
}

const searchScriptsSchema = z.object({
	query: z
		.string()
		.describe('The query to search for, e.g. send email, list stripe invoices, etc..')
})

const searchScriptsToolDef = createToolDef(
	searchScriptsSchema,
	'search_scripts',
	'Search for scripts in the workspace. Returns array of {path, summary} objects.'
)

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

/**
 * Recursively resolves all $ref references in a JSON Schema by inlining them.
 * This ensures the schema is fully self-contained for AI providers that don't
 * support external references or have strict schema validation (e.g., Google/Gemini).
 *
 * @param schema - The schema object to resolve
 * @param rootSchema - The root schema document containing all definitions
 * @param visited - Set of visited $ref paths to prevent infinite recursion
 * @returns Fully resolved schema with all $ref references inlined
 */
function resolveSchemaRefs(schema: any, rootSchema: any, visited = new Set<string>()): any {
	if (!schema || typeof schema !== 'object') return schema

	// Handle $ref
	if (schema.$ref) {
		const refPath = schema.$ref.replace('#/', '').split('/')

		// Prevent infinite recursion with circular refs
		if (visited.has(schema.$ref)) {
			return { type: 'object' } // Fallback for circular refs
		}
		visited.add(schema.$ref)

		let resolved = rootSchema
		for (const part of refPath) {
			resolved = resolved[part]
		}

		// Recursively resolve the referenced schema
		return resolveSchemaRefs(resolved, rootSchema, new Set(visited))
	}

	// Handle arrays
	if (Array.isArray(schema)) {
		return schema.map((item) => resolveSchemaRefs(item, rootSchema, visited))
	}

	// Handle objects - recursively process all properties
	const result: any = {}
	for (const key in schema) {
		result[key] = resolveSchemaRefs(schema[key], rootSchema, visited)
	}
	return result
}

const addModuleToolDef: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		strict: false,
		name: 'add_module',
		description:
			"Add a new module to the flow. Use afterId to insert after a specific module (null to insert at the beginning), or insideId+branchPath to insert into branches/loops. Note: The IDs 'failure', 'preprocessor', and 'Input' are reserved and cannot be used.",
		parameters: {
			type: 'object',
			properties: {
				afterId: {
					type: ['string', 'null'],
					description:
						'ID of the module to insert after. Use null to insert at the beginning. Can be used with insideId+branchPath to specify position within a container.'
				},
				insideId: {
					type: ['string', 'null'],
					description:
						'ID of the container module (branch/loop/branchall/branchone) to insert into. Use with branchPath to add a module inside a container, or with branchPath=null to add a new branch to branchall/branchone.'
				},
				branchPath: {
					type: ['string', 'null'],
					description:
						"Path to insert a module inside a container: 'modules' (for loops), 'branches.0'/'branches.1'/etc (to add inside a specific branch), 'default' (for branchone default branch), or 'tools' (for aiagent). Use null with insideId pointing to a branchall/branchone to add a NEW branch (value should be a branch object with summary, modules, etc.)."
				},
				value: {
					...resolveSchemaRefs(openFlowSchema.components.schemas.FlowModule, openFlowSchema),
					description: 'Complete module object including id, summary, and value fields'
				}
			},
			required: ['value']
		}
	}
}

const removeModuleSchema = z.object({
	id: z.string().describe('ID of the module to remove')
})

const removeModuleToolDef = createToolDef(
	removeModuleSchema,
	'remove_module',
	"Remove a module from the flow by its ID. Searches recursively through all nested structures. Note: The IDs 'failure', 'preprocessor', and 'Input' are reserved and cannot be removed."
)

const removeBranchSchema = z.object({
	insideId: z.string().describe('ID of the branchall/branchone container'),
	branchIndex: z.number().int().min(0).describe('Index of the branch to remove (0-based)')
})

const removeBranchToolDef = createToolDef(
	removeBranchSchema,
	'remove_branch',
	'Remove a branch from a branchall/branchone by its index. Use this to delete an entire branch including all modules inside it.'
)

const modifyModuleToolDef: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		strict: false,
		name: 'modify_module',
		description:
			"Modify an existing module (full replacement). Use for changing configuration, transforms, or conditions. Not for adding/removing nested modules. Note: The IDs 'failure', 'preprocessor', and 'Input' are reserved and cannot be modified.",
		parameters: {
			type: 'object',
			properties: {
				id: {
					type: 'string',
					description: 'ID of the module to modify'
				},
				value: {
					...resolveSchemaRefs(openFlowSchema.components.schemas.FlowModule, openFlowSchema),
					description:
						'Complete new module object (full replacement). Use this to change module configuration, input_transforms, branch conditions, etc. Do NOT use this to add/remove modules inside branches/loops - use add_module/remove_module for that.'
				}
			},
			required: ['id', 'value']
		}
	}
}

const setFlowSchemaToolDef: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		strict: false,
		name: 'set_flow_schema',
		description:
			'Set or update the flow input schema. Defines what parameters the flow accepts when executed.',
		parameters: {
			type: 'object',
			properties: {
				schema: {
					type: 'object',
					description: 'Flow input schema defining the parameters the flow accepts'
				}
			},
			required: ['schema']
		}
	}
}

/** Restricted module IDs that cannot be used in add/modify/remove operations */
const RESTRICTED_MODULE_IDS = Object.values(SPECIAL_MODULE_IDS)

function isRestrictedModuleId(id: string): boolean {
	return RESTRICTED_MODULE_IDS.includes(id as (typeof RESTRICTED_MODULE_IDS)[number])
}

class WorkspaceScriptsSearch {
	private uf: uFuzzy
	private workspace: string | undefined = undefined
	private scripts: Script[] | undefined = undefined

	constructor() {
		this.uf = new uFuzzy()
	}

	private async init(workspace: string) {
		this.scripts = await ScriptService.listScripts({
			workspace
		})
		this.workspace = workspace
	}

	async search(query: string, workspace: string) {
		if (this.scripts === undefined || this.workspace !== workspace) {
			await this.init(workspace)
		}

		const scripts = this.scripts

		if (!scripts) {
			throw new Error('Failed to load scripts')
		}

		const results = this.uf.search(
			scripts.map((s) => (emptyString(s.summary) ? s.path : s.summary + ' (' + s.path + ')')),
			query.trim()
		)
		const scriptResults =
			results[2]?.map((id) => ({
				path: scripts[id].path,
				summary: scripts[id].summary
			})) ?? []

		return scriptResults
	}
}

// Will be overridden by setSchema
const testRunFlowSchema = z.object({
	args: z
		.object({})
		.nullable()
		.optional()
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
	'Execute a test run of a specific step in the flow'
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

const workspaceScriptsSearch = new WorkspaceScriptsSearch()

/**
 * Recursively finds a module by ID in the flow structure
 */
function findModuleInFlow(modules: FlowModule[], id: string): FlowModule | undefined {
	for (const module of modules) {
		if (module.id === id) {
			return module
		}

		// Search in nested structures
		if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
			if (module.value.modules) {
				const found = findModuleInFlow(module.value.modules, id)
				if (found) return found
			}
		} else if (module.value.type === 'branchone') {
			if (module.value.branches) {
				for (const branch of module.value.branches) {
					if (branch.modules) {
						const found = findModuleInFlow(branch.modules, id)
						if (found) return found
					}
				}
			}
			if (module.value.default) {
				const found = findModuleInFlow(module.value.default, id)
				if (found) return found
			}
		} else if (module.value.type === 'branchall') {
			if (module.value.branches) {
				for (const branch of module.value.branches) {
					if (branch.modules) {
						const found = findModuleInFlow(branch.modules, id)
						if (found) return found
					}
				}
			}
		} else if (module.value.type === 'aiagent') {
			// Search in AI agent tools
			if (module.value.tools) {
				for (const tool of module.value.tools) {
					if (tool.id === id) {
						// Return a pseudo-FlowModule for compatibility
						return { id: tool.id, value: tool.value, summary: tool.summary } as FlowModule
					}
				}
			}
		}
	}
	return undefined
}

/**
 * Recursively removes a module by ID from the flow structure
 * Returns the updated modules array
 */
function removeModuleFromFlow(modules: FlowModule[], id: string): FlowModule[] {
	const result: FlowModule[] = []

	for (const module of modules) {
		if (module.id === id) {
			// Skip this module (remove it)
			continue
		}

		const newModule = { ...module }

		// Recursively remove from nested structures
		if (newModule.value.type === 'forloopflow' || newModule.value.type === 'whileloopflow') {
			if (newModule.value.modules) {
				newModule.value = {
					...newModule.value,
					modules: removeModuleFromFlow(newModule.value.modules, id)
				}
			}
		} else if (newModule.value.type === 'branchone') {
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? removeModuleFromFlow(branch.modules, id) : []
					}))
				}
			}
			if (newModule.value.default) {
				newModule.value = {
					...newModule.value,
					default: removeModuleFromFlow(newModule.value.default, id)
				}
			}
		} else if (newModule.value.type === 'branchall') {
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? removeModuleFromFlow(branch.modules, id) : []
					}))
				}
			}
		} else if (newModule.value.type === 'aiagent') {
			// Remove tool from AI agent's tools array
			if (newModule.value.tools) {
				newModule.value = {
					...newModule.value,
					tools: newModule.value.tools.filter((tool) => tool.id !== id)
				}
			}
		}

		result.push(newModule)
	}

	return result
}

/**
 * Recursively removes a branch by index from a branchall/branchone container
 * Returns the updated modules array
 */
function removeBranchFromFlow(
	modules: FlowModule[],
	containerId: string,
	branchIndex: number
): FlowModule[] {
	return modules.map((module) => {
		if (module.id === containerId) {
			if (module.value.type === 'branchall') {
				const branches = module.value.branches || []
				if (branchIndex < 0 || branchIndex >= branches.length) {
					throw new Error(
						`Branch index ${branchIndex} out of bounds (0-${branches.length - 1})`
					)
				}
				return {
					...module,
					value: {
						...module.value,
						branches: branches.filter((_, i) => i !== branchIndex)
					}
				} as FlowModule
			}
			if (module.value.type === 'branchone') {
				const branches = module.value.branches || []
				if (branchIndex < 0 || branchIndex >= branches.length) {
					throw new Error(
						`Branch index ${branchIndex} out of bounds (0-${branches.length - 1})`
					)
				}
				return {
					...module,
					value: {
						...module.value,
						branches: branches.filter((_, i) => i !== branchIndex)
					}
				} as FlowModule
			}
			throw new Error(`Module '${containerId}' is not a branchall/branchone container`)
		}

		// Recursively search nested structures
		const newModule = { ...module }
		if (newModule.value.type === 'forloopflow' || newModule.value.type === 'whileloopflow') {
			if (newModule.value.modules) {
				newModule.value = {
					...newModule.value,
					modules: removeBranchFromFlow(newModule.value.modules, containerId, branchIndex)
				}
			}
		} else if (newModule.value.type === 'branchone') {
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules
							? removeBranchFromFlow(branch.modules, containerId, branchIndex)
							: []
					}))
				}
			}
			if (newModule.value.default) {
				newModule.value = {
					...newModule.value,
					default: removeBranchFromFlow(newModule.value.default, containerId, branchIndex)
				}
			}
		} else if (newModule.value.type === 'branchall') {
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules
							? removeBranchFromFlow(branch.modules, containerId, branchIndex)
							: []
					}))
				}
			}
		}

		return newModule
	})
}

/**
 * Parses a branch path string into navigation components
 * Examples: 'branches.0' -> {type: 'branches', index: 0}
 *           'default' -> {type: 'default'}
 *           'modules' -> {type: 'modules'}
 */
function parseBranchPath(path: string): { type: string; index?: number } {
	if (path === 'default') {
		return { type: 'default' }
	}
	if (path === 'modules') {
		return { type: 'modules' }
	}
	if (path === 'tools') {
		return { type: 'tools' }
	}

	const match = path.match(/^(branches)\.(\d+)$/)
	if (match) {
		return { type: match[1], index: parseInt(match[2], 10) }
	}

	throw new Error(`Invalid branch path: ${path}`)
}

/**
 * Gets the target array for module insertion based on insideId and branchPath
 */
function getTargetArray(
	modules: FlowModule[],
	insideId: string,
	branchPath: string
): FlowModule[] | undefined {
	const container = findModuleInFlow(modules, insideId)
	if (!container) {
		return undefined
	}

	const parsed = parseBranchPath(branchPath)

	if (container.value.type === 'forloopflow' || container.value.type === 'whileloopflow') {
		if (parsed.type === 'modules') {
			return container.value.modules || []
		}
		throw new Error(`Invalid branchPath '${branchPath}' for loop module. Use 'modules'`)
	} else if (container.value.type === 'branchone') {
		if (parsed.type === 'branches' && parsed.index !== undefined) {
			return container.value.branches?.[parsed.index]?.modules
		} else if (parsed.type === 'default') {
			return container.value.default
		}
		throw new Error(
			`Invalid branchPath '${branchPath}' for branchone module. Use 'branches.N' or 'default'`
		)
	} else if (container.value.type === 'branchall') {
		if (parsed.type === 'branches' && parsed.index !== undefined) {
			return container.value.branches?.[parsed.index]?.modules
		}
		throw new Error(`Invalid branchPath '${branchPath}' for branchall module. Use 'branches.N'`)
	} else if (container.value.type === 'aiagent') {
		if (parsed.type === 'tools') {
			// Return tools array (AgentTool[]), caller handles the different structure
			return (container.value.tools as any) || []
		}
		throw new Error(`Invalid branchPath '${branchPath}' for aiagent module. Use 'tools'`)
	}

	throw new Error(`Module '${insideId}' is not a container type`)
}

/**
 * Updates a nested array within a container module
 */
function updateNestedArray(
	module: FlowModule,
	branchPath: string,
	updatedArray: FlowModule[]
): FlowModule {
	const parsed = parseBranchPath(branchPath)
	const newModule = { ...module }

	if (newModule.value.type === 'forloopflow' || newModule.value.type === 'whileloopflow') {
		if (parsed.type === 'modules') {
			newModule.value = {
				...newModule.value,
				modules: updatedArray
			}
		}
	} else if (newModule.value.type === 'branchone') {
		if (parsed.type === 'branches' && parsed.index !== undefined && newModule.value.branches) {
			const newBranches = [...newModule.value.branches]
			newBranches[parsed.index] = {
				...newBranches[parsed.index],
				modules: updatedArray
			}
			newModule.value = {
				...newModule.value,
				branches: newBranches
			}
		} else if (parsed.type === 'default') {
			newModule.value = {
				...newModule.value,
				default: updatedArray
			}
		}
	} else if (newModule.value.type === 'branchall') {
		if (parsed.type === 'branches' && parsed.index !== undefined && newModule.value.branches) {
			const newBranches = [...newModule.value.branches]
			newBranches[parsed.index] = {
				...newBranches[parsed.index],
				modules: updatedArray
			}
			newModule.value = {
				...newModule.value,
				branches: newBranches
			}
		}
	} else if (newModule.value.type === 'aiagent') {
		if (parsed.type === 'tools') {
			// Note: updatedArray is actually AgentTool[] when dealing with AI agents
			newModule.value = {
				...newModule.value,
				tools: updatedArray as any
			}
		}
	}

	return newModule
}

/**
 * Recursively adds a module to the flow structure
 */
function addModuleToFlow(
	modules: FlowModule[],
	afterId: string | null,
	insideId: string | null,
	branchPath: string | null,
	newModule: FlowModule
): FlowModule[] {
	// Case 1a: Adding a NEW branch to branchall/branchone (insideId set, branchPath null)
	if (insideId && branchPath === null) {
		return modules.map((module) => {
			if (module.id === insideId) {
				// Adding a new branch to branchall
				if (module.value.type === 'branchall') {
					const newBranch = {
						summary: (newModule as any).summary || '',
						skip_failure: (newModule as any).skip_failure ?? false,
						modules: (newModule as any).modules || []
					}
					return {
						...module,
						value: {
							...module.value,
							branches: [...(module.value.branches || []), newBranch]
						}
					} as FlowModule
				}
				// Adding a new branch to branchone
				if (module.value.type === 'branchone') {
					const newBranch = {
						summary: (newModule as any).summary || '',
						expr: (newModule as any).expr || 'false',
						modules: (newModule as any).modules || []
					}
					return {
						...module,
						value: {
							...module.value,
							branches: [...(module.value.branches || []), newBranch]
						}
					} as FlowModule
				}
				throw new Error(
					`Cannot add branch to module '${insideId}': branchPath=null is only valid for branchall/branchone containers`
				)
			}

			// Recursively search nested structures for the target container
			const newModuleCopy = { ...module }
			if (
				newModuleCopy.value.type === 'forloopflow' ||
				newModuleCopy.value.type === 'whileloopflow'
			) {
				if (newModuleCopy.value.modules) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						modules: addModuleToFlow(
							newModuleCopy.value.modules,
							afterId,
							insideId,
							branchPath,
							newModule
						)
					}
				}
			} else if (newModuleCopy.value.type === 'branchone') {
				if (newModuleCopy.value.branches) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						branches: newModuleCopy.value.branches.map((branch) => ({
							...branch,
							modules: branch.modules
								? addModuleToFlow(branch.modules, afterId, insideId, branchPath, newModule)
								: []
						}))
					}
				}
				if (newModuleCopy.value.default) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						default: addModuleToFlow(
							newModuleCopy.value.default,
							afterId,
							insideId,
							branchPath,
							newModule
						)
					}
				}
			} else if (newModuleCopy.value.type === 'branchall') {
				if (newModuleCopy.value.branches) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						branches: newModuleCopy.value.branches.map((branch) => ({
							...branch,
							modules: branch.modules
								? addModuleToFlow(branch.modules, afterId, insideId, branchPath, newModule)
								: []
						}))
					}
				}
			}
			return newModuleCopy
		})
	}

	// Case 1b: Adding inside a container (insideId + branchPath both set)
	if (insideId && branchPath) {
		return modules.map((module) => {
			if (module.id === insideId) {
				// Special handling for AI agent tools
				if (module.value.type === 'aiagent' && branchPath === 'tools') {
					// For AI agents, newModule structure is { id, summary, value: { tool_type, ...FlowModuleValue } }
					// The value should already include tool_type from the caller
					const newTool = {
						id: newModule.id,
						summary: newModule.summary,
						value: newModule.value as any
					}
					return {
						...module,
						value: {
							...module.value,
							tools: [...(module.value.tools || []), newTool]
						}
					} as FlowModule
				}

				const targetArray = getTargetArray(modules, insideId, branchPath)
				if (!targetArray) {
					throw new Error(
						`Cannot find target array for insideId '${insideId}' with branchPath '${branchPath}'`
					)
				}
				const updatedArray =
					afterId !== null
						? addModuleToFlow(targetArray, afterId, null, null, newModule)
						: [newModule, ...targetArray] // afterId null = insert at beginning
				return updateNestedArray(module, branchPath, updatedArray)
			}

			// Recursively search nested structures
			const newModuleCopy = { ...module }
			if (
				newModuleCopy.value.type === 'forloopflow' ||
				newModuleCopy.value.type === 'whileloopflow'
			) {
				if (newModuleCopy.value.modules) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						modules: addModuleToFlow(
							newModuleCopy.value.modules,
							afterId,
							insideId,
							branchPath,
							newModule
						)
					}
				}
			} else if (newModuleCopy.value.type === 'branchone') {
				if (newModuleCopy.value.branches) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						branches: newModuleCopy.value.branches.map((branch) => ({
							...branch,
							modules: branch.modules
								? addModuleToFlow(branch.modules, afterId, insideId, branchPath, newModule)
								: []
						}))
					}
				}
				if (newModuleCopy.value.default) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						default: addModuleToFlow(
							newModuleCopy.value.default,
							afterId,
							insideId,
							branchPath,
							newModule
						)
					}
				}
			} else if (newModuleCopy.value.type === 'branchall') {
				if (newModuleCopy.value.branches) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						branches: newModuleCopy.value.branches.map((branch) => ({
							...branch,
							modules: branch.modules
								? addModuleToFlow(branch.modules, afterId, insideId, branchPath, newModule)
								: []
						}))
					}
				}
			}

			return newModuleCopy
		})
	}

	// Case 2: Adding at current level after a specific module
	if (afterId !== null) {
		const result: FlowModule[] = []
		for (const module of modules) {
			result.push(module)
			if (module.id === afterId) {
				result.push(newModule)
			}
		}
		return result
	}

	// Case 3: afterId is null - insert at the beginning
	return [newModule, ...modules]
}

/**
 * Recursively replaces a module by ID
 */
function replaceModuleInFlow(
	modules: FlowModule[],
	id: string,
	newModule: FlowModule
): FlowModule[] {
	return modules.map((module) => {
		if (module.id === id) {
			return { ...newModule, id } // Ensure ID remains the same
		}

		const newModuleCopy = { ...module }

		// Recursively replace in nested structures
		if (
			newModuleCopy.value.type === 'forloopflow' ||
			newModuleCopy.value.type === 'whileloopflow'
		) {
			if (newModuleCopy.value.modules) {
				newModuleCopy.value = {
					...newModuleCopy.value,
					modules: replaceModuleInFlow(newModuleCopy.value.modules, id, newModule)
				}
			}
		} else if (newModuleCopy.value.type === 'branchone') {
			if (newModuleCopy.value.branches) {
				newModuleCopy.value = {
					...newModuleCopy.value,
					branches: newModuleCopy.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? replaceModuleInFlow(branch.modules, id, newModule) : []
					}))
				}
			}
			if (newModuleCopy.value.default) {
				newModuleCopy.value = {
					...newModuleCopy.value,
					default: replaceModuleInFlow(newModuleCopy.value.default, id, newModule)
				}
			}
		} else if (newModuleCopy.value.type === 'branchall') {
			if (newModuleCopy.value.branches) {
				newModuleCopy.value = {
					...newModuleCopy.value,
					branches: newModuleCopy.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? replaceModuleInFlow(branch.modules, id, newModule) : []
					}))
				}
			}
		} else if (newModuleCopy.value.type === 'aiagent') {
			// Replace tool in AI agent's tools array
			if (newModuleCopy.value.tools) {
				newModuleCopy.value = {
					...newModuleCopy.value,
					tools: newModuleCopy.value.tools.map((tool) =>
						tool.id === id
							? { id, summary: newModule.summary, value: newModule.value as any }
							: tool
					)
				}
			}
		}

		return newModuleCopy
	})
}

/**
 * Storage for inline scripts extracted from flow modules.
 * Maps module IDs to their rawscript content for token-efficient transmission to AI.
 */
class InlineScriptStore {
	private scripts: Map<string, string> = new Map()

	clear() {
		this.scripts.clear()
	}

	set(moduleId: string, content: string) {
		this.scripts.set(moduleId, content)
	}

	get(moduleId: string): string | undefined {
		return this.scripts.get(moduleId)
	}

	has(moduleId: string): boolean {
		return this.scripts.has(moduleId)
	}

	getAll(): Record<string, string> {
		return Object.fromEntries(this.scripts.entries())
	}
}

const inlineScriptStore = new InlineScriptStore()

/**
 * Recursively extracts all rawscript content from flow modules and stores them.
 * Replaces the content with references like "inline_script.{module_id}".
 */
function extractAndReplaceInlineScripts(modules: FlowModule[]): FlowModule[] {
	if (!modules || !Array.isArray(modules)) {
		return []
	}

	return modules.map((module) => {
		const newModule = { ...module }

		if (newModule.value.type === 'rawscript' && newModule.value.content) {
			// Store the original content
			inlineScriptStore.set(module.id, newModule.value.content)

			// Replace with reference
			newModule.value = {
				...newModule.value,
				content: `inline_script.${module.id}`
			}
		} else if (newModule.value.type === 'forloopflow' || newModule.value.type === 'whileloopflow') {
			// Recursively process nested modules in loops
			if (newModule.value.modules) {
				newModule.value = {
					...newModule.value,
					modules: extractAndReplaceInlineScripts(newModule.value.modules)
				}
			}
		} else if (newModule.value.type === 'branchone') {
			// Process branches and default modules
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? extractAndReplaceInlineScripts(branch.modules) : []
					}))
				}
			}
			if (newModule.value.default) {
				newModule.value = {
					...newModule.value,
					default: extractAndReplaceInlineScripts(newModule.value.default)
				}
			}
		} else if (newModule.value.type === 'branchall') {
			// Process all branches
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? extractAndReplaceInlineScripts(branch.modules) : []
					}))
				}
			}
		} else if (newModule.value.type === 'aiagent') {
			// Process AI agent tools
			if (newModule.value.tools) {
				newModule.value = {
					...newModule.value,
					tools: newModule.value.tools.map((tool) => {
						if (
							tool.value &&
							'tool_type' in tool.value &&
							tool.value.tool_type === 'flowmodule' &&
							'type' in tool.value &&
							tool.value.type === 'rawscript' &&
							'content' in tool.value &&
							tool.value.content
						) {
							inlineScriptStore.set(tool.id, tool.value.content as string)
							return {
								...tool,
								value: {
									...tool.value,
									content: `inline_script.${tool.id}`
								}
							}
						}
						return tool
					})
				}
			}
		}

		return newModule
	})
}

/**
 * Recursively restores inline script references back to their full content.
 * If content matches pattern "inline_script.{id}", looks up and restores the original.
 * If content doesn't match (new/modified script), keeps it as-is.
 */
export function restoreInlineScriptReferences(modules: FlowModule[]): FlowModule[] {
	return modules.map((module) => {
		const newModule = { ...module }

		if (newModule.value.type === 'rawscript' && newModule.value.content) {
			const content = newModule.value.content
			// Check if it's a reference
			const match = content.match(/^inline_script\.(.+)$/)
			if (match) {
				const moduleId = match[1]
				const storedContent = inlineScriptStore.get(moduleId)
				if (storedContent !== undefined) {
					// Restore original content
					newModule.value = {
						...newModule.value,
						content: storedContent
					}
				}
				// If not found in store, keep the reference as-is (shouldn't happen normally)
			}
			// If not a reference, it's new/modified content - keep as-is
		} else if (newModule.value.type === 'forloopflow' || newModule.value.type === 'whileloopflow') {
			// Recursively process nested modules in loops
			if (newModule.value.modules) {
				newModule.value = {
					...newModule.value,
					modules: restoreInlineScriptReferences(newModule.value.modules)
				}
			}
		} else if (newModule.value.type === 'branchone') {
			// Process branches and default modules
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? restoreInlineScriptReferences(branch.modules) : []
					}))
				}
			}
			if (newModule.value.default) {
				newModule.value = {
					...newModule.value,
					default: restoreInlineScriptReferences(newModule.value.default)
				}
			}
		} else if (newModule.value.type === 'branchall') {
			// Process all branches
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? restoreInlineScriptReferences(branch.modules) : []
					}))
				}
			}
		} else if (newModule.value.type === 'aiagent') {
			// Process AI agent tools
			if (newModule.value.tools) {
				newModule.value = {
					...newModule.value,
					tools: newModule.value.tools.map((tool) => {
						if (
							tool.value &&
							'tool_type' in tool.value &&
							tool.value.tool_type === 'flowmodule' &&
							'type' in tool.value &&
							tool.value.type === 'rawscript' &&
							'content' in tool.value &&
							tool.value.content
						) {
							const content = tool.value.content as string
							const match = content.match(/^inline_script\.(.+)$/)
							if (match) {
								const toolId = match[1]
								const storedContent = inlineScriptStore.get(toolId)
								if (storedContent !== undefined) {
									return {
										...tool,
										value: {
											...tool.value,
											content: storedContent
										}
									}
								}
							}
						}
						return tool
					})
				}
			}
		}

		return newModule
	})
}

/**
 * Recursively finds any unresolved inline script references in flow modules.
 * Returns array of module IDs that still have `inline_script.{id}` patterns.
 */
export function findUnresolvedInlineScriptRefs(modules: FlowModule[]): string[] {
	const unresolvedRefs: string[] = []

	function checkModule(module: FlowModule) {
		if (module.value.type === 'rawscript' && module.value.content) {
			const match = module.value.content.match(/^inline_script\.(.+)$/)
			if (match) {
				unresolvedRefs.push(match[1])
			}
		} else if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
			if (module.value.modules) {
				module.value.modules.forEach(checkModule)
			}
		} else if (module.value.type === 'branchone') {
			if (module.value.branches) {
				module.value.branches.forEach((branch) => {
					branch.modules?.forEach(checkModule)
				})
			}
			if (module.value.default) {
				module.value.default.forEach(checkModule)
			}
		} else if (module.value.type === 'branchall') {
			if (module.value.branches) {
				module.value.branches.forEach((branch) => {
					branch.modules?.forEach(checkModule)
				})
			}
		} else if (module.value.type === 'aiagent') {
			// Check AI agent tools
			if (module.value.tools) {
				for (const tool of module.value.tools) {
					if (
						tool.value &&
						'tool_type' in tool.value &&
						tool.value.tool_type === 'flowmodule' &&
						'type' in tool.value &&
						tool.value.type === 'rawscript' &&
						'content' in tool.value &&
						tool.value.content
					) {
						const match = (tool.value.content as string).match(/^inline_script\.(.+)$/)
						if (match) {
							unresolvedRefs.push(match[1])
						}
					}
				}
			}
		}
	}

	modules.forEach(checkModule)
	return unresolvedRefs
}

export const flowTools: Tool<FlowAIChatHelpers>[] = [
	createSearchHubScriptsTool(false),
	createDbSchemaTool<FlowAIChatHelpers>(),
	{
		def: searchScriptsToolDef,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, {
				content: 'Searching for workspace scripts related to "' + args.query + '"...'
			})
			const parsedArgs = searchScriptsSchema.parse(args)
			const scriptResults = await workspaceScriptsSearch.search(parsedArgs.query, workspace)
			toolCallbacks.setToolStatus(toolId, {
				content:
					'Found ' +
					scriptResults.length +
					' scripts in the workspace related to "' +
					args.query +
					'"'
			})
			return JSON.stringify(scriptResults)
		}
	},
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
		confirmationMessage: 'Run flow test',
		showDetails: true
	},
	{
		// set strict to false to avoid issues with open ai models
		def: { ...testRunStepToolDef, function: { ...testRunStepToolDef.function, strict: false } },
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

			// Find the step in the flow
			const modules = helpers.getModules()
			let targetModule: FlowModule | undefined = findModuleById(modules, stepId)

			if (!targetModule) {
				toolCallbacks.setToolStatus(toolId, {
					content: `Step '${stepId}' not found in flow`,
					error: `Step with id '${stepId}' does not exist in the current flow`
				})
				throw new Error(
					`Step with id '${stepId}' not found in flow. Available steps: ${modules.map((m) => m.id).join(', ')}`
				)
			}

			const module = targetModule
			const moduleValue = module.value

			if (moduleValue.type === 'rawscript') {
				// Test raw script step

				return executeTestRun({
					jobStarter: () =>
						JobService.runScriptPreview({
							workspace: workspace,
							requestBody: {
								content: moduleValue.content ?? '',
								language: moduleValue.language,
								args:
									module.id === SPECIAL_MODULE_IDS.PREPROCESSOR
										? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...stepArgs }
										: stepArgs
							}
						}),
					workspace,
					toolCallbacks,
					toolId,
					startMessage: `Starting test run of step '${stepId}'...`,
					contextName: 'script'
				})
			} else if (moduleValue.type === 'script') {
				// Test script step - need to get the script content
				const script = moduleValue.hash
					? await ScriptService.getScriptByHash({
							workspace: workspace,
							hash: moduleValue.hash
						})
					: await ScriptService.getScriptByPath({
							workspace: workspace,
							path: moduleValue.path
						})

				return executeTestRun({
					jobStarter: () =>
						JobService.runScriptPreview({
							workspace: workspace,
							requestBody: {
								content: script.content,
								language: script.language,
								args:
									module.id === SPECIAL_MODULE_IDS.PREPROCESSOR
										? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...stepArgs }
										: stepArgs
							}
						}),
					workspace,
					toolCallbacks,
					toolId,
					startMessage: `Starting test run of script step '${stepId}'...`,
					contextName: 'script'
				})
			} else if (moduleValue.type === 'flow') {
				// Test flow step
				return executeTestRun({
					jobStarter: () =>
						JobService.runFlowByPath({
							workspace: workspace,
							path: moduleValue.path,
							requestBody: stepArgs
						}),
					workspace,
					toolCallbacks,
					toolId,
					startMessage: `Starting test run of flow step '${stepId}'...`,
					contextName: 'flow'
				})
			} else {
				toolCallbacks.setToolStatus(toolId, {
					content: `Step type '${moduleValue.type}' not supported for testing`,
					error: `Cannot test step of type '${moduleValue.type}'`
				})
				throw new Error(
					`Cannot test step of type '${moduleValue.type}'. Supported types: rawscript, script, flow`
				)
			}
		},
		requiresConfirmation: true,
		confirmationMessage: 'Run flow step test',
		showDetails: true
	},
	{
		def: inspectInlineScriptToolDef,
		fn: async ({ args, toolCallbacks, toolId }) => {
			const parsedArgs = inspectInlineScriptSchema.parse(args)
			const moduleId = parsedArgs.moduleId

			toolCallbacks.setToolStatus(toolId, {
				content: `Retrieving inline script content for module '${moduleId}'...`
			})

			const content = inlineScriptStore.get(moduleId)

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

			// Update store to keep it coherent (for subsequent set_flow_json calls with references)
			inlineScriptStore.set(moduleId, code)

			// Update the flow directly via helper
			await helpers.setCode(moduleId, code)

			toolCallbacks.setToolStatus(toolId, { content: `Code updated for module '${moduleId}'` })
			return `Code for module '${moduleId}' has been updated successfully.`
		}
	},
	{
		def: { ...addModuleToolDef, function: { ...addModuleToolDef.function, strict: false } },
		streamArguments: true,
		showDetails: true,
		showFade: true,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const afterId = (args.afterId ?? null) as string | null
			const insideId = (args.insideId ?? null) as string | null
			const branchPath = (args.branchPath ?? null) as string | null
			let value = args.value

			// Parse value if it's a JSON string
			if (typeof value === 'string') {
				try {
					value = JSON.parse(value)
				} catch (e) {
					throw new Error(`Failed to parse value as JSON: ${(e as Error).message}`)
				}
			}

			// Validation
			// branchPath can be null when adding a new branch to branchall/branchone
			// In that case, value should be a branch object with summary, modules, etc.
			const isAddingNewBranch = insideId && branchPath === null

			if (!isAddingNewBranch) {
				// Adding a regular module - requires id
				if (!value.id) {
					throw new Error('Module value must include an id field')
				}
				// Check for restricted IDs
				if (isRestrictedModuleId(value.id)) {
					throw new Error(`Restricted id '${value.id}', can't be used, should choose an other`)
				}
			}

			const statusMessage = isAddingNewBranch
				? `Adding new branch to '${insideId}'...`
				: `Adding module '${value.id}'...`
			toolCallbacks.setToolStatus(toolId, { content: statusMessage })

			const { flow } = helpers.getFlowAndSelectedId()

			let processedValue = value

			// When adding a branch (not a module), skip ID checks and inline script handling
			if (!isAddingNewBranch) {
				// Check for duplicate ID
				const existing = findModuleInFlow(flow.value.modules, value.id)
				if (existing) {
					throw new Error(`Module with id '${value.id}' already exists`)
				}

				// Handle inline script storage if this is a rawscript with full content
				if (
					processedValue.value?.type === 'rawscript' &&
					processedValue.value?.content &&
					!processedValue.value.content.startsWith('inline_script.')
				) {
					// Store the content and replace with reference
					inlineScriptStore.set(processedValue.id, processedValue.value.content)
					processedValue = {
						...processedValue,
						value: {
							...processedValue.value,
							content: `inline_script.${processedValue.id}`
						}
					}
				}
			}

			// Add the module
			const updatedModules = addModuleToFlow(
				flow.value.modules,
				afterId,
				insideId,
				branchPath,
				processedValue as FlowModule
			)

			// Apply via setFlowJson to trigger proper snapshot and diff tracking
			const updatedFlow = {
				...flow.value,
				modules: updatedModules
			}

			await helpers.setFlowJson(JSON.stringify(updatedFlow))

			toolCallbacks.setToolStatus(toolId, {
				content: `Module '${value.id}' added successfully`,
				result: 'Success'
			})
			return `Module '${value.id}' has been added to the flow.`
		}
	},
	{
		def: { ...removeModuleToolDef, function: { ...removeModuleToolDef.function, strict: false } },
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = removeModuleSchema.parse(args)
			const { id } = parsedArgs

			// Check for restricted IDs
			if (isRestrictedModuleId(id)) {
				throw new Error(`Restricted id '${id}', can't be used, should choose an other`)
			}

			toolCallbacks.setToolStatus(toolId, { content: `Removing module '${id}'...` })

			const { flow } = helpers.getFlowAndSelectedId()

			// Check module exists
			const existing = findModuleInFlow(flow.value.modules, id)
			if (!existing) {
				throw new Error(`Module with id '${id}' not found`)
			}

			// Remove the module
			const updatedModules = removeModuleFromFlow(flow.value.modules, id)

			// Apply via setFlowJson to trigger proper snapshot and diff tracking
			const updatedFlow = {
				...flow.value,
				modules: updatedModules
			}

			await helpers.setFlowJson(JSON.stringify(updatedFlow))

			toolCallbacks.setToolStatus(toolId, { content: `Module '${id}' removed successfully` })
			return `Module '${id}' has been removed from the flow.`
		}
	},
	{
		def: removeBranchToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = removeBranchSchema.parse(args)
			const { insideId, branchIndex } = parsedArgs

			toolCallbacks.setToolStatus(toolId, {
				content: `Removing branch ${branchIndex} from '${insideId}'...`
			})

			const { flow } = helpers.getFlowAndSelectedId()

			// Check container exists
			const container = findModuleInFlow(flow.value.modules, insideId)
			if (!container) {
				throw new Error(`Container module with id '${insideId}' not found`)
			}

			// Validate it's a branchall/branchone
			if (container.value.type !== 'branchall' && container.value.type !== 'branchone') {
				throw new Error(
					`Module '${insideId}' is not a branchall/branchone (type: ${container.value.type})`
				)
			}

			// Remove the branch
			const updatedModules = removeBranchFromFlow(flow.value.modules, insideId, branchIndex)

			// Apply via setFlowJson
			const updatedFlow = {
				...flow.value,
				modules: updatedModules
			}

			await helpers.setFlowJson(JSON.stringify(updatedFlow))

			toolCallbacks.setToolStatus(toolId, {
				content: `Branch ${branchIndex} removed from '${insideId}'`
			})
			return `Branch ${branchIndex} has been removed from '${insideId}'.`
		}
	},
	{
		def: { ...modifyModuleToolDef, function: { ...modifyModuleToolDef.function, strict: false } },
		streamArguments: true,
		showDetails: true,
		showFade: true,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			let { id, value } = args

			// Check for restricted IDs
			if (isRestrictedModuleId(id)) {
				throw new Error(`Restricted id '${id}', can't be used, should choose an other`)
			}

			// Parse value if it's a JSON string
			if (typeof value === 'string') {
				try {
					value = JSON.parse(value)
				} catch (e) {
					throw new Error(`Failed to parse value as JSON: ${(e as Error).message}`)
				}
			}

			toolCallbacks.setToolStatus(toolId, { content: `Modifying module '${id}'...` })

			const { flow } = helpers.getFlowAndSelectedId()

			// Check module exists
			const existing = findModuleInFlow(flow.value.modules, id)
			if (!existing) {
				throw new Error(`Module with id '${id}' not found`)
			}

			// Handle inline script storage if this is a rawscript with full content
			let processedValue = value
			if (
				processedValue.value?.type === 'rawscript' &&
				processedValue.value?.content &&
				!processedValue.value.content.startsWith('inline_script.')
			) {
				// Store the content and replace with reference
				inlineScriptStore.set(id, processedValue.value.content)
				processedValue = {
					...processedValue,
					value: {
						...processedValue.value,
						content: `inline_script.${id}`
					}
				}
			}

			// Replace the module
			const updatedModules = replaceModuleInFlow(
				flow.value.modules,
				id,
				processedValue as FlowModule
			)

			// Apply via setFlowJson to trigger proper snapshot and diff tracking
			const updatedFlow = {
				...flow.value,
				modules: updatedModules
			}

			await helpers.setFlowJson(JSON.stringify(updatedFlow))

			toolCallbacks.setToolStatus(toolId, {
				content: `Module '${id}' modified successfully`,
				result: 'Success'
			})
			return `Module '${id}' has been modified.`
		}
	},
	{
		def: { ...setFlowSchemaToolDef, function: { ...setFlowSchemaToolDef.function, strict: false } },
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			let { schema } = args

			// If schema is a JSON string, parse it to an object
			if (typeof schema === 'string') {
				try {
					schema = JSON.parse(schema)
				} catch (e) {
					// If it fails to parse, keep it as-is and let it fail downstream
					console.warn('SCHEMA failed to parse as JSON string', e)
				}
			}

			toolCallbacks.setToolStatus(toolId, { content: 'Setting flow input schema...' })

			const { flow } = helpers.getFlowAndSelectedId()

			// Update the flow with new schema
			const updatedFlow = {
				...flow.value,
				schema
			}

			await helpers.setFlowJson(JSON.stringify(updatedFlow))

			toolCallbacks.setToolStatus(toolId, { content: 'Flow input schema updated successfully' })
			return 'Flow input schema has been updated.'
		}
	}
]

/**
 * Formats the OpenFlow schema for inclusion in the AI system prompt.
 * Extracts only the component schemas and formats them as JSON for the AI to reference.
 */
function formatOpenFlowSchemaForPrompt(): string {
	const schemas = openFlowSchema.components?.schemas
	if (!schemas) {
		return 'Schema not available'
	}

	// Create a simplified schema reference that's easier for the AI to parse
	return JSON.stringify(schemas, null, 2)
}

export function prepareFlowSystemMessage(customPrompt?: string): ChatCompletionSystemMessageParam {
	let content = `You are a helpful assistant that creates and edits workflows on the Windmill platform.

## IMPORTANT RULES

**Reserved IDs - Do NOT use these in add_module, modify_module, or remove_module:**
- \`failure\` - Reserved for failure handler module
- \`preprocessor\` - Reserved for preprocessor module
- \`Input\` - Reserved for flow input reference

## Tool Selection Guide

**Flow Modification:**
- **Add a new module** → \`add_module\`
- **Remove a module** → \`remove_module\`
- **Add a new branch to branchall/branchone** → \`add_module\` with \`branchPath: null\`
- **Remove a branch from branchall/branchone** → \`remove_branch\`
- **Change module code only** → \`set_module_code\`
- **Change module config/transforms/conditions** → \`modify_module\`
- **Update flow input parameters** → \`set_flow_schema\`

**Code & Scripts:**
- **View existing inline script code** → \`inspect_inline_script\`
- **Get language-specific coding instructions** → \`get_instructions_for_code_generation\` (call BEFORE writing code)
- **Find workspace scripts** → \`search_scripts\`
- **Find Windmill Hub scripts** → \`search_hub_scripts\`

**Testing:**
- **Test entire flow** → \`test_run_flow\`
- **Test single step** → \`test_run_step\`

**Resources & Schema:**
- **Search resource types** → \`resource_type\`
- **Get database schema** → \`get_db_schema\`

## Common Mistakes to Avoid

- **Don't use \`modify_module\` to add/remove nested modules** - Use \`add_module\`/\`remove_module\` instead
- **Don't forget \`input_transforms\`** - Rawscript parameters won't receive values without them
- **Don't use spaces in module IDs** - Use underscores (e.g., \`fetch_data\` not \`fetch data\`)
- **Don't reference future steps** - \`results.step_id\` only works for steps that execute before the current one
- **Don't create duplicate IDs** - Each module ID must be unique in the flow

## Flow Modification Tools

### add_module
Add a new module to the flow, or add a new branch to a branchall/branchone.

**Parameters:**
- \`afterId\`: ID of module to insert after, or \`null\` to insert at beginning
- \`insideId\` + \`branchPath\`: For inserting into containers (branches/loops/AI agents)
- \`insideId\` + \`branchPath: null\`: For adding a NEW branch to branchall/branchone
- \`value\`: The module object (or branch object when adding a new branch)

**Valid \`branchPath\` values:**
- \`"modules"\` - for forloopflow/whileloopflow
- \`"branches.0"\`, \`"branches.1"\`, etc. - to add inside a specific branch
- \`"default"\` - for branchone only
- \`"tools"\` - for aiagent
- \`null\` - to add a NEW branch to branchall/branchone

**Examples:**
\`\`\`javascript
// Insert after step_a
add_module({ afterId: "step_a", value: {...} })

// Insert at beginning of flow
add_module({ afterId: null, value: {...} })

// Insert into first branch, at beginning
add_module({ insideId: "branch_step", branchPath: "branches.0", afterId: null, value: {...} })

// Insert into first branch, after step_x
add_module({ insideId: "branch_step", branchPath: "branches.0", afterId: "step_x", value: {...} })

// Insert into loop
add_module({ insideId: "loop_step", branchPath: "modules", afterId: null, value: {...} })

// Add a NEW branch to branchall (branchPath: null)
add_module({ insideId: "my_branchall", branchPath: null, value: { summary: "New Branch", skip_failure: false, modules: [] } })

// Add a NEW branch to branchone (branchPath: null)
add_module({ insideId: "my_branchone", branchPath: null, value: { summary: "New Condition", expr: "results.step_a > 10", modules: [] } })
\`\`\`

### remove_module
Remove a module by ID.
\`\`\`javascript
remove_module({ id: "step_b" })
\`\`\`

### remove_branch
Remove a branch from a branchall/branchone by its index (0-based).
\`\`\`javascript
// Remove the first branch (index 0) from a branchall
remove_branch({ insideId: "my_branchall", branchIndex: 0 })

// Remove the second branch (index 1) from a branchone
remove_branch({ insideId: "my_branchone", branchIndex: 1 })
\`\`\`
**Note:** This removes the entire branch including all modules inside it.

### modify_module
Update an existing module (full replacement). Use for changing configuration, input_transforms, branch conditions, etc.
Do NOT use for adding/removing nested modules - use add_module/remove_module instead.
\`\`\`javascript
modify_module({ id: "step_a", value: {...} })
\`\`\`

### set_module_code
Modify only the code of an existing inline script module. Use for quick code-only changes.
\`\`\`javascript
set_module_code({ moduleId: "step_a", code: "..." })
\`\`\`

### set_flow_schema
Set/update flow input parameters.
\`\`\`javascript
set_flow_schema({ schema: { type: "object", properties: { user_id: { type: "string" } }, required: ["user_id"] } })
\`\`\`

Follow the user instructions carefully.
At the end of your changes, explain precisely what you did and what the flow does now.
ALWAYS test your modifications. You have access to the \`test_run_flow\` and \`test_run_step\` tools to test the flow and steps. If you only modified a single step, use the \`test_run_step\` tool to test it. If you modified the flow, use the \`test_run_flow\` tool to test it. If the user cancels the test run, do not try again and wait for the next user instruction.
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

**To add a new inline script module:**
- Use \`add_module\` with the full code content directly (not a reference)
- Avoid coding in single lines, always use multi-line code blocks.
- The system will automatically store and optimize it

**To inspect existing code:**
- Use \`inspect_inline_script\` tool to view the current code: \`inspect_inline_script({ moduleId: "step_a" })\`

### Input Transforms for Rawscripts

Rawscript modules use \`input_transforms\` to map function parameters to values. Each key in \`input_transforms\` corresponds to a parameter name in your script's \`main\` function.

**Transform Types:**
- \`static\`: Fixed value passed directly
- \`javascript\`: Dynamic expression evaluated at runtime

**Available Variables in JavaScript Expressions:**
- \`flow_input.{property}\` - Access flow input parameters
- \`results.{step_id}\` - Access output from a previous step
- \`flow_input.iter.value\` - Current item when inside a for-loop
- \`flow_input.iter.index\` - Current index when inside a for-loop

**Example - Rawscript using flow input and previous step result:**
\`\`\`json
{
  "id": "step_b",
  "value": {
    "type": "rawscript",
    "language": "bun",
    "content": "export async function main(userId: string, data: any[]) {
		return "Hello, world!";
	}",
    "input_transforms": {
      "userId": {
        "type": "javascript",
        "expr": "flow_input.user_id"
      },
      "data": {
        "type": "javascript",
        "expr": "results.step_a"
      }
    }
  }
}
\`\`\`

**Example - Static value:**
\`\`\`json
{
  "input_transforms": {
    "limit": {
      "type": "static",
      "value": 100
    }
  }
}
\`\`\`

**Important:** The parameter names in \`input_transforms\` must match the function parameter names in your script. When you create or modify a rawscript, always define \`input_transforms\` to connect it to flow inputs or results from other steps.

### Other Key Concepts
- **Resources**: For flow inputs, use type "object" with format "resource-<type>". For step inputs, use "$res:path/to/resource"
- **Module IDs**: Must be unique and valid identifiers. Used to reference results via \`results.step_id\`
- **Module types**: Use 'bun' as default language for rawscript if unspecified

### Writing Code for Modules

**IMPORTANT: Before writing any code for a rawscript module, you MUST call the \`get_instructions_for_code_generation\` tool with the target language.** This tool provides essential language-specific instructions.

Always call this tool first when:
- Creating a new rawscript module
- Modifying existing code in a module
- Setting code via \`set_module_code\`

Example: Before writing TypeScript/Bun code, call \`get_instructions_for_code_generation({ language: "bun" })\`

### Creating New Steps

1. **Search for existing scripts first** (unless user explicitly asks to write from scratch):
   - First: \`search_scripts\` to find workspace scripts
   - Then: \`search_hub_scripts\` (only consider highly relevant results)
   - Only create a raw script if no suitable script is found

2. **Add the module using \`add_module\`:**
   - If using existing script: \`add_module({ afterId: "previous_step", value: { id: "new_step", value: { type: "script", path: "f/folder/script" } } })\`
   - If creating rawscript:
     - Default language is 'bun' if not specified
     - **First call \`get_instructions_for_code_generation\` to get the correct code format**
     - Include full code in the content field
     - Always define \`input_transforms\` to connect parameters to flow inputs or previous step results

3. **Update flow schema if needed:**
   - If your module references flow_input properties that don't exist yet, add them using \`set_flow_schema\`

### AI Agent Tools

AI agents can use tools to accomplish tasks. To manage tools for an AI agent:

- **Adding a tool to an AI agent**: Use \`add_module\` with \`insideId\` set to the agent's ID and \`branchPath: "tools"\`
  - Tool order doesn't affect execution, so you can omit \`afterId\` (defaults to inserting at beginning)
  - Example: \`add_module({ insideId: "ai_agent_step", branchPath: "tools", value: { id: "search_docs", summary: "Search documentation", value: { tool_type: "flowmodule", type: "rawscript", language: "bun", content: "...", input_transforms: {} } } })\`

- **Removing a tool from an AI agent**: Use \`remove_module\` with the tool's ID
  - The tool will be found and removed from the agent's tools array

- **Modifying a tool**: Use \`modify_module\` with the tool's ID
  - Example: \`modify_module({ id: "search_docs", value: { ... } })\`

- **Tool IDs**: Cannot contain spaces - use underscores (e.g., \`get_user_data\` not \`get user data\`)
- **Tool summaries**: Unlike other module summaries, tool summaries cannot contain spaces, use underscores instead.

- **Tool types**:
  - \`flowmodule\`: A script/flow that the agent can call (same as regular flow modules but with \`tool_type: "flowmodule"\`)
  - \`mcp\`: Reference to an MCP server tool

**Example - Adding a rawscript tool to an agent:**
\`\`\`json
add_module({
  insideId: "my_agent",
  branchPath: "tools",
  value: {
    id: "fetch_weather",
    summary: "Get current weather for a location",
    value: {
      tool_type: "flowmodule",
      type: "rawscript",
      language: "bun",
      content: "export async function main(location: string) { ... }",
      input_transforms: {
        location: { type: "static", value: "" }
      }
    }
  }
})
\`\`\`

## Resource Types
On Windmill, credentials and configuration are stored in resources. Resource types define the format of the resource.
- Use the \`resource_type\` tool to search for available resource types (e.g. stripe, google, postgresql, etc.)
- If the user needs a resource as flow input, set the property type in the schema to "object" and add a key called "format" set to "resource-nameofresourcetype" (e.g. "resource-stripe")
- If the user wants a specific resource as step input, set the step value to a static string in the format: "$res:path/to/resource"

### OpenFlow Schema Reference
Below is the complete OpenAPI schema for OpenFlow. All field descriptions and behaviors are defined here. Refer to this as the authoritative reference when generating flow JSON:

\`\`\`json
${formatOpenFlowSchemaForPrompt()}
\`\`\`

The schema includes detailed descriptions for:
- **FlowModuleValue types**: rawscript, script, flow, forloopflow, whileloopflow, branchone, branchall, identity, aiagent
- **Module configuration**: stop_after_if, skip_if, suspend, sleep, cache_ttl, retry, mock, timeout
- **InputTransform**: static vs javascript, available variables (results, flow_input, flow_input.iter)
- **Special modules**: preprocessor_module, failure_module
- **Loop options**: iterator, parallel, parallelism, skip_failures
- **Branch types**: BranchOne (first match), BranchAll (all execute)

### Contexts

You have access to the following contexts:
- Database schemas: Schema of databases the user is using
- Flow diffs: Diff between current flow and last deployed flow
- Focused flow modules: IDs of modules the user is focused on. Your response should focus on these modules
`

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
	selectedContext: ContextElement[] = []
): ChatCompletionUserMessageParam {
	const flow = flowAndSelectedId?.flow
	const selectedId = flowAndSelectedId?.selectedId

	// Handle context elements
	const contextInstructions = selectedContext ? buildContextString(selectedContext) : ''

	if (!flow || !selectedId) {
		let userMessage = `## INSTRUCTIONS:
${instructions}`
		return {
			role: 'user',
			content: userMessage
		}
	}

	const codePieces = selectedContext.filter((c) => c.type === 'flow_module_code_piece')

	// Clear the inline script store and extract inline scripts for token optimization
	inlineScriptStore.clear()
	const optimizedModules = extractAndReplaceInlineScripts(flow.value.modules)

	// Apply code pieces to the optimized modules (returns YAML string)
	const flowModulesYaml = applyCodePiecesToFlowModules(codePieces, optimizedModules)

	// Handle preprocessor and failure modules
	let optimizedPreprocessor = flow.value.preprocessor_module
	if (optimizedPreprocessor?.value?.type === 'rawscript' && optimizedPreprocessor.value.content) {
		inlineScriptStore.set(optimizedPreprocessor.id, optimizedPreprocessor.value.content)
		optimizedPreprocessor = {
			...optimizedPreprocessor,
			value: {
				...optimizedPreprocessor.value,
				content: `inline_script.${optimizedPreprocessor.id}`
			}
		}
	}

	let optimizedFailure = flow.value.failure_module
	if (optimizedFailure?.value?.type === 'rawscript' && optimizedFailure.value.content) {
		inlineScriptStore.set(optimizedFailure.id, optimizedFailure.value.content)
		optimizedFailure = {
			...optimizedFailure,
			value: {
				...optimizedFailure.value,
				content: `inline_script.${optimizedFailure.id}`
			}
		}
	}

	const finalFlow = {
		schema: flow.schema,
		modules: flowModulesYaml,
		preprocessor_module: optimizedPreprocessor,
		failure_module: optimizedFailure
	}

	let flowContent = `## CURRENT FLOW JSON:
${JSON.stringify(finalFlow, null, 2)}

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
