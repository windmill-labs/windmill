import { describe, expect, it } from 'vitest'
import { runVariantComparison, writeAppComparisonResults } from './appEvalRunner'
import { BASELINE_VARIANT } from './variants'

// Get API key from environment - tests will be skipped if not set
// @ts-ignore
const OPENROUTER_API_KEY = process.env.OPENROUTER_API_KEY

// Skip all tests if no API key is provided
const describeWithApiKey = OPENROUTER_API_KEY ? describe : describe.skip

describeWithApiKey('App Chat LLM Evaluation', () => {
	const TEST_TIMEOUT = 120_000

	it(
		'test1: creates a simple counter app',
		async () => {
			const USER_PROMPT = `Create a counter app with increment/decrement buttons`
			const results = await runVariantComparison(
				USER_PROMPT,
				[BASELINE_VARIANT],
				OPENROUTER_API_KEY!
			)
			// Write results to files
			const { summaryPath, appPaths } = await writeAppComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`App files: ${appPaths.join(', ')}`)

			expect(true).toBe(true)
		},
		TEST_TIMEOUT
	)
})
