import type { FlowModule } from '$lib/gen'
import { evaluateWithLLM, BASE_EVALUATOR_RESPONSE_FORMAT } from '../shared'
import type { EvaluationResult } from '../shared'

/**
 * Expected flow structure for evaluation.
 */
export interface ExpectedFlow {
	summary?: string
	value: {
		modules: FlowModule[]
	}
	schema?: Record<string, any>
}

/**
 * Flow-specific evaluator system prompt.
 */
const FLOW_EVALUATOR_SYSTEM_PROMPT = `You are an expert evaluator for Windmill flow definitions. Your task is to evaluate a generated flow against:
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

${BASE_EVALUATOR_RESPONSE_FORMAT}`

/**
 * Evaluates how well a generated flow matches an expected flow and user request using an LLM.
 * Returns a resemblance score (0-100), a qualitative statement, and any missing requirements.
 */
export async function evaluateFlowComparison(
	generatedFlow: ExpectedFlow,
	expectedFlow: ExpectedFlow,
	userPrompt: string
): Promise<EvaluationResult> {
	// @ts-ignore
	const apiKey = process.env.OPENROUTER_API_KEY

	return evaluateWithLLM({
		userPrompt,
		generatedOutput: generatedFlow,
		expectedOutput: expectedFlow,
		evaluatorSystemPrompt: FLOW_EVALUATOR_SYSTEM_PROMPT,
		apiKey
	})
}
