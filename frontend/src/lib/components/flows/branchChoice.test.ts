import { describe, expect, it } from 'vitest'
import type { AiDecision } from '$lib/gen'
import { branchesForChoiceQuestion, decisionChoiceQuestions } from './branchChoice'

describe('branch on a choice question', () => {
	const decision = {
		type: 'aidecision',
		input_transforms: {
			provider: { type: 'static', value: { kind: 'typesafe', resource: '', model: 'jev-latest' } },
			state: { type: 'javascript', expr: 'flow_input.text' },
			questions: {
				type: 'static',
				value: {
					intent: { type: 'choice', criteria: { refund: 'Money back', "it's broken": 'A bug' } },
					'how bad': { type: 'choice', criteria: { low: 'Minor', high: 'Severe' } },
					urgent: { type: 'noul' }
				}
			}
		}
	} as AiDecision

	it('tests each option the way the engine reads the answer, skipping options already tested', () => {
		const [intent, howBad] = decisionChoiceQuestions(decision)
		expect(decisionChoiceQuestions(decision).map((q) => q.name)).toEqual(['intent', 'how bad'])

		const branches = branchesForChoiceQuestion(intent, [])
		expect(branches.map((b) => [b.summary, b.expr])).toEqual([
			['refund', `previous_result.output.intent.choice === "refund"`],
			["it's broken", `previous_result.output.intent.choice === "it's broken"`]
		])
		expect(branchesForChoiceQuestion(intent, branches.slice(0, 1))).toEqual(branches.slice(1))
		expect(branchesForChoiceQuestion(howBad, [])[0].expr).toBe(
			`previous_result.output["how bad"].choice === "low"`
		)
	})
})
