import { FlowService, ScriptService } from '$lib/gen'
import { getFlowPrompt, getScriptPrompt } from '$system_prompts'
import type { Flow, Script } from '$lib/gen/types.gen'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import { z } from 'zod'
import { createToolDef, type Tool } from '../shared'
import {
	globalDraftStore,
	getGlobalDraftKey,
	type GlobalDraftBase,
	type GlobalDraftItem,
	type GlobalWorkspaceItemType
} from './draftStore.svelte'

const ITEM_TYPES = ['script', 'flow'] as const satisfies readonly GlobalWorkspaceItemType[]
const MAX_LIST_LIMIT = 100

const SCRIPT_INSTRUCTION_LANGUAGES = [
	'bunnative',
	'nativets',
	'bun',
	'deno',
	'python3',
	'php',
	'rust',
	'go',
	'bash',
	'postgresql',
	'mysql',
	'bigquery',
	'snowflake',
	'mssql',
	'graphql',
	'powershell',
	'csharp',
	'java',
	'duckdb',
	'rlang',
	'oracledb',
	'ansible',
	'nu',
	'ruby'
] as const

const SYSTEM_PROMPT_SCRIPT_LANGUAGES = new Set<string>([
	'bunnative',
	'nativets',
	'bun',
	'deno',
	'python3',
	'php',
	'rust',
	'go',
	'bash',
	'postgresql',
	'mysql',
	'bigquery',
	'snowflake',
	'mssql',
	'graphql',
	'powershell',
	'csharp',
	'java',
	'duckdb',
	'rlang'
])

const itemTypeSchema = z.enum(ITEM_TYPES)
const scriptInstructionLanguageSchema = z.enum(SCRIPT_INSTRUCTION_LANGUAGES)
type ScriptInstructionLanguage = z.infer<typeof scriptInstructionLanguageSchema>
const DEFAULT_SCRIPT_LANGUAGE: ScriptInstructionLanguage = 'bun'

const getInstructionsSchema = z.object({
	subject: itemTypeSchema.describe('The workspace item type to get authoring instructions for.'),
	language: scriptInstructionLanguageSchema
		.optional()
		.describe(
			'Required when subject is script. Use the existing script language when modifying, or the requested target language when creating.'
		)
})

const listWorkspaceItemsSchema = z.object({
	types: z
		.array(itemTypeSchema)
		.optional()
		.describe('Optional item types to list. Defaults to scripts and flows.'),
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
	path: z.string().describe('Workspace path of the item to read.')
})

const writeWorkspaceItemSchema = z.object({
	type: itemTypeSchema,
	path: z.string().describe('Workspace path for the new draft item.'),
	draft: z.any().describe('The complete draft item payload for the item type.'),
	overwrite: z
		.boolean()
		.optional()
		.describe('If true, replace an existing AI draft with the same type and path.')
})

const modifyWorkspaceItemSchema = z.object({
	type: itemTypeSchema,
	path: z.string().describe('Workspace path of the existing item or AI draft to modify.'),
	draft: z.any().describe('The complete modified draft item payload for the item type.')
})

type WorkspaceItemRead = {
	type: GlobalWorkspaceItemType
	path: string
	summary?: string
	version?: string | number
	editedAt?: string
	value: unknown
}

type WorkspaceItemMetadata = {
	type: GlobalWorkspaceItemType
	path: string
	source: 'workspace' | 'draft' | 'workspace+draft'
	summary?: string
	version?: string | number
	editedAt?: string
	draftStatus?: GlobalDraftItem['status']
	size?: number
	language?: string
	kind?: string
	moduleCount?: number
}

const GLOBAL_SYSTEM_PROMPT = `You are Windmill's global workspace assistant.

You can inspect workspace scripts and flows, then create draft changes in the frontend AI draft store.

Important rules:
- Writes and modifications are drafts only. They do not save, deploy, or mutate backend workspace items.
- Use list_workspace_items before broad reads.
- Use read_workspace_item before modifying an existing item unless the user already provided the complete current item.
- Use get_instructions before writing or modifying a script or flow. For scripts, pass the target language; when modifying, use the language from the item you read.
- Keep context targeted. Do not read unrelated items.
- Be explicit with the user when you create or update a draft.`

function getRequestedTypes(
	types: GlobalWorkspaceItemType[] | undefined
): GlobalWorkspaceItemType[] {
	return types && types.length > 0 ? types : [...ITEM_TYPES]
}

function getJsonSize(value: unknown): number | undefined {
	try {
		return JSON.stringify(value)?.length
	} catch {
		return undefined
	}
}

function getObjectString(value: unknown, key: string): string | undefined {
	if (!value || typeof value !== 'object') return undefined
	const candidate = (value as Record<string, unknown>)[key]
	return typeof candidate === 'string' ? candidate : undefined
}

