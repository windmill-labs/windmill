import { describe, it, expect } from 'vitest'
import {
	runVariantComparison,
	validateModules,
	validateToolCalls,
	formatToolCalls,
	type EvalResult
} from './evalRunner'
import { writeComparisonResults } from './evalResultsWriter'
import { BASELINE_VARIANT, NO_FULL_SCHEMA_VARIANT } from './variants'

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
				{ model: 'anthropic/claude-haiku-4.5' }
			)

			// Write results to files
			const { summaryPath, flowPaths } = await writeComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`Flow files: ${flowPaths.join(', ')}`)

			// Assert all variants succeeded
			for (const result of results) {
				expect(result.success, `${result.variantName} should succeed`).toBe(true)
				// expect(result.modules.length, `${result.variantName} should create 1 module`).toBe(1)
			}
		},
		TEST_TIMEOUT * 2 // Double timeout for comparison tests
	)
})

/**
 * Helper to validate eval result and log details on failure
 */
function assertEvalResult(
	result: EvalResult,
	expected: {
		modules: Array<{ type: string }>
		tools: string[]
	},
	testName: string
) {
	expect(result.success).toBe(true)
	console.log(
		`[${testName}] Tokens: ${result.tokenUsage.total}, Tools: [${result.toolsCalled.join(', ')}]`
	)

	const moduleValidation = validateModules(result.modules, expected.modules)
	if (!moduleValidation.valid) {
		console.log('Tool calls with arguments:\n' + formatToolCalls(result.toolCallDetails))
	}
	expect(moduleValidation.valid, moduleValidation.message).toBe(true)

	const toolValidation = validateToolCalls(result.toolsCalled, expected.tools)
	if (!toolValidation.valid) {
		console.log('Tool calls with arguments:\n' + formatToolCalls(result.toolCallDetails))
	}
	expect(toolValidation.valid, toolValidation.message).toBe(true)
}
