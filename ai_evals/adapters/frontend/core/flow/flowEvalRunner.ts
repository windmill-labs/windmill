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
import { createFlowFileHelpers } from './fileHelpers'
import { evaluateFlowComparison, type ExpectedFlow } from './flowEvalComparison'
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

// Re-export for convenience
export type { ExpectedFlow } from './flowEvalComparison'

/**
 * Flow-specific evaluation result.
 */
export interface FlowEvalResult extends BaseEvalResult<ExtendedOpenFlow> {
	/** Alias for output to maintain API compatibility */
	flow: ExtendedOpenFlow
}

/**
 * Options for running a flow evaluation.
 */
export interface FlowEvalOptions {
	initialModules?: FlowModule[]
	initialSchema?: Record<string, any>
	model?: string
	customSystemPrompt?: string
	maxIterations?: number
	variant?: VariantConfig
	expectedFlow?: ExpectedFlow
	/** AI provider (inferred from model name if omitted) */
	provider?: AIProvider
	workspaceRoot?: string
}

/**
 * Flow-specific variant defaults.
 */
const flowDefaults: VariantDefaults<FlowAIChatHelpers> = {
	prepareSystemMessage: prepareFlowSystemMessage,
	tools: flowTools as Tool<FlowAIChatHelpers>[]
}

/**
 * Runs a flow chat evaluation using the shared chat loop (same code path as production).
 */
export async function runFlowEval(
	userPrompt: string,
	apiKey: string,
	options?: FlowEvalOptions
): Promise<FlowEvalResult> {
	const workspaceRoot =
		options?.workspaceRoot ??
		(await mkdtemp(join(tmpdir(), 'wmill-frontend-flow-benchmark-')))
	const { helpers, getFlow, cleanup } = await createFlowFileHelpers(
		options?.initialModules ?? [],
		options?.initialSchema,
		workspaceRoot
	)

	try {
		const variantName = options?.variant?.name ?? 'baseline'
		const systemMessage = resolveSystemPrompt(
			options?.variant,
			flowDefaults,
			options?.customSystemPrompt
		)
		const { tools } = resolveTools(options?.variant, flowDefaults)
		const model = resolveModel(options?.variant, options?.model)
		const userMessage = prepareFlowUserMessage(userPrompt, helpers.getFlowAndSelectedId(), [])

		const rawResult = await runEval({
			userPrompt,
			systemMessage,
			userMessage,
			tools,
			helpers,
			apiKey,
			getOutput: getFlow,
			options: {
				maxIterations: options?.maxIterations,
				model,
				workspace: workspaceRoot,
				provider: options?.provider
			}
		})

		let evaluationResult: EvaluationResult | undefined
		if (options?.expectedFlow) {
			const generatedFlow = getFlow()
			evaluationResult = await evaluateFlowComparison(
				{
					summary: generatedFlow.summary,
					value: { modules: generatedFlow.value.modules },
					schema: generatedFlow.schema
				},
				options.expectedFlow,
				userPrompt
			)
		}

		return {
			...rawResult,
			variantName,
			flow: rawResult.output,
			evaluationResult
		}
	} finally {
		await cleanup()
	}
}

/**
 * Per-variant provider override.
 */
export interface VariantProviderOverride {
	provider: AIProvider
	apiKey: string
}

/**
 * Runs the same prompt against multiple variants sequentially for comparison.
 * Accepts optional per-variant provider/apiKey overrides.
 */
export async function runVariantComparison(
	userPrompt: string,
	variants: VariantConfig[],
	defaultApiKey: string,
	baseOptions?: Omit<FlowEvalOptions, 'variant'>,
	providerOverrides?: VariantProviderOverride[]
): Promise<FlowEvalResult[]> {
	return await Promise.all(
		variants.map(async (variant, i) => {
			const override = providerOverrides?.[i]
			return await runFlowEval(userPrompt, override?.apiKey ?? defaultApiKey, {
				...baseOptions,
				variant,
				provider: override?.provider ?? baseOptions?.provider
			})
		})
	)
}
