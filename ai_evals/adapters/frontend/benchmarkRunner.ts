import type { AIProvider } from '$lib/gen/types.gen'
import { loadAppEvalCases, loadFlowEvalCases, loadScriptEvalCases } from './core/evalCaseLoader'
import type { VariantConfig } from './core/shared'
import {
	allRequiredChecksPassed,
	buildJudgeChecks,
	getRequiredFailedChecks,
	requiredCheck,
	validateAppArtifact,
	validateFlowArtifact,
	validateScriptArtifact,
	type BenchmarkCheck
} from '../shared/validators'

export type FrontendBenchmarkSurface = 'flow' | 'app' | 'script'

export interface FrontendBenchmarkConfig {
	provider?: AIProvider
	model?: string
	systemPrompt?: {
		mode: 'append' | 'replace'
		content: string
	}
}

export interface FrontendBenchmarkAttempt {
	attempt: number
	passed: boolean
	durationMs: number
	assistantMessageCount: number
	toolCallCount: number
	toolsUsed: string[]
	checks: BenchmarkCheck[]
	requiredFailedChecks: string[]
	judgeScore: number | null
	judgeStatement: string | null
	error: string | null
}

export interface FrontendBenchmarkCaseResult {
	caseId: string
	attempts: FrontendBenchmarkAttempt[]
}

export interface FrontendBenchmarkPayload {
	surface: FrontendBenchmarkSurface
	runs: number
	provider: AIProvider
	model: string
	judgeModel: string | null
	caseResults: FrontendBenchmarkCaseResult[]
}

const DEFAULT_MIN_JUDGE_SCORE = 80
const DEFAULT_PROVIDER: AIProvider = 'anthropic'
const DEFAULT_MODEL = 'claude-haiku-4-5-20251001'
const FRONTEND_JUDGE_MODEL = 'claude-sonnet-4-6'

export async function runFrontendBenchmarkFromEnv(): Promise<FrontendBenchmarkPayload> {
	return runFrontendBenchmark({
		surface: parseSurface(process.env.WMILL_FRONTEND_AI_EVAL_SURFACE),
		caseIds: parseOptionalJsonStringArray(process.env.WMILL_FRONTEND_AI_EVAL_CASE_IDS),
		runs: parsePositiveInteger(process.env.WMILL_FRONTEND_AI_EVAL_RUNS, 'WMILL_FRONTEND_AI_EVAL_RUNS'),
		config: parseConfig(process.env.WMILL_FRONTEND_AI_EVAL_CONFIG)
	})
}

export async function runFrontendBenchmark(input: {
	surface: FrontendBenchmarkSurface
	caseIds: string[]
	runs: number
	config?: FrontendBenchmarkConfig
}): Promise<FrontendBenchmarkPayload> {
	switch (input.surface) {
		case 'flow':
			return await runFlowBenchmark({
				surface: 'flow',
				caseIds: input.caseIds,
				runs: input.runs,
				config: input.config
			})
		case 'app':
			return await runAppBenchmark({
				surface: 'app',
				caseIds: input.caseIds,
				runs: input.runs,
				config: input.config
			})
		case 'script':
			return await runScriptBenchmark({
				surface: 'script',
				caseIds: input.caseIds,
				runs: input.runs,
				config: input.config
			})
		default:
			throw new Error(`Unsupported frontend benchmark surface: ${String(input.surface)}`)
	}
}

async function runFlowBenchmark(input: {
	surface: 'flow'
	caseIds: string[]
	runs: number
	config?: FrontendBenchmarkConfig
}): Promise<FrontendBenchmarkPayload> {
	const { runFlowEval } = await import('./core/flow/flowEvalRunner')
	const allCases = loadFlowEvalCases()
	const selectedCases = resolveCases(allCases, input.caseIds, 'frontend flow')
	const resolvedConfig = resolveConfig(input.config)

	return {
		surface: input.surface,
		runs: input.runs,
		provider: resolvedConfig.provider,
		model: resolvedConfig.model,
		judgeModel: FRONTEND_JUDGE_MODEL,
		caseResults: await Promise.all(
			selectedCases.map(async (testCase) => ({
				caseId: testCase.id,
				attempts: await runRepeated(input.runs, async (attempt) => {
					const startedAt = Date.now()
					const result = await runFlowEval(
						testCase.userPrompt,
						getApiKeyForProvider(resolvedConfig.provider),
						{
								initialModules: testCase.initialFlow?.value?.modules,
								initialSchema: testCase.initialFlow?.schema,
								expectedFlow: testCase.expectedFlow as unknown as Parameters<
									typeof runFlowEval
								>[2]['expectedFlow'],
								variant: resolvedConfig.variant,
								provider: resolvedConfig.provider,
								model: resolvedConfig.model
						}
					)

					const minJudgeScore = testCase.minJudgeScore ?? DEFAULT_MIN_JUDGE_SCORE
					const checks = [
						requiredCheck('chat run succeeded', result.success, result.error),
						...validateFlowArtifact({
							generatedFlow: {
								value: { modules: result.flow.value.modules },
								schema: result.flow.schema
							},
							expectedFlow: testCase.expectedFlow
						}),
						...buildJudgeChecks({
							evaluationResult: result.evaluationResult,
							minJudgeScore
						})
					]

					return {
						attempt,
						passed: allRequiredChecksPassed(checks),
						durationMs: Date.now() - startedAt,
						assistantMessageCount: result.iterations,
						toolCallCount: result.toolCallsCount,
						toolsUsed: uniqueStrings(result.toolsCalled),
						checks,
						requiredFailedChecks: getRequiredFailedChecks(checks),
						judgeScore: result.evaluationResult?.resemblanceScore ?? null,
						judgeStatement: result.evaluationResult?.statement ?? null,
						error: result.error ?? result.evaluationResult?.error ?? null
					} satisfies FrontendBenchmarkAttempt
				})
			}))
		)
	}
}

