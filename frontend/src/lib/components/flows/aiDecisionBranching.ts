import { getContext, setContext } from 'svelte'
import type { BranchOne, FlowModule, OpenFlow } from '$lib/gen'
import { branchableQuestions, type BranchKind } from './aiDecisionQuestions'
import { getAllModules } from './flowExplorer'

/** A Branch to one condition on a decision's question, in the forms the branch helper writes and
 *  reads back: one option of a choice, or the `yes` of a yes/no question above a threshold. A
 *  condition written any other way is left alone. */
export type ChoiceCondition = {
	decisionId: string
	question: string
	kind: BranchKind
	option: string
}

/** The one branch the helper writes for a yes/no question; the default branch is its "no". */
export const YES = 'yes'
const YES_THRESHOLD = 0.5

const IDENT = /^[A-Za-z_$][\w$]*$/

function key(k: string): string {
	return IDENT.test(k) ? `.${k}` : `[${JSON.stringify(k)}]`
}

const KEY = String.raw`(?:\.([A-Za-z_$][\w$]*)|\[("(?:[^"\\]|\\.)*")\])`
const CONDITION = new RegExp(
	String.raw`^results${KEY}\.output${KEY}\.choice === ("(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')$`
)
// Any threshold: the helper writes 0.5, and people tune it.
const NOUL_CONDITION = new RegExp(
	String.raw`^results${KEY}\.output${KEY}\.noul >=? (?:0|1|0?\.\d+)$`
)

/** A JS string literal as the helper writes it (double quotes) or as people and the AI chat
 *  usually do (single quotes). */
function stringLiteral(literal: string): string {
	return literal.startsWith("'")
		? literal.slice(1, -1).replace(/\\(.)/g, '$1')
		: JSON.parse(literal)
}

export function choiceConditionExpr({
	decisionId,
	question,
	kind,
	option
}: ChoiceCondition): string {
	const answer = `results${key(decisionId)}.output${key(question)}`
	return kind === 'noul'
		? `${answer}.noul >= ${YES_THRESHOLD}`
		: `${answer}.choice === ${JSON.stringify(option)}`
}

export function parseChoiceCondition(expr: string | undefined): ChoiceCondition | undefined {
	const trimmed = expr?.trim()
	try {
		const m = trimmed?.match(CONDITION)
		if (m) {
			return {
				decisionId: m[1] ?? JSON.parse(m[2]),
				question: m[3] ?? JSON.parse(m[4]),
				kind: 'choice',
				option: stringLiteral(m[5])
			}
		}
		const n = trimmed?.match(NOUL_CONDITION)
		if (n) {
			return {
				decisionId: n[1] ?? JSON.parse(n[2]),
				question: n[3] ?? JSON.parse(n[4]),
				kind: 'noul',
				option: YES
			}
		}
	} catch {
		return undefined
	}
	return undefined
}

/** The questions of a decision step a Branch to one can route on, when they are set statically. */
export function decisionChoiceQuestions(
	decision: FlowModule
): { name: string; kind: BranchKind; options: string[] }[] {
	if (decision.value.type !== 'aidecision') return []
	const questions = decision.value.input_transforms?.questions
	return questions?.type === 'static' ? branchableQuestions(questions.value) : []
}

function conditionsOf(routing: FlowModule): (ChoiceCondition | undefined)[] {
	return routing.value.type === 'branchone'
		? routing.value.branches.map((b) => parseChoiceCondition(b.expr))
		: []
}

/** The Branch to one that routes on this question, if any. */
export function findRouting(
	flow: OpenFlow,
	decisionId: string,
	question: string
): FlowModule | undefined {
	return getAllModules(flow.value.modules, flow.value.failure_module).find((m) =>
		conditionsOf(m).some((c) => c?.decisionId === decisionId && c.question === question)
	)
}

/** Options of the question no branch of `routing` handles yet. */
export function missingOptions(
	routing: FlowModule,
	decisionId: string,
	question: string,
	options: string[]
): string[] {
	const handled = new Set(
		conditionsOf(routing)
			.filter((c) => c?.decisionId === decisionId && c.question === question)
			.map((c) => c!.option)
	)
	return options.filter((o) => !handled.has(o))
}

export function choiceBranches(
	decisionId: string,
	question: string,
	kind: BranchKind,
	options: string[]
) {
	return options.map((option) => ({
		summary: option,
		expr: choiceConditionExpr({ decisionId, question, kind, option }),
		modules: [] as FlowModule[]
	}))
}

export type RoutingCheck = {
	decisionId: string
	question: string
	kind: BranchKind
	missing: string[]
	/** Branches, by index, whose option the question no longer offers. */
	stale: { index: number; option: string }[]
}

/** For each decision question a Branch to one routes on, what its branches lack and what they
 *  handle that the question no longer offers; a question removed from the decision offers nothing.
 *  Questions set from an expression are skipped: their options are only known at run time. */
export function checkRouting(flow: OpenFlow, routing: FlowModule): RoutingCheck[] {
	const conditions = conditionsOf(routing)
	const modules = getAllModules(flow.value.modules, flow.value.failure_module)
	const pairs = new Map<string, { decisionId: string; question: string }>()
	for (const c of conditions) {
		if (c) pairs.set(JSON.stringify([c.decisionId, c.question]), c)
	}
	const checks: RoutingCheck[] = []
	for (const { decisionId, question } of pairs.values()) {
		const decision = modules.find((m) => m.id === decisionId)
		if (
			decision?.value.type !== 'aidecision' ||
			decision.value.input_transforms?.questions?.type !== 'static'
		) {
			continue
		}
		const asked = decisionChoiceQuestions(decision).find((q) => q.name === question)
		const options = asked?.options ?? []
		// A branch also goes stale when its question changes kind, a choice turned yes/no or back.
		const stale = conditions.flatMap((c, index) =>
			c?.decisionId === decisionId &&
			c.question === question &&
			(c.kind !== asked?.kind || !options.includes(c.option))
				? [{ index, option: c.option }]
				: []
		)
		const missing = asked ? missingOptions(routing, decisionId, question, options) : []
		if (missing.length || stale.length) {
			checks.push({ decisionId, question, kind: asked?.kind ?? 'choice', missing, stale })
		}
	}
	return checks
}

export function addChoiceBranches(
	routing: FlowModule,
	decisionId: string,
	question: string,
	kind: BranchKind,
	options: string[]
) {
	;(routing.value as BranchOne).branches.push(
		...choiceBranches(decisionId, question, kind, options)
	)
}

const DECISION_STEP = Symbol('aiDecisionStep')

/** The AI decision step whose questions are being edited, for the questions editor to offer
 *  branching on them. Its `id` is undefined where branching does not apply: a decision used as
 *  an agent tool, or a test form. */
export type AiDecisionStepContext = { readonly id: string | undefined }

export function setAiDecisionStep(step: AiDecisionStepContext): void {
	setContext(DECISION_STEP, step)
}

export function getAiDecisionStep(): AiDecisionStepContext | undefined {
	return getContext(DECISION_STEP)
}

/** Option names as the editor cites them in a sentence: quoted, so a name reads as a value. */
export function quoteOptions(options: string[]): string {
	return options.map((o) => `“${o}”`).join(', ')
}
