import { describe, it, expect } from 'vitest'
import {
	runFlowEval,
	validateModules,
	validateToolCalls,
	formatToolCalls,
	type EvalResult
} from './evalRunner'

// Get API key from environment - tests will be skipped if not set
// @ts-ignore
const OPENAI_API_KEY = process.env.OPENAI_API_KEY

// Skip all tests if no API key is provided
const describeWithApiKey = OPENAI_API_KEY ? describe : describe.skip

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

describeWithApiKey('Flow Chat LLM Evaluation', () => {
	const TEST_TIMEOUT = 120_000
	if (!OPENAI_API_KEY) {
		console.warn('OPENAI_API_KEY is not set, skipping tests')
	}

	it.only(
		'should add a simple rawscript module',
		async () => {
			const result = await runFlowEval('Add a step that prints Hello World', OPENAI_API_KEY!, {
				model: 'gpt-4o-mini'
			})

			assertEvalResult(result, {
				modules: [{ type: 'rawscript' }],
				tools: ['add_module', 'test_run_flow']
			}, 'TEST 1')
		},
		TEST_TIMEOUT
	)
})

// Unit tests for validateModules helper (no API needed)
describe('validateModules', () => {
	it('returns valid for matching modules', () => {
		const modules = [
			{ id: 'a', value: { type: 'rawscript' } },
			{ id: 'b', value: { type: 'forloopflow', modules: [] } }
		] as any

		const result = validateModules(modules, [{ type: 'rawscript' }, { type: 'forloopflow' }])

		expect(result.valid).toBe(true)
	})

	it('returns invalid for count mismatch', () => {
		const modules = [{ id: 'a', value: { type: 'rawscript' } }] as any

		const result = validateModules(modules, [{ type: 'rawscript' }, { type: 'rawscript' }])

		expect(result.valid).toBe(false)
		expect(result.message).toContain('count mismatch')
	})

	it('returns invalid for type mismatch', () => {
		const modules = [{ id: 'a', value: { type: 'rawscript' } }] as any

		const result = validateModules(modules, [{ type: 'forloopflow' }])

		expect(result.valid).toBe(false)
		expect(result.message).toContain('type mismatch')
	})
})

// Unit tests for validateToolCalls helper (no API needed)
describe('validateToolCalls', () => {
	it('returns valid for matching tool calls', () => {
		const actual = ['add_module', 'add_module', 'modify_module']
		const expected = ['add_module', 'add_module', 'modify_module']

		const result = validateToolCalls(actual, expected)

		expect(result.valid).toBe(true)
	})

	it('returns invalid for count mismatch', () => {
		const actual = ['add_module']
		const expected = ['add_module', 'add_module']

		const result = validateToolCalls(actual, expected)

		expect(result.valid).toBe(false)
		expect(result.message).toContain('count mismatch')
		expect(result.message).toContain('Called: [add_module]')
	})

	it('returns invalid for tool name mismatch', () => {
		const actual = ['add_module', 'remove_module']
		const expected = ['add_module', 'modify_module']

		const result = validateToolCalls(actual, expected)

		expect(result.valid).toBe(false)
		expect(result.message).toContain('mismatch')
		expect(result.message).toContain('modify_module')
		expect(result.message).toContain('remove_module')
	})

	it('returns valid for empty arrays', () => {
		const result = validateToolCalls([], [])

		expect(result.valid).toBe(true)
	})
})
