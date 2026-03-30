import { expect } from 'vitest'
import type { BaseEvalResult } from './types'

export function assertSuccessfulEvalRun<TOutput>(
	result: BaseEvalResult<TOutput>,
	options: { requireJudge?: boolean; minJudgeScore?: number } = {}
) {
	expect(result.success).toBe(true)
	expect(result.error).toBeUndefined()

	if (!options.requireJudge) {
		return
	}

	expect(result.evaluationResult).toBeDefined()
	expect(result.evaluationResult?.success).toBe(true)

	if (options.minJudgeScore !== undefined) {
		expect(result.evaluationResult?.resemblanceScore ?? 0).toBeGreaterThanOrEqual(
			options.minJudgeScore
		)
	}
}
