import type { BranchOne, FlowModule } from '$lib/gen'
import {
	findFlowNode,
	findModuleInFlow,
	findModuleParent,
	getChildModuleBranches,
	type FlowModuleTree
} from './flowTree'

// A step's error handler is recognised from the flow each time, never stored: the branchone
// right after a step with continue on error, one of whose branches tests that step's `.error`.
// Moving either step, or putting another between them, just ends the recognition.

/** How the graph marks an error handler's paths: the On success default, the error branches. */
export type BranchTone = 'success' | 'error'

export const ON_SUCCESS = 'On success'
export const ON_ERROR = 'On error'

/** The `results.<id>.error` fields a step that continued on error carries. */
const ERROR_FIELDS = ['message', 'name', 'stack', 'step_id'] as const

// `?.` on the step: a step that succeeds with no result reads as `results.<G> === null`, and
// `null.error` would throw, failing the branchone on the success path.
function errorRef(guardedId: string): string {
	return `results.${guardedId}?.error`
}

function escapeRegExp(s: string): string {
	return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

/** `.name`, `?.name` or a quoted `[name]`, the accessors the worker resolves on `results`. */
function accessSource(name: string): string {
	const n = escapeRegExp(name)
	return `(?:\\??\\.${n}|(?:\\?\\.)?\\[\\s*(?:"${n}"|'${n}')\\s*\\])`
}

/** `results.<G>.error` in any accessor form; the lookahead keeps `.errors` and `.error_count`
 *  from reading as the error. */
function errorRefSource(guardedId: string): string {
	return `(?<![\\w$])results${accessSource(guardedId)}${accessSource('error')}(?![\\w$])`
}

/** Whether a branch predicate is true on `<guardedId>`'s error: it reads `results.<G>.error`
 *  at least once without negating it (`!ref`, `ref == null`), which would test success. */
export function testsError(expr: string | undefined, guardedId: string): boolean {
	const re = new RegExp(
		`(!*)\\s*${errorRefSource(guardedId)}(\\s*[=!]==?\\s*(?:null|undefined)(?![\\w$]))?`,
		'g'
	)
	return [...(expr ?? '').matchAll(re)].some(([, bangs, comparison]) => {
		const equalsNothing = comparison !== undefined && comparison.trim().startsWith('=')
		return bangs.length % 2 === 1 ? equalsNothing : !equalsNothing
	})
}

function testsErrorOf(branchone: BranchOne, guardedId: string): boolean {
	return branchone.branches.some((b) => testsError(b.expr, guardedId))
}

/** The step list `stepId` sits in, or nothing when there is no "after this step": the flow's
 *  error handler, the preprocessor and agent tools. */
export function stepList(flow: FlowModuleTree, stepId: string) {
	const match = findFlowNode(flow, stepId)
	if (!match || !('container' in match) || match.location.type === 'aiagent') return undefined
	return { modules: match.container, index: match.location.index, module: match.module }
}

/** The branchone right after `stepId` that tests its error, whether or not the step continues
 *  on error. */
export function errorBranchesAfter(flow: FlowModuleTree, stepId: string): FlowModule | undefined {
	const list = stepList(flow, stepId)
	const next = list?.modules[list.index + 1]
	if (next?.value.type !== 'branchone') return undefined
	return testsErrorOf(next.value, stepId) ? next : undefined
}

/** The branchone right after `stepId` that handles its errors, if any. */
export function errorHandlerOf(flow: FlowModuleTree, stepId: string): FlowModule | undefined {
	const list = stepList(flow, stepId)
	if (!list?.module.continue_on_error) return undefined
	return errorBranchesAfter(flow, stepId)
}

const CONTAINERS: string[] = ['forloopflow', 'whileloopflow', 'branchone', 'branchall']

/** Whether the error handling shortcut applies to `stepId`: a step list position to insert
 *  after, and not a container, which run settings don't offer Continue on error. */
export function canAddErrorHandling(flow: FlowModuleTree, stepId: string): boolean {
	const type = stepList(flow, stepId)?.module.value.type
	return type !== undefined && !CONTAINERS.includes(type)
}

/** Whether a container around `stepId` lets the flow go on past its failure: one that continues
 *  on error, a loop that skips failures, or a branchall branch that skips failure. */
function failureAbsorbedAbove(flow: FlowModuleTree, stepId: string): boolean {
	let location = findModuleParent(flow, stepId)
	while (location && 'parentId' in location) {
		const parent = findModuleInFlow(flow, location.parentId)
		if (!parent) return false
		const v = parent.value
		if (parent.continue_on_error) return true
		if ((v.type === 'forloopflow' || v.type === 'whileloopflow') && v.skip_failures) return true
		if (v.type === 'branchall' && location.type === 'branchall-branch') {
			if (v.branches[location.branchIndex]?.skip_failure) return true
		}
		location = findModuleParent(flow, parent.id)
	}
	return false
}

/** Whether a failed `stepId` should point to the shortcut: its failure stopped the flow and no
 *  branches after it test its error. */
export function suggestsErrorHandling(flow: FlowModuleTree, stepId: string): boolean {
	if (!canAddErrorHandling(flow, stepId)) return false
	if (findModuleInFlow(flow, stepId)?.continue_on_error) return false
	if (failureAbsorbedAbove(flow, stepId)) return false
	return errorBranchesAfter(flow, stepId) === undefined
}

/** The id of the step whose errors `branchoneId` handles, if it is an error handler. */
export function guardedStepOf(flow: FlowModuleTree, branchoneId: string): string | undefined {
	const list = stepList(flow, branchoneId)
	const previous = list && list.index > 0 ? list.modules[list.index - 1] : undefined
	if (!previous) return undefined
	return errorHandlerOf(flow, previous.id)?.id === branchoneId ? previous.id : undefined
}

/** Every error handler in `flow`, by id, to the id of the step it guards: `guardedStepOf` for the
 *  whole flow in one walk. */
export function errorHandlerIds(flow: FlowModuleTree): Map<string, string> {
	const handlers = new Map<string, string>()
	const visit = (modules: FlowModule[]) =>
		modules.forEach((m, i) => {
			const previous = modules[i - 1]
			if (
				previous?.continue_on_error &&
				m.value.type === 'branchone' &&
				testsErrorOf(m.value, previous.id)
			) {
				handlers.set(m.id, previous.id)
			}
			// Agent tools are not a step list: no step comes after another.
			if (m.value.type !== 'aiagent') getChildModuleBranches(m).forEach(visit)
		})
	visit(flow.modules ?? [])
	return handlers
}

/** Index of the catch-all branch, a predicate true on any error of `<G>` and nothing else
 *  (`!!ref`, `ref`, `Boolean(ref)`, `ref != null`, …), or undefined. The first one wins: on
 *  error nothing after it can run. */
export function catchAllIndex(handler: FlowModule, guardedId: string): number | undefined {
	if (handler.value.type !== 'branchone') return undefined
	const ref = errorRefSource(guardedId)
	const catchAll = new RegExp(
		`^(?:(?:!!)?${ref}|Boolean\\(${ref}\\)|${ref}!=(?:null|undefined)|${ref}!==undefined)$`
	)
	const i = handler.value.branches.findIndex((b) => {
		let expr = (b.expr ?? '').replace(/\s+/g, '')
		while (/^\(.*\)$/.test(expr)) expr = expr.slice(1, -1)
		return catchAll.test(expr)
	})
	return i === -1 ? undefined : i
}

/** A granular case for an error handler: placed before the catch-all, so it can match first. */
export function errorCaseBranch(guardedId: string) {
	return { summary: '', expr: `${errorRef(guardedId)}?.name === ''`, modules: [] as FlowModule[] }
}

/** Where a new branch goes in an error handler of `guardedId`: before the catch-all. */
export function errorCaseIndex(handler: FlowModule, guardedId: string): number {
	if (handler.value.type !== 'branchone') return 0
	return catchAllIndex(handler, guardedId) ?? handler.value.branches.length
}

/** `results.<G>` as the predicate editor's autocompletion sees it: with the error fields a
 *  failure carries, next to whatever the step's last preview returned. */
export function withErrorShape(priorIds: Record<string, any>, guardedId: string) {
	const error = Object.fromEntries(ERROR_FIELDS.map((f) => [f, '']))
	const preview = priorIds[guardedId]
	const isObject = preview != null && typeof preview === 'object' && !Array.isArray(preview)
	return { ...priorIds, [guardedId]: { error, ...(isObject ? preview : {}) } }
}

/** Fill a freshly inserted branchone as the error handler of `guardedId`: an empty On success
 *  default and an empty On error catch-all. */
export function fillErrorHandler(branchone: FlowModule, guardedId: string): void {
	if (branchone.value.type !== 'branchone') return
	branchone.summary = 'Error handling'
	branchone.value.default_summary = ON_SUCCESS
	branchone.value.branches = [{ summary: ON_ERROR, expr: `!!${errorRef(guardedId)}`, modules: [] }]
}