function getDraftSummary(draft: GlobalDraftItem): string | undefined {
	return getObjectString(draft.draft, 'summary') ?? getObjectString(draft.base?.value, 'summary')
}

function itemMatches(
	item: Pick<WorkspaceItemMetadata, 'path' | 'summary'>,
	query: string | undefined,
	pathPrefix: string | undefined
): boolean {
	if (pathPrefix && !item.path.startsWith(pathPrefix)) {
		return false
	}

	const normalizedQuery = query?.trim().toLowerCase()
	if (!normalizedQuery) {
		return true
	}

	return (
		item.path.toLowerCase().includes(normalizedQuery) ||
		(item.summary?.toLowerCase().includes(normalizedQuery) ?? false)
	)
}

function scriptToMetadata(script: Script): WorkspaceItemMetadata {
	return {
		type: 'script',
		path: script.path,
		source: 'workspace',
		summary: script.summary,
		version: script.hash,
		size: script.content?.length,
		language: script.language,
		kind: script.kind
	}
}

function flowToMetadata(flow: Flow): WorkspaceItemMetadata {
	return {
		type: 'flow',
		path: flow.path,
		source: 'workspace',
		summary: flow.summary,
		version: flow.version_id,
		editedAt: flow.edited_at,
		moduleCount: flow.value?.modules?.length ?? 0
	}
}

function draftToMetadata(draft: GlobalDraftItem, hasWorkspaceItem: boolean): WorkspaceItemMetadata {
	return {
		type: draft.type,
		path: draft.path,
		source: hasWorkspaceItem || draft.status === 'modified' ? 'workspace+draft' : 'draft',
		summary: getDraftSummary(draft),
		draftStatus: draft.status,
		editedAt: draft.updatedAt,
		size: getJsonSize(draft.draft)
	}
}

function scriptToRead(script: Script): WorkspaceItemRead {
	return {
		type: 'script',
		path: script.path,
		summary: script.summary,
		version: script.hash,
		value: {
			path: script.path,
			summary: script.summary,
			description: script.description,
			language: script.language,
			content: script.content,
			schema: script.schema,
			kind: script.kind,
			tag: script.tag,
			lock: script.lock,
			modules: script.modules,
			envs: script.envs
		}
	}
}

function flowToRead(flow: Flow): WorkspaceItemRead {
	return {
		type: 'flow',
		path: flow.path,
		summary: flow.summary,
		version: flow.version_id,
		editedAt: flow.edited_at,
		value: {
			path: flow.path,
			summary: flow.summary,
			description: flow.description,
			schema: flow.schema,
			value: flow.value,
			tag: flow.tag,
			priority: flow.priority,
			dedicated_worker: flow.dedicated_worker,
			timeout: flow.timeout
		}
	}
}

async function workspaceItemExists(
	type: GlobalWorkspaceItemType,
	path: string,
	workspace: string
): Promise<boolean> {
	switch (type) {
		case 'script':
			return ScriptService.existsScriptByPath({ workspace, path })
		case 'flow':
			return FlowService.existsFlowByPath({ workspace, path })
	}
}

async function readWorkspaceItem(
	type: GlobalWorkspaceItemType,
	path: string,
	workspace: string
): Promise<WorkspaceItemRead> {
	switch (type) {
		case 'script':
			return scriptToRead(await ScriptService.getScriptByPath({ workspace, path }))
		case 'flow':
			return flowToRead(await FlowService.getFlowByPath({ workspace, path }))
	}
}

async function listWorkspaceMetadata(
	types: GlobalWorkspaceItemType[],
	workspace: string,
	pathPrefix: string | undefined,
	perPage: number
): Promise<Map<string, WorkspaceItemMetadata>> {
	const items = new Map<string, WorkspaceItemMetadata>()

	if (types.includes('script')) {
		const scripts = await ScriptService.listScripts({
			workspace,
			pathStart: pathPrefix,
			perPage,
			includeDraftOnly: true,
			withoutDescription: true
		})
		for (const script of scripts) {
			items.set(getGlobalDraftKey('script', script.path), scriptToMetadata(script))
		}
	}

	if (types.includes('flow')) {
		const flows = await FlowService.listFlows({
			workspace,
			pathStart: pathPrefix,
			perPage,
			includeDraftOnly: true,
			withoutDescription: true
		})
		for (const flow of flows) {
			items.set(getGlobalDraftKey('flow', flow.path), flowToMetadata(flow))
		}
	}

	return items
}

