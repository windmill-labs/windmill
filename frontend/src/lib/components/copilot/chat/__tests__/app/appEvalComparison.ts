import type { AppFiles, BackendRunnable } from '../../app/core'
import { evaluateWithLLM, BASE_EVALUATOR_RESPONSE_FORMAT } from '../shared'
import type { EvaluationResult } from '../shared'

/**
 * Expected app structure for evaluation.
 */
export interface ExpectedApp {
	frontend: Record<string, string>
	backend: Record<string, BackendRunnable>
}

/**
 * App-specific evaluator system prompt.
 */
const APP_EVALUATOR_SYSTEM_PROMPT = `You are an expert evaluator for Windmill Raw App definitions. Your task is to evaluate a generated app against:
1. The original user request/prompt
2. An expected reference app

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
2. **Frontend Structure**: Are the frontend files correctly organized and implemented?
   - Is the code valid TypeScript/JavaScript?
   - Are components properly structured?
   - Are backend calls correctly made?
3. **Backend Structure**: Are the backend runnables correctly configured?
   - Do inline scripts have proper main functions?
   - Are types and paths correct for non-inline runnables?
4. **Integration**: Does the frontend correctly call the backend?
   - Are the runnable keys correctly referenced?
   - Are arguments passed correctly?
5. **Code Quality**: Is the code functionally correct and well-structured?

## Important Notes
- Minor differences in variable names, code formatting, or exact wording are acceptable
- Focus on functional equivalence, not character-by-character matching
- The generated app should achieve the same outcome as described in the user request
- Extra helper functions or slightly different approaches can still score high if they accomplish the goal

${BASE_EVALUATOR_RESPONSE_FORMAT}`

/**
 * Evaluates how well a generated app matches an expected app and user request using an LLM.
 * Returns a resemblance score (0-100), a qualitative statement, and any missing requirements.
 */
export async function evaluateAppComparison(
	generatedApp: AppFiles,
	expectedApp: ExpectedApp,
	userPrompt: string
): Promise<EvaluationResult> {
	// @ts-ignore
	const apiKey = process.env.OPENROUTER_API_KEY

	return evaluateWithLLM({
		userPrompt,
		generatedOutput: generatedApp,
		expectedOutput: expectedApp,
		evaluatorSystemPrompt: APP_EVALUATOR_SYSTEM_PROMPT,
		apiKey
	})
}
