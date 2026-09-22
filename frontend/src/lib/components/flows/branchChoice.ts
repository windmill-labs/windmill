import type { AiDecision, BranchOne, FlowModule, FlowModuleValue } from '$lib/gen'

/** A step that runs the first branch whose condition holds: a branchone, or an AI decision. */
export type BranchChoiceValue = BranchOne | AiDecision
export type ChoiceBranch = BranchOne['branches'][number]

export function isBranchChoice(value: FlowModuleValue): value is BranchChoiceValue {
	return value.type === 'branchone' || value.type === 'aidecision'
}

// An AI decision leaves `branches` and `default` out until it has one, so reads go through these.
export function choiceBranches(value: BranchChoiceValue): ChoiceBranch[] {
	return value.branches ?? []
}

export function choiceDefault(value: BranchChoiceValue): FlowModule[] {
	return value.default ?? []
}

/** For writes: attaches the arrays an AI decision may not have yet. */
export function ensureChoiceArrays(value: BranchChoiceValue): {
	branches: ChoiceBranch[]
	default: FlowModule[]
} {
	value.branches ??= []
	value.default ??= []
	return value as { branches: ChoiceBranch[]; default: FlowModule[] }
}

/**
 * Whether the step lays out as branches. An AI decision does once it has any, which is also
 * when the engine runs one: with neither branches nor a default it is a single job.
 */
export function hasChoiceBranches(value: FlowModuleValue): value is BranchChoiceValue {
	return (
		value.type === 'branchone' ||
		(value.type === 'aidecision' &&
			((value.branches?.length ?? 0) > 0 || (value.default?.length ?? 0) > 0))
	)
}

/** `[default, ...branches]`, the order the graph lays them out and the run status indexes. */
export function choiceModuleArrays(value: BranchChoiceValue): FlowModule[][] {
	return [choiceDefault(value), ...choiceBranches(value).map((b) => b.modules)]
}

export type DecisionChoiceQuestion = { name: string; options: string[] }

/** The choice questions an AI decision asks, when they are set statically. */
export function decisionChoiceQuestions(value: AiDecision): DecisionChoiceQuestion[] {
	const questions = value.input_transforms?.questions
	if (questions?.type !== 'static' || !questions.value || typeof questions.value !== 'object') {
		return []
	}
	return Object.entries(questions.value as Record<string, any>)
		.filter(
			([, q]) =>
				q?.type === 'choice' &&
				q.criteria &&
				typeof q.criteria === 'object' &&
				!Array.isArray(q.criteria)
		)
		.map(([name, q]) => ({ name, options: Object.keys(q.criteria) }))
}

/** One branch per option of a choice question, leaving out the options a branch already tests. */
export function branchesForChoiceQuestion(
	question: DecisionChoiceQuestion,
	existing: ChoiceBranch[]
): ChoiceBranch[] {
	const answer = /^[A-Za-z_$][\w$]*$/.test(question.name)
		? `previous_result.output.${question.name}`
		: `previous_result.output[${JSON.stringify(question.name)}]`
	const tested = new Set(existing.map((b) => b.expr))
	return question.options
		.map((option) => ({
			summary: option,
			expr: `${answer}.choice === ${JSON.stringify(option)}`,
			modules: []
		}))
		.filter((branch) => !tested.has(branch.expr))
}