function getScriptInstructions(language: ScriptInstructionLanguage | undefined): string {
	const selectedLanguage = language ?? DEFAULT_SCRIPT_LANGUAGE
	const languageAvailabilityNote = SYSTEM_PROMPT_SCRIPT_LANGUAGES.has(selectedLanguage)
		? `Using the shared system_prompts script guidance for \`${selectedLanguage}\`.`
		: `No dedicated system_prompts language skill exists yet for \`${selectedLanguage}\`; preserve the existing runtime conventions and use the generic Windmill script guidance below.`
	const defaultLanguageNote = language
		? ''
		: `\n- No script language was provided. Default to \`${DEFAULT_SCRIPT_LANGUAGE}\` only for new TypeScript scripts; if the user requested another language or you read an existing script, call get_instructions again with that language.`

	return `# Global draft script instructions

${languageAvailabilityNote}

- Global mode writes complete draft payloads only; it does not save, deploy, run, or generate metadata.
- Draft payloads for scripts should be NewScript-like objects with: \`path\`, \`summary\`, optional \`description\`, \`language\`, \`content\`, optional JSON Schema \`schema\`, optional \`kind\`, \`tag\`, \`envs\`, \`lock\`, and \`modules\` when needed.
- Use workspace paths such as \`f/folder/name\` or \`u/username/name\`. Preserve the current path/language/kind when modifying unless the user asked to change them.
- For normal scripts, \`kind\` should be \`script\`; use \`preprocessor\`, \`failure\`, etc. only when that is the actual script role.
- Return the full desired script draft to \`write_workspace_item\` or \`modify_workspace_item\`, not a patch or partial object.${defaultLanguageNote}

# Windmill script authoring reference (${selectedLanguage})

${getScriptPrompt(selectedLanguage)}`
}

function getFlowInstructions(): string {
	return `# Global draft flow instructions

- Global mode writes complete draft payloads only; it does not save, deploy, run, scaffold local files, or generate metadata.
- Draft payloads for flows should be Flow-like objects with: \`path\`, \`summary\`, optional \`description\`, optional input \`schema\`, and \`value\` containing the OpenFlow value.
- \`value.modules\` contains normal sequential modules. Use top-level \`value.preprocessor_module\` and \`value.failure_module\` for special modules; do not put \`preprocessor\` or \`failure\` in \`value.modules\`.
- Every module needs a stable unique \`id\` and a useful \`summary\` when the schema supports it.
- Prefer path/script/flow modules when composing existing workspace logic. Use rawscript modules only when new inline code is needed.
- When writing rawscript module code, call \`get_instructions\` with \`subject: "script"\` and the rawscript language first.
- Return the full desired flow draft to \`write_workspace_item\` or \`modify_workspace_item\`, not a patch or partial object.

# Windmill flow authoring reference

${getFlowPrompt()}`
}

