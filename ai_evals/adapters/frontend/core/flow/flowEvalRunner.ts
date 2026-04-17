import { mkdtemp } from 'fs/promises'
import { tmpdir } from 'os'
import { join } from 'path'
import type { FlowModule } from '$lib/gen'
import type { AIProvider } from '$lib/gen/types.gen'
import type { ExtendedOpenFlow } from '$lib/components/flows/types'
import {
	flowTools,
	prepareFlowSystemMessage,
	prepareFlowUserMessage,
	type FlowAIChatHelpers
} from '../../../../../frontend/src/lib/components/copilot/chat/flow/core'
import type { Tool as ProductionTool } from '../../../../../frontend/src/lib/components/copilot/chat/shared'
import { createFlowFileHelpers, type FlowWorkspaceFixtures } from './fileHelpers'
import { runEval } from '../shared'
import type { ModeRunContext } from '../../../../core/types'
import type { TokenUsage } from '../shared/types'

export interface FlowFixture {
	value?: {
		modules?: FlowModule[]
		preprocessor_module?: FlowModule
		failure_module?: FlowModule
	}
	schema?: Record<string, unknown>
}

export interface FlowEvalResult {
	success: boolean
	flow: ExtendedOpenFlow
	error?: string
	assistantMessageCount: number
	toolCallCount: number
	toolsUsed: string[]
	tokenUsage: TokenUsage
}

export interface FlowEvalOptions {
	initialFlow?: FlowFixture
	workspaceFixtures?: FlowWorkspaceFixtures
	model?: string
	maxIterations?: number
	provider?: AIProvider
	workspaceRoot?: string
	runContext?: ModeRunContext
}

export async function runFlowEval(
	userPrompt: string,
	apiKey: string,
	options?: FlowEvalOptions
): Promise<FlowEvalResult> {
	const workspaceRoot =
		options?.workspaceRoot ??
		(await mkdtemp(join(tmpdir(), 'wmill-frontend-flow-benchmark-')))
	const { helpers, getFlow, cleanup } = await createFlowFileHelpers(
		options?.initialFlow?.value?.modules ?? [],
		options?.initialFlow?.schema,
		options?.initialFlow?.value?.preprocessor_module,
		options?.initialFlow?.value?.failure_module,
		workspaceRoot,
		options?.workspaceFixtures
	)

	try {
		const systemMessage = prepareFlowSystemMessage()
		const tools = flowTools as ProductionTool<FlowAIChatHelpers>[]
		const model = options?.model ?? 'claude-haiku-4-5-20251001'
		const userMessage = prepareFlowUserMessage(
			userPrompt,
			helpers.getFlowAndSelectedId(),
			[],
			helpers.inlineScriptSession
		)

		const rawResult = await runEval({
			userPrompt,
			systemMessage,
			userMessage,
			tools,
			helpers,
			apiKey,
			getOutput: getFlow,
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
			flow: rawResult.output,
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
