import { z } from 'zod'
import type { FlowModule, FlowValue } from '$lib/gen'
import { collectAllFlowModuleIdsFromModules } from '$lib/components/flows/flowTree'
import {
	collectInvalidAgentToolNames,
	collectProviderlessAgentIds
} from '$lib/components/flows/agentToolTree'
import { validateAiAgentProviders, type AiAgentProviderCatalog } from './aiAgentProviders'
import { SPECIAL_MODULE_IDS } from '../shared'
import { findUnresolvedInlineScriptRefs, type InlineScriptSession } from './inlineScriptsUtils'
import {
	replaceNewInlineScriptRefsWithEmptyCode,
	validateFlowGroups,
	validateFlowNotes,
	type FlowGroup,
	type FlowNote
} from './helperUtils'
import { flowModuleSchema, flowModulesSchema } from './openFlowZod.gen'
import {
	FLOW_VALUE_SETTINGS_KEYS,
	flowValueSettingsSchema,
	pickFlowValueSettings,
	type FlowValueSettings
} from './flowValueSettings'

export {
	FLOW_VALUE_SETTINGS_KEYS,
	pickFlowValueSettings,
	type FlowValueSettings
} from './flowValueSettings'

/**
 * Compact, agent-friendly representation of a flow.
 *
 * Inline rawscript content is replaced with `inline_script.<moduleId>` placeholders
 * by `buildEditableFlowJson` so flows stay short in tool I/O. Use the matching
 * InlineScriptSession to recover the actual content.
 */
export type EditableFlowJson = {
	modules: FlowModule[]
	schema: Record<string, any> | null
	preprocessor_module: FlowModule | null
	failure_module: FlowModule | null
	groups: FlowGroup[] | null
	notes: FlowNote[] | null
} & FlowValueSettings

/** Optional input to `validateEditableFlowJson` and `validateFlowModules`. */
type FlowValidationContext = {
	/** Custom modules schema to validate against (defaults to flowModulesSchema). */
	modulesSchema?: z.ZodTypeAny
	/** Workspace AI provider resources, to check the provider config of AI agent steps
	 * against. Omitted, only the shape of a provider config is checked. */
	aiProviders?: AiAgentProviderCatalog
	/** Sink for provider findings that did not block the write. Callers surface these in the
	 * tool result. */
	aiProviderWarnings?: string[]
}

/**
 * Navigate to a schema at a given path, handling arrays, objects, unions, and wrappers.
 * Uses Zod 4 internal structure.
 */
function getSchemaAtPath(
	schema: z.ZodType,
	path: (string | number)[],
	data?: any
): z.ZodType | null {
	let current: z.ZodType = schema
	let currentData = data

	for (let i = 0; i < path.length; i++) {
		const segment = path[i]

		if (!current || !(current as any)._def) return null

		let type = (current as any)._def.type

		// Unwrap optional/nullable/default/catch
		while (['optional', 'nullable', 'default', 'catch'].includes(type)) {
			current = (current as any)._def.innerType
			if (!current || !(current as any)._def) return null
			type = (current as any)._def.type
		}

		if (type === 'array') {
			if (typeof segment === 'number') {
				current = (current as any)._def.element
				if (currentData && Array.isArray(currentData)) {
					currentData = currentData[segment]
				}
				continue
			}
			current = (current as any)._def.element
			i--
			continue
		}

		if (type === 'object') {
			const shape = (current as any)._def.shape
			const key = String(segment)
			if (shape && key in shape) {
				current = shape[key]
				if (currentData && typeof currentData === 'object') {
					currentData = currentData[key]
				}
				continue
			}
			return null
		}

		if (type === 'union') {
			const options = (current as any)._def.options
			if (options) {
				if (currentData && typeof currentData === 'object') {
					const typeValue = currentData.type
					if (typeValue) {
						for (const option of options) {
							const optionShape = (option as any)._def?.shape
							const optionType = optionShape?.type?._def?.values?.[0]
							if (optionType === typeValue) {
								const remainingPath = path.slice(i)
								const result = getSchemaAtPath(option, remainingPath, currentData)
								if (result) return result
							}
						}
					}
				}
				for (const option of options) {
					const remainingPath = path.slice(i)
					const result = getSchemaAtPath(option, remainingPath, currentData)
					if (result) return result
				}
			}
			return null
		}

		if (type === 'record') {
			current = (current as any)._def.valueType
			if (!current) return null
			if (currentData && typeof currentData === 'object') {
				currentData = currentData[segment]
			}
			continue
		}

		return null
	}

	return current
}

