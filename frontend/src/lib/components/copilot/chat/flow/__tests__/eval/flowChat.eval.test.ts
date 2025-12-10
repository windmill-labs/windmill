import { describe, it, expect } from 'vitest'
import { runVariantComparison, type ExpectedFlow } from './evalRunner'
import { writeComparisonResults } from './evalResultsWriter'
import { BASELINE_VARIANT, MINIMAL_SINGLE_TOOL_VARIANT } from './variants'
// @ts-ignore - JSON import
import expectedTest1 from './expected/test1.json'
// @ts-ignore - JSON import
import expectedTest2 from './expected/test2.json'
// @ts-ignore - JSON import
import expectedTest3 from './expected/test3.json'
// @ts-ignore - JSON import
import expectedTest4 from './expected/test4.json'
// @ts-ignore - JSON import
import expectedTest5 from './expected/test5_modify_simple.json'
// @ts-ignore - JSON import
import expectedTest6 from './expected/test6_modify_medium.json'
// @ts-ignore - JSON import
import expectedTest7 from './expected/test7_modify_complex.json'
// @ts-ignore - JSON import
import initialTest5 from './initial/test5_initial.json'
// @ts-ignore - JSON import
import initialTest6 from './initial/test6_initial.json'
// @ts-ignore - JSON import
import initialTest7 from './initial/test7_initial.json'
import type { FlowModule } from '$lib/gen'

// Get API key from environment - tests will be skipped if not set
// @ts-ignore
// const OPENAI_API_KEY = process.env.OPENAI_API_KEY
const OPENROUTER_API_KEY = process.env.OPENROUTER_API_KEY

// Skip all tests if no API key is provided
// const describeWithApiKey = OPENAI_API_KEY ? describe : describe.skip
const describeWithApiKey = OPENROUTER_API_KEY ? describe : describe.skip

const MODELS = ['google/gemini-2.5-flash', 'anthropic/claude-haiku-4.5', 'openai/gpt-4o']

const VARIANTS = [
	...MODELS.map((model) => ({
		...BASELINE_VARIANT,
		model,
		name: `baseline-${model.replace('/', '-')}`
	})),
	...MODELS.map((model) => ({
		...MINIMAL_SINGLE_TOOL_VARIANT,
		model,
		name: `minimal-single-tool-${model.replace('/', '-')}`
	}))
]

