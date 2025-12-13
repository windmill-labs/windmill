import OpenAI from 'openai'
import type { EvaluationResult } from './types'

/**
 * Parameters for LLM-based evaluation.
 */
export interface EvaluateParams {
	/** The user's original request/prompt */
	userPrompt: string
	/** The generated output to evaluate */
	generatedOutput: unknown
	/** The expected/reference output */
	expectedOutput: unknown
	/** Domain-specific system prompt for the evaluator */
	evaluatorSystemPrompt: string
	/** API key for OpenRouter */
	apiKey: string
	/** Model to use for evaluation (default: 'anthropic/claude-sonnet-4.5') */
	model?: string
}

/**
 * Base evaluator system prompt template.
 * Domain-specific evaluators should build on this structure.
 */
export const BASE_EVALUATOR_RESPONSE_FORMAT = `
## Response Format
You MUST respond with valid JSON only, no additional text:
{
  "resemblanceScore": <0-100 integer>,
  "statement": "<brief 1-2 sentence summary of how well the output matches the user request and expected output>",
  "missingRequirements": ["<list any requirements from user prompt that are missing or incorrectly implemented>"]
}

Score guidelines:
- 90-100: Fully addresses user request, functionally equivalent to expected output
- 70-89: Addresses most user requirements, same overall structure with minor differences
- 50-69: Partially addresses user request, achieves similar goal but different approach
- 30-49: Missing significant requirements from user request
- 0-29: Does not address user request or significantly incorrect`

/**
 * Evaluates how well a generated output matches an expected output using an LLM.
 * Returns a resemblance score (0-100), a qualitative statement, and any missing requirements.
 *
 * @param params Evaluation parameters including prompts, outputs, and API configuration
 * @returns Evaluation result with score, statement, and missing requirements
 */
export async function evaluateWithLLM(params: EvaluateParams): Promise<EvaluationResult> {
	const {
		userPrompt,
		generatedOutput,
		expectedOutput,
		evaluatorSystemPrompt,
		apiKey,
		model = 'anthropic/claude-sonnet-4.5'
	} = params

	const client = new OpenAI({ baseURL: 'https://openrouter.ai/api/v1', apiKey })

	const userMessage = `## User's Original Request
${userPrompt}

## Expected Reference Output
\`\`\`json
${JSON.stringify(expectedOutput, null, 2)}
\`\`\`

## Generated Output
\`\`\`json
${JSON.stringify(generatedOutput, null, 2)}
\`\`\`

Please evaluate how well the generated output:
1. Fulfills ALL requirements from the user's original request
2. Matches the structure and logic of the expected reference output`

	try {
		const response = await client.chat.completions.create({
			model,
			messages: [
				{ role: 'system', content: evaluatorSystemPrompt },
				{ role: 'user', content: userMessage }
			],
			temperature: 0
		})

		const content = response.choices[0]?.message?.content
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
			// Remove markdown code block wrapper
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
