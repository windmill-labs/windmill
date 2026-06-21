// Bounded-cascade graph engine for `wmill pipeline run --to`.
//
// MIRROR of frontend/src/lib/components/assets/AssetGraph/boundedCascade.ts —
// the two have no shared import path (frontend is a separate package), so keep
// them in sync. The grammar is intentionally tiny: there is no dbt-style
// `--select` string. The user names a start (a schedule / manual root) and one
// or more end nodes; the run is the "path between" them:
//
//     descendants(start) ∩ (ancestors(ends) ∪ ends) ∪ {start}
//
// Node ids: assets `${kind}:${path}` (e.g. `datatable:main/raw`); runnables
// `script:${path}`. Operates on the asset-graph payload shape used by
// pipeline.ts.

export type BCGraph = {
  runnables: { path: string; usage_kind: "script" | "flow" | "job" }[];
  assets: { kind: string; path: string }[];
  edges: {
    runnable_kind: string;
    runnable_path: string;
    asset_kind: string;
    asset_path: string;
    access_type?: "r" | "w" | "rw";
  }[];
  triggers: (
    | {
        trigger_kind: "asset";
        asset_kind: string;
        asset_path: string;
        runnable_kind: string;
        runnable_path: string;
      }
    | { trigger_kind: string; runnable_kind: string; runnable_path: string }
  )[];
};

export const SCRIPT_PREFIX = "script:";
export const scriptNodeId = (path: string): string => `${SCRIPT_PREFIX}${path}`;
export const isScriptNode = (id: string): boolean => id.startsWith(SCRIPT_PREFIX);
export const scriptPathOf = (id: string): string => id.slice(SCRIPT_PREFIX.length);
const assetNodeId = (kind: string, path: string): string => `${kind}:${path}`;

// Native trigger kinds that fan out per-event — never bounded-run starts.
// `webhook` / `data_upload` have no trigger row in the graph payload, so a root
// whose only entry is one of those reads as a manual root.
const EVENT_TRIGGER_KINDS = new Set([
  "kafka",
  "mqtt",
  "nats",
  "postgres",
  "sqs",
  "gcp",
  "email",
]);

/** Resolve an asset URI (`datatable://x`, `s3://b/k`, …) to its node id. */
export function assetUriToNodeId(uri: string): string | undefined {
  const m = uri.match(/^([a-z0-9_]+):\/\/(.+)$/i);
  if (!m) return undefined;
  const prefix = m[1].toLowerCase();
  const kind = prefix === "s3" ? "s3object" : prefix;
  return `${kind}:${m[2]}`;
}

export type LineageDag = {
  down: Map<string, Set<string>>;
  up: Map<string, Set<string>>;
  nodes: Set<string>;
};

function addEdge(dag: LineageDag, a: string, b: string) {
  if (a === b) return;
  dag.nodes.add(a);
  dag.nodes.add(b);
  (dag.down.get(a) ?? dag.down.set(a, new Set()).get(a)!).add(b);
  (dag.up.get(b) ?? dag.up.set(b, new Set()).get(b)!).add(a);
}

/** Unified upstream→downstream lineage DAG over scripts ∪ assets. */
export function buildLineageDag(g: BCGraph): LineageDag {
  const dag: LineageDag = { down: new Map(), up: new Map(), nodes: new Set() };
  for (const r of g.runnables ?? []) {
    if (r.usage_kind === "script") dag.nodes.add(scriptNodeId(r.path));
  }
  for (const a of g.assets ?? []) dag.nodes.add(assetNodeId(a.kind, a.path));
  for (const e of g.edges ?? []) {
    if (e.runnable_kind !== "script") continue;
    const aid = assetNodeId(e.asset_kind, e.asset_path);
    const access = e.access_type ?? "r";
    if (access === "w" || access === "rw") {
      addEdge(dag, scriptNodeId(e.runnable_path), aid); // producer
    } else if (access === "r") {
      addEdge(dag, aid, scriptNodeId(e.runnable_path)); // pure reader
    }
  }
  for (const t of g.triggers ?? []) {
    if (t.trigger_kind !== "asset" || t.runnable_kind !== "script") continue;
    const at = t as Extract<BCGraph["triggers"][number], { trigger_kind: "asset" }>;
    addEdge(dag, assetNodeId(at.asset_kind, at.asset_path), scriptNodeId(at.runnable_path));
  }
  return dag;
}

function closure(adj: Map<string, Set<string>>, start: string): Set<string> {
  const seen = new Set<string>();
  const queue = [start];
  while (queue.length > 0) {
    const cur = queue.shift()!;
    for (const n of adj.get(cur) ?? []) {
      if (seen.has(n)) continue;
      seen.add(n);
      queue.push(n);
    }
  }
  return seen;
}

export const descendants = (dag: LineageDag, n: string): Set<string> => closure(dag.down, n);
export const ancestors = (dag: LineageDag, n: string): Set<string> => closure(dag.up, n);

