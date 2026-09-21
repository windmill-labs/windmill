/**
 * The whole assembled surface of a GLOBAL-mode chat — prompt sections and tools —
 * in one place, so a new section cannot be added to one rebuild path and forgotten
 * on another. Plan mode's decoration is not here: it is applied per request, later.
 */
import type { ChatCompletionSystemMessageParam } from 'openai/resources/index.mjs'
import type { Tool } from '../shared'
import {
	getSessionContextPromptSection,
	globalToolsFor,
	prepareGlobalSystemMessage,
	type AiSkillListItem,
	type GlobalPromptIdentity,
	type SessionPromptContext
} from './core'
import { createMcpTools, type McpServer } from './mcpTools'
import { getPipelinePromptSection, pipelineTools, type PipelineContext } from '../pipeline/core'

export type GlobalAssemblyOpts = {
	previewTools?: boolean
	user?: GlobalPromptIdentity
	skills?: AiSkillListItem[]
	mcpServers?: McpServer[]
	sessionContext?: SessionPromptContext
	pipelineContext?: PipelineContext
}

export function assembleGlobalSystemMessage(
	instructions: { workspace?: string; user?: string } | undefined,
	opts: GlobalAssemblyOpts
): ChatCompletionSystemMessageParam {
	// Forwarded whole, not field by field. Adding an option to GlobalAssemblyOpts is
	// checked — the compiler rejects one its callers set but the type lacks. Re-listing
	// the fields in this call is not: forget one and it is silently dropped.
	const systemMessage = prepareGlobalSystemMessage(instructions, opts)
	if (opts.sessionContext) {
		systemMessage.content += getSessionContextPromptSection(opts.sessionContext)
	}
	if (opts.pipelineContext) {
		systemMessage.content += getPipelinePromptSection(opts.pipelineContext)
	}
	return systemMessage
}

export function assembleGlobalTools(opts: GlobalAssemblyOpts): Tool<any>[] {
	return [
		...globalToolsFor({ sessionPreview: opts.previewTools ?? false }),
		...(opts.pipelineContext ? pipelineTools : []),
		...createMcpTools(opts.mcpServers ?? [])
	]
}
