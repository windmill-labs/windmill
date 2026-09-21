import { JobService, type Job, type GetFlowAllResultsResponse } from '$lib/gen'
import { isWindmillTooBigObject } from '$lib/components/job_args'

/**
 * Model-facing view of a run for the global chat's get_run tool: the job's own
 * summary, args, result and logs, plus — when the run has steps — its execution
 * tree. The backend endpoint (get_flow_all_results) enumerates every job of the
 * tree with per-entry truncated results; this module shapes that flat list into
 * a compact per-step tree the model can read in one tool result. Step addresses
 * ('b/c', 'b[12]/c') are resolved server-side by the same endpoint for
 * full-result drill-down.
 *
 * `summarizeRun` also backs list_runs' per-job summary, so it lives here rather
 * than in core.ts: get_run needs it, and importing it back would be circular.
 */

export type FlowResultEntry = GetFlowAllResultsResponse['entries'][number]

/** Per-entry result budget requested from the server for the tree view. */
export const TREE_RESULT_HEAD_CHARS = 700
/** Cap on a full payload handed to the model: a drilled step result, and the
 * run's own args, result and logs. */
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

/** Postgres LEFT()/length() count code points while JS .length/.slice count
 * UTF-16 units — compare and cut in code points so astral characters neither
 * hide a truncation nor get split through a surrogate pair. */
function countCodePoints(s: string): number {
	let n = 0
	for (const _ of s) n++
	return n
}

function sliceCodePointSafe(s: string, maxUnits: number): string {
	const cut = s.slice(0, maxUnits)
	const last = cut.charCodeAt(cut.length - 1)
	// drop a trailing lone high surrogate
	return last >= 0xd800 && last <= 0xdbff ? cut.slice(0, -1) : cut
}

function sliceCodePointSafeEnd(s: string, maxUnits: number): string {
	const cut = s.slice(-maxUnits)
	const first = cut.charCodeAt(0)
	// drop a leading lone low surrogate
	return first >= 0xdc00 && first <= 0xdfff ? cut.slice(1) : cut
}

