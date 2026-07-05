import type { AssetGraphResponse, NativeTriggerKind } from './types'
import { assetKey, isWriteEdge } from './lib'

// Bounded-cascade selective execution: instead of a dbt-style `--select`
// grammar (a compile-time file selector dbt needs because it has no runtime
// DAG), Windmill already runs a live cascade. So the primitive here is to
// *bound* that cascade — start at a pipeline entrypoint, fan downstream, but
// stop at one or more chosen end node(s). The matched set is the "path
// between" start and the ends:
//
//     descendants(start) ∩ (ancestors(ends) ∪ ends) ∪ {start}
//
// Pure module (no Svelte runes) so the graph algebra is unit-testable and can
// be ported verbatim to the CLI. See `boundedCascade.test.ts`, and the mirror
// at `cli/src/commands/pipeline/boundedCascade.ts` (keep them in sync).

// Unified lineage-DAG node ids. Assets use `${asset_kind}:${asset_path}`
// (identical to `assetKey`); runnables are prefixed so a script path can never
// collide with an asset key. Flows are excluded — they aren't cascade members
// (mirrors graphTraversal/asset_dispatch).
export const SCRIPT_PREFIX = 'script:'

export function scriptNodeId(path: string): string {
	return `${SCRIPT_PREFIX}${path}`
}
export function isScriptNode(id: string): boolean {
	return id.startsWith(SCRIPT_PREFIX)
}
export function scriptPathOf(id: string): string {
	return id.slice(SCRIPT_PREFIX.length)
}

/** Resolve an asset URI (`datatable://x`, `s3://b/k`, …) to its node id. */
export function assetUriToNodeId(uri: string): string | undefined {
	const m = uri.match(/^([a-z0-9_]+):\/\/(.+)$/i)
	if (!m) return undefined
	const prefix = m[1].toLowerCase()
	// `s3` is the URI prefix for the `s3object` asset kind (mirrors the CLI
	// `assetUri` and the canvas). All other kinds use their name verbatim.
	const kind = prefix === 's3' ? 's3object' : prefix
	return `${kind}:${m[2]}`
}

// Native trigger kinds that fan out *per event*: a single event always flows
// through the whole reactive downstream, so "run up to X now" is not a
// meaningful gesture and these are never offered as bounded-run starts.
// `webhook`/`data_upload` have no trigger row in `/assets/graph`, so a root
// whose only entry is one of those reads as a *manual* root below.
const EVENT_TRIGGER_KINDS: ReadonlySet<string> = new Set<NativeTriggerKind>([
	'kafka',
	'mqtt',
	'nats',
	'postgres',
	'sqs',
	'gcp',
	'email'
])

export type LineageDag = {
	/** upstream node id → set of direct downstream node ids. */
	down: Map<string, Set<string>>
	/** downstream node id → set of direct upstream node ids. */
	up: Map<string, Set<string>>
	/** Every node id (scripts + assets), including isolated ones. */
	nodes: Set<string>
}

/**
 * Build the directed upstream→downstream lineage DAG over scripts ∪ assets:
 *   - producer script → asset   (write / rw edges)
 *   - asset → reader script      (pure-read edges — a data dependency)
 *   - asset → subscriber script  (`// on <asset>` triggers)
 *
 * An `rw` edge is treated as production only (script → asset); emitting the
 * reverse asset → script too would make every upsert a 2-cycle through its own
 * asset.
 */
export function buildLineageDag(g: AssetGraphResponse): LineageDag {
	const down = new Map<string, Set<string>>()
	const up = new Map<string, Set<string>>()
	const nodes = new Set<string>()

	const addEdge = (a: string, b: string) => {
		if (a === b) return
		nodes.add(a)
		nodes.add(b)
		;(down.get(a) ?? down.set(a, new Set()).get(a)!).add(b)
		;(up.get(b) ?? up.set(b, new Set()).get(b)!).add(a)
	}

	// Register every node up front so isolated scripts/assets still appear.
	for (const r of g.runnables ?? []) {
		if (r.usage_kind === 'script') nodes.add(scriptNodeId(r.path))
	}
	for (const a of g.assets ?? []) nodes.add(`${a.kind}:${a.path}`)

	for (const e of g.edges ?? []) {
		if (e.runnable_kind !== 'script') continue
		const aid = assetKey(e)
		if (isWriteEdge(e)) {
			addEdge(scriptNodeId(e.runnable_path), aid)
		} else if ((e.access_type ?? 'r') === 'r') {
			addEdge(aid, scriptNodeId(e.runnable_path))
		}
	}
	for (const t of g.triggers ?? []) {
		if (t.trigger_kind !== 'asset' || t.runnable_kind !== 'script') continue
		addEdge(assetKey(t), scriptNodeId(t.runnable_path))
	}

	return { down, up, nodes }
}

