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
	type SessionPromptContext
} from './core'
import { createMcpTools } from './mcpTools'
import { getPipelinePromptSection, pipelineTools, type PipelineContext } from '../pipeline/core'

// Derived, not retyped: an option added to prepareGlobalSystemMessage is reachable
// here at once. Hand-listing the four would compile fine while leaving the new one
// unreachable from this path, and only the eval harness — which calls the builder
// directly — would get it.
export type GlobalAssemblyOpts = NonNullable<Parameters<typeof prepareGlobalSystemMessage>[1]> & {
	sessionContext?: SessionPromptContext
	pipelineContext?: PipelineContext
}

/** The sections gate on the same capabilities the tools do, so a tool withheld from
 * this session is never described to the model by the prompt that ships beside it. */

export function assembleGlobalSystemMessage(
	instructions: Parameters<typeof prepareGlobalSystemMessage>[0],
	opts: GlobalAssemblyOpts
): ChatCompletionSystemMessageParam {
	const systemMessage = prepareGlobalSystemMessage(instructions, opts)
	if (opts.sessionContext) {
		systemMessage.content += getSessionContextPromptSection(opts.sessionContext, opts.access)
	}
	if (opts.pipelineContext) {
		systemMessage.content += getPipelinePromptSection(opts.pipelineContext, opts.access)
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
