import { describe, expect, it } from 'vitest'
import type { FlowModule, OpenFlow } from '$lib/gen'
import {
	checkRouting,
	choiceBranches,
	choiceConditionExpr,
	parseChoiceCondition
} from './aiDecisionBranching'
import { questionsFitRows, questionsToRows, rowsToQuestions } from './aiDecisionQuestions'

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

function withBranches(branches: { expr: string; modules: FlowModule[] }[]): [OpenFlow, FlowModule] {
	const f = flow(branches)
	return [f, f.value.modules[1]]
}

describe('choice conditions', () => {
	it('reads back the conditions it writes, even for names that are not identifiers', () => {
		for (const c of [
			{ decisionId: 'd', question: 'intent', kind: 'choice' as const, option: 'refund' },
			{ decisionId: 'd', question: 'the intent', kind: 'choice' as const, option: `it's "odd"` },
			{ decisionId: 'd', question: 'angry', kind: 'noul' as const, option: 'yes' }
		]) {
			expect(parseChoiceCondition(choiceConditionExpr(c))).toEqual(c)
		}
		expect(parseChoiceCondition('results.d.output.intent.choice !== "refund"')).toBeUndefined()
		expect(parseChoiceCondition("results.d.output.intent.choice === 'it\\'s'")).toEqual({
			decisionId: 'd',
			question: 'intent',
			kind: 'choice',
			option: "it's"
		})
		// A tuned threshold still reads as the question's yes branch.
		expect(parseChoiceCondition('results.d.output.angry.noul > 0.75')).toEqual({
			decisionId: 'd',
			question: 'angry',
			kind: 'noul',
			option: 'yes'
		})
	})

	it('flags the options a branch is missing and the branches whose option is gone', () => {
		const f = flow([
			...choiceBranches('d', 'intent', 'choice', ['refund', 'cancel']),
			{ expr: 'flow_input.x', modules: [] }
		])
		expect(checkRouting(f, f.value.modules[1])).toEqual([
			{
				decisionId: 'd',
				question: 'intent',
				kind: 'choice',
				missing: ['bug'],
				stale: [{ index: 1, option: 'cancel' }]
			}
		])
	})
})

describe('a removed question', () => {
	it('leaves every branch routing on it stale', () => {
		const f = flow(choiceBranches('d', 'gone', 'choice', ['refund']))
		expect(checkRouting(f, f.value.modules[1])).toEqual([
			{
				decisionId: 'd',
				question: 'gone',
				kind: 'choice',
				missing: [],
				stale: [{ index: 0, option: 'refund' }]
			}
		])
	})

	it('leaves a yes branch stale once its question is no longer a yes/no', () => {
		expect(checkRouting(...withBranches(choiceBranches('d', 'angry', 'noul', ['yes'])))).toEqual([])
		expect(checkRouting(...withBranches(choiceBranches('d', 'intent', 'noul', ['yes'])))).toEqual([
			{
				decisionId: 'd',
				question: 'intent',
				kind: 'choice',
				missing: ['refund', 'bug'],
				stale: [{ index: 0, option: 'yes' }]
			}
		])
	})
})

describe('questions editor rows', () => {
	it('round-trips every question type', () => {
		expect(rowsToQuestions(questionsToRows(questions))).toEqual(questions)
		expect(questionsFitRows(questions)).toBe(true)
	})

	it('leaves structured instructions to the JSON editor', () => {
		expect(
			questionsFitRows({ q: { type: 'noul', instructions: { ask: 'Is it urgent?', context: [] } } })
		).toBe(false)
	})
})
