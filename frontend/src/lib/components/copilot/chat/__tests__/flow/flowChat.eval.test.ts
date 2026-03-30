import { describe, it } from 'vitest'
import { runVariantComparison, writeFlowComparisonResults, type ExpectedFlow } from './flowEvalRunner'
import { BASELINE_VARIANT, MINIMAL_SINGLE_TOOL_VARIANT } from './variants'
import type { FlowModule } from '$lib/gen'
import type { AIProvider } from '$lib/gen/types.gen'
import { loadFlowEvalCases } from '../shared/evalCaseLoader'
import { assertSuccessfulEvalRun } from '../shared/evalAssertions'

// Get API keys from environment - tests will be skipped if none are set
// @ts-ignore
const OPENAI_API_KEY = process.env.OPENAI_API_KEY
// @ts-ignore
const ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY

const hasAnyKey = OPENAI_API_KEY || ANTHROPIC_API_KEY
const hasJudgeKey = Boolean(ANTHROPIC_API_KEY)
const describeWithApiKey = hasAnyKey ? describe : describe.skip
const FLOW_CASES = loadFlowEvalCases()

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
		...MINIMAL_SINGLE_TOOL_VARIANT,
		model: mv.model,
		name: `minimal-single-tool-${mv.provider}-${mv.model}`,
		_provider: mv.provider,
		_apiKey: mv.apiKey
	}))
]

describeWithApiKey('Flow Chat LLM Evaluation', () => {
	const TEST_TIMEOUT = 120_000
	if (!hasAnyKey) {
		console.warn('No API keys set (OPENAI_API_KEY or ANTHROPIC_API_KEY), skipping tests')
	}
	if (!hasJudgeKey) {
		console.warn('ANTHROPIC_API_KEY not set, flow evals will skip judge-score assertions')
	}

	for (const testCase of FLOW_CASES) {
		it(
			testCase.title,
			async () => {
				const results = await runVariantComparison(
					testCase.userPrompt,
					VARIANTS,
					VARIANTS[0]._apiKey,
					{
						initialModules: testCase.initialFlow?.value?.modules as FlowModule[] | undefined,
						initialSchema: testCase.initialFlow?.schema,
						expectedFlow: testCase.expectedFlow as unknown as ExpectedFlow
					},
					VARIANTS.map((v) => ({ provider: v._provider, apiKey: v._apiKey }))
				)

				const { summaryPath, flowPaths } = await writeFlowComparisonResults(testCase.userPrompt, results)
				console.log(`\nResults written to: ${summaryPath}`)
				console.log(`Flow files: ${flowPaths.join(', ')}`)

				for (const result of results) {
					assertSuccessfulEvalRun(result, {
						requireJudge: hasJudgeKey,
						minJudgeScore: testCase.minJudgeScore
					})

					if (result.evaluationResult) {
						console.log(
							`[${result.variantName}] Resemblance Score: ${result.evaluationResult.resemblanceScore}/100`
						)
						console.log(`[${result.variantName}] Statement: ${result.evaluationResult.statement}`)
						if (
							result.evaluationResult.missingRequirements &&
							result.evaluationResult.missingRequirements.length > 0
						) {
							console.log(
								`[${result.variantName}] Missing: ${result.evaluationResult.missingRequirements.join(', ')}`
							)
						}
					}
				}
			},
			TEST_TIMEOUT
		)
	}
})
