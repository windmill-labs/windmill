// @ts-ignore - Node.js fs/promises
import { readFile } from 'fs/promises'
// @ts-ignore - Node.js path
import { dirname, join, resolve } from 'path'
// @ts-ignore - Node.js url
import { fileURLToPath } from 'url'
import type { AIProvider } from '$lib/gen/types.gen'
import { loadAppEvalCases, loadFlowEvalCases } from '../shared/evalCaseLoader'
import type { VariantConfig } from '../shared'

export type FrontendBenchmarkSurface = 'flow' | 'app'

interface VariantPromptConfigDefault {
	type: 'default'
}

interface VariantPromptConfigDefaultWithCustom {
	type: 'default-with-custom'
	custom?: string
	customPath?: string
}

interface VariantPromptConfigCustom {
	type: 'custom'
	content?: string
	contentPath?: string
}

interface VariantToolsConfigDefault {
	type: 'default'
}

interface VariantToolsConfigSubset {
	type: 'subset'
	include: string[]
}

export interface FrontendBenchmarkVariantManifest {
	id: string
	description?: string
	provider?: AIProvider
	model?: string
	systemPrompt?:
		| VariantPromptConfigDefault
		| VariantPromptConfigDefaultWithCustom
		| VariantPromptConfigCustom
	tools?: VariantToolsConfigDefault | VariantToolsConfigSubset
}

export interface FrontendBenchmarkAttempt {
	attempt: number
	passed: boolean
	durationMs: number
	assistantMessageCount: number
	toolCallCount: number
	toolsUsed: string[]
	checks: Array<{ name: string; passed: boolean; required?: boolean }>
	requiredFailedChecks: string[]
	judgeScore: number | null
	judgeStatement: string | null
	error: string | null
}

export interface FrontendBenchmarkCaseResult {
	caseId: string
	attempts: FrontendBenchmarkAttempt[]
}

export interface FrontendBenchmarkVariantResult {
	variant: string
	provider: AIProvider
	model: string
	judgeModel: string | null
	caseResults: FrontendBenchmarkCaseResult[]
}

export interface FrontendBenchmarkPayload {
	surface: FrontendBenchmarkSurface
	runs: number
	variants: FrontendBenchmarkVariantResult[]
}

const DEFAULT_MIN_JUDGE_SCORE = 80
const FRONTEND_JUDGE_MODEL = 'claude-sonnet-4-6'

export async function runFrontendBenchmarkFromEnv(): Promise<FrontendBenchmarkPayload> {
	return runFrontendBenchmark({
		surface: parseSurface(process.env.WMILL_FRONTEND_AI_EVAL_SURFACE),
		caseIds: parseJsonStringArray(process.env.WMILL_FRONTEND_AI_EVAL_CASE_IDS, 'WMILL_FRONTEND_AI_EVAL_CASE_IDS'),
		variantIds: parseJsonStringArray(
			process.env.WMILL_FRONTEND_AI_EVAL_VARIANT_IDS,
			'WMILL_FRONTEND_AI_EVAL_VARIANT_IDS'
		),
		runs: parsePositiveInteger(process.env.WMILL_FRONTEND_AI_EVAL_RUNS, 'WMILL_FRONTEND_AI_EVAL_RUNS')
	})
}

export async function runFrontendBenchmark(input: {
	surface: FrontendBenchmarkSurface
	caseIds: string[]
	variantIds: string[]
	runs: number
}): Promise<FrontendBenchmarkPayload> {
	if (input.variantIds.length === 0) {
		throw new Error('Frontend benchmark requires at least one variant id')
	}

	switch (input.surface) {
		case 'flow':
			return await runFlowBenchmark(input)
		case 'app':
			return await runAppBenchmark(input)
		default:
			throw new Error(`Unsupported frontend benchmark surface: ${String(input.surface)}`)
	}
}

function resolveRepoRoot(): string {
	const currentDir = dirname(fileURLToPath(import.meta.url))
	const repoRoot = resolve(currentDir, '../../../../../../../..')
	return repoRoot
}

