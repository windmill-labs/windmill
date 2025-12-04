import { describe, it, expect } from 'vitest'
import { runFlowEval, validateModules, validateToolCalls } from './evalRunner'

// Get API key from environment - tests will be skipped if not set
// @ts-ignore
const OPENAI_API_KEY = process.env.OPENAI_API_KEY

// Skip all tests if no API key is provided
const describeWithApiKey = OPENAI_API_KEY ? describe : describe.skip

describeWithApiKey('Flow Chat LLM Evaluation', () => {
	// Increase timeout for API calls
	const TEST_TIMEOUT = 120_000

	it.only(
		'should add a simple rawscript module',
		async () => {
			const result = await runFlowEval('Add a step that prints Hello World', OPENAI_API_KEY!, {
				model: 'gpt-4o-mini'
			})

			expect(result.success).toBe(true)
			console.log(`[TEST 1] Token usage: ${result.tokenUsage.total}, Tool calls: ${result.toolCallsCount}, Tools: [${result.toolsCalled.join(', ')}]`)

			const moduleValidation = validateModules(result.modules, [{ type: 'rawscript' }])
			expect(moduleValidation.valid, moduleValidation.message).toBe(true)

			const toolValidation = validateToolCalls(result.toolsCalled, ['add_module'])
			expect(toolValidation.valid, toolValidation.message).toBe(true)
		},
		TEST_TIMEOUT
	)

	it(
		'should add two sequential steps',
		async () => {
			const result = await runFlowEval(
				'Add a step that fetches data, then add another step that processes it',
				OPENAI_API_KEY!
			)

			expect(result.success).toBe(true)
			console.log(`[TEST 2] Token usage: ${result.tokenUsage.total}, Tool calls: ${result.toolCallsCount}, Tools: [${result.toolsCalled.join(', ')}]`)

			const validation = validateModules(result.modules, [
				{ type: 'rawscript' },
				{ type: 'rawscript' }
			])
			expect(validation.valid, validation.message).toBe(true)

			// Expect two add_module calls for two steps
			const toolValidation = validateToolCalls(result.toolsCalled, ['add_module', 'add_module'])
			expect(toolValidation.valid, toolValidation.message).toBe(true)
		},
		TEST_TIMEOUT
	)

	it(
		'should add a forloop with a nested step',
		async () => {
			const result = await runFlowEval(
				'Add a for loop that iterates over [1,2,3] with a step inside that logs the current item',
				OPENAI_API_KEY!
			)

			expect(result.success).toBe(true)
			console.log(`[TEST 3] Token usage: ${result.tokenUsage.total}, Tool calls: ${result.toolCallsCount}, Tools: [${result.toolsCalled.join(', ')}]`)

			const validation = validateModules(result.modules, [{ type: 'forloopflow' }])
			expect(validation.valid, validation.message).toBe(true)

			// Check nested module exists
			const loop = result.modules[0]
			expect(loop.value.type).toBe('forloopflow')
			if (loop.value.type === 'forloopflow') {
				expect(loop.value.modules.length).toBeGreaterThan(0)
			}

			// For a forloop, we expect add_module for the loop, then add_module for the nested step
			const toolValidation = validateToolCalls(result.toolsCalled, ['add_module', 'add_module'])
			expect(toolValidation.valid, toolValidation.message).toBe(true)
		},
		TEST_TIMEOUT
	)

	it(
		'should add a branchone (conditional) step',
		async () => {
			const result = await runFlowEval(
				'Add a branch step with two conditions: one for when x > 10 and one for when x <= 10',
				OPENAI_API_KEY!,
				{
					initialSchema: {
						type: 'object',
						properties: { x: { type: 'number' } },
						required: ['x']
					}
				}
			)

			expect(result.success).toBe(true)
			console.log(`[TEST 4] Token usage: ${result.tokenUsage.total}, Tool calls: ${result.toolCallsCount}, Tools: [${result.toolsCalled.join(', ')}]`)

			const validation = validateModules(result.modules, [{ type: 'branchone' }])
			expect(validation.valid, validation.message).toBe(true)

			// For a branch, expect add_module for the branchone
			const toolValidation = validateToolCalls(result.toolsCalled, ['add_module'])
			expect(toolValidation.valid, toolValidation.message).toBe(true)
		},
		TEST_TIMEOUT
	)

	it(
		'should handle custom system prompt',
		async () => {
			const result = await runFlowEval('Add a step to process data', OPENAI_API_KEY!, {
				customSystemPrompt:
					'IMPORTANT: Always use TypeScript (bun) for all scripts. Never use Python.'
			})

			expect(result.success).toBe(true)
			console.log(`[TEST 5] Token usage: ${result.tokenUsage.total}, Tool calls: ${result.toolCallsCount}, Tools: [${result.toolsCalled.join(', ')}]`)

			// Should have at least one module
			expect(result.modules.length).toBeGreaterThan(0)

			// If it's a rawscript, check it's using bun
			const firstModule = result.modules[0]
			if (firstModule.value.type === 'rawscript') {
				expect(firstModule.value.language).toBe('bun')
			}

			// Expect add_module for the step
			const toolValidation = validateToolCalls(result.toolsCalled, ['add_module'])
			expect(toolValidation.valid, toolValidation.message).toBe(true)
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