function formatJsonSchemaForError(jsonSchema: any): string {
	if (jsonSchema.type === 'object' && jsonSchema.properties) {
		const props = Object.entries(jsonSchema.properties)
			.slice(0, 5)
			.map(([k, v]: [string, any]) => {
				if (v.enum) {
					return `${k}: ${v.enum.map((e: any) => JSON.stringify(e)).join('|')}`
				}
				return `${k}: ${v.type || 'any'}`
			})
			.join(', ')
		const moreProps =
			Object.keys(jsonSchema.properties).length > 5
				? `, ... (${Object.keys(jsonSchema.properties).length - 5} more)`
				: ''
		const required = jsonSchema.required?.length
			? ` (required: ${jsonSchema.required.join(', ')})`
			: ''
		return `{ ${props}${moreProps} }${required}`
	}

	if (jsonSchema.const !== undefined) return JSON.stringify(jsonSchema.const)
	if (jsonSchema.enum) {
		return `one of: ${jsonSchema.enum.map((v: any) => JSON.stringify(v)).join(', ')}`
	}
	if (jsonSchema.oneOf)
		return jsonSchema.oneOf.map((s: any) => formatJsonSchemaForError(s)).join(' | ')
	if (jsonSchema.anyOf)
		return jsonSchema.anyOf.map((s: any) => formatJsonSchemaForError(s)).join(' | ')
	if (jsonSchema.description) return jsonSchema.description
	return jsonSchema.type || JSON.stringify(jsonSchema)
}

function getExpectedFormat(schema: z.ZodType): string | null {
	if (!schema || !(schema as any)._def) return null

	let current = schema
	while ((current as any)._def.type === 'optional' || (current as any)._def.type === 'nullable') {
		current = (current as any)._def.innerType
		if (!current || !(current as any)._def) break
	}

	try {
		const jsonSchema = z.toJSONSchema(schema)
		if (
			Object.keys(jsonSchema).length <= 1 ||
			(Object.keys(jsonSchema).length === 1 && jsonSchema.$schema)
		) {
			return null
		}
		const formatted = formatJsonSchemaForError(jsonSchema)
		if (formatted && formatted !== 'unknown' && !formatted.startsWith('{')) return formatted
		if (formatted && formatted.startsWith('{') && formatted !== '{ }') return formatted
	} catch {
		// Ignore errors from toJSONSchema
	}
	return null
}

