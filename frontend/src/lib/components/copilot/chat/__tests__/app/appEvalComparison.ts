import Anthropic from '@anthropic-ai/sdk'
import type { AppFiles, BackendRunnable } from '../../app/core'
import { BASE_EVALUATOR_RESPONSE_FORMAT } from '../shared'
import type { EvaluationResult } from '../shared'

/**
 * Expected app structure for evaluation.
 */
export interface ExpectedApp {
	frontend: Record<string, string>
	backend: Record<string, BackendRunnable>
}

/**
 * Initial app state for evaluation context.
 */
export interface InitialApp {
	frontend: Record<string, string>
	backend: Record<string, BackendRunnable>
}

/**
 * System prompt for evaluating app generation without a reference expected app.
 * Evaluates based on user request fulfillment and appropriate modifications to initial state.
 */
const APP_GENERATION_EVALUATOR_SYSTEM_PROMPT = `You are an expert evaluator for Windmill Raw App definitions. Your task is to evaluate a generated app based on:
1. The original user request/prompt
2. The initial app state (if any) - this is what the app looked like before the AI made changes

## Windmill Raw App Context
- Raw Apps consist of frontend files and backend runnables
- Frontend files are TypeScript/JavaScript files bundled with esbuild (entrypoint: index.tsx)
- Backend runnables can be: inline scripts (TypeScript/Python), workspace scripts, workspace flows, or hub scripts
- Frontend calls backend using \`await backend.<runnable_key>(args...)\`
- Each backend runnable has a key (identifier), name (description), type, and configuration

## Backend Runnable Types
- **inline**: Custom code with \`inlineScript.language\` and \`inlineScript.content\`
- **script**: Workspace script reference with \`path\`
- **flow**: Workspace flow reference with \`path\`
- **hubscript**: Hub script reference with \`path\`

## Evaluation Criteria
1. **User Request Fulfillment**: Does the generated app address ALL requirements from the user's original prompt?
   - Are all requested features implemented?
   - Does the frontend UI match the requirements?
   - Are the correct backend runnables created?
2. **Appropriate Modifications** (if initial app was provided):
   - Were the changes made relevant to the user's request?
   - Was existing functionality preserved where appropriate?
   - Were only necessary changes made (no unnecessary removals or additions)?
3. **Frontend Structure**: Are the frontend files correctly organized and implemented?
   - Is the code valid TypeScript/JavaScript?
   - Are components properly structured?
   - Are backend calls correctly made?
4. **Backend Structure**: Are the backend runnables correctly configured?
   - Do inline scripts have proper main functions?
   - Are types and paths correct for non-inline runnables?
5. **Integration**: Does the frontend correctly call the backend?
   - Are the runnable keys correctly referenced?
   - Are arguments passed correctly?
6. **Code Quality**: Is the code functionally correct and well-structured?

## Important Notes
- Focus on whether the user's request was fulfilled, not on stylistic preferences
- If an initial app was provided, evaluate the appropriateness of the changes made
- For new apps (no initial state), evaluate completeness and correctness
- Extra helper functions or slightly different approaches can still score high if they accomplish the goal

${BASE_EVALUATOR_RESPONSE_FORMAT}`

/**
 * Evaluates how well a generated app fulfills the user's request, considering any initial app state.
 * Uses Anthropic API directly.
 */
export async function evaluateAppGeneration(
	userPrompt: string,
	generatedApp: AppFiles,
	initialApp?: InitialApp
): Promise<EvaluationResult> {
	// @ts-ignore
	const apiKey = process.env.ANTHROPIC_API_KEY
	if (!apiKey) {
		return {
			success: false,
			resemblanceScore: 0,
			statement: 'No API key available for evaluation',
			error: 'ANTHROPIC_API_KEY not set'
		}
	}

	const client = new Anthropic({ apiKey })

	let userMessage = `## User's Original Request
${userPrompt}

`

	if (initialApp) {
		userMessage += `## Initial App State (before AI modifications)
\`\`\`json
${JSON.stringify(initialApp, null, 2)}
\`\`\`

`
	} else {
		userMessage += `## Initial App State
No initial app was provided - this is a new app created from scratch.

`
	}

	userMessage += `## Generated App
\`\`\`json
${JSON.stringify(generatedApp, null, 2)}
\`\`\`

Please evaluate how well the generated app:
1. Fulfills ALL requirements from the user's original request
2. ${initialApp ? 'Makes appropriate modifications to the initial app state' : 'Implements a complete and correct new app'}`

	try {
		const response = await client.messages.create({
			model: 'claude-sonnet-4-6',
			max_tokens: 2048,
			system: APP_GENERATION_EVALUATOR_SYSTEM_PROMPT,
			messages: [
				{ role: 'user', content: userMessage }
			],
			temperature: 0
		})

		const textBlock = response.content.find((block) => block.type === 'text')
		const content = textBlock?.text
		if (!content) {
			return {
				success: false,
				resemblanceScore: 0,
				statement: 'No response from evaluator',
				error: 'Empty response from LLM'
			}
		}

		// Parse JSON response - handle potential markdown code blocks
		let jsonContent = content.trim()
		if (jsonContent.startsWith('```')) {
			jsonContent = jsonContent.replace(/^```(?:json)?\n?/, '').replace(/\n?```$/, '')
		}

		const parsed = JSON.parse(jsonContent) as {
			resemblanceScore: number
			statement: string
			missingRequirements?: string[]
		}

		return {
			success: true,
			resemblanceScore: Math.max(0, Math.min(100, Math.round(parsed.resemblanceScore))),
			statement: parsed.statement,
			missingRequirements: parsed.missingRequirements ?? []
		}
	} catch (err) {
		const errorMessage = err instanceof Error ? err.message : String(err)
		return {
			success: false,
			resemblanceScore: 0,
			statement: 'Evaluation failed',
			error: errorMessage
		}
	}
}
