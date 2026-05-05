import { FlowService, ScriptService } from '$lib/gen'
import { $ScriptLang } from '$lib/gen/schemas.gen'
import type { Flow, Script, ScriptLang } from '$lib/gen/types.gen'
import { getFlowPrompt, getScriptPrompt } from '$system_prompts'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import { z } from 'zod'
import { createToolDef, type Tool } from '../shared'
import {
	globalDraftStore,
	type WorkspaceItem,
	type WorkspaceItemType
} from './draftStore.svelte'

const ITEM_TYPES = ['script', 'flow'] as const satisfies readonly WorkspaceItemType[]
const MAX_LIST_LIMIT = 100

const itemTypeSchema = z.enum(ITEM_TYPES)
const scriptLangSchema = z.enum($ScriptLang.enum)

const getInstructionsSchema = z.object({
	subject: itemTypeSchema.describe('The workspace item type to get authoring instructions for.'),
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
	path: z.string().describe('Workspace path of the item.'),
	summary: z.string().optional().describe('Short human-readable summary of the item.'),
	language: scriptLangSchema
		.optional()
		.describe('Script language. Required for scripts, omitted for flows.'),
	value: z
		.any()
		.describe(
			'Script source code (string) when type is script, or the OpenFlow value object when type is flow.'
		)
})

const GLOBAL_SYSTEM_PROMPT = `You are Windmill's global workspace assistant.

You can inspect workspace scripts and flows, then create draft changes in the frontend AI draft store.

Important rules:
- write_workspace_item creates a draft only. It does not save, deploy, or mutate workspace items.
- Use list_workspace_items before broad reads.
- Use read_workspace_item before overwriting an existing item with write_workspace_item, unless the user already provided the complete current item.
- Use get_instructions before writing a script or flow. For scripts, pass the target language; when modifying, use the language from the item you read.
- A workspace item is { type, path, summary?, language?, value, isDraft }. For scripts, value is the source code string and language is required. For flows, value is the OpenFlow value object.
- Keep context targeted. Do not read unrelated items.
- Be explicit with the user when you create or update a draft.`

function getRequestedTypes(types: WorkspaceItemType[] | undefined): WorkspaceItemType[] {
	return types && types.length > 0 ? types : [...ITEM_TYPES]
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

async function workspaceItemExists(
	type: WorkspaceItemType,
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
	type: WorkspaceItemType,
	path: string,
	workspace: string
): Promise<WorkspaceItem> {
	switch (type) {
		case 'script':
			return scriptToItem(await ScriptService.getScriptByPath({ workspace, path }), true)
		case 'flow':
			return flowToItem(await FlowService.getFlowByPath({ workspace, path }), true)
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
- Return the full desired source as \`value\` to \`write_workspace_item\`, not a patch.${note}

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
- Return the full desired OpenFlow value as \`value\` to \`write_workspace_item\`, not a patch.

# Windmill flow authoring reference

${getFlowPrompt()}`
}

function getInstructions(subject: WorkspaceItemType, language?: ScriptLang): string {
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
			'List workspace scripts, flows, and AI drafts. Returns metadata only (no value).'
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
				byKey.set(`${item.type}:${item.path}`, item)
			}

			for (const draft of globalDraftStore.listDrafts()) {
				if (!types.includes(draft.type)) continue
				byKey.set(`${draft.type}:${draft.path}`, { ...draft, value: undefined })
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
			const draft = globalDraftStore.getDraft(parsed.type, parsed.path)
			if (draft) {
				toolCallbacks.setToolStatus(toolId, {
					content: `Read AI draft ${parsed.type} "${parsed.path}"`
				})
				return JSON.stringify(draft, null, 2)
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Reading ${parsed.type} "${parsed.path}"...`
			})
			const item = await readWorkspaceItem(parsed.type, parsed.path, workspace)
			toolCallbacks.setToolStatus(toolId, { content: `Read ${parsed.type} "${parsed.path}"` })
			return JSON.stringify(item, null, 2)
		}
	},
	{
		def: createToolDef(
			writeWorkspaceItemSchema,
			'write_workspace_item',
			'Create or overwrite an AI draft workspace item. Does not save or deploy to the workspace. Read the existing item first when overwriting.'
		),
		showDetails: true,
		streamArguments: true,
		showFade: true,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsed = writeWorkspaceItemSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, {
				content: `Writing draft ${parsed.type} "${parsed.path}"...`
			})

			if (parsed.type === 'script' && !parsed.language) {
				const message = `language is required when type is script.`
				toolCallbacks.setToolStatus(toolId, { content: message, error: message })
				return JSON.stringify({ success: false, error: message })
			}

			const exists =
				globalDraftStore.getDraft(parsed.type, parsed.path) !== undefined ||
				(await workspaceItemExists(parsed.type, parsed.path, workspace))

			const item = globalDraftStore.setDraft({
				type: parsed.type,
				path: parsed.path,
				summary: parsed.summary,
				language: parsed.language,
				value: parsed.value,
				isDraft: true
			})

			const verb = exists ? 'Updated' : 'Created'
			toolCallbacks.setToolStatus(toolId, {
				content: `${verb} AI draft ${parsed.type} "${parsed.path}"`,
				result: `Draft ${verb.toLowerCase()}`
			})
			return JSON.stringify(
				{
					success: true,
					message: `${verb} AI draft ${parsed.type} "${parsed.path}". The workspace was not saved or deployed.`,
					item
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
