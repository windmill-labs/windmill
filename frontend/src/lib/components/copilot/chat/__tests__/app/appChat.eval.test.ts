import { describe, it } from 'vitest'
import { runVariantComparison, writeAppComparisonResults } from './appEvalRunner'
import { BASELINE_VARIANT, STREAMLINED_VARIANT } from './variants'
import { loadAppFixtureForEval } from './appFixtureLoader'
import type { AIProvider } from '$lib/gen/types.gen'
import { loadAppEvalCases } from '../shared/evalCaseLoader'
import { assertSuccessfulEvalRun } from '../shared/evalAssertions'

// Get API keys from environment - tests will be skipped if none are set
// @ts-ignore
const OPENAI_API_KEY = process.env.OPENAI_API_KEY
// @ts-ignore
const ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY

const hasAnyKey = OPENAI_API_KEY || ANTHROPIC_API_KEY
const hasJudgeKey = Boolean(ANTHROPIC_API_KEY)
const describeWithApiKey = hasAnyKey ? describe : describe.skip
const APP_CASES = loadAppEvalCases()

// Build model variants based on available keys
interface ModelVariant {
	model: string
	provider: AIProvider
	apiKey: string
}

const MODEL_VARIANTS: ModelVariant[] = [
	...(OPENAI_API_KEY
		? [{ model: 'gpt-4o', provider: 'openai' as AIProvider, apiKey: OPENAI_API_KEY }]
		: []),
	...(ANTHROPIC_API_KEY
		? [
				{
					model: 'claude-haiku-4-5-20241022',
					provider: 'anthropic' as AIProvider,
					apiKey: ANTHROPIC_API_KEY
				}
			]
		: [])
]

const VARIANTS = [
	...MODEL_VARIANTS.map((mv) => ({
		...BASELINE_VARIANT,
		model: mv.model,
		name: `baseline-${mv.provider}-${mv.model}`,
		_provider: mv.provider,
		_apiKey: mv.apiKey
	})),
	...MODEL_VARIANTS.map((mv) => ({
		...STREAMLINED_VARIANT,
		model: mv.model,
		name: `streamlined-${mv.provider}-${mv.model}`,
		_provider: mv.provider,
		_apiKey: mv.apiKey
	}))
]

describeWithApiKey('App Chat LLM Evaluation', () => {
	const TEST_TIMEOUT = 120_000
	if (!hasAnyKey) {
		console.warn('No API keys set (OPENAI_API_KEY or ANTHROPIC_API_KEY), skipping tests')
	}
	if (!hasJudgeKey) {
		console.warn('ANTHROPIC_API_KEY not set, app evals will skip judge-score assertions')
	}

	for (const testCase of APP_CASES) {
		it(
			testCase.title,
			async () => {
				const initialState = testCase.initialAppFixturePath
					? await loadAppFixtureForEval(testCase.initialAppFixturePath)
					: undefined

				const results = await runVariantComparison(
					testCase.userPrompt,
					VARIANTS,
					VARIANTS[0]._apiKey,
					initialState,
					VARIANTS.map((v) => ({ provider: v._provider, apiKey: v._apiKey }))
				)

				const { summaryPath, appPaths } = await writeAppComparisonResults(testCase.userPrompt, results)
				console.log(`\nResults written to: ${summaryPath}`)
				console.log(`App files: ${appPaths.join(', ')}`)

				for (const result of results) {
					assertSuccessfulEvalRun(result, {
						requireJudge: hasJudgeKey,
						minJudgeScore: testCase.minJudgeScore
					})
				}
			},
			TEST_TIMEOUT
		)
	}
})