async function runFlowBenchmark(input: {
	surface: 'flow'
	caseIds: string[]
	variantIds: string[]
	runs: number
}): Promise<FrontendBenchmarkPayload> {
	const { runFlowEval } = await import('../flow/flowEvalRunner')
	const allCases = loadFlowEvalCases()
	const selectedCases =
		input.caseIds.length === 0
			? allCases
			: input.caseIds.map((caseId) => {
					const testCase = allCases.find((entry) => entry.id === caseId)
					if (!testCase) {
						throw new Error(`Unknown frontend flow case: ${caseId}`)
					}
					return testCase
				})

	const variants = await Promise.all(
		input.variantIds.map((variantId) => loadFrontendVariant('flow', variantId))
	)

	return {
		surface: input.surface,
		runs: input.runs,
		variants: await Promise.all(
			variants.map(async (variant) => ({
				variant: variant.id,
				provider: variant.provider,
				model: variant.model,
				judgeModel: FRONTEND_JUDGE_MODEL,
				caseResults: await Promise.all(
					selectedCases.map(async (testCase) => ({
						caseId: testCase.id,
						attempts: await runRepeated(input.runs, async (attempt) => {
							const startedAt = Date.now()
							const result = await runFlowEval(
								testCase.userPrompt,
								getApiKeyForProvider(variant.provider),
								{
									initialModules: testCase.initialFlow?.value?.modules,
									initialSchema: testCase.initialFlow?.schema,
									expectedFlow: testCase.expectedFlow,
									variant: variant.config,
									provider: variant.provider,
									model: variant.model
								}
							)

							const minJudgeScore = testCase.minJudgeScore ?? DEFAULT_MIN_JUDGE_SCORE
							const checks = [
								{ name: 'chat run succeeded', passed: result.success, required: true },
								{
									name: 'judge evaluation succeeded',
									passed: Boolean(result.evaluationResult?.success),
									required: true
								},
								{
									name: `judge score >= ${minJudgeScore}`,
									passed: (result.evaluationResult?.resemblanceScore ?? 0) >= minJudgeScore,
									required: true
								}
							]

							return {
								attempt,
								passed: checks.every((check) => check.passed),
								durationMs: Date.now() - startedAt,
								assistantMessageCount: result.iterations,
								toolCallCount: result.toolCallsCount,
								toolsUsed: uniqueStrings(result.toolsCalled),
								checks,
								requiredFailedChecks: checks
									.filter((check) => check.required !== false && !check.passed)
									.map((check) => check.name),
								judgeScore: result.evaluationResult?.resemblanceScore ?? null,
								judgeStatement: result.evaluationResult?.statement ?? null,
								error: result.error ?? result.evaluationResult?.error ?? null
							} satisfies FrontendBenchmarkAttempt
						})
					}))
				)
			}))
		)
	}
}

async function runAppBenchmark(input: {
	surface: 'app'
	caseIds: string[]
	variantIds: string[]
	runs: number
}): Promise<FrontendBenchmarkPayload> {
	const { runAppEval } = await import('../app/appEvalRunner')
	const { loadAppFixtureForEval } = await import('../app/appFixtureLoader')
	const allCases = loadAppEvalCases()
	const selectedCases =
		input.caseIds.length === 0
			? allCases
			: input.caseIds.map((caseId) => {
					const testCase = allCases.find((entry) => entry.id === caseId)
					if (!testCase) {
						throw new Error(`Unknown frontend app case: ${caseId}`)
					}
					return testCase
				})

	const variants = await Promise.all(
		input.variantIds.map((variantId) => loadFrontendVariant('app', variantId))
	)

	return {
		surface: input.surface,
		runs: input.runs,
		variants: await Promise.all(
			variants.map(async (variant) => ({
				variant: variant.id,
				provider: variant.provider,
				model: variant.model,
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
									getApiKeyForProvider(variant.provider),
									{
										...fixture,
										variant: variant.config,
										provider: variant.provider,
										model: variant.model
									}
								)

								const minJudgeScore = testCase.minJudgeScore ?? DEFAULT_MIN_JUDGE_SCORE
								const checks = [
									{ name: 'chat run succeeded', passed: result.success, required: true },
									{
										name: 'judge evaluation succeeded',
										passed: Boolean(result.evaluationResult?.success),
										required: true
									},
									{
										name: `judge score >= ${minJudgeScore}`,
										passed: (result.evaluationResult?.resemblanceScore ?? 0) >= minJudgeScore,
										required: true
									}
								]

								return {
									attempt,
									passed: checks.every((check) => check.passed),
									durationMs: Date.now() - startedAt,
									assistantMessageCount: result.iterations,
									toolCallCount: result.toolCallsCount,
									toolsUsed: uniqueStrings(result.toolsCalled),
									checks,
									requiredFailedChecks: checks
										.filter((check) => check.required !== false && !check.passed)
										.map((check) => check.name),
									judgeScore: result.evaluationResult?.resemblanceScore ?? null,
									judgeStatement: result.evaluationResult?.statement ?? null,
									error: result.error ?? result.evaluationResult?.error ?? null
								} satisfies FrontendBenchmarkAttempt
							})
						}
					})
				)
			}))
		)
	}
}