async function runAppBenchmark(input: {
	surface: 'app'
	caseIds: string[]
	runs: number
	config?: FrontendBenchmarkConfig
}): Promise<FrontendBenchmarkPayload> {
	const { runAppEval } = await import('./core/app/appEvalRunner')
	const { loadAppFixtureForEval } = await import('./core/app/appFixtureLoader')
	const allCases = loadAppEvalCases()
	const selectedCases = resolveCases(allCases, input.caseIds, 'frontend app')
	const resolvedConfig = resolveConfig(input.config)

	return {
		surface: input.surface,
		runs: input.runs,
		provider: resolvedConfig.provider,
		model: resolvedConfig.model,
		judgeModel: FRONTEND_JUDGE_MODEL,
		caseResults: await Promise.all(
			selectedCases.map(async (testCase) => {
				const fixture = testCase.initialAppFixturePath
					? await loadAppFixtureForEval(testCase.initialAppFixturePath)
					: { initialFrontend: {}, initialBackend: {} }

				return {
					caseId: testCase.id,
					attempts: await runRepeated(input.runs, async (attempt) => {
						const startedAt = Date.now()
						const result = await runAppEval(
							testCase.userPrompt,
							getApiKeyForProvider(resolvedConfig.provider),
							{
								...fixture,
								variant: resolvedConfig.variant,
								provider: resolvedConfig.provider,
								model: resolvedConfig.model
							}
						)

						const minJudgeScore = testCase.minJudgeScore ?? DEFAULT_MIN_JUDGE_SCORE
						const checks = [
							requiredCheck('chat run succeeded', result.success, result.error),
							...validateAppArtifact({
								generatedApp: result.files,
								initialApp: testCase.initialAppFixturePath
									? {
											frontend: fixture.initialFrontend,
											backend: fixture.initialBackend
										}
									: undefined
							}),
							...buildJudgeChecks({
								evaluationResult: result.evaluationResult,
								minJudgeScore
							})
						]

						return {
							attempt,
							passed: allRequiredChecksPassed(checks),
							durationMs: Date.now() - startedAt,
							assistantMessageCount: result.iterations,
							toolCallCount: result.toolCallsCount,
							toolsUsed: uniqueStrings(result.toolsCalled),
							checks,
							requiredFailedChecks: getRequiredFailedChecks(checks),
							judgeScore: result.evaluationResult?.resemblanceScore ?? null,
							judgeStatement: result.evaluationResult?.statement ?? null,
							error: result.error ?? result.evaluationResult?.error ?? null
						} satisfies FrontendBenchmarkAttempt
					})
				}
			})
		)
	}
}

async function runScriptBenchmark(input: {
	surface: 'script'
	caseIds: string[]
	runs: number
	config?: FrontendBenchmarkConfig
}): Promise<FrontendBenchmarkPayload> {
	const { runScriptEval } = await import('./core/script/scriptEvalRunner')
	const allCases = loadScriptEvalCases()
	const selectedCases = resolveCases(allCases, input.caseIds, 'frontend script')
	const resolvedConfig = resolveConfig(input.config)

	return {
		surface: input.surface,
		runs: input.runs,
		provider: resolvedConfig.provider,
		model: resolvedConfig.model,
		judgeModel: FRONTEND_JUDGE_MODEL,
		caseResults: await Promise.all(
			selectedCases.map(async (testCase) => ({
				caseId: testCase.id,
				attempts: await runRepeated(input.runs, async (attempt) => {
					const startedAt = Date.now()
					const result = await runScriptEval(
						testCase.userPrompt,
						getApiKeyForProvider(resolvedConfig.provider),
						{
							initialScript: testCase.initialScript ?? testCase.expectedScript,
							expectedScript: testCase.expectedScript,
							variant: resolvedConfig.variant,
							provider: resolvedConfig.provider,
							model: resolvedConfig.model
						}
					)

					const minJudgeScore = testCase.minJudgeScore ?? DEFAULT_MIN_JUDGE_SCORE
					const checks = [
						requiredCheck('chat run succeeded', result.success, result.error),
						...validateScriptArtifact({
							generatedScript: result.script,
							expectedScript: testCase.expectedScript,
							initialScript: testCase.initialScript
						}),
						...buildJudgeChecks({
							evaluationResult: result.evaluationResult,
							minJudgeScore
						})
					]

					return {
						attempt,
						passed: allRequiredChecksPassed(checks),
						durationMs: Date.now() - startedAt,
						assistantMessageCount: result.iterations,
						toolCallCount: result.toolCallsCount,
						toolsUsed: uniqueStrings(result.toolsCalled),
						checks,
						requiredFailedChecks: getRequiredFailedChecks(checks),
						judgeScore: result.evaluationResult?.resemblanceScore ?? null,
						judgeStatement: result.evaluationResult?.statement ?? null,
						error: result.error ?? result.evaluationResult?.error ?? null
					} satisfies FrontendBenchmarkAttempt
				})
			}))
		)
	}
}

