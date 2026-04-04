import { mkdtemp } from 'fs/promises'
import { tmpdir } from 'os'
import { join } from 'path'
import type {
	AppFiles,
	BackendRunnable,
	AppAIChatHelpers
} from '../../../../../frontend/src/lib/components/copilot/chat/app/core'
import {
	getAppTools,
	prepareAppSystemMessage,
	prepareAppUserMessage
} from '../../../../../frontend/src/lib/components/copilot/chat/app/core'
import { createAppFileHelpers } from './fileHelpers'
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
	workspaceRoot?: string
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
	const workspaceRoot =
		options?.workspaceRoot ??
		(await mkdtemp(join(tmpdir(), 'wmill-frontend-app-benchmark-')))
	const { helpers, getFiles, cleanup } = await createAppFileHelpers(
		options?.initialFrontend ?? {},
		options?.initialBackend ?? {},
		workspaceRoot
	)

	try {
		const variantName = options?.variant?.name ?? 'baseline'
		const systemMessage = resolveSystemPrompt(
			options?.variant,
			appDefaults,
			options?.customSystemPrompt
		)
		const { tools } = resolveTools(options?.variant, appDefaults)
		const model = resolveModel(options?.variant, options?.model)
		const userMessage = prepareAppUserMessage(userPrompt, helpers.getSelectedContext())

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
				workspace: workspaceRoot,
				provider: options?.provider
			}
		})

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
	baseOptions?: Omit<AppEvalOptions, 'variant'>,
	providerOverrides?: VariantProviderOverride[]
): Promise<AppEvalResult[]> {
	return await Promise.all(
		variants.map(async (variant, i) => {
			const override = providerOverrides?.[i]
			return await runAppEval(userPrompt, override?.apiKey ?? defaultApiKey, {
				...baseOptions,
				variant,
				provider: override?.provider ?? baseOptions?.provider
			})
		})
	)
}