export function validateFlowModules(
	rawModules: unknown,
	ctx: FlowValidationContext = {}
): FlowModule[] {
	if (!Array.isArray(rawModules)) {
		throw new Error('Flow modules must be an array')
	}

	const parsedModules = rawModules as FlowModule[]
	const modulesSchema = ctx.modulesSchema ?? flowModulesSchema
	const result = modulesSchema.safeParse(parsedModules)
	if (!result.success) {
		const errors = result.error.issues.slice(0, 5).map((e) => {
			const path = e.path
			const moduleIndex = typeof path[0] === 'number' ? path[0] : undefined
			const moduleId = moduleIndex !== undefined ? parsedModules[moduleIndex]?.id : undefined
			const fieldPath = path.slice(1).join('.')

			let message = e.message
			if (e.code === 'invalid_type') {
				const targetSchema = getSchemaAtPath(
					modulesSchema,
					path as (string | number)[],
					parsedModules
				)
				if (targetSchema) {
					const expectedFormat = getExpectedFormat(targetSchema)
					if (expectedFormat) {
						message += `\n    Expected format: ${expectedFormat}`
					}
				}
			}

			if (moduleId) {
				return `Module "${moduleId}" -> ${fieldPath}: ${message}`
			}
			return `${path.join('.')}: ${message}`
		})

		throw new Error(`Invalid flow modules:\n${errors.join('\n')}`)
	}

	const ids = collectAllFlowModuleIdsFromModules(parsedModules)
	if (ids.length !== new Set(ids).size) {
		throw new Error('Duplicate module IDs found in flow')
	}

	const reservedIds = ids.filter(
		(id) => id === SPECIAL_MODULE_IDS.PREPROCESSOR || id === SPECIAL_MODULE_IDS.FAILURE
	)
	if (reservedIds.length > 0) {
		throw new Error(
			'Special modules must be provided via preprocessor_module and failure_module, not inside modules'
		)
	}

	// Not expressible in the schema: `provider` is required only when the step is standalone, and
	// making AiAgent a conditional union breaks the FlowModuleValue discriminated union it belongs to.
	const providerless = collectProviderlessAgentIds(parsedModules)
	if (providerless.length > 0) {
		throw new Error(
			`AI agent modules ${providerless
				.map((id) => `"${id}"`)
				.join(
					', '
				)} need a provider input transform, or an "agent" path linking them to a saved agent`
		)
	}

	// An agent tool's `summary` is the name the LLM sees; the worker rejects anything outside
	// `^[a-zA-Z0-9_]+$`, so a flow written with a spaced name saves but fails on every run.
	const invalidToolNames = collectInvalidAgentToolNames(parsedModules)
	if (invalidToolNames.length > 0) {
		throw new Error(
			`Invalid AI agent tool name(s): ${invalidToolNames
				.map(
					(t) =>
						`agent "${t.agentId}" tool "${t.toolId}" is named ${JSON.stringify(t.name)} - ${t.error}`
				)
				.join(
					'; '
				)}. The tool's "summary" is the name the agent calls it by: use underscores instead of spaces (e.g. "search_docs").`
		)
	}

	validateAiAgentProviders(parsedModules, ctx.aiProviders, ctx.aiProviderWarnings)

	return parsedModules
}

export function validateFlowSchema(rawSchema: unknown): Record<string, any> | null {
	if (rawSchema == null) return null
	if (typeof rawSchema !== 'object' || Array.isArray(rawSchema)) {
		throw new Error('Flow schema must be an object or null')
	}
	return rawSchema as Record<string, any>
}

function validateOptionalFlowModule(rawModule: unknown, fieldName: string): FlowModule | null {
	if (rawModule == null) return null

	const result = flowModuleSchema.safeParse(rawModule)
	if (!result.success) {
		const error = result.error.issues[0]
		throw new Error(`Invalid ${fieldName}: ${error?.message ?? 'unknown error'}`)
	}
	return result.data
}

export const EDITABLE_FLOW_STRUCTURAL_KEYS = [
	'modules',
	'schema',
	'preprocessor_module',
	'failure_module',
	'groups',
	'notes'
] as const

/**
 * Parse and validate a raw object as an `EditableFlowJson`. Validates module
 * shape, schema shape, optional special modules (with their reserved ids),
 * groups, top-level flow settings, and that no module ids collide.
 */
