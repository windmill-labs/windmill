import { JobService, type GetFlowAllResultsResponse } from '$lib/gen'

/**
 * Model-facing view of a flow run's execution tree for the global chat's
 * get_flow_run_details tool. The backend endpoint (get_flow_all_results)
 * enumerates every job of the tree with per-entry truncated results; this
 * module shapes that flat list into a compact per-step tree the model can
 * read in one tool result. Step addresses ('b/c', 'b[12]/c') are resolved
 * server-side by the same endpoint for full-result drill-down.
 */

export type FlowResultEntry = GetFlowAllResultsResponse['entries'][number]

/** Per-entry result budget requested from the server for the tree view. */
export const TREE_RESULT_HEAD_CHARS = 700
/** Cap on a drilled single-step full result handed to the model. */
export const STEP_RESULT_MAX_CHARS = 12000
/** Cap on the whole rendered tree; heads shrink progressively to fit. */
const TREE_TOTAL_BUDGET_CHARS = 20000
/** Failed loop iterations shown with a result head (the rest are indices only). */
const MAX_FAILED_ITERATIONS_SHOWN = 3
/** Head-shrink ladder tried in order until the rendered tree fits the budget.
 * `successHead` caps results of succeeded steps separately so failure detail
 * survives longest. */
const SHRINK_LADDER = [
	{ head: TREE_RESULT_HEAD_CHARS, successHead: TREE_RESULT_HEAD_CHARS },
	{ head: 250, successHead: 250 },
	{ head: 250, successHead: 0 },
	{ head: 100, successHead: 0 }
]

export interface FlowTreeNode {
	entry: FlowResultEntry
	children: FlowTreeNode[]
}

/** Rebuild the parent/child tree from the server's depth-first flat list. */
export function buildFlowTree(entries: FlowResultEntry[]): FlowTreeNode | undefined {
	let root: FlowTreeNode | undefined
	const stack: FlowTreeNode[] = []
	for (const entry of entries) {
		const node: FlowTreeNode = { entry, children: [] }
		while (stack.length && stack[stack.length - 1].entry.depth >= entry.depth) {
			stack.pop()
		}
		const parent = stack[stack.length - 1]
		if (parent) {
			parent.children.push(node)
		} else if (!root) {
			root = node
		}
		stack.push(node)
	}
	return root
}

interface ChildGroup {
	stepId: string
	nodes: FlowTreeNode[]
}

/** Group sibling jobs by flow step id (loop iterations / branches of one step
 * form one group). Keyed rather than run-length so ordering quirks between
 * parallel iterations can't split a step into two groups. */
function groupChildren(children: FlowTreeNode[]): ChildGroup[] {
	const groups = new Map<string, ChildGroup>()
	for (const child of children) {
		const stepId = child.entry.flow_step_id ?? `job:${child.entry.job_id}`
		let group = groups.get(stepId)
		if (!group) {
			group = { stepId, nodes: [] }
			groups.set(stepId, group)
		}
		group.nodes.push(child)
	}
	return Array.from(groups.values())
}

interface ShapeOpts {
	head: number
	successHead: number
}

function shapeResult(
	entry: FlowResultEntry,
	opts: ShapeOpts
): { result?: string; result_total_chars?: number } {
	if (entry.result_prefix === undefined || entry.result_prefix === null) return {}
	const budget = entry.success ? opts.successHead : opts.head
	if (budget <= 0) return { result_total_chars: entry.result_length ?? undefined }
	const head = entry.result_prefix.slice(0, budget)
	const total = entry.result_length ?? entry.result_prefix.length
	return {
		result: head,
		...(total > head.length ? { result_total_chars: total } : {})
	}
}

function shapeStep(node: FlowTreeNode, opts: ShapeOpts, iteration?: number): Record<string, any> {
	const entry = node.entry
	const shaped: Record<string, any> = {
		...(entry.step_path ? { step: entry.step_path } : {}),
		...(iteration !== undefined ? { iteration } : {}),
		label: entry.label,
		job_id: entry.job_id,
		status: entry.status,
		...(entry.duration_ms !== undefined && entry.duration_ms !== null
			? { duration_ms: entry.duration_ms }
			: {}),
		...shapeResult(entry, opts)
	}
	const childSteps = shapeChildren(node.children, opts)
	if (childSteps.length > 0) {
		shaped.steps = childSteps
	}
	return shaped
}

