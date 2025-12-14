import OpenAI from 'openai'

export interface EvalComparisonResult {
	success: boolean
	resemblanceScore: number
	statement: string
	missingRequirements?: string[]
	error?: string
}

interface ExpectedFlow {
	summary?: string
	value: {
		modules: unknown[]
	}
	schema?: unknown
}

const EVALUATOR_SYSTEM_PROMPT = `You are an expert evaluator for Windmill flow definitions. Your task is to evaluate a generated flow against:
1. The original user request/prompt
2. An expected reference flow

## Windmill Flow Context
- Flows consist of modules (steps) that execute sequentially
- Module types include: rawscript, forloopflow, branchone, branchall, script, flow, aiagent
- Each module has an id, value (containing type and config), and may have input_transforms
- input_transforms connect modules using expressions like "results.previous_step". Valid input_transforms are: static, javascript. Valid variables in javascript expressions are: results, flow_input, flow_input.iter.value (for forloopflow), flow_input.iter.index (for forloopflow).
- forloopflow contains nested modules that execute per iteration with access to flow_input.iter.value
- branchone executes first matching branch, branchall executes all matching branches
- Branches have conditional expressions (expr) that determine execution
- aiagent modules contain tools array with tool definitions

## Evaluation Criteria
1. **User Request Fulfillment**: Does the generated flow address ALL requirements from the user's original prompt?
   - Are all requested steps present?
   - Are the requested features implemented (loops, branches, specific logic)?
   - Does the schema match what the user requested for inputs?
2. **Structure**: Are the module types and nesting structure appropriate for the task?
3. **Logic**: Does the flow accomplish the intended logical task?
4. **Connections**: Are input_transforms connecting data correctly between steps?
5. **Completeness**: Are all required steps present with no major omissions?
6. **Code Quality**: Is the code functionally correct (exact syntax doesn't need to match)?

## Important Notes
- Minor differences in variable names, code formatting, or exact wording are acceptable
- Focus on functional equivalence, not character-by-character matching
- The generated flow should achieve the same outcome as described in the user request
- Extra helper steps or slightly different approaches can still score high if they accomplish the goal
- If the user requested specific module types (like aiagent), verify they are used correctly

## Response Format
You MUST respond with valid JSON only, no additional text:
{
  "resemblanceScore": <0-100 integer>,
  "statement": "<brief 1-2 sentence summary of how well the flow matches the user request and expected flow>",
  "missingRequirements": ["<list any requirements from user prompt that are missing or incorrectly implemented>"]
}

Score guidelines:
- 90-100: Fully addresses user request, functionally equivalent to expected flow
- 70-89: Addresses most user requirements, same overall structure with minor differences
- 50-69: Partially addresses user request, achieves similar goal but different approach
- 30-49: Missing significant requirements from user request
- 0-29: Does not address user request or significantly incorrect`

/**
 * Evaluates how well a generated flow matches an expected flow and user request using an LLM.
 * Returns a resemblance score (0-100), a qualitative statement, and any missing requirements.
 */
export async function evaluateFlowComparison(
	generatedFlow: ExpectedFlow,
	expectedFlow: ExpectedFlow,
	userPrompt: string
): Promise<EvalComparisonResult> {
	const model = 'anthropic/claude-sonnet-4.5'
	// @ts-ignore
	const apiKey = process.env.OPENROUTER_API_KEY
	const client = new OpenAI({ baseURL: 'https://openrouter.ai/api/v1', apiKey })

	const userMessage = `## User's Original Request
${userPrompt}

## Expected Reference Flow
\`\`\`json
${JSON.stringify(expectedFlow, null, 2)}
\`\`\`

## Generated Flow
\`\`\`json
${JSON.stringify(generatedFlow, null, 2)}
\`\`\`

Please evaluate how well the generated flow:
1. Fulfills ALL requirements from the user's original request
2. Matches the structure and logic of the expected reference flow`

	try {
		const response = await client.chat.completions.create({
			model,
			messages: [
				{ role: 'system', content: EVALUATOR_SYSTEM_PROMPT },
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
