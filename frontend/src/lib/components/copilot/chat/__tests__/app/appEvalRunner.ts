import type { AppFiles, BackendRunnable, AppAIChatHelpers } from '../../app/core'
import { getAppTools, prepareAppSystemMessage, prepareAppUserMessage } from '../../app/core'
import { createAppEvalHelpers } from './appEvalHelpers'
import { evaluateAppGeneration, type InitialApp } from './appEvalComparison'
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
import { writeAppComparisonResultsToFolders } from './appResultsWriter'
import type { AIProvider } from '$lib/gen/types.gen'

// Re-export for convenience
export type { InitialApp } from './appEvalComparison'

/**
 * App-specific evaluation result.
 */
export interface AppEvalResult extends BaseEvalResult<AppFiles> {
	/** Alias for output to maintain API compatibility */
	files: AppFiles
}

/**
 * Options for running an app evaluation.
 */
export interface AppEvalOptions {
	initialFrontend?: Record<string, string>
	initialBackend?: Record<string, BackendRunnable>
	model?: string
	customSystemPrompt?: string
	maxIterations?: number
	variant?: VariantConfig
	/** Whether to evaluate the generated app with LLM. Default: true. Set to false to skip evaluation. */
	evaluateWithLLM?: boolean
	/** AI provider (inferred from model name if omitted) */
	provider?: AIProvider
}

/**
 * App-specific variant defaults.
 */
const appDefaults: VariantDefaults<AppAIChatHelpers> = {
	prepareSystemMessage: prepareAppSystemMessage,
	tools: getAppTools() as Tool<AppAIChatHelpers>[]
}

/**
 * Runs an app chat evaluation using the shared chat loop (same code path as production).
 */
export async function runAppEval(
	userPrompt: string,
	apiKey: string,
	options?: AppEvalOptions
): Promise<AppEvalResult> {
	const { helpers, getFiles } = createAppEvalHelpers(
		options?.initialFrontend ?? {},
		options?.initialBackend ?? {}
	)

	// Resolve variant configuration
	const variantName = options?.variant?.name ?? 'baseline'
	const systemMessage = resolveSystemPrompt(
		options?.variant,
		appDefaults,
		options?.customSystemPrompt
	)
	const { tools } = resolveTools(options?.variant, appDefaults)
	const model = resolveModel(options?.variant, options?.model)

	// Build user message
	const userMessage = prepareAppUserMessage(userPrompt, helpers.getSelectedContext())

	// Run the base evaluation
	const rawResult = await runEval({
		userPrompt,
		systemMessage,
		userMessage,
		tools,
		helpers,
		apiKey,
		getOutput: getFiles,
		options: {
			maxIterations: options?.maxIterations,
			model,
			workspace: 'test-workspace',
			provider: options?.provider
		}
	})

	// Run LLM evaluation unless explicitly disabled
	let evaluationResult: EvaluationResult | undefined
	if (options?.evaluateWithLLM !== false) {
		const generatedApp = getFiles()
		const initialApp: InitialApp | undefined =
			options?.initialFrontend || options?.initialBackend
				? {
						frontend: options.initialFrontend ?? {},
						backend: options.initialBackend ?? {}
					}
				: undefined
		evaluationResult = await evaluateAppGeneration(userPrompt, generatedApp, initialApp)
	}

	return {
		...rawResult,
		variantName,
		files: rawResult.output,
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
	baseOptions?: Omit<AppEvalOptions, 'variant'>,
	providerOverrides?: VariantProviderOverride[]
): Promise<AppEvalResult[]> {
	const results: AppEvalResult[] = await Promise.all(
		variants.map(async (variant, i) => {
			const override = providerOverrides?.[i]
			return await runAppEval(userPrompt, override?.apiKey ?? defaultApiKey, {
				...baseOptions,
				variant,
				provider: override?.provider ?? baseOptions?.provider
			})
		})
	)
	return results
}

/**
 * Writes app comparison results to a folder-based structure.
 * Each variant gets its own folder with frontend/, backend/, and details.json.
 */
export async function writeAppComparisonResults(
	userPrompt: string,
	results: AppEvalResult[],
	outputDir?: string
): Promise<{ summaryPath: string; appPaths: string[] }> {
	// @ts-ignore
	const { dirname, join } = await import('path')
	// @ts-ignore
	const { fileURLToPath } = await import('url')

	const __filename = fileURLToPath(import.meta.url)
	const __dirname = dirname(__filename)

	const resultsDir = outputDir ?? join(__dirname, 'results')

	const result = await writeAppComparisonResultsToFolders({
		userPrompt,
		results,
		outputDir: resultsDir
	})

	return { summaryPath: result.summaryPath, appPaths: result.variantPaths }
}
