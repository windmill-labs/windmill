/**
 * The whole assembled surface of a GLOBAL-mode chat — prompt sections and tools —
 * in one place, so a new section cannot be added to one rebuild path and forgotten
 * on another. Plan mode's decoration is not here: it is applied per request, later.
 */
import type { ChatCompletionSystemMessageParam } from 'openai/resources/index.mjs'
import type { SessionTool } from '../sessionCapabilities'
import {
	getSessionContextPromptSection,
	globalToolsFor,
	prepareGlobalSystemMessage,
	type SessionPromptContext
} from './core'
import { createMcpTools } from './mcpTools'
import { getPipelinePromptSection, pipelineTools } from '../pipeline/core'
import { getFolderInstructionsPromptSection, type FolderInstruction } from '../folderInstructions'
import { BROWSER_TOOLS_PROMPT, browserTools } from './browserTools'

// Derived, not retyped: an option added to prepareGlobalSystemMessage is reachable
// here at once. Hand-listing the four would compile fine while leaving the new one
// unreachable from this path, and only the eval harness — which calls the builder
// directly — would get it.
export type GlobalAssemblyOpts = NonNullable<Parameters<typeof prepareGlobalSystemMessage>[1]> & {
	sessionContext?: SessionPromptContext
	/** Folders of the open pipeline editors; the pipeline tools and prompt come with any. */
	pipelineFolders?: readonly string[]
	folderInstructions?: readonly FolderInstruction[]
}

export function assembleGlobalSystemMessage(
	instructions: Parameters<typeof prepareGlobalSystemMessage>[0],
	opts: GlobalAssemblyOpts
): ChatCompletionSystemMessageParam {
	const systemMessage = prepareGlobalSystemMessage(instructions, opts)
	systemMessage.content += getFolderInstructionsPromptSection(opts.folderInstructions ?? [])
	if (opts.sessionContext) {
		systemMessage.content += getSessionContextPromptSection(opts.sessionContext, opts.access)
	}
	if (opts.pipelineFolders?.length) {
		systemMessage.content += getPipelinePromptSection(opts.pipelineFolders, opts.access)
	}
	if (opts.browserTools) {
		systemMessage.content += BROWSER_TOOLS_PROMPT
	}
	return systemMessage
}

export function assembleGlobalTools(opts: GlobalAssemblyOpts): SessionTool<any>[] {
	const global = globalToolsFor({ sessionPreview: opts.previewTools ?? false })
	return [
		// The side panel has no page to open one in.
		...(opts.browserTools
			? [...global.filter((t) => t.def.function.name !== 'open_page'), ...browserTools]
			: global),
		...(opts.pipelineFolders?.length ? pipelineTools : []),
		...createMcpTools(opts.mcpServers ?? [])
	]
}