export function validateEditableFlowJson(
	rawFlow: unknown,
	ctx: FlowValidationContext = {}
): EditableFlowJson {
	if (!rawFlow || typeof rawFlow !== 'object' || Array.isArray(rawFlow)) {
		throw new Error('Flow JSON must be an object')
	}

	const flow = rawFlow as Record<string, unknown>

	// Reject unknown top-level keys: silently dropping them would make patch
	// tools report success for edits that never land on the flow.
	const allowedKeys = new Set<string>([
		...EDITABLE_FLOW_STRUCTURAL_KEYS,
		...FLOW_VALUE_SETTINGS_KEYS
	])
	const unknownKeys = Object.keys(flow).filter((key) => !allowedKeys.has(key))
	if (unknownKeys.length > 0) {
		throw new Error(
			`Unknown top-level flow key(s): ${unknownKeys.join(', ')}. Allowed keys: ${[...allowedKeys].join(', ')}`
		)
	}

	const settingsResult = flowValueSettingsSchema.safeParse(flow)
	if (!settingsResult.success) {
		const issue = settingsResult.error.issues[0]
		const path = issue?.path?.join('.') ?? 'settings'
		throw new Error(`Invalid flow setting ${path}: ${issue?.message ?? 'unknown error'}`)
	}
	const settings = pickFlowValueSettings(settingsResult.data)

	const modules = validateFlowModules(flow.modules, ctx)
	const schema = validateFlowSchema(flow.schema)
	const preprocessorModule = validateOptionalFlowModule(
		flow.preprocessor_module,
		'preprocessor_module'
	)
	const failureModule = validateOptionalFlowModule(flow.failure_module, 'failure_module')
	const groupModuleIds = new Set(collectAllFlowModuleIdsFromModules(modules))
	const groups = validateFlowGroups(flow.groups, groupModuleIds)
	const notes = validateFlowNotes(flow.notes, groupModuleIds)

	if (preprocessorModule) {
		if (preprocessorModule.id !== SPECIAL_MODULE_IDS.PREPROCESSOR) {
			throw new Error(
				`Invalid preprocessor_module: id must be "${SPECIAL_MODULE_IDS.PREPROCESSOR}"`
			)
		}
		if (
			preprocessorModule.value.type !== 'rawscript' &&
			preprocessorModule.value.type !== 'script'
		) {
			throw new Error(
				'Invalid preprocessor_module: only "rawscript" and "script" modules are supported'
			)
		}
	}
	if (failureModule) {
		if (failureModule.id !== SPECIAL_MODULE_IDS.FAILURE) {
			throw new Error(`Invalid failure_module: id must be "${SPECIAL_MODULE_IDS.FAILURE}"`)
		}
		if (failureModule.value.type !== 'rawscript' && failureModule.value.type !== 'script') {
			throw new Error('Invalid failure_module: only "rawscript" and "script" modules are supported')
		}
	}

	const ids = new Set(collectAllFlowModuleIdsFromModules(modules))
	if (preprocessorModule) {
		if (ids.has(preprocessorModule.id)) {
			throw new Error(`Duplicate module ID found in preprocessor_module: ${preprocessorModule.id}`)
		}
		ids.add(preprocessorModule.id)
	}
	if (failureModule && ids.has(failureModule.id)) {
		throw new Error(`Duplicate module ID found in failure_module: ${failureModule.id}`)
	}

	return {
		modules,
		schema,
		preprocessor_module: preprocessorModule,
		failure_module: failureModule,
		groups,
		notes,
		...settings
	}
}

/**
 * Build the agent-facing compact view of a flow. When an `InlineScriptSession`
 * is provided, every rawscript module's `content` is moved into the session
 * and replaced with the placeholder `inline_script.<moduleId>` (preprocessor
 * and failure modules included).
 */
export function buildEditableFlowJson(
	flow: { value: FlowValue; schema?: Record<string, any> | null | undefined },
	inlineScriptSession?: InlineScriptSession
): EditableFlowJson {
	const modules = inlineScriptSession
		? inlineScriptSession.extractAndReplaceInlineScripts(flow.value.modules)
		: flow.value.modules

	let preprocessorModule = flow.value.preprocessor_module
	if (
		preprocessorModule?.value?.type === 'rawscript' &&
		typeof preprocessorModule.value.content === 'string' &&
		inlineScriptSession
	) {
		inlineScriptSession.set(preprocessorModule.id, preprocessorModule.value.content)
		preprocessorModule = {
			...preprocessorModule,
			value: {
				...preprocessorModule.value,
				content: `inline_script.${preprocessorModule.id}`
			}
		}
	}

	let failureModule = flow.value.failure_module
	if (
		failureModule?.value?.type === 'rawscript' &&
		typeof failureModule.value.content === 'string' &&
		inlineScriptSession
	) {
		inlineScriptSession.set(failureModule.id, failureModule.value.content)
		failureModule = {
			...failureModule,
			value: {
				...failureModule.value,
				content: `inline_script.${failureModule.id}`
			}
		}
	}

	return {
		modules,
		schema: flow.schema ?? null,
		preprocessor_module: preprocessorModule ?? null,
		failure_module: failureModule ?? null,
		groups: flow.value.groups ?? null,
		notes: flow.value.notes ?? null,
		...pickFlowValueSettings(flow.value)
	}
}