/** BFS closure over an adjacency map, excluding `start`. */
function closure(adj: Map<string, Set<string>>, start: string): Set<string> {
	const seen = new Set<string>()
	const queue = [start]
	while (queue.length > 0) {
		const cur = queue.shift()!
		for (const n of adj.get(cur) ?? []) {
			if (seen.has(n)) continue
			seen.add(n)
			queue.push(n)
		}
	}
	// A cycle back to `start` would have re-added it; the contract is to
	// exclude the node itself.
	seen.delete(start)
	return seen
}

/** Transitive downstream of `n` (excludes `n`). Cycle-safe. */
export function descendants(dag: LineageDag, n: string): Set<string> {
	return closure(dag.down, n)
}
/** Transitive upstream of `n` (excludes `n`). Cycle-safe. */
export function ancestors(dag: LineageDag, n: string): Set<string> {
	return closure(dag.up, n)
}

/**
 * Nodes reachable from `starts` over the lineage DAG, treating `barriers` as cut
 * points: a barrier node is neither included NOR traversed through, so a node
 * reachable ONLY via a barrier is excluded while one also reachable via another
 * path stays. Mirror of the CLI `reachableCutting`. Used to keep event handlers
 * (and their event-only downstream) out of a cascade run — running such a
 * consumer whose producer was skipped would feed it missing/stale inputs.
 */
export function reachableCutting(
	dag: LineageDag,
	starts: Iterable<string>,
	barriers: Set<string>
): Set<string> {
	const seen = new Set<string>()
	const queue: string[] = []
	for (const s of starts) {
		if (barriers.has(s) || seen.has(s)) continue
		seen.add(s)
		queue.push(s)
	}
	while (queue.length > 0) {
		const n = queue.shift()!
		for (const next of dag.down.get(n) ?? []) {
			if (barriers.has(next) || seen.has(next)) continue
			seen.add(next)
			queue.push(next)
		}
	}
	return seen
}

export type BoundedResult = {
	/** Path-between node set (scripts + assets), always including `start`. */
	nodes: Set<string>
	/** Ends that are actually reachable downstream of `start`. */
	reachableEnds: string[]
	/** Ends passed in that are not downstream of `start` (ignored, surfaced). */
	droppedEnds: string[]
}

/**
 * Nodes on any path from `start` to any of `ends` (inclusive of both). Ends
 * not reachable from `start` are dropped and reported. With no reachable end
 * the result is just `{start}` — the caller decides whether to warn or fall
 * back to the full downstream.
 */
export function boundedSet(dag: LineageDag, start: string, ends: string[]): BoundedResult {
	const desc = descendants(dag, start)
	const downSet = new Set(desc)
	downSet.add(start)

	const reachableEnds = ends.filter((e) => downSet.has(e))
	const droppedEnds = ends.filter((e) => !downSet.has(e))
	if (reachableEnds.length === 0) {
		return { nodes: new Set([start]), reachableEnds, droppedEnds }
	}

	const upClosure = new Set<string>()
	for (const e of reachableEnds) {
		upClosure.add(e)
		for (const a of ancestors(dag, e)) upClosure.add(a)
	}
	const nodes = new Set<string>()
	for (const n of downSet) if (upClosure.has(n)) nodes.add(n)
	nodes.add(start)
	return { nodes, reachableEnds, droppedEnds }
}

/**
 * Script node ids eligible to *start* a bounded run: schedule-triggered
 * scripts, or manual roots (not an asset subscriber and not event-triggered).
 * Event-driven entrypoints are excluded — they have no "run up to X" gesture.
 */
