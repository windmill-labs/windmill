import { describe } from 'vitest'

// Get API key from environment - tests will be skipped if not set
// @ts-ignore
const OPENROUTER_API_KEY = process.env.OPENROUTER_API_KEY

// Skip all tests if no API key is provided
const describeWithApiKey = OPENROUTER_API_KEY ? describe : describe.skip

describeWithApiKey('App Chat LLM Evaluation', () => {
	// Test cases to be added later
	//
	// Example structure:
	//
	// import { runVariantComparison, writeAppComparisonResults, type ExpectedApp } from './appEvalRunner'
	// import { BASELINE_VARIANT } from './variants'
	//
	// const TEST_TIMEOUT = 120_000
	//
	// it('test1: creates a simple counter app', async () => {
	//   const USER_PROMPT = `Create a counter app with increment/decrement buttons`
	//   const results = await runVariantComparison(USER_PROMPT, [BASELINE_VARIANT], OPENROUTER_API_KEY!, {
	//     expectedApp: expectedTest1 as ExpectedApp
	//   })
	//   // ... assertions
	// }, TEST_TIMEOUT)
})