export function restoreSpecialRawscriptModule(
	module: FlowModule | null,
	session: InlineScriptSession
): FlowModule | null {
	if (!module || module.value.type !== 'rawscript' || !module.value.content) return module
	const match = module.value.content.match(/^inline_script\.(.+)$/)
	if (!match) return module
	const content = session.get(match[1])
	if (content === undefined) return module
	return { ...module, value: { ...module.value, content } }
}

/**
 * Inverse of `buildEditableFlowJson`. Replaces `inline_script.<moduleId>`
 * placeholders in `editable.modules` and the special modules with the content
 * stored in `session`.
 *
 * The compact view is the full state for the settings in
 * `FLOW_VALUE_SETTINGS_KEYS`: a settings key absent from `editable` is removed
 * from the result, so patches can unset them. Fields of the original FlowValue
 * outside that list are preserved untouched.
 *
 * Pair with `buildEditableFlowJson` for round-trip patches: extract → patch
 * the compact view → restore.
 */
export function applyEditableFlowJsonToFlow(
	originalValue: FlowValue,
	editable: EditableFlowJson,
	session: InlineScriptSession
): FlowValue {
	const result: FlowValue = {
		...originalValue,
		modules: session.restoreInlineScriptReferences(editable.modules),
		preprocessor_module:
			restoreSpecialRawscriptModule(editable.preprocessor_module, session) ?? undefined,
		failure_module: restoreSpecialRawscriptModule(editable.failure_module, session) ?? undefined,
		groups: editable.groups ?? undefined,
		notes: editable.notes ?? undefined
	}
	for (const key of FLOW_VALUE_SETTINGS_KEYS) {
		if (editable[key] !== undefined) {
			;(result as Record<string, unknown>)[key] = editable[key]
		} else {
			delete (result as Record<string, unknown>)[key]
		}
	}
	return result
}

/**
 * Post-restore normalization for draft flow writes. A placeholder that survived
 * restore either names its own module — a new inline body, blanked in place so
 * the caller's empty-body warning sends the model to the set-module-code tool —
 * or names nothing, in which case the write is rejected so a literal
 * `inline_script.*` string is never persisted as runnable content.
 */
export function finalizeUnresolvedInlineScripts(value: {
	modules: FlowModule[]
	preprocessor_module?: FlowModule | null
	failure_module?: FlowModule | null
}): void {
	const specials = [value.preprocessor_module, value.failure_module].filter(
		(module): module is FlowModule => module != null
	)
	const blanked = new Set<string>()
	replaceNewInlineScriptRefsWithEmptyCode(value.modules, blanked)
	replaceNewInlineScriptRefsWithEmptyCode(specials, blanked)
	const unresolved = [
		...findUnresolvedInlineScriptRefs(value.modules),
		...findUnresolvedInlineScriptRefs(specials)
	]
	if (unresolved.length > 0) {
		const refs = unresolved.map((id) => `"inline_script.${id}"`).join(', ')
		throw new Error(
			`Unresolved inline script reference(s): ${refs}. A rawscript "content" placeholder must be "inline_script.<its own module id>" (a new body, filled afterwards with the set-module-code tool) or reference an existing rawscript module to keep its current body. Nothing was saved.`
		)
	}
}
