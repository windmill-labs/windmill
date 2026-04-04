import { evaluateWithLLM, BASE_EVALUATOR_RESPONSE_FORMAT } from '../shared'
import type { EvaluationResult } from '../shared'
import type { ScriptEvalState } from './fileHelpers'

const SCRIPT_EVALUATOR_SYSTEM_PROMPT = `You are an expert evaluator for Windmill script generation.

Your task is to evaluate a generated script against:
1. The original user request/prompt
2. An expected reference script

## Windmill Script Context
- Scripts should usually export a \`main\` function unless the request clearly requires a \`preprocessor\`
- The generated output may contain extra imports, helper functions, or comments
- Minor differences in formatting, variable names, or extra safe guards are acceptable
- Focus on whether the code fulfills the requested behavior and has the correct overall structure

## Evaluation Criteria
1. **User Request Fulfillment**: Does the generated script address the requested behavior?
2. **Entrypoint Correctness**: Does it export the right main entrypoint and shape?
3. **Functional Equivalence**: Does it implement the same logic as the expected script, even if syntax differs?
4. **Windmill Fit**: Is the script appropriate for a Windmill script context?

${BASE_EVALUATOR_RESPONSE_FORMAT}`

export async function evaluateScriptComparison(
	generatedScript: ScriptEvalState,
	expectedScript: ScriptEvalState,
	userPrompt: string
): Promise<EvaluationResult> {
	return evaluateWithLLM({
		userPrompt,
		generatedOutput: generatedScript,
		expectedOutput: expectedScript,
		evaluatorSystemPrompt: SCRIPT_EVALUATOR_SYSTEM_PROMPT
	})
}