function getInstructions(
	subject: GlobalWorkspaceItemType,
	language?: ScriptInstructionLanguage
): string {
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
			const parsedArgs = getInstructionsSchema.parse(args)
			const subjectLabel =
				parsedArgs.subject === 'script' && parsedArgs.language
					? `${parsedArgs.subject} (${parsedArgs.language})`
					: parsedArgs.subject
			toolCallbacks.setToolStatus(toolId, {
				content: `Loaded ${subjectLabel} instructions`
			})
			return getInstructions(parsedArgs.subject, parsedArgs.language)
		}
	},
	{
		def: createToolDef(
			listWorkspaceItemsSchema,
			'list_workspace_items',
			'List workspace scripts, flows, and AI drafts. Returns metadata only.'
		),
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsedArgs = listWorkspaceItemsSchema.parse(args)
			const types = getRequestedTypes(parsedArgs.types)
			const limit = parsedArgs.limit ?? 50
			toolCallbacks.setToolStatus(toolId, { content: 'Listing workspace items...' })

			const workspaceItems = await listWorkspaceMetadata(
				types,
				workspace,
				parsedArgs.path_prefix,
				Math.min(limit, MAX_LIST_LIMIT)
			)

			for (const draft of globalDraftStore.listDrafts()) {
				if (!types.includes(draft.type)) continue
				const key = getGlobalDraftKey(draft.type, draft.path)
				const existing = workspaceItems.get(key)
				const draftMetadata = draftToMetadata(draft, existing !== undefined)
				workspaceItems.set(key, {
					...existing,
					...draftMetadata,
					summary: draftMetadata.summary ?? existing?.summary,
					version: existing?.version ?? draft.base?.version
				})
			}

			const results = Array.from(workspaceItems.values())
				.filter((item) => itemMatches(item, parsedArgs.query, parsedArgs.path_prefix))
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
			'Read one workspace item or AI draft by type and path.'
		),
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsedArgs = readWorkspaceItemSchema.parse(args)
			const draft = globalDraftStore.getDraft(parsedArgs.type, parsedArgs.path)
			if (draft) {
				toolCallbacks.setToolStatus(toolId, {
					content: `Read AI draft ${parsedArgs.type} "${parsedArgs.path}"`
				})
				return JSON.stringify(
					{
						type: draft.type,
						path: draft.path,
						source: draft.status === 'new' ? 'draft' : 'workspace+draft',
						draftStatus: draft.status,
						baseVersion: draft.base?.version,
						baseEditedAt: draft.base?.editedAt,
						draft: draft.draft
					},
					null,
					2
				)
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Reading ${parsedArgs.type} "${parsedArgs.path}"...`
			})
			const item = await readWorkspaceItem(parsedArgs.type, parsedArgs.path, workspace)
			toolCallbacks.setToolStatus(toolId, {
				content: `Read ${parsedArgs.type} "${parsedArgs.path}"`
			})
			return JSON.stringify({ ...item, source: 'workspace' }, null, 2)
		}
	},
	{
		def: createToolDef(
			writeWorkspaceItemSchema,
			'write_workspace_item',
			'Create a new AI draft item. This does not save or deploy anything to the workspace.'
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsedArgs = writeWorkspaceItemSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, {
				content: `Creating draft ${parsedArgs.type} "${parsedArgs.path}"...`
			})

			if (await workspaceItemExists(parsedArgs.type, parsedArgs.path, workspace)) {
				const message = `${parsedArgs.type} "${parsedArgs.path}" already exists in the workspace. Use modify_workspace_item to create a draft overlay.`
				toolCallbacks.setToolStatus(toolId, { content: message, error: message })
				return JSON.stringify({ success: false, error: message })
			}

			try {
				const draft = globalDraftStore.setNewDraft(
					parsedArgs.type,
					parsedArgs.path,
					parsedArgs.draft,
					parsedArgs.overwrite ?? false
				)
				toolCallbacks.setToolStatus(toolId, {
					content: `Created AI draft ${parsedArgs.type} "${parsedArgs.path}"`,
					result: 'Draft created'
				})
				return JSON.stringify(
					{
						success: true,
						message: `Created AI draft ${parsedArgs.type} "${parsedArgs.path}". The workspace was not saved or deployed.`,
						draft
					},
					null,
					2
				)
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error)
				toolCallbacks.setToolStatus(toolId, { content: message, error: message })
				return JSON.stringify({ success: false, error: message })
			}
		}
	},
	{
		def: createToolDef(
			modifyWorkspaceItemSchema,
			'modify_workspace_item',
			'Create or update an AI draft overlay for an existing workspace item or draft. This does not save or deploy anything.'
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsedArgs = modifyWorkspaceItemSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, {
				content: `Updating draft ${parsedArgs.type} "${parsedArgs.path}"...`
			})

			const existingDraft = globalDraftStore.getDraft(parsedArgs.type, parsedArgs.path)
			if (existingDraft?.status === 'new') {
				const draft = globalDraftStore.setNewDraft(
					parsedArgs.type,
					parsedArgs.path,
					parsedArgs.draft,
					true
				)
				toolCallbacks.setToolStatus(toolId, {
					content: `Updated AI draft ${parsedArgs.type} "${parsedArgs.path}"`,
					result: 'Draft updated'
				})
				return JSON.stringify(
					{
						success: true,
						message: `Updated new AI draft ${parsedArgs.type} "${parsedArgs.path}". The workspace was not saved or deployed.`,
						draft
					},
					null,
					2
				)
			}

			let base: GlobalDraftBase | undefined = existingDraft?.base
			if (!base) {
				if (!(await workspaceItemExists(parsedArgs.type, parsedArgs.path, workspace))) {
					const message = `${parsedArgs.type} "${parsedArgs.path}" does not exist in the workspace or AI draft store. Use write_workspace_item for new items.`
					toolCallbacks.setToolStatus(toolId, { content: message, error: message })
					return JSON.stringify({ success: false, error: message })
				}
				const currentItem = await readWorkspaceItem(parsedArgs.type, parsedArgs.path, workspace)
				base = {
					source: 'workspace',
					version: currentItem.version,
					editedAt: currentItem.editedAt,
					value: currentItem.value
				}
			}

			const draft = globalDraftStore.setModifiedDraft(
				parsedArgs.type,
				parsedArgs.path,
				base,
				parsedArgs.draft
			)
			toolCallbacks.setToolStatus(toolId, {
				content: `Updated AI draft ${parsedArgs.type} "${parsedArgs.path}"`,
				result: 'Draft updated'
			})
			return JSON.stringify(
				{
					success: true,
					message: `Updated AI draft ${parsedArgs.type} "${parsedArgs.path}". The workspace was not saved or deployed.`,
					baseVersion: base.version,
					baseEditedAt: base.editedAt,
					draft
				},
				null,
				2
			)
		}
	}
]

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