function shapeGroup(group: ChildGroup, opts: ShapeOpts): Record<string, any> {
	if (group.nodes.length === 1 && group.nodes[0].entry.sibling_count <= 1) {
		return shapeStep(group.nodes[0], opts)
	}

	const first = group.nodes[0].entry
	if (first.parent_module_type === 'branchall') {
		return {
			step: first.step_path ?? group.stepId,
			type: 'branchall',
			branches: group.nodes.map((n) => shapeStep(n, opts))
		}
	}

	// Loop-like fan-out (forloopflow, whileloopflow, aiagent actions, or any
	// other multi-job step): tally statuses, show failed iterations (capped) and
	// the latest one, elide the rest.
	const byIndex = [...group.nodes].sort((a, b) => a.entry.sibling_index - b.entry.sibling_index)
	const ok = byIndex.filter((n) => n.entry.status === 'success').length
	const failedNodes = byIndex.filter(
		(n) => n.entry.status === 'failure' || n.entry.status === 'canceled'
	)
	const unfinished = byIndex.length - ok - failedNodes.length

	const shown = failedNodes.slice(0, MAX_FAILED_ITERATIONS_SHOWN)
	const last = byIndex[byIndex.length - 1]
	if (!shown.includes(last)) {
		shown.push(last)
	}

	return {
		step: first.step_path ?? group.stepId,
		type: first.parent_module_type || 'loop',
		iterations: byIndex.length,
		ok,
		...(failedNodes.length > 0
			? { failed_iterations: failedNodes.map((n) => n.entry.sibling_index) }
			: {}),
		...(unfinished > 0 ? { unfinished } : {}),
		iterations_shown: shown.map((n) => shapeStep(n, opts, n.entry.sibling_index)),
		...(byIndex.length > shown.length ? { iterations_elided: byIndex.length - shown.length } : {})
	}
}

function shapeChildren(children: FlowTreeNode[], opts: ShapeOpts): Record<string, any>[] {
	return groupChildren(children).map((g) => shapeGroup(g, opts))
}

function renderTree(
	root: FlowTreeNode,
	rootJobNote: string | undefined,
	opts: ShapeOpts
): Record<string, any> {
	const run = shapeStep(root, opts)
	const steps = run.steps
	delete run.steps
	// The backend labels every depth-0 job "Flow"; correct that for the graceful
	// non-flow case so the model doesn't mistake a plain script run for a flow.
	if (!steps && root.entry.kind !== 'flow' && root.entry.kind !== 'flowpreview') {
		run.label = `Job (${root.entry.kind})`
	}
	return {
		...(rootJobNote ? { note: rootJobNote } : {}),
		run,
		...(steps ? { steps } : {}),
		hint: `Results are truncated. Call get_flow_run_details again with step="<step>" (e.g. "b/c", or "b[12]" for one loop iteration) for a step's full result.`
	}
}

/** Render the whole tree, shrinking result heads until it fits the budget. */
export function shapeFlowRunTree(response: GetFlowAllResultsResponse): string {
	const root = buildFlowTree(response.entries)
	if (!root) {
		return 'No jobs found for this run.'
	}
	const rootJobNote = response.root_job
		? `This job is a step of a larger flow run — its enclosing run is ${response.root_job}; pass that id to see more of the tree.`
		: undefined

	let rendered = ''
	for (const opts of SHRINK_LADDER) {
		rendered = JSON.stringify(renderTree(root, rootJobNote, opts), null, 1)
		if (rendered.length <= TREE_TOTAL_BUDGET_CHARS) {
			return rendered
		}
	}
	return (
		rendered.slice(0, TREE_TOTAL_BUDGET_CHARS) +
		`\n… (tree truncated at ${TREE_TOTAL_BUDGET_CHARS} chars — drill into specific steps with the step parameter)`
	)
}

/** Entry point of the get_flow_run_details tool. Without `step`: the compact
 * tree. With `step`: that job's full (capped) result, resolved server-side. */
export async function getFlowRunDetails(
	workspace: string,
	id: string,
	step?: string
): Promise<string> {
	if (!step) {
		return shapeFlowRunTree(
			await JobService.getFlowAllResults({ workspace, id, maxResultLen: TREE_RESULT_HEAD_CHARS })
		)
	}

	// Drill-down: the server resolves the address directly (a few indexed
	// lookups, no tree enumeration) and returns the single job as an entry.
	const response = await JobService.getFlowAllResults({
		workspace,
		id,
		maxResultLen: STEP_RESULT_MAX_CHARS,
		step
	})
	if (response.step_error) {
		return response.step_error
	}
	const entry = response.entries[0]
	if (!entry) {
		return 'No jobs found for this run.'
	}
	if (entry.status === 'running' || entry.status === 'queued' || entry.status === 'suspended') {
		return `Step "${step}" (job ${entry.job_id}) has not completed yet — status: ${entry.status}.`
	}
	if (entry.result_prefix === undefined || entry.result_prefix === null) {
		return `Step "${step}" (job ${entry.job_id}, ${entry.status}) has no recorded result.`
	}
	const total = entry.result_length ?? entry.result_prefix.length
	const capped =
		total > entry.result_prefix.length
			? entry.result_prefix + `\n… (result truncated: ${total} chars total)`
			: entry.result_prefix
	return `Step "${step}" (job ${entry.job_id}, ${entry.status}) result:\n${capped}`
}
