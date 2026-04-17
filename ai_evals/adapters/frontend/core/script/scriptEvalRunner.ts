import { mkdtemp } from 'fs/promises'
import { tmpdir } from 'os'
import { join } from 'path'
import type { AIProvider, AIProviderModel } from '$lib/gen/types.gen'
import type { ContextElement } from '../../../../../frontend/src/lib/components/copilot/chat/context'
import {
	prepareScriptSystemMessage,
	prepareScriptTools,
	prepareScriptUserMessage,
	type ScriptChatHelpers
} from '../../../../../frontend/src/lib/components/copilot/chat/script/core'
import type { Tool as ProductionTool } from '../../../../../frontend/src/lib/components/copilot/chat/shared'
import { createScriptFileHelpers, type ScriptEvalState } from './fileHelpers'
import { runEval } from '../shared'
import type { ModeRunContext } from '../../../../core/types'
import type { TokenUsage } from '../shared/types'

export interface ScriptEvalResult {
	success: boolean
	script: ScriptEvalState
	error?: string
	assistantMessageCount: number
	toolCallCount: number
	toolsUsed: string[]
	tokenUsage: TokenUsage
}

export interface ScriptEvalOptions {
	initialScript: ScriptEvalState
	model?: string
	maxIterations?: number
	provider?: AIProvider
	workspaceRoot?: string
	runContext?: ModeRunContext
}

function resolveModelProvider(
	model: string,
	provider?: AIProvider
): AIProviderModel {
	if (provider) {
		return { provider, model }
	}
	if (model.startsWith('claude')) {
		return { provider: 'anthropic', model }
	}
	return { provider: 'openai', model }
}

export async function runScriptEval(
	userPrompt: string,
	apiKey: string,
	options: ScriptEvalOptions
): Promise<ScriptEvalResult> {
	const workspaceRoot =
		options.workspaceRoot ?? (await mkdtemp(join(tmpdir(), 'wmill-frontend-script-benchmark-')))
	const { helpers, getScript, cleanup } = await createScriptFileHelpers(
		options.initialScript,
		workspaceRoot
	)

	try {
		const model = options.model ?? 'claude-haiku-4-5-20251001'
		const modelProvider = resolveModelProvider(model, options.provider)
		const selectedContext: ContextElement[] = []
		const systemMessage = prepareScriptSystemMessage(
			modelProvider,
			options.initialScript.lang,
			{}
		)
		const tools = prepareScriptTools(
			modelProvider,
			options.initialScript.lang,
			selectedContext
		) as ProductionTool<ScriptChatHelpers>[]
		const userMessage = prepareScriptUserMessage(userPrompt, selectedContext)

		const rawResult = await runEval({
			userPrompt,
			systemMessage,
			userMessage,
			tools,
			helpers,
			apiKey,
			getOutput: getScript,
			onAssistantMessageStart: options.runContext?.onAssistantMessageStart,
			onAssistantToken: options.runContext?.onAssistantChunk,
			onAssistantMessageEnd: options.runContext?.onAssistantMessageEnd,
			onToolCall: options.runContext?.onToolCall,
			options: {
				maxIterations: options.maxIterations,
				model,
				workspace: workspaceRoot,
				provider: modelProvider.provider
			}
		})

		return {
			script: rawResult.output,
			success: rawResult.success,
			error: rawResult.error,
			assistantMessageCount: rawResult.iterations,
			toolCallCount: rawResult.toolCallsCount,
			toolsUsed: rawResult.toolsCalled,
			tokenUsage: rawResult.tokenUsage
		}
	} finally {
		await cleanup()
	}
}