async function loadFrontendVariant(
	surface: FrontendBenchmarkSurface,
	variantId: string
): Promise<{
	id: string
	provider: AIProvider
	model: string
	config: VariantConfig
}> {
	const repoRoot = resolveRepoRoot()
	const manifestPath = join(
		repoRoot,
		'ai_evals',
		'variants',
		'frontend',
		surface,
		`${variantId}.json`
	)
	const manifest = JSON.parse(
		await readFile(manifestPath, 'utf8')
	) as FrontendBenchmarkVariantManifest

	if (!manifest.id) {
		throw new Error(`Frontend variant manifest missing id: ${manifestPath}`)
	}

	return {
		id: manifest.id,
		provider: manifest.provider ?? 'anthropic',
		model: manifest.model ?? 'claude-haiku-4-5-20251001',
		config: {
			name: manifest.id,
			description: manifest.description,
			systemPrompt: manifest.systemPrompt
				? await resolveSystemPromptConfig(manifestPath, manifest.systemPrompt)
				: { type: 'default' },
			tools: manifest.tools ?? { type: 'default' }
		}
	}
}

async function resolveSystemPromptConfig(
	manifestPath: string,
	config: FrontendBenchmarkVariantManifest['systemPrompt']
): Promise<VariantConfig['systemPrompt']> {
	if (!config || config.type === 'default') {
		return { type: 'default' }
	}

	if (config.type === 'default-with-custom') {
		const custom = config.custom ?? (config.customPath ? await readPromptFile(manifestPath, config.customPath) : undefined)
		if (!custom) {
			throw new Error(
				`Frontend variant ${manifestPath} must provide systemPrompt.custom or systemPrompt.customPath`
			)
		}
		return {
			type: 'default-with-custom',
			custom
		}
	}

	const content =
		config.content ?? (config.contentPath ? await readPromptFile(manifestPath, config.contentPath) : undefined)
	if (!content) {
		throw new Error(
			`Frontend variant ${manifestPath} must provide systemPrompt.content or systemPrompt.contentPath`
		)
	}
	return {
		type: 'custom',
		content
	}
}

async function readPromptFile(manifestPath: string, relativePath: string): Promise<string> {
	return await readFile(join(dirname(manifestPath), relativePath), 'utf8')
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

function parseJsonStringArray(value: string | undefined, envName: string): string[] {
	if (!value) {
		throw new Error(`Missing required ${envName}`)
	}
	const parsed = JSON.parse(value) as unknown
	if (!Array.isArray(parsed) || parsed.some((entry) => typeof entry !== 'string')) {
		throw new Error(`${envName} must be a JSON string array`)
	}
	return parsed
}

function parseSurface(value: string | undefined): FrontendBenchmarkSurface {
	if (value === 'flow' || value === 'app') {
		return value
	}
	throw new Error('WMILL_FRONTEND_AI_EVAL_SURFACE must be "flow" or "app"')
}

function parsePositiveInteger(value: string | undefined, envName: string): number {
	const parsed = Number.parseInt(value ?? '', 10)
	if (!Number.isInteger(parsed) || parsed < 1) {
		throw new Error(`${envName} must be a positive integer`)
	}
	return parsed
}

function uniqueStrings(values: string[]): string[] {
	return [...new Set(values)].sort((left, right) => left.localeCompare(right))
}
