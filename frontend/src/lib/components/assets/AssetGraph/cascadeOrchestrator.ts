import type { DownstreamClosure, InducedSchedule } from './graphTraversal'

// Client-side orchestration of a pipeline chain dev-run ("Run + downstream"
// over a graph that contains drafts). The backend asset-trigger dispatcher
// only resolves deployed rows, so draft chains are executed from here: jobs
// run in topological order, each hop launched explicitly (callers pass
// `_wmill_skip_asset_dispatch` so the backend dispatcher never double-fires
// the deployed part of a mixed chain). Data still flows through real storage
// — an upstream preview materializes its table/object and the downstream
// re-reads it — so no values are passed between hops.
//
// This is a dev-time test action, not the production cascade: event-time
// semantics (debounce, retry, AND-join slots, chain-depth cap) intentionally
// don't apply. The schedule is the plain topological execution of the
// closure: a node runs once all its in-closure upstreams succeeded.
//
// Pure module (no Svelte runes) so the scheduling is unit-testable; reactive
// progress is delivered via `onUpdate` snapshots.

export type CascadeNodeStatus = 'pending' | 'running' | 'success' | 'failure' | 'skipped'

export type CascadeNodeState = {
	status: CascadeNodeStatus
	jobId?: string
	error?: string
}

export type CascadeRunOptions = {
	/** Schedulable closure of `root` (computeDownstreamClosure). */
	closure: DownstreamClosure
	/** The script the user pressed Run on — executed first. */
	root: string
	/** Launch one script (preview for drafts, by-path for deployed); returns the job id. */
	launch: (path: string) => Promise<string>
	/** Resolve once the job reaches a terminal state. */
	waitTerminal: (jobId: string) => Promise<'success' | 'failure'>
	/** Snapshot of all node states, emitted on every transition. */
	onUpdate?: (statuses: Map<string, CascadeNodeState>) => void
}

export type CascadeRunResult = {
	/** True when every node in the closure (and the root) succeeded. */
	ok: boolean
	statuses: Map<string, CascadeNodeState>
}

/**
 * Execute root + its downstream closure. Independent branches run
 * concurrently; a failure stops *scheduling* (in-flight jobs are left to
 * finish and report) and everything not yet started ends as 'skipped'.
 */
export async function runCascade(opts: CascadeRunOptions): Promise<CascadeRunResult> {
	const { closure, root, launch, waitTerminal, onUpdate } = opts
	const statuses = new Map<string, CascadeNodeState>()
	statuses.set(root, { status: 'pending' })
	for (const n of closure.nodes) statuses.set(n, { status: 'pending' })
	const remaining = new Map(closure.indegree)
	let failed = false
	const inFlight = new Set<Promise<void>>()

	const emit = () => onUpdate?.(new Map(statuses))

	function schedule(path: string) {
		const p = runNode(path).finally(() => inFlight.delete(p))
		inFlight.add(p)
	}

	async function runNode(path: string): Promise<void> {
		statuses.set(path, { status: 'running' })
		emit()
		let jobId: string | undefined
		try {
			jobId = await launch(path)
			statuses.set(path, { status: 'running', jobId })
			emit()
			const term = await waitTerminal(jobId)
			statuses.set(path, { status: term, jobId })
			emit()
			if (term === 'failure') {
				failed = true
				return
			}
		} catch (e) {
			statuses.set(path, {
				status: 'failure',
				jobId,
				error: e instanceof Error ? e.message : String(e)
			})
			emit()
			failed = true
			return
		}
		// Success: unlock subscribers whose in-closure upstreams are now all
		// done. Synchronous decrements — exactly one upstream hits zero.
		for (const s of closure.edges.get(path) ?? []) {
			const d = (remaining.get(s) ?? 0) - 1
			remaining.set(s, d)
			if (d === 0 && !failed) schedule(s)
		}
	}

	schedule(root)
	while (inFlight.size > 0) {
		await Promise.race(inFlight)
	}

	for (const [n, st] of statuses) {
		if (st.status === 'pending') statuses.set(n, { status: 'skipped' })
	}
	emit()

	const ok = !failed && [...statuses.values()].every((s) => s.status === 'success')
	return { ok, statuses }
}

export type SelectionRunOptions = {
	/** Multi-root induced schedule of the selected scripts (computeInducedSchedule). */
	schedule: InducedSchedule
	/** Launch one script (preview for drafts, by-path for deployed); returns the job id. */
	launch: (path: string) => Promise<string>
	/** Resolve once the job reaches a terminal state. */
	waitTerminal: (jobId: string) => Promise<'success' | 'failure'>
	/** Snapshot of all node states, emitted on every transition. */
	onUpdate?: (statuses: Map<string, CascadeNodeState>) => void
}

/**
 * Execute an arbitrary selected set of scripts (e.g. a bounded-cascade
 * selection) in topological order. Unlike `runCascade` there is no single
 * privileged root — every `schedule.roots` entry is seeded at once and a node
 * runs as soon as its in-set upstreams all succeed. A failure abandons only
 * that node's lineage (its transitive descendants end 'skipped'); INDEPENDENT
 * branches keep running. In-flight jobs always finish.
 */
export async function runSelection(opts: SelectionRunOptions): Promise<CascadeRunResult> {
	const { schedule, launch, waitTerminal, onUpdate } = opts
	const statuses = new Map<string, CascadeNodeState>()
	for (const n of schedule.nodes) statuses.set(n, { status: 'pending' })
	const remaining = new Map(schedule.indegree)
	let failed = false
	// Nodes a failed prerequisite has made unrunnable — the transitive descendants
	// of every failure. Gating scheduling on this (rather than a single global
	// fail-fast flag) is what lets independent branches finish: only the lineage
	// below a failure is skipped, not every node that happens to become ready
	// afterwards. Poisoned nodes are never scheduled, so they end 'skipped' below.
	const poisoned = new Set<string>()
	const inFlight = new Set<Promise<void>>()

	const emit = () => onUpdate?.(new Map(statuses))

	function poison(path: string) {
		const stack = [path]
		while (stack.length > 0) {
			const n = stack.pop()!
			for (const s of schedule.edges.get(n) ?? []) {
				if (!poisoned.has(s)) {
					poisoned.add(s)
					stack.push(s)
				}
			}
		}
	}

	function schedule_(path: string) {
		const p = runNode(path).finally(() => inFlight.delete(p))
		inFlight.add(p)
	}

	async function runNode(path: string): Promise<void> {
		statuses.set(path, { status: 'running' })
		emit()
		let jobId: string | undefined
		try {
			jobId = await launch(path)
			statuses.set(path, { status: 'running', jobId })
			emit()
			const term = await waitTerminal(jobId)
			statuses.set(path, { status: term, jobId })
			emit()
			if (term === 'failure') {
				failed = true
				poison(path)
				return
			}
		} catch (e) {
			statuses.set(path, {
				status: 'failure',
				jobId,
				error: e instanceof Error ? e.message : String(e)
			})
			emit()
			failed = true
			poison(path)
			return
		}
		for (const s of schedule.edges.get(path) ?? []) {
			const d = (remaining.get(s) ?? 0) - 1
			remaining.set(s, d)
			if (d === 0 && !poisoned.has(s)) schedule_(s)
		}
	}

	for (const r of schedule.roots) schedule_(r)
	while (inFlight.size > 0) {
		await Promise.race(inFlight)
	}

	for (const [n, st] of statuses) {
		if (st.status === 'pending') statuses.set(n, { status: 'skipped' })
	}
	emit()

	const ok = !failed && [...statuses.values()].every((s) => s.status === 'success')
	return { ok, statuses }
}
