import OpenAI from 'openai'

export interface EvalComparisonResult {
	success: boolean
	resemblanceScore: number
	statement: string
	error?: string
}

interface ExpectedFlow {
	summary?: string
	value: {
		modules: unknown[]
	}
	schema?: unknown
}

const EVALUATOR_SYSTEM_PROMPT = `You are an expert evaluator for Windmill flow definitions. Your task is to compare a generated flow against an expected flow and assess how well they match.

## Windmill Flow Context
- Flows consist of modules (steps) that execute sequentially
- Module types include: rawscript, forloopflow, branchone, branchall, script, flow
- Each module has an id, value (containing type and config), and may have input_transforms
- input_transforms connect modules using expressions like "results.previous_step"
- forloopflow contains nested modules that execute per iteration with access to flow_input.iter.value
- branchone executes first matching branch, branchall executes all matching branches
- Branches have conditional expressions (expr) that determine execution

## Evaluation Criteria
1. Structure: Are the module types and nesting structure similar? (e.g., loop containing branch)
2. Logic: Does the flow accomplish the same logical task?
3. Connections: Are input_transforms connecting data correctly between steps?
4. Completeness: Are all required steps present?
5. Code Quality: Is the code functionally equivalent (exact syntax doesn't need to match)?

## Important Notes
- Minor differences in variable names, code formatting, or exact wording are acceptable
- Focus on functional equivalence, not character-by-character matching
- The generated flow should achieve the same outcome as the expected flow
- Extra helper steps or slightly different approaches can still score high if they accomplish the goal

## Response Format
You MUST respond with valid JSON only, no additional text:
{
  "resemblanceScore": <0-100 integer>,
  "statement": "<brief 1-2 sentence summary of how well they match>",
}

Score guidelines:
- 90-100: Functionally equivalent, minor cosmetic differences only
- 70-89: Same overall structure, some implementation differences
- 50-69: Achieves similar goal but different approach
- 30-49: Partially matches, missing significant parts
- 0-29: Significantly different or incorrect`

/**
 * Evaluates how well a generated flow matches an expected flow using an LLM.
 * Returns a resemblance score (0-100) and a qualitative statement.
 */
export async function evaluateFlowComparison(
	generatedFlow: ExpectedFlow,
	expectedFlow: ExpectedFlow
): Promise<EvalComparisonResult> {
	const model = 'openai/gpt-4o'
	// @ts-ignore
	const apiKey = process.env.OPENROUTER_API_KEY
	const client = new OpenAI({ baseURL: 'https://openrouter.ai/api/v1', apiKey })

	const userMessage = `## Expected Flow
\`\`\`json
${JSON.stringify(expectedFlow, null, 2)}
\`\`\`

## Generated Flow
\`\`\`json
${JSON.stringify(generatedFlow, null, 2)}
\`\`\`

Please evaluate how well the generated flow matches the expected flow.`

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
		}

		return {
			success: true,
			resemblanceScore: Math.max(0, Math.min(100, Math.round(parsed.resemblanceScore))),
			statement: parsed.statement
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
