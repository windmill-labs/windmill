import type { Tool } from '../shared'
import type { PipelineContext } from '../pipeline/core'
import { getPipelinePromptSection, pipelineTools } from '../pipeline/core'
import { createMcpTools, type McpServer } from './mcpTools'
import {
	getSessionContextPromptSection,
	globalToolsFor,
	prepareGlobalSystemMessage,
	type AiSkillListItem,
	type SessionPromptContext
} from './core'
import type { SessionAccess } from './sessionAccess'

/**
 * Assembly of what a GLOBAL-mode chat ships: the system prompt's sections, and the
 * tool sources they document. Both live here so production and the capability tests
 * go through the same code — a section or a tool source added to one cannot be
 * missing from the other, which is how a prompt once kept naming tools the filter
 * had already withheld.
 */

export type GlobalAssemblyOptions = {
	previewTools?: boolean
	user?: { username: string; is_admin?: boolean; folders?: string[]; folders_read?: string[] }
	skills?: AiSkillListItem[]
	mcpServers?: McpServer[]
	access?: SessionAccess
	/** Appended when the chat is a session; carries the operating/parent workspace. */
	sessionContext?: SessionPromptContext
	/** Appended while a /pipeline editor has registered its helpers. */
	pipelineContext?: PipelineContext
}

/** The system message as sent, minus plan mode's decoration — that is applied later,
 * per request, by `planModeController`. */
export function assembleGlobalSystemMessage(
	instructions: { workspace?: string; user?: string } | undefined,
	opts: GlobalAssemblyOptions
): ReturnType<typeof prepareGlobalSystemMessage> {
	const message = prepareGlobalSystemMessage(instructions, opts)
	let content = typeof message.content === 'string' ? message.content : ''
	if (opts.sessionContext) {
		content += getSessionContextPromptSection(opts.sessionContext, opts.access)
	}
	if (opts.pipelineContext) {
		content += getPipelinePromptSection(opts.pipelineContext, opts.access)
	}
	return { ...message, content }
}

/** Every tool source a GLOBAL chat assembles, unfiltered. Plan mode's two tools are
 * not here: they are merged at request time, next to the capability filter itself. */
export function assembleGlobalTools(opts: {
	sessionPreview: boolean
	pipeline: boolean
	mcpServers?: McpServer[]
}): Tool<any>[] {
	return [
		...globalToolsFor({ sessionPreview: opts.sessionPreview }),
		...(opts.pipeline ? pipelineTools : []),
		...createMcpTools(opts.mcpServers ?? [])
	]
}
