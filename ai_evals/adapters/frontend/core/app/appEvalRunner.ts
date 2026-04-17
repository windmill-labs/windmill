import { mkdtemp } from 'fs/promises'
import { tmpdir } from 'os'
import { join } from 'path'
import type {
	BackendRunnable,
	AppAIChatHelpers
} from '../../../../../frontend/src/lib/components/copilot/chat/app/core'
import {
	getAppTools,
	prepareAppSystemMessage,
	prepareAppUserMessage
} from '../../../../../frontend/src/lib/components/copilot/chat/app/core'
import type { Tool as ProductionTool } from '../../../../../frontend/src/lib/components/copilot/chat/shared'
import { createAppFileHelpers } from './fileHelpers'
import { runEval } from '../shared'
import type { AIProvider } from '$lib/gen/types.gen'
import type { ModeRunContext } from '../../../../core/types'
import type { TokenUsage } from '../shared/types'
import type { AppFilesState } from '../../../../core/validators'

export interface AppEvalResult {
	success: boolean
	files: AppFilesState
	error?: string
	assistantMessageCount: number
	toolCallCount: number
	toolsUsed: string[]
	tokenUsage: TokenUsage
}

export interface AppEvalOptions {
	initialFrontend?: Record<string, string>
	initialBackend?: Record<string, BackendRunnable>
	initialDatatables?: AppFilesState['datatables']
	model?: string
	maxIterations?: number
	provider?: AIProvider
	workspaceRoot?: string
	runContext?: ModeRunContext
}

export async function runAppEval(
	userPrompt: string,
	apiKey: string,
	options?: AppEvalOptions
): Promise<AppEvalResult> {
	const workspaceRoot =
		options?.workspaceRoot ??
		(await mkdtemp(join(tmpdir(), 'wmill-frontend-app-benchmark-')))
	const { helpers, getEvalState, cleanup } = await createAppFileHelpers(
		options?.initialFrontend ?? {},
		options?.initialBackend ?? {},
		options?.initialDatatables ?? [],
		workspaceRoot
	)

	try {
		const systemMessage = prepareAppSystemMessage()
		const tools = getAppTools() as ProductionTool<AppAIChatHelpers>[]
		const model = options?.model ?? 'claude-haiku-4-5-20251001'
		const userMessage = prepareAppUserMessage(userPrompt, helpers.getSelectedContext())

		const rawResult = await runEval({
			userPrompt,
			systemMessage,
			userMessage,
			tools,
			helpers,
			apiKey,
			getOutput: getEvalState,
			onAssistantMessageStart: options?.runContext?.onAssistantMessageStart,
			onAssistantToken: options?.runContext?.onAssistantChunk,
			onAssistantMessageEnd: options?.runContext?.onAssistantMessageEnd,
			onToolCall: options?.runContext?.onToolCall,
			options: {
				maxIterations: options?.maxIterations,
				model,
				workspace: workspaceRoot,
				provider: options?.provider
			}
		})

		return {
			files: rawResult.output,
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
