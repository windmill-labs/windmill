import { $ScriptLang, FlowService, ScriptService } from '$lib/gen'
import { getFlowPrompt, getScriptPrompt } from '$system_prompts'
import type { Flow, Script, ScriptLang } from '$lib/gen/types.gen'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import { z } from 'zod'
import { createToolDef, type Tool } from '../shared'
import {
	globalDraftStore,
	getGlobalDraftKey,
	type GlobalWorkspaceItem,
	type GlobalWorkspaceItemType
} from './draftStore.svelte'

const ITEM_TYPES = ['script', 'flow'] as const satisfies readonly GlobalWorkspaceItemType[]
const MAX_LIST_LIMIT = 100
const DEFAULT_SCRIPT_LANGUAGE: ScriptLang = 'bun'

const itemTypeSchema = z.enum(ITEM_TYPES)
const scriptLanguageSchema = z.enum($ScriptLang.enum)

const getInstructionsSchema = z.object({
	subject: itemTypeSchema.describe('The workspace item type to get authoring instructions for.'),
	language: scriptLanguageSchema
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

const workspaceItemDraftBaseSchema = z.object({
	type: itemTypeSchema,
	path: z.string().describe('Workspace path of the item.'),
	summary: z.string().optional().describe('Short summary of the item.'),
	value: z
		.any()
		.describe('For scripts, the complete script source code. For flows, the complete OpenFlow value.'),
	language: scriptLanguageSchema
		.optional()
		.describe('Required for script items. Omit for flow items.')
})

type WorkspaceItemDraftInput = z.infer<typeof workspaceItemDraftBaseSchema>

function validateWorkspaceItemDraft(
	item: WorkspaceItemDraftInput,
	ctx: z.RefinementCtx
): void {
	if (item.type === 'script' && !item.language) {
		ctx.addIssue({
			code: 'custom',
			path: ['language'],
			message: 'language is required for script items'
		})
	}
}

const writeWorkspaceItemSchema = workspaceItemDraftBaseSchema
	.extend({
		overwrite: z
			.boolean()
			.optional()
			.describe('If true, replace an existing AI draft with the same type and path.')
	})
	.superRefine(validateWorkspaceItemDraft)

const modifyWorkspaceItemSchema = workspaceItemDraftBaseSchema.superRefine(validateWorkspaceItemDraft)

type WorkspaceItemMetadata = Omit<GlobalWorkspaceItem, 'value'>

const GLOBAL_SYSTEM_PROMPT = `You are Windmill's global workspace assistant.

You can inspect workspace scripts and flows, then create draft changes in the frontend AI draft store.

Important rules:
- Writes and modifications are drafts only. They do not save, deploy, or mutate backend workspace items.
- A workspace item is { type, path, summary, value, language, isDraft }.
- For scripts, value is the complete script source code and language is required.
- For flows, value is the complete OpenFlow value and language is omitted.
- list_workspace_items returns metadata only and omits value. Use read_workspace_item for the current value.
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

function scriptToWorkspaceItem(script: Script): GlobalWorkspaceItem {
	return {
		type: 'script',
		path: script.path,
		summary: script.summary,
		value: script.content,
		language: script.language,
		isDraft: false
	}
}

function flowToWorkspaceItem(flow: Flow): GlobalWorkspaceItem {
	return {
		type: 'flow',
		path: flow.path,
		summary: flow.summary,
		value: flow.value,
		isDraft: false
	}
}

function toMetadata(item: GlobalWorkspaceItem): WorkspaceItemMetadata {
	return {
		type: item.type,
		path: item.path,
		summary: item.summary,
		language: item.language,
		isDraft: item.isDraft
	}
}

function toDraftInput(item: WorkspaceItemDraftInput): Omit<GlobalWorkspaceItem, 'isDraft'> {
	return {
		type: item.type,
		path: item.path,
		summary: item.summary,
		value: item.value,
		language: item.type === 'script' ? item.language : undefined
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
): Promise<GlobalWorkspaceItem> {
	switch (type) {
		case 'script':
			return scriptToWorkspaceItem(await ScriptService.getScriptByPath({ workspace, path }))
		case 'flow':
			return flowToWorkspaceItem(await FlowService.getFlowByPath({ workspace, path }))
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
			items.set(getGlobalDraftKey('script', script.path), toMetadata(scriptToWorkspaceItem(script)))
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
			items.set(getGlobalDraftKey('flow', flow.path), toMetadata(flowToWorkspaceItem(flow)))
		}
	}

	return items
}

function getScriptInstructions(language: ScriptLang | undefined): string {
	const selectedLanguage = language ?? DEFAULT_SCRIPT_LANGUAGE
	const defaultLanguageNote = language
		? ''
		: `\n- No script language was provided. Default to \`${DEFAULT_SCRIPT_LANGUAGE}\` only for new TypeScript scripts; if the user requested another language or you read an existing script, call get_instructions again with that language.`

	return `# Global draft script instructions

- Global mode writes complete draft payloads only; it does not save, deploy, run, or generate metadata.
- Draft payloads for scripts are workspace items: \`{ type: "script", path, summary, value, language }\`.
- For scripts, \`value\` is the complete script source code and \`language\` is required.
- Use workspace paths such as \`f/folder/name\` or \`u/username/name\`. Preserve the current path/language when modifying unless the user asked to change them.
- Return the full desired script draft to \`write_workspace_item\` or \`modify_workspace_item\`, not a patch or partial object.${defaultLanguageNote}

# Windmill script authoring reference (${selectedLanguage})

${getScriptPrompt(selectedLanguage)}`
}

function getFlowInstructions(): string {
	return `# Global draft flow instructions

- Global mode writes complete draft payloads only; it does not save, deploy, run, scaffold local files, or generate metadata.
- Draft payloads for flows are workspace items: \`{ type: "flow", path, summary, value }\`.
- For flows, \`value\` is the complete OpenFlow value. Omit \`language\`.
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
	language?: ScriptLang
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
				const draftMetadata = toMetadata(draft)
				workspaceItems.set(key, {
					...draftMetadata,
					summary: draftMetadata.summary ?? existing?.summary
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
				return JSON.stringify(draft, null, 2)
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Reading ${parsedArgs.type} "${parsedArgs.path}"...`
			})
			const item = await readWorkspaceItem(parsedArgs.type, parsedArgs.path, workspace)
			toolCallbacks.setToolStatus(toolId, {
				content: `Read ${parsedArgs.type} "${parsedArgs.path}"`
			})
			return JSON.stringify(item, null, 2)
		}
	},
	{
		def: createToolDef(
			writeWorkspaceItemSchema,
			'write_workspace_item',
			'Create a new AI draft workspace item. This does not save or deploy anything to the workspace.'
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsedArgs = writeWorkspaceItemSchema.parse(args)
			const draftInput = toDraftInput(parsedArgs)
			toolCallbacks.setToolStatus(toolId, {
				content: `Creating draft ${draftInput.type} "${draftInput.path}"...`
			})

			if (await workspaceItemExists(draftInput.type, draftInput.path, workspace)) {
				const message = `${draftInput.type} "${draftInput.path}" already exists in the workspace. Use modify_workspace_item to create a draft overlay.`
				toolCallbacks.setToolStatus(toolId, { content: message, error: message })
				return JSON.stringify({ success: false, error: message })
			}

			try {
				const draft = globalDraftStore.setNewDraft(draftInput, parsedArgs.overwrite ?? false)
				toolCallbacks.setToolStatus(toolId, {
					content: `Created AI draft ${draft.type} "${draft.path}"`,
					result: 'Draft created'
				})
				return JSON.stringify(
					{
						success: true,
						message: `Created AI draft ${draft.type} "${draft.path}". The workspace was not saved or deployed.`,
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
			const draftInput = toDraftInput(parsedArgs)
			toolCallbacks.setToolStatus(toolId, {
				content: `Updating draft ${draftInput.type} "${draftInput.path}"...`
			})

			const existingDraft = globalDraftStore.getDraft(draftInput.type, draftInput.path)
			if (!existingDraft && !(await workspaceItemExists(draftInput.type, draftInput.path, workspace))) {
				const message = `${draftInput.type} "${draftInput.path}" does not exist in the workspace or AI draft store. Use write_workspace_item for new items.`
				toolCallbacks.setToolStatus(toolId, { content: message, error: message })
				return JSON.stringify({ success: false, error: message })
			}

			const draft = globalDraftStore.setModifiedDraft(draftInput)
			toolCallbacks.setToolStatus(toolId, {
				content: `Updated AI draft ${draft.type} "${draft.path}"`,
				result: 'Draft updated'
			})
			return JSON.stringify(
				{
					success: true,
					message: `Updated AI draft ${draft.type} "${draft.path}". The workspace was not saved or deployed.`,
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
