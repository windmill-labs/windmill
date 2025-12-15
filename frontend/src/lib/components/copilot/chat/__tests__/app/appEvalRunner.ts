import type { AppFiles, BackendRunnable, AppAIChatHelpers } from '../../app/core'
import { getAppTools, prepareAppSystemMessage, prepareAppUserMessage } from '../../app/core'
import { createAppEvalHelpers } from './appEvalHelpers'
import { evaluateAppComparison, type ExpectedApp } from './appEvalComparison'
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
export type { ExpectedApp } from './appEvalComparison'

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
	expectedApp?: ExpectedApp
}

/**
 * App-specific variant defaults.
 */
const appDefaults: VariantDefaults<AppAIChatHelpers> = {
	prepareSystemMessage: prepareAppSystemMessage,
	tools: getAppTools() as Tool<AppAIChatHelpers>[]
}

/**
 * Runs an app chat evaluation with real OpenAI API calls.
 * Executes tool calls using the actual app tools from core.ts or variant-configured tools.
 */
export async function runAppEval(
	userPrompt: string,
	openaiApiKey: string,
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
	const { toolDefs, tools } = resolveTools(options?.variant, appDefaults)
	const model = resolveModel(options?.variant, options?.model)

	// Build user message
	const userMessage = prepareAppUserMessage(userPrompt, helpers.getSelectedContext())

	// Run the base evaluation
	const rawResult = await runEval({
		userPrompt,
		systemMessage,
		userMessage,
		toolDefs,
		tools,
		helpers,
		apiKey: openaiApiKey,
		getOutput: getFiles,
		options: {
			maxIterations: options?.maxIterations,
			model,
			workspace: 'test-workspace'
		}
	})

	// Run evaluation if expected app is provided
	let evaluationResult: EvaluationResult | undefined
	if (options?.expectedApp) {
		const generatedApp = getFiles()
		evaluationResult = await evaluateAppComparison(generatedApp, options.expectedApp, userPrompt)
	}

	return {
		...rawResult,
		variantName,
		files: rawResult.output,
		evaluationResult
	}
}

/**
 * Runs the same prompt against multiple variants sequentially for comparison.
 * Returns results in the same order as the input variants.
 */
export async function runVariantComparison(
	userPrompt: string,
	variants: VariantConfig[],
	openaiApiKey: string,
	baseOptions?: Omit<AppEvalOptions, 'variant'>
): Promise<AppEvalResult[]> {
	const results: AppEvalResult[] = await Promise.all(
		variants.map(async (variant) => {
			return await runAppEval(userPrompt, openaiApiKey, {
				...baseOptions,
				variant
			})
		})
	)
	return results
}

/**
 * Writes app comparison results to files.
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

	const result = await writeComparisonResults({
		userPrompt,
		results,
		outputDir: resultsDir,
		formatOutput: (files: AppFiles) => files,
		outputLabel: 'app'
	})

	return { summaryPath: result.summaryPath, appPaths: result.outputPaths }
}
