import { mkdtemp } from 'fs/promises'
import { tmpdir } from 'os'
import { join } from 'path'
import type { AIProvider, AIProviderModel, ScriptLang } from '$lib/gen/types.gen'
import type { ContextElement } from '../../../../../frontend/src/lib/components/copilot/chat/context'
import {
	prepareScriptSystemMessage,
	prepareScriptTools,
	prepareScriptUserMessage,
	type ScriptChatHelpers
} from '../../../../../frontend/src/lib/components/copilot/chat/script/core'
import { createScriptFileHelpers, type ScriptEvalState } from './fileHelpers'
import { evaluateScriptComparison } from './scriptEvalComparison'
import {
	runEval,
	resolveSystemPrompt,
	resolveTools,
	resolveModel,
	type VariantConfig,
	type BaseEvalResult,
	type EvaluationResult,
	type Tool,
	type VariantDefaults
} from '../shared'

export interface ScriptEvalResult extends BaseEvalResult<ScriptEvalState> {
	script: ScriptEvalState
}

export interface ScriptEvalOptions {
	initialScript: ScriptEvalState
	model?: string
	customSystemPrompt?: string
	maxIterations?: number
	variant?: VariantConfig
	expectedScript?: ScriptEvalState
	provider?: AIProvider
	workspaceRoot?: string
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
		const variantName = options.variant?.name ?? 'baseline'
		const model = resolveModel(options.variant, options.model)
		const modelProvider = resolveModelProvider(model, options.provider)
		const selectedContext: ContextElement[] = []
		const scriptDefaults: VariantDefaults<ScriptChatHelpers> = {
			prepareSystemMessage: (customPrompt?: string) =>
				prepareScriptSystemMessage(
					modelProvider,
					options.initialScript.lang,
					{},
					customPrompt ?? options.customSystemPrompt
				),
			tools: prepareScriptTools(
				modelProvider,
				options.initialScript.lang,
				selectedContext
			) as Tool<ScriptChatHelpers>[]
		}

		const systemMessage = resolveSystemPrompt(
			options.variant,
			scriptDefaults,
			options.customSystemPrompt
		)
		const { tools } = resolveTools(options.variant, scriptDefaults)
		const userMessage = prepareScriptUserMessage(userPrompt, selectedContext)

		const rawResult = await runEval({
			userPrompt,
			systemMessage,
			userMessage,
			tools,
			helpers,
			apiKey,
			getOutput: getScript,
			options: {
				maxIterations: options.maxIterations,
				model,
				workspace: workspaceRoot,
				provider: modelProvider.provider
			}
		})

		let evaluationResult: EvaluationResult | undefined
		if (options.expectedScript) {
			evaluationResult = await evaluateScriptComparison(
				getScript(),
				options.expectedScript,
				userPrompt
			)
		}

		return {
			...rawResult,
			variantName,
			script: rawResult.output,
			evaluationResult
		}
	} finally {
		await cleanup()
	}
}
