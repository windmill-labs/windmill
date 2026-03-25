import { describe, expect, it } from 'vitest'
import { runVariantComparison, writeAppComparisonResults } from './appEvalRunner'
import { BASELINE_VARIANT, STREAMLINED_VARIANT } from './variants'
import { loadAppFixtureForEval } from './appFixtureLoader'
// @ts-ignore - Node.js path
import { dirname, join } from 'path'
// @ts-ignore - Node.js url
import { fileURLToPath } from 'url'
import type { AIProvider } from '$lib/gen/types.gen'

// Get API keys from environment - tests will be skipped if none are set
// @ts-ignore
const OPENAI_API_KEY = process.env.OPENAI_API_KEY
// @ts-ignore
const ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY

const hasAnyKey = OPENAI_API_KEY || ANTHROPIC_API_KEY
const describeWithApiKey = hasAnyKey ? describe : describe.skip

// Get __dirname equivalent for ES modules
const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

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

	it(
		'test1: creates a simple counter app',
		async () => {
			const USER_PROMPT = `Create a counter app with increment/decrement buttons`
			const results = await runVariantComparison(
				USER_PROMPT,
				VARIANTS,
				VARIANTS[0]._apiKey,
				undefined,
				VARIANTS.map((v) => ({ provider: v._provider, apiKey: v._apiKey }))
			)
			const { summaryPath, appPaths } = await writeAppComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`App files: ${appPaths.join(', ')}`)

			expect(true).toBe(true)
		},
		TEST_TIMEOUT
	)

	it(
		'test2: modifies existing counter app to add reset button',
		async () => {
			const { initialFrontend, initialBackend } = await loadAppFixtureForEval(
				join(__dirname, 'initial', 'test1_counter_app')
			)

			const USER_PROMPT = `Add a reset button that sets the counter back to 0`
			const results = await runVariantComparison(
				USER_PROMPT,
				VARIANTS,
				VARIANTS[0]._apiKey,
				{
					initialFrontend,
					initialBackend
				},
				VARIANTS.map((v) => ({ provider: v._provider, apiKey: v._apiKey }))
			)
			const { summaryPath, appPaths } = await writeAppComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`App files: ${appPaths.join(', ')}`)

			expect(true).toBe(true)
		},
		TEST_TIMEOUT
	)

	// ==================== Shopping Cart Tests ====================

	it(
		'test3: shopping cart - add quantity selector',
		async () => {
			const { initialFrontend, initialBackend } = await loadAppFixtureForEval(
				join(__dirname, 'initial', 'shopping_cart')
			)

			const USER_PROMPT = `Add a quantity selector (+ and - buttons) to each cart item so users can adjust quantities without removing and re-adding items`
			const results = await runVariantComparison(
				USER_PROMPT,
				VARIANTS,
				VARIANTS[0]._apiKey,
				{
					initialFrontend,
					initialBackend
				},
				VARIANTS.map((v) => ({ provider: v._provider, apiKey: v._apiKey }))
			)

			const { summaryPath, appPaths } = await writeAppComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`App files: ${appPaths.join(', ')}`)

			expect(true).toBe(true)
		},
		TEST_TIMEOUT
	)

	it(
		'test4: shopping cart - add discount code',
		async () => {
			const { initialFrontend, initialBackend } = await loadAppFixtureForEval(
				join(__dirname, 'initial', 'shopping_cart')
			)

			const USER_PROMPT = `Add a discount code input field in the cart. When the code "SAVE10" is entered, apply a 10% discount to the total`
			const results = await runVariantComparison(
				USER_PROMPT,
				VARIANTS,
				VARIANTS[0]._apiKey,
				{
					initialFrontend,
					initialBackend
				},
				VARIANTS.map((v) => ({ provider: v._provider, apiKey: v._apiKey }))
			)

			const { summaryPath, appPaths } = await writeAppComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`App files: ${appPaths.join(', ')}`)

			expect(true).toBe(true)
		},
		TEST_TIMEOUT
	)

	// ==================== File Manager Tests ====================

	it(
		'test5: file manager - add search bar',
		async () => {
			const { initialFrontend, initialBackend } = await loadAppFixtureForEval(
				join(__dirname, 'initial', 'file_manager')
			)

			const USER_PROMPT = `Add a search bar in the toolbar that filters files and folders by name as the user types`
			const results = await runVariantComparison(
				USER_PROMPT,
				VARIANTS,
				VARIANTS[0]._apiKey,
				{
					initialFrontend,
					initialBackend
				},
				VARIANTS.map((v) => ({ provider: v._provider, apiKey: v._apiKey }))
			)

			const { summaryPath, appPaths } = await writeAppComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`App files: ${appPaths.join(', ')}`)

			expect(true).toBe(true)
		},
		TEST_TIMEOUT
	)

	it(
		'test6: file manager - show file details',
		async () => {
			const { initialFrontend, initialBackend } = await loadAppFixtureForEval(
				join(__dirname, 'initial', 'file_manager')
			)

			const USER_PROMPT = `Show file size (formatted as KB/MB) and modified date in the file list for each item`
			const results = await runVariantComparison(
				USER_PROMPT,
				VARIANTS,
				VARIANTS[0]._apiKey,
				{
					initialFrontend,
					initialBackend
				},
				VARIANTS.map((v) => ({ provider: v._provider, apiKey: v._apiKey }))
			)

			const { summaryPath, appPaths } = await writeAppComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`App files: ${appPaths.join(', ')}`)

			expect(true).toBe(true)
		},
		TEST_TIMEOUT
	)

	it(
		'test7: file manager - add select all checkbox',
		async () => {
			const { initialFrontend, initialBackend } = await loadAppFixtureForEval(
				join(__dirname, 'initial', 'file_manager')
			)

			const USER_PROMPT = `Add a "Select All" checkbox in the file list header and individual checkboxes for each file. Add a "Delete Selected" button that appears when items are selected`
			const results = await runVariantComparison(
				USER_PROMPT,
				VARIANTS,
				VARIANTS[0]._apiKey,
				{
					initialFrontend,
					initialBackend
				},
				VARIANTS.map((v) => ({ provider: v._provider, apiKey: v._apiKey }))
			)

			const { summaryPath, appPaths } = await writeAppComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`App files: ${appPaths.join(', ')}`)

			expect(true).toBe(true)
		},
		TEST_TIMEOUT
	)

	// ==================== From-Scratch Creation Tests ====================

	it(
		'test8: create quiz app from scratch',
		async () => {
			const USER_PROMPT = `Create a multiple choice quiz app with 5 questions about general knowledge. Show one question at a time with 4 answer options. Track the score and show results at the end with percentage correct.`
			const results = await runVariantComparison(
				USER_PROMPT,
				VARIANTS,
				VARIANTS[0]._apiKey,
				undefined,
				VARIANTS.map((v) => ({ provider: v._provider, apiKey: v._apiKey }))
			)

			const { summaryPath, appPaths } = await writeAppComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`App files: ${appPaths.join(', ')}`)

			expect(true).toBe(true)
		},
		TEST_TIMEOUT
	)

	it(
		'test9: create recipe book from scratch',
		async () => {
			const USER_PROMPT = `Create a recipe book app where users can add recipes with a name, ingredients list, and instructions. Include a search bar to filter recipes by name and the ability to delete recipes.`
			const results = await runVariantComparison(
				USER_PROMPT,
				VARIANTS,
				VARIANTS[0]._apiKey,
				undefined,
				VARIANTS.map((v) => ({ provider: v._provider, apiKey: v._apiKey }))
			)

			const { summaryPath, appPaths } = await writeAppComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`App files: ${appPaths.join(', ')}`)

			expect(true).toBe(true)
		},
		TEST_TIMEOUT
	)
})
