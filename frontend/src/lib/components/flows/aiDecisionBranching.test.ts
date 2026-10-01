import { describe, expect, it } from 'vitest'
import type { FlowModule, OpenFlow } from '$lib/gen'
import {
	checkRouting,
	choiceBranches,
	choiceConditionExpr,
	parseChoiceCondition
} from './aiDecisionBranching'
import { questionsToRows, rowsToQuestions } from './aiDecisionQuestions'

const questions = {
	intent: {
		type: 'choice',
		instructions: 'What does the customer want?',
		criteria: { refund: 'Money back', bug: 'Something is broken' }
	},
	urgency: { type: 'score', instructions: 'How urgent?', criteria: ['Later', 'Now'] },
	angry: { type: 'noul', instructions: 'Is the customer angry?' }
}

function flow(branches: { expr: string; modules: FlowModule[] }[]): OpenFlow {
	return {
		summary: '',
		value: {
			modules: [
				{
					id: 'd',
					value: {
						type: 'aidecision',
						input_transforms: {
							provider: { type: 'static', value: {} },
							state: { type: 'static', value: 'x' },
							questions: { type: 'static', value: questions }
						}
					}
				},
				{ id: 'route', value: { type: 'branchone', branches, default: [] } }
			]
		}
	} as OpenFlow
}

describe('choice conditions', () => {
	it('reads back the conditions it writes, even for names that are not identifiers', () => {
		for (const c of [
			{ decisionId: 'd', question: 'intent', option: 'refund' },
			{ decisionId: 'd', question: 'the intent', option: `it's "odd"` }
		]) {
			expect(parseChoiceCondition(choiceConditionExpr(c))).toEqual(c)
		}
		expect(parseChoiceCondition('results.d.output.intent.choice !== "refund"')).toBeUndefined()
	})

	it('flags the options a branch is missing and the branches whose option is gone', () => {
		const f = flow([
			...choiceBranches('d', 'intent', ['refund', 'cancel']),
			{ expr: 'flow_input.x', modules: [] }
		])
		expect(checkRouting(f, f.value.modules[1])).toEqual([
			{
				decisionId: 'd',
				question: 'intent',
				missing: ['bug'],
				stale: [{ index: 1, option: 'cancel' }]
			}
		])
	})
})

describe('questions editor rows', () => {
	it('round-trips every question type', () => {
		expect(rowsToQuestions(questionsToRows(questions))).toEqual(questions)
	})
})