function resolveConfig(config?: FrontendBenchmarkConfig): {
	provider: AIProvider
	model: string
	variant: VariantConfig | undefined
} {
	const provider = config?.provider ?? DEFAULT_PROVIDER
	const model = config?.model ?? DEFAULT_MODEL

	if (!config?.systemPrompt) {
		return {
			provider,
			model,
			variant: undefined
		}
	}

	return {
		provider,
		model,
		variant: {
			name: config.systemPrompt.mode === 'replace' ? 'custom-system-prompt' : 'appended-system-prompt',
			systemPrompt:
				config.systemPrompt.mode === 'replace'
					? { type: 'custom', content: config.systemPrompt.content }
					: { type: 'default-with-custom', custom: config.systemPrompt.content },
			tools: { type: 'default' },
			model
		}
	}
}

function resolveCases<T extends { id: string }>(
	allCases: T[],
	caseIds: string[],
	surfaceLabel: string
): T[] {
	if (caseIds.length === 0) {
		return allCases
	}

	return caseIds.map((caseId) => {
		const testCase = allCases.find((entry) => entry.id === caseId)
		if (!testCase) {
			throw new Error(`Unknown ${surfaceLabel} case: ${caseId}`)
		}
		return testCase
	})
}

function parseConfig(value: string | undefined): FrontendBenchmarkConfig | undefined {
	if (!value) {
		return undefined
	}

	const parsed = JSON.parse(value) as FrontendBenchmarkConfig
	if (!parsed || typeof parsed !== 'object') {
		throw new Error('WMILL_FRONTEND_AI_EVAL_CONFIG must be a JSON object')
	}

	if (parsed.provider && parsed.provider !== 'anthropic' && parsed.provider !== 'openai') {
		throw new Error('WMILL_FRONTEND_AI_EVAL_CONFIG.provider must be "anthropic" or "openai"')
	}

	if (parsed.systemPrompt) {
		const systemPrompt = parsed.systemPrompt
		if (
			(systemPrompt.mode !== 'append' && systemPrompt.mode !== 'replace') ||
			typeof systemPrompt.content !== 'string' ||
			systemPrompt.content.trim().length === 0
		) {
			throw new Error(
				'WMILL_FRONTEND_AI_EVAL_CONFIG.systemPrompt must include mode "append" or "replace" and non-empty content'
			)
		}
	}

	return parsed
}

function parseOptionalJsonStringArray(value: string | undefined): string[] {
	if (!value) {
		return []
	}
	const parsed = JSON.parse(value) as unknown
	if (!Array.isArray(parsed) || parsed.some((entry) => typeof entry !== 'string')) {
		throw new Error('WMILL_FRONTEND_AI_EVAL_CASE_IDS must be a JSON string array')
	}
	return parsed
}

function parseSurface(value: string | undefined): FrontendBenchmarkSurface {
	if (value === 'flow' || value === 'app' || value === 'script') {
		return value
	}
	throw new Error('WMILL_FRONTEND_AI_EVAL_SURFACE must be "flow", "app", or "script"')
}

function parsePositiveInteger(value: string | undefined, envName: string): number {
	const parsed = Number.parseInt(value ?? '', 10)
	if (!Number.isInteger(parsed) || parsed < 1) {
		throw new Error(`${envName} must be a positive integer`)
	}
	return parsed
}

function getApiKeyForProvider(provider: AIProvider): string {
	if (provider === 'anthropic') {
		const apiKey = process.env.ANTHROPIC_API_KEY
		if (!apiKey) {
			throw new Error('ANTHROPIC_API_KEY is required for frontend benchmark runs')
		}
		return apiKey
	}

	if (provider === 'openai') {
		const apiKey = process.env.OPENAI_API_KEY
		if (!apiKey) {
			throw new Error('OPENAI_API_KEY is required for frontend benchmark runs')
		}
		return apiKey
	}

	throw new Error(`Unsupported frontend benchmark provider: ${provider}`)
}

async function runRepeated<T>(runs: number, fn: (attempt: number) => Promise<T>): Promise<T[]> {
	const results: T[] = []
	for (let attempt = 1; attempt <= runs; attempt += 1) {
		results.push(await fn(attempt))
	}
	return results
}

function uniqueStrings(values: string[]): string[] {
	return [...new Set(values)].sort((left, right) => left.localeCompare(right))
}