export function validStarts(g: AssetGraphResponse): Set<string> {
	const subscribers = new Set<string>()
	const scheduleScripts = new Set<string>()
	const eventScripts = new Set<string>()
	for (const t of g.triggers ?? []) {
		if (t.runnable_kind !== 'script') continue
		if (t.trigger_kind === 'asset') subscribers.add(t.runnable_path)
		else if (t.trigger_kind === 'schedule') scheduleScripts.add(t.runnable_path)
		else if (EVENT_TRIGGER_KINDS.has(t.trigger_kind)) eventScripts.add(t.runnable_path)
	}

	const out = new Set<string>()
	for (const r of g.runnables ?? []) {
		if (r.usage_kind !== 'script') continue
		const p = r.path
		if (scheduleScripts.has(p)) out.add(scriptNodeId(p))
		else if (!subscribers.has(p) && !eventScripts.has(p)) out.add(scriptNodeId(p))
	}
	return out
}

/**
 * Script node ids eligible as an EXPLICIT bounded-run start from *anywhere* in
 * the DAG (dbt's `--select model+`): every script that can run with empty args —
 * i.e. all scripts except event-triggered ones (kafka/mqtt/nats/postgres/sqs/
 * gcp/email fan out per event and have no "run now" gesture). Unlike
 * `validStarts` (schedule/manual roots only), this INCLUDES mid-DAG asset
 * subscribers and pure readers, so "Run + downstream" can begin at any model —
 * that node plus its transitive downstream runs, upstream is never re-run.
 * (`webhook`/`data_upload` have no trigger row here — same as `validStarts` they
 * read as manual roots and are already included.)
 */
export function validFromStarts(g: AssetGraphResponse): Set<string> {
	const eventScripts = new Set<string>()
	for (const t of g.triggers ?? []) {
		if (t.runnable_kind !== 'script') continue
		if (EVENT_TRIGGER_KINDS.has(t.trigger_kind)) eventScripts.add(t.runnable_path)
	}
	// Seed with schedule/manual roots: `validStarts` lets a schedule identity win
	// over a secondary event trigger, so a scheduled root that also carries e.g.
	// a `// on kafka` stays `--from`-eligible even though it's an event script.
	const out = new Set<string>(validStarts(g))
	for (const r of g.runnables ?? []) {
		if (r.usage_kind !== 'script') continue
		if (!eventScripts.has(r.path)) out.add(scriptNodeId(r.path))
	}
	return out
}

/**
 * Script node ids carrying an event trigger (kafka/mqtt/nats/postgres/sqs/gcp/
 * email) — they fan out per event and can't run with empty args, so a cascade
 * must cut them (as `barriers` for `reachableCutting`) even when they're a
 * lineage descendant of the start. Mirror of the CLI `nonAutorunTriggerScripts`,
 * minus `webhook`/`data_upload`: those have no trigger row in `/assets/graph`
 * so they can't be detected here and read as manual roots (same limitation as
 * `validStarts`).
 */
export function nonAutorunTriggerScripts(g: AssetGraphResponse): Set<string> {
	const out = new Set<string>()
	for (const t of g.triggers ?? []) {
		if (t.runnable_kind === 'script' && EVENT_TRIGGER_KINDS.has(t.trigger_kind)) {
			out.add(scriptNodeId(t.runnable_path))
		}
	}
	return out
}

/** Project a node-id set to the script paths it contains (run targets). */
export function scriptsOf(nodes: Iterable<string>): string[] {
	const out: string[] = []
	for (const id of nodes) if (isScriptNode(id)) out.push(scriptPathOf(id))
	return out
}

/**
 * Producer→consumer adjacency (script path → downstream script paths) over the
 * lineage DAG: one hop through a single asset, following BOTH `// on`
 * subscribers *and* pure-read data dependencies. Unlike
 * `graphTraversal.buildDownstreamMap` (subscriber-only, which models the
 * production dispatch), this orders a script that merely reads an upstream
 * asset after its producer — so a bounded selection containing such a reader
 * schedules correctly. Mirror of the CLI `topoOrder` adjacency.
 */
export function buildLineageDownstreamMap(g: AssetGraphResponse): Map<string, Set<string>> {
	const dag = buildLineageDag(g)
	const map = new Map<string, Set<string>>()
	for (const id of dag.nodes) {
		if (!isScriptNode(id)) continue
		const s = scriptPathOf(id)
		const subs = new Set<string>()
		for (const asset of dag.down.get(id) ?? []) {
			for (const sub of dag.down.get(asset) ?? []) {
				if (isScriptNode(sub) && scriptPathOf(sub) !== s) subs.add(scriptPathOf(sub))
			}
		}
		if (subs.size > 0) map.set(s, subs)
	}
	return map
}