function shapeResult(
	entry: FlowResultEntry,
	opts: ShapeOpts
): { result?: string; result_total_chars?: number } {
	if (entry.result_prefix === undefined || entry.result_prefix === null) return {}
	const budget = entry.success ? opts.successHead : opts.head
	if (budget <= 0) return { result_total_chars: entry.result_length ?? undefined }
	const head = sliceCodePointSafe(entry.result_prefix, budget)
	const total = entry.result_length ?? countCodePoints(entry.result_prefix)
	return {
		result: head,
		...(total > countCodePoints(head) ? { result_total_chars: total } : {})
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

/** Module types whose sibling jobs are iterations/branches; sibling jobs of
 * any other step are retry attempts of that step. */
const FAN_OUT_MODULE_TYPES = new Set([
	'forloopflow',
	'whileloopflow',
	'branchall',
	'branchone',
	'aiagent',
	'aidecision'
])

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

	const byIndex = [...group.nodes].sort((a, b) => a.entry.sibling_index - b.entry.sibling_index)

	if (first.parent_module_type && !FAN_OUT_MODULE_TYPES.has(first.parent_module_type)) {
		// Retried step: siblings are attempts of the same step, the last one is
		// the final outcome — render it as the step, keeping earlier attempts as
		// status-only references.
		const shaped = shapeStep(byIndex[byIndex.length - 1], opts)
		shaped.attempts = byIndex.length
		shaped.previous_attempts = byIndex.slice(0, -1).map((n) => ({
			attempt: n.entry.sibling_index,
			status: n.entry.status,
			job_id: n.entry.job_id
		}))
		return shaped
	}

	// Loop-like fan-out (forloopflow, whileloopflow, aiagent actions, or any
	// other multi-job step): tally statuses, show failed iterations (capped) and
	// the latest one, elide the rest.
	const ok = byIndex.filter((n) => n.entry.status === 'success').length
	const failedNodes = byIndex.filter(
		(n) => n.entry.status === 'failure' || n.entry.status === 'canceled'
	)
	const skipped = byIndex.filter((n) => n.entry.status === 'skipped').length
	// skipped is terminal — only running/queued/suspended count as unfinished
	const unfinished = byIndex.length - ok - failedNodes.length - skipped

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
		...(skipped > 0 ? { skipped } : {}),
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
	opts: ShapeOpts,
	runOverrides?: Record<string, unknown>
): Record<string, any> {
	const run = shapeStep(root, opts)
	const steps = run.steps
	delete run.steps
	// The backend labels every depth-0 job "Flow"; correct that for the graceful
	// non-flow case so the model doesn't mistake a plain script run for a flow.
	if (!steps && root.entry.kind !== 'flow' && root.entry.kind !== 'flowpreview') {
		run.label = `Job (${root.entry.kind})`
	}
	if (runOverrides) {
		// A replacement result supersedes the tree entry's head, and the stale
		// truncation marker must go with it. Without one the head stays: it is a
		// real server-side prefix of a payload `getJob` would only hand back as a
		// WINDMILL_TOO_BIG placeholder.
		if ('result' in runOverrides) {
			delete run.result
			delete run.result_total_chars
		}
		Object.assign(run, runOverrides)
	}
	return {
		...(rootJobNote ? { note: rootJobNote } : {}),
		run,
		...(steps
			? {
					steps,
					hint: `Step results are truncated. Call get_run again with step="<step>" (e.g. "b/c", or "b[12]" for one loop iteration) for a step's result in full (up to ${STEP_RESULT_MAX_CHARS} chars), or with id set to a step's job_id for that step's own args, result and logs.`
				}
			: {})
	}
}

/** Render the whole tree, shrinking result heads until it fits the budget. */
export function shapeFlowRunTree(
	response: GetFlowAllResultsResponse,
	runOverrides?: Record<string, unknown>
): string {
	const root = buildFlowTree(response.entries)
	if (!root) {
		// The tree is the optional half: what the caller already read of the job
		// still answers the question, so don't drop it with the missing tree.
		return runOverrides
			? JSON.stringify({ run: runOverrides }, null, 1)
			: 'No jobs found for this run.'
	}
	const notes = [
		...(response.enclosing_job
			? [
					`This job is a step of a larger flow run — its enclosing run is ${response.enclosing_job}; pass that id to see more of the tree.`
				]
			: []),
		...(response.truncated
			? [
					`The run has more jobs than the server returns — this tree only covers the first ${response.entries.length} (depth-first), so tallies may undercount.`
				]
			: []),
		...(response.scope_filtered
			? [`Your token is tag-scoped: steps running on other tags are omitted from this tree.`]
			: [])
	]
	const rootJobNote = notes.length > 0 ? notes.join(' ') : undefined

	// The budget caps the tree, not the run's own args/result/logs — those are
	// capped on their own and the model asked for them, so they don't count here
	// and the last-resort slice keeps room for them (`run` precedes `steps`).
	const overrideChars = runOverrides ? JSON.stringify(runOverrides, null, 1).length : 0
	let rendered = ''
	for (const opts of SHRINK_LADDER) {
		rendered = JSON.stringify(renderTree(root, rootJobNote, opts, runOverrides), null, 1)
		if (rendered.length - overrideChars <= TREE_TOTAL_BUDGET_CHARS) {
			return rendered
		}
	}
	return (
		rendered.slice(0, TREE_TOTAL_BUDGET_CHARS + overrideChars) +
		`\n… (tree truncated at ${TREE_TOTAL_BUDGET_CHARS} chars — drill into specific steps with the step parameter)`
	)
}

// Compact metadata for one run. The raw Job carries args/result/logs/raw_code
// which can be huge — this is only what's needed to identify a run, so list_runs
// can return one entry per job.
export function summarizeRun(job: Job): Record<string, unknown> {
	const base = {
		id: job.id,
		job_kind: job.job_kind,
		path: job.script_path,
		created_by: job.created_by,
		created_at: job.created_at,
		started_at: job.started_at,
		schedule_path: job.schedule_path,
		is_flow_step: job.is_flow_step,
		tag: job.tag,
		worker: job.worker
	}
	if ('success' in job) {
		// CompletedJob. `success` is true for a skipped job too, so is_skipped has
		// to be read first or a skipped step reports as a successful one.
		return {
			...base,
			status: job.canceled
				? 'canceled'
				: job.is_skipped
					? 'skipped'
					: job.success
						? 'success'
						: 'failure',
			duration_ms: job.duration_ms
		}
	}
	// QueuedJob. A running job with suspends outstanding is parked — on an approval
	// step or on a parallelism slot — not working, and `running` alone hides that.
	return {
		...base,
		status: job.running ? (job.suspend ? 'suspended' : 'running') : 'queued'
	}
}

/** Cap a payload to STEP_RESULT_MAX_CHARS. `tail` keeps the end instead of the
 * start — for logs, where the failure is at the bottom.
 *
 * Counts and cuts without materialising the string: logs arrive whole and
 * unbounded, and spreading one into an array of code points costs ~9x its size
 * (a 10MB log measured +90MB). Cutting in UTF-16 units keeps at most the budget
 * in code points, never more, so an astral-heavy payload is trimmed slightly
 * shorter than advertised rather than overshooting. */
function cap(text: string, tail = false): string {
	// UTF-16 length is never below the code-point count, so anything passing this
	// is already under the cap and needs no counting pass at all.
	if (text.length <= STEP_RESULT_MAX_CHARS) return text
	const total = countCodePoints(text)
	if (total <= STEP_RESULT_MAX_CHARS) return text
	const note = `… (truncated: ${total} chars total)`
	return tail
		? `${note}\n${sliceCodePointSafeEnd(text, STEP_RESULT_MAX_CHARS)}`
		: `${sliceCodePointSafe(text, STEP_RESULT_MAX_CHARS)}\n${note}`
}

/** Told apart from an empty log so the model doesn't report "no logs" for a run
 * whose logs it simply failed to read. */
const LOGS_UNREADABLE = 'Logs could not be read for this run.'

/** `getJob` swaps a payload over ~90KB for a marker rather than sending it
 * (`get_job_query!` in backend/windmill-api/src/jobs.rs), and the two fields use
 * different ones: a result becomes the bare string, args become exactly
 * `{reason: <marker>}`. Each field matches only its own form, so a payload that
 * merely carries that string in a `reason` of its own stays the run's value. */
const TOO_BIG_RESULT = 'WINDMILL_TOO_BIG'

function stringify(value: unknown): string {
	return typeof value === 'string' ? value : JSON.stringify(value, null, 1)
}

/** An elided args/result payload is reported, never fetched around: the
 * endpoints that return those whole take no length parameter, so recovering a
 * usable head would mean pulling the entire payload (up to MAX_RESULT_SIZE_MB,
 * 500 by default) into the tab to keep 12k of it. The flag is the whole report —
 * the result keeps the head the step tree already carries beside it. */
function shapeRunArgs(job: Job): Record<string, unknown> {
	if (!job.args) return {}
	return isWindmillTooBigObject(job.args)
		? { args_truncated: true }
		: { args: cap(stringify(job.args)) }
}

function shapeRunResult(job: Job): Record<string, unknown> {
	if (!('result' in job) || job.result === undefined) return {}
	// No `result` key when elided, so the tree's `result_prefix` — a real
	// server-side head of the same payload — stands in its place unoverridden.
	return job.result === TOO_BIG_RESULT
		? { result_truncated: true }
		: { result: cap(stringify(job.result)) }
}

/** Why a run ended badly. Kept out of summarizeRun, which list_runs pays per job. */
function diagnoseRun(job: Job): Record<string, unknown> {
	return {
		...(job.canceled_by ? { canceled_by: job.canceled_by } : {}),
		...(job.canceled_reason ? { canceled_reason: job.canceled_reason } : {}),
		...(job.mem_peak ? { mem_peak_kb: job.mem_peak } : {})
	}
}

/** What get_run answers with: the model's payload, and — when the call addresses one
 * job — that job's id, which the card renders the run from. The two are separate
 * audiences: `text` is capped for the model, `jobId` is how the user gets the whole
 * thing. An address resolving to several jobs (a loop's `b`), an unfinished step or an
 * unknown one carries no id, and the call renders as an ordinary tool row. */
export type RunInspection = { text: string; jobId?: string }

/** Entry point of the get_run tool. Without `step`: the run's summary, args,
 * result and logs, plus the per-step tree when the run has steps. With `step`:
 * that step's full (capped) result, resolved server-side. */
export async function getRun(workspace: string, id: string, step?: string): Promise<RunInspection> {
	if (!step) {
		// Only the job read is load-bearing: logs and the step tree each answer
		// part of the question, so neither failing should cost the model the rest.
		const [job, logs, results] = await Promise.all([
			JobService.getJob({ workspace, id, noLogs: true, noCode: true }),
			// The dedicated endpoint rather than the job's own `logs` field, which holds too
			// little to serve a tail — RunScriptCard's fetch has the mechanism. Only this
			// job's own logs either way: a flow's are its orchestration lines, not its steps'.
			JobService.getJobLogs({
				workspace,
				id,
				// Suppress the "to remove ansi colors, use: sed ..." hint the backend
				// otherwise prepends — noise for the model, and not actual stripping.
				removeAnsiWarnings: true
			}).catch(() => LOGS_UNREADABLE),
			// `.catch` alone would leave the tree half-guarded: like the log read, this
			// one can fail by resolving `undefined` rather than rejecting, and reading
			// `.entries` off that throws — losing the job and logs already in hand.
			JobService.getFlowAllResults({ workspace, id, maxResultLen: TREE_RESULT_HEAD_CHARS })
				.catch(() => undefined)
				.then((r) => r ?? ({ entries: [] } as GetFlowAllResultsResponse))
		])
		const { id: _id, ...summary } = summarizeRun(job)
		const payloads = { ...shapeRunArgs(job), ...shapeRunResult(job) }
		// A read that fails without rejecting still arrives here: the generated client
		// resolves `undefined` when it cannot read the body (a truncated response, a
		// dropped connection). Logs are the one payload where empty is a real answer,
		// so that has to be told apart from an empty log rather than reported as one.
		const shapedLogs =
			typeof logs !== 'string' || logs === LOGS_UNREADABLE
				? LOGS_UNREADABLE
				: logs.trim()
					? cap(logs, true)
					: 'No logs for this run.'
		return {
			text: shapeFlowRunTree(results, {
				...summary,
				...diagnoseRun(job),
				// A successful read always carries the job itself as the root entry, so
				// no entries means the read failed — and nothing else would name the run.
				...(results.entries.length === 0 ? { job_id: id, steps_unavailable: true } : {}),
				...payloads,
				logs: shapedLogs
			}),
			jobId: id
		}
	}

	return getStepResult(workspace, id, step)
}

/** One step's result in full, addressed by step path. The server resolves the
 * address directly (a few indexed lookups, no tree enumeration) and returns the
 * single job as an entry. */
async function getStepResult(workspace: string, id: string, step: string): Promise<RunInspection> {
	const response = await JobService.getFlowAllResults({
		workspace,
		id,
		maxResultLen: STEP_RESULT_MAX_CHARS,
		step
	})
	if (response.step_error) {
		return {
			text:
				response.step_error +
				(response.scope_filtered
					? ' (Steps running on tags outside your token’s scope are hidden.)'
					: '')
		}
	}
	const entry = response.entries[0]
	if (!entry) {
		return { text: 'No jobs found for this run.' }
	}
	if (entry.status === 'running' || entry.status === 'queued' || entry.status === 'suspended') {
		return {
			text: `Step "${step}" (job ${entry.job_id}) has not completed yet — status: ${entry.status}.`
		}
	}
	// Every completed step is a job of its own, so the card renders it from source —
	// including a skipped one, whose inputs and logs are all there is to see.
	if (entry.result_prefix === undefined || entry.result_prefix === null) {
		return {
			text: `Step "${step}" (job ${entry.job_id}, ${entry.status}) has no recorded result.`,
			jobId: entry.job_id
		}
	}
	const total = entry.result_length ?? countCodePoints(entry.result_prefix)
	const capped =
		total > countCodePoints(entry.result_prefix)
			? entry.result_prefix + `\n… (result truncated: ${total} chars total)`
			: entry.result_prefix
	return {
		text: `Step "${step}" (job ${entry.job_id}, ${entry.status}) result:\n${capped}`,
		jobId: entry.job_id
	}
}
