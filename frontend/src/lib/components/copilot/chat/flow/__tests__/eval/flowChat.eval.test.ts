import { describe, it, expect } from 'vitest'
import {
	runFlowEval,
	runVariantComparison,
	formatComparisonResults,
	validateModules,
	validateToolCalls,
	formatToolCalls,
	type EvalResult
} from './evalRunner'
import { BASELINE_VARIANT, MINIMAL_SINGLE_TOOL_VARIANT } from './variants'

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
			const results = await runVariantComparison(
				'Add a step that prints Hello World',
				[BASELINE_VARIANT, MINIMAL_SINGLE_TOOL_VARIANT],
				OPENROUTER_API_KEY!,
				{ model: 'gpt-4o-mini' }
			)

			// Log comparison table
			console.log('\n--- Variant Comparison Results ---')
			console.log(formatComparisonResults(results))

			// Assert all variants succeeded
			for (const result of results) {
				expect(result.success, `${result.variantName} should succeed`).toBe(true)
				expect(result.modules.length, `${result.variantName} should create 1 module`).toBe(1)
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
