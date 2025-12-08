import { describe, it, expect } from 'vitest'
import { runVariantComparison, type ExpectedFlow } from './evalRunner'
import { writeComparisonResults } from './evalResultsWriter'
import { BASELINE_VARIANT, NO_FULL_SCHEMA_VARIANT } from './variants'
// @ts-ignore - JSON import
import expectedTest1 from './expected/test1.json'

// Get API key from environment - tests will be skipped if not set
// @ts-ignore
// const OPENAI_API_KEY = process.env.OPENAI_API_KEY
const OPENROUTER_API_KEY = process.env.OPENROUTER_API_KEY

// Skip all tests if no API key is provided
// const describeWithApiKey = OPENAI_API_KEY ? describe : describe.skip
const describeWithApiKey = OPENROUTER_API_KEY ? describe : describe.skip

describeWithApiKey('Flow Chat LLM Evaluation', () => {
	const TEST_TIMEOUT = 120_000
	if (!OPENROUTER_API_KEY) {
		console.warn('OPENROUTER_API_KEY is not set, skipping tests')
	}

	it.only(
		'example: compare variants on simple task',
		async () => {
			const USER_PROMPT = `
THIS IS A TEST, CODE SHOULD BE MINIMAL FUNCTIONING CODE, IF WE NEED RETURN VALUES RETURN EXAMPLE VALUES

STEP 1: Fetch mock users from api
STEP 2: Filter only active users:
STEP 3: Loop on all users
STEP 4: Do branches based on user's role, do different action based on that. Roles are admin, user, moderator
STEP 5: Return action taken for each user
`
			const results = await runVariantComparison(
				USER_PROMPT,
				[BASELINE_VARIANT, NO_FULL_SCHEMA_VARIANT],
				OPENROUTER_API_KEY!,
				{
					model: 'google/gemini-2.5-flash',
					expectedFlow: expectedTest1 as ExpectedFlow
				}
			)

			// Write results to files
			const { summaryPath, flowPaths } = await writeComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`Flow files: ${flowPaths.join(', ')}`)

			// Assert all variants succeeded
			for (const result of results) {
				expect(result.success, `${result.variantName} should succeed`).toBe(true)

				// Log evaluation results
				if (result.evaluationResult) {
					console.log(
						`[${result.variantName}] Resemblance Score: ${result.evaluationResult.resemblanceScore}/100`
					)
					console.log(`[${result.variantName}] Statement: ${result.evaluationResult.statement}`)
				}
			}
		},
		TEST_TIMEOUT * 2 // Double timeout for comparison tests
	)
})