export type BoundedResult = {
  nodes: Set<string>;
  reachableEnds: string[];
  droppedEnds: string[];
};

/** Path-between node set for `start` and `ends` (inclusive). */
export function boundedSet(dag: LineageDag, start: string, ends: string[]): BoundedResult {
  const downSet = new Set(descendants(dag, start));
  downSet.add(start);
  const reachableEnds = ends.filter((e) => downSet.has(e));
  const droppedEnds = ends.filter((e) => !downSet.has(e));
  if (reachableEnds.length === 0) {
    return { nodes: new Set([start]), reachableEnds, droppedEnds };
  }
  const upClosure = new Set<string>();
  for (const e of reachableEnds) {
    upClosure.add(e);
    for (const a of ancestors(dag, e)) upClosure.add(a);
  }
  const nodes = new Set<string>();
  for (const n of downSet) if (upClosure.has(n)) nodes.add(n);
  nodes.add(start);
  return { nodes, reachableEnds, droppedEnds };
}

/** Script node ids eligible to start a bounded run. */
export function validStarts(g: BCGraph): Set<string> {
  const subscribers = new Set<string>();
  const scheduleScripts = new Set<string>();
  const eventScripts = new Set<string>();
  for (const t of g.triggers ?? []) {
    if (t.runnable_kind !== "script") continue;
    if (t.trigger_kind === "asset") subscribers.add(t.runnable_path);
    else if (t.trigger_kind === "schedule") scheduleScripts.add(t.runnable_path);
    else if (EVENT_TRIGGER_KINDS.has(t.trigger_kind)) eventScripts.add(t.runnable_path);
  }
  const out = new Set<string>();
  for (const r of g.runnables ?? []) {
    if (r.usage_kind !== "script") continue;
    const p = r.path;
    if (scheduleScripts.has(p)) out.add(scriptNodeId(p));
    else if (!subscribers.has(p) && !eventScripts.has(p)) out.add(scriptNodeId(p));
  }
  return out;
}

/** Project a node-id set to the script paths it contains. */
export function scriptsOf(nodes: Iterable<string>): string[] {
  const out: string[] = [];
  for (const id of nodes) if (isScriptNode(id)) out.push(scriptPathOf(id));
  return out;
}

/**
 * Resolve a CLI `--to` / `--from` token to a node id, or undefined if it
 * matches nothing. Asset URIs (`kind://path`) resolve to the asset node; a bare
 * token matches a runnable by exact path or by short (last-segment) name.
 */
export function resolveToken(g: BCGraph, token: string): string | undefined {
  if (token.includes("://")) {
    const id = assetUriToNodeId(token);
    return id && g.assets.some((a) => `${a.kind}:${a.path}` === id) ? id : undefined;
  }
  const scripts = (g.runnables ?? []).filter((r) => r.usage_kind === "script");
  const exact = scripts.find((r) => r.path === token);
  if (exact) return scriptNodeId(exact.path);
  const byShort = scripts.filter((r) => (r.path.split("/").pop() ?? r.path) === token);
  return byShort.length === 1 ? scriptNodeId(byShort[0].path) : undefined;
}

/**
 * Topological order of `scripts` over the in-set producer→subscriber edges
 * (assets collapsed). Scripts on a cycle are returned in `cyclic` and excluded
 * from `order`. Serial-run friendly: every script comes after its in-set
 * upstreams.
 */
export function topoOrder(
  g: BCGraph,
  scripts: Set<string>,
): { order: string[]; cyclic: string[] } {
  const dag = buildLineageDag(g);
  const down = new Map<string, Set<string>>();
  const indegree = new Map<string, number>();
  for (const s of scripts) indegree.set(s, 0);
  // One-hop (through a single asset) script→script edges, restricted to the set.
  for (const s of scripts) {
    const sid = scriptNodeId(s);
    const oneHop = new Set<string>();
    for (const asset of dag.down.get(sid) ?? []) {
      for (const sub of dag.down.get(asset) ?? []) {
        if (isScriptNode(sub)) {
          const p = scriptPathOf(sub);
          if (p !== s && scripts.has(p)) oneHop.add(p);
        }
      }
    }
    if (oneHop.size > 0) {
      down.set(s, oneHop);
      for (const p of oneHop) indegree.set(p, (indegree.get(p) ?? 0) + 1);
    }
  }
  const ready = [...scripts].filter((s) => (indegree.get(s) ?? 0) === 0);
  const remaining = new Map(indegree);
  const order: string[] = [];
  while (ready.length > 0) {
    const n = ready.shift()!;
    order.push(n);
    for (const p of down.get(n) ?? []) {
      const d = (remaining.get(p) ?? 0) - 1;
      remaining.set(p, d);
      if (d === 0) ready.push(p);
    }
  }
  const orderedSet = new Set(order);
  const cyclic = [...scripts].filter((s) => !orderedSet.has(s));
  return { order, cyclic };
}
