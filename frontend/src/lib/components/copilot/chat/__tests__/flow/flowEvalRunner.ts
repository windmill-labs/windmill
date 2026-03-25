import type { FlowModule } from '$lib/gen'
import type { AIProvider } from '$lib/gen/types.gen'
import type { ExtendedOpenFlow } from '$lib/components/flows/types'
import { flowTools, prepareFlowSystemMessage, prepareFlowUserMessage, type FlowAIChatHelpers } from '../../flow/core'
import { createFlowEvalHelpers } from './flowEvalHelpers'
import { evaluateFlowComparison, type ExpectedFlow } from './flowEvalComparison'
import {
	runEval,
	resolveSystemPrompt,
	resolveTools,
	resolveModel,
	writeComparisonResults,
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
	const { helpers, getFlow } = createFlowEvalHelpers(
		options?.initialModules ?? [],
		options?.initialSchema
	)

	// Resolve variant configuration
	const variantName = options?.variant?.name ?? 'baseline'
	const systemMessage = resolveSystemPrompt(options?.variant, flowDefaults, options?.customSystemPrompt)
	const { tools } = resolveTools(options?.variant, flowDefaults)
	const model = resolveModel(options?.variant, options?.model)

	// Build user message
	const userMessage = prepareFlowUserMessage(userPrompt, helpers.getFlowAndSelectedId(), [])

	// Run the base evaluation
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
			workspace: 'test-workspace',
			provider: options?.provider
		}
	})

	// Run evaluation if expected flow is provided
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
	const results: FlowEvalResult[] = await Promise.all(
		variants.map(async (variant, i) => {
			const override = providerOverrides?.[i]
			return await runFlowEval(userPrompt, override?.apiKey ?? defaultApiKey, {
				...baseOptions,
				variant,
				provider: override?.provider ?? baseOptions?.provider
			})
		})
	)
	return results
}

/**
 * Writes flow comparison results to files.
 */
export async function writeFlowComparisonResults(
	userPrompt: string,
	results: FlowEvalResult[],
	outputDir?: string
): Promise<{ summaryPath: string; flowPaths: string[] }> {
	// @ts-ignore
	const { dirname, join } = await import('path')
	// @ts-ignore
	const { fileURLToPath } = await import('url')

	const __filename = fileURLToPath(import.meta.url)
	const __dirname = dirname(__filename)

	const resultsDir = outputDir ?? join(__dirname, 'results')

	const result = await writeComparisonResults({
		userPrompt,
		results,
		outputDir: resultsDir,
		formatOutput: (flow: ExtendedOpenFlow) => ({
			summary: flow.summary ?? '',
			value: {
				modules: flow.value.modules
			},
			schema: flow.schema ?? {
				$schema: 'https://json-schema.org/draft/2020-12/schema',
				properties: {},
				required: [],
				type: 'object'
			}
		}),
		outputLabel: 'flow'
	})

	return { summaryPath: result.summaryPath, flowPaths: result.outputPaths }
}
