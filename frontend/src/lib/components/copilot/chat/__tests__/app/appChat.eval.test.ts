import { describe, expect, it } from 'vitest'
import { runVariantComparison, writeAppComparisonResults } from './appEvalRunner'
import { BASELINE_VARIANT, STREAMLINED_VARIANT } from './variants'
import { loadAppFixtureForEval } from './appFixtureLoader'
// @ts-ignore - Node.js path
import { dirname, join } from 'path'
// @ts-ignore - Node.js url
import { fileURLToPath } from 'url'

// Get API key from environment - tests will be skipped if not set
// @ts-ignore
const OPENROUTER_API_KEY = process.env.OPENROUTER_API_KEY

// Skip all tests if no API key is provided
const describeWithApiKey = OPENROUTER_API_KEY ? describe : describe.skip

// Get __dirname equivalent for ES modules
const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

const MODELS = ['google/gemini-2.5-flash', 'anthropic/claude-haiku-4.5', 'openai/gpt-4o']
const VARIANTS = [
	...MODELS.map((model) => ({
		...BASELINE_VARIANT,
		model,
		name: `baseline-${model.replace('/', '-')}`
	})),
	...MODELS.map((model) => ({
		...STREAMLINED_VARIANT,
		model,
		name: `streamlined-${model.replace('/', '-')}`
	}))
]

describeWithApiKey('App Chat LLM Evaluation', () => {
	const TEST_TIMEOUT = 120_000
	if (!OPENROUTER_API_KEY) {
		console.warn('OPENROUTER_API_KEY is not set, skipping tests')
	}

	it(
		'test1: creates a simple counter app',
		async () => {
			const USER_PROMPT = `Create a counter app with increment/decrement buttons`
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!)
			// Write results to files
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
			// Load initial app from fixture folder
			const { initialFrontend, initialBackend } = await loadAppFixtureForEval(
				join(__dirname, 'initial', 'test1_counter_app')
			)

			const USER_PROMPT = `Add a reset button that sets the counter back to 0`
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!, {
				initialFrontend,
				initialBackend
			})
			// Write results to files
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
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!, {
				initialFrontend,
				initialBackend
			})

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
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!, {
				initialFrontend,
				initialBackend
			})

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
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!, {
				initialFrontend,
				initialBackend
			})

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
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!, {
				initialFrontend,
				initialBackend
			})

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
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!, {
				initialFrontend,
				initialBackend
			})

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
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!)

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
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!)

			const { summaryPath, appPaths } = await writeAppComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`App files: ${appPaths.join(', ')}`)

			expect(true).toBe(true)
		},
		TEST_TIMEOUT
	)
})