describeWithApiKey('Flow Chat LLM Evaluation', () => {
	const TEST_TIMEOUT = 120_000
	if (!OPENROUTER_API_KEY) {
		console.warn('OPENROUTER_API_KEY is not set, skipping tests')
	}

	it(
		'test1: user role-based actions with loop and branches',
		async () => {
			const USER_PROMPT = `
THIS IS A TEST, CODE SHOULD BE MINIMAL FUNCTIONING CODE, IF WE NEED RETURN VALUES RETURN EXAMPLE VALUES

STEP 1: Fetch mock users from api
STEP 2: Filter only active users:
STEP 3: Loop on all users
STEP 4: Do branches based on user's role, do different action based on that. Roles are admin, user, moderator
STEP 5: Return action taken for each user
`
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!, {
				expectedFlow: expectedTest1 as ExpectedFlow
			})

			// Write results to files
			const { summaryPath, flowPaths } = await writeComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`Flow files: ${flowPaths.join(', ')}`)

			// Assert all variants succeeded
			for (const result of results) {
				expect(true).toBe(true)

				// Log evaluation results
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

	it(
		'test2: e-commerce order processing with inventory check and branching',
		async () => {
			const USER_PROMPT = `
THIS IS A TEST, CODE SHOULD BE MINIMAL FUNCTIONING CODE, IF WE NEED RETURN VALUES RETURN EXAMPLE VALUES

STEP 1: Receive order data from input (order has items array with name/price/quantity, customer_email, shipping_address)
STEP 2: Validate order - check all items have valid price > 0 and quantity > 0, return validation result
STEP 3: Calculate order total with 8% tax rate
STEP 4: Check inventory for each item (loop through items, return mock availability)
STEP 5: Branch based on inventory - if all items available, create shipment record; otherwise create backorder record
STEP 6: Send confirmation (mock email to customer_email)
STEP 7: Return final order summary with status
`
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!, {
				expectedFlow: expectedTest2 as ExpectedFlow
			})

			const { summaryPath, flowPaths } = await writeComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`Flow files: ${flowPaths.join(', ')}`)

			for (const result of results) {
				expect(true).toBe(true)

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

	it(
		'test3: data pipeline with parallel processing and quality-based routing',
		async () => {
			const USER_PROMPT = `
THIS IS A TEST, CODE SHOULD BE MINIMAL FUNCTIONING CODE, IF WE NEED RETURN VALUES RETURN EXAMPLE VALUES

STEP 1: Fetch list of data sources from configuration (return mock array of 3 source objects with id and url)
STEP 2: For each data source in parallel:
  - Fetch raw data from the source (mock fetch returning sample records)
  - Transform/clean the data (filter out invalid entries)
  - Validate the transformed data (return validation score 0-100)
STEP 3: Aggregate all validated data into single dataset with combined records
STEP 4: Calculate overall data quality score (average of all validation scores)
STEP 5: Branch based on quality score:
  - If score >= 90: Store in primary database and return success
  - If score >= 70 and < 90: Store in secondary database with warning flag
  - If score < 70: Store in quarantine and send alert
STEP 6: Return processing report with statistics (total records, quality score, destination)
`
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!, {
				expectedFlow: expectedTest3 as ExpectedFlow
			})

			const { summaryPath, flowPaths } = await writeComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`Flow files: ${flowPaths.join(', ')}`)

			for (const result of results) {
				expect(true).toBe(true)

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

	it(
		'test4: AI agent with tools for customer support',
		async () => {
			const USER_PROMPT = `
THIS IS A TEST, CODE SHOULD BE MINIMAL FUNCTIONING CODE, IF WE NEED RETURN VALUES RETURN EXAMPLE VALUES

Create a customer support flow with an AI agent:

STEP 1: Receive customer query from input (customer_id string, query_text string)
STEP 2: Fetch customer profile and order history (mock data based on customer_id)
STEP 3: Use an AI agent to handle the customer query. The agent should have access to these tools:
  - lookup_order: Takes order_id, returns order details (mock data)
  - check_refund_eligibility: Takes order_id, returns eligibility status and reason
  - create_support_ticket: Takes description and priority (low/medium/high), returns ticket_id
  - search_faq: Takes search_query, returns relevant FAQ answers
  The agent should use the customer profile context and respond helpfully.
STEP 4: Log the interaction to audit trail (customer_id, query, response summary)
STEP 5: Return the agent's response and any actions taken
`
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!, {
				expectedFlow: expectedTest4 as ExpectedFlow
			})

			const { summaryPath, flowPaths } = await writeComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`Flow files: ${flowPaths.join(', ')}`)

			for (const result of results) {
				expect(true).toBe(true)

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

	// ==================== MODIFICATION TESTS ====================
	// These tests evaluate the LLM's ability to modify existing flows

	it(
		'test5: simple modification - add validation step to existing flow',
		async () => {
			const USER_PROMPT = `
THIS IS A TEST, CODE SHOULD BE MINIMAL FUNCTIONING CODE, IF WE NEED RETURN VALUES RETURN EXAMPLE VALUES

Modify this existing flow to add error handling:
- Add a new step after process_data called "validate_data" to validate the processed data
- The validation step should check if the data array is not empty
- If validation fails (empty array), it should return an error object with message "No data to save"
- If validation passes, return the data for the next step
- Update save_results to handle the validation result appropriately
`
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!, {
				initialModules: initialTest5.value.modules as FlowModule[],
				initialSchema: initialTest5.schema,
				expectedFlow: expectedTest5 as ExpectedFlow
			})

			const { summaryPath, flowPaths } = await writeComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`Flow files: ${flowPaths.join(', ')}`)

			for (const result of results) {
				expect(true).toBe(true)

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

	it(
		'test6: medium modification - add branching inside existing loop',
		async () => {
			const USER_PROMPT = `
THIS IS A TEST, CODE SHOULD BE MINIMAL FUNCTIONING CODE, IF WE NEED RETURN VALUES RETURN EXAMPLE VALUES

Modify the order processing loop to handle different order types:
- Inside the loop_orders, replace the simple process_order step with branching based on order.type
- For type "express": add a step called handle_express that marks as priority and calculates express shipping cost ($15.99)
- For type "standard": add a step called handle_standard that calculates standard shipping cost ($5.99)
- For type "pickup": add a step called handle_pickup that marks as no shipping required (cost $0)
- Move the original process_order step to the default branch for unknown order types
- Each branch step should return the orderId, shipping cost, and shipping type
`
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!, {
				initialModules: initialTest6.value.modules as FlowModule[],
				initialSchema: initialTest6.schema,
				expectedFlow: expectedTest6 as ExpectedFlow
			})

			const { summaryPath, flowPaths } = await writeComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`Flow files: ${flowPaths.join(', ')}`)

			for (const result of results) {
				expect(true).toBe(true)

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

	it(
		'test7: complex modification - refactor sequential to parallel execution',
		async () => {
			const USER_PROMPT = `
THIS IS A TEST, CODE SHOULD BE MINIMAL FUNCTIONING CODE, IF WE NEED RETURN VALUES RETURN EXAMPLE VALUES

Refactor this flow for better performance by parallelizing the enrichment steps:
- The three enrichment steps (enrich_price, enrich_inventory, enrich_reviews) currently run sequentially
- Wrap them in a parallel branch (branchall) called "parallel_enrichment" so they run concurrently
- Each enrichment step should include basic error handling with try/catch that returns a fallback value if it fails
- Update the combine_data step to receive results from the parallel branch (results.parallel_enrichment returns an array of branch results)
- The combine_data step should check if any enrichment used a fallback value and set a hasFallbacks flag
- Keep get_item as the first step and return_result as the last step unchanged
`
			const results = await runVariantComparison(USER_PROMPT, VARIANTS, OPENROUTER_API_KEY!, {
				initialModules: initialTest7.value.modules as FlowModule[],
				initialSchema: initialTest7.schema,
				expectedFlow: expectedTest7 as ExpectedFlow
			})

			const { summaryPath, flowPaths } = await writeComparisonResults(USER_PROMPT, results)
			console.log(`\nResults written to: ${summaryPath}`)
			console.log(`Flow files: ${flowPaths.join(', ')}`)

			for (const result of results) {
				expect(true).toBe(true)

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
})
