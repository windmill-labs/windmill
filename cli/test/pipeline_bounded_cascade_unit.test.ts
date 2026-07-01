import { expect, test } from "bun:test";

// Mirror of
// frontend/src/lib/components/assets/AssetGraph/boundedCascade.test.ts — keep
// the two engines in sync. (Bun project → bun:test, not Deno.)
import {
  type BCGraph,
  ancestors,
  assetUriToNodeId,
  boundedSet,
  buildLineageDag,
  descendants,
  resolveToken,
  scriptNodeId,
  scriptsOf,
  topoOrder,
  validStarts,
  eventTriggerScripts,
} from "../src/commands/pipeline/boundedCascade.ts";

type W = [script: string, asset: string];
type S = [script: string, asset: string];
type R = [script: string, asset: string];

function graph(opts: {
  scripts?: string[];
  writes?: W[];
  reads?: R[];
  subs?: S[];
  native?: Array<[kind: string, script: string]>;
}): BCGraph {
  const { scripts = [], writes = [], reads = [], subs = [], native = [] } = opts;
  return {
    assets: [],
    runnables: scripts.map((p) => ({ path: p, usage_kind: "script" as const })),
    edges: [
      ...writes.map(([s, a]) => ({
        runnable_kind: "script",
        runnable_path: s,
        asset_kind: "datatable",
        asset_path: a,
        access_type: "w" as const,
      })),
      ...reads.map(([s, a]) => ({
        runnable_kind: "script",
        runnable_path: s,
        asset_kind: "datatable",
        asset_path: a,
        access_type: "r" as const,
      })),
    ],
    triggers: [
      ...subs.map(([s, a]) => ({
        trigger_kind: "asset" as const,
        asset_kind: "datatable",
        asset_path: a,
        runnable_kind: "script",
        runnable_path: s,
      })),
      ...native.map(([kind, s]) => ({
        trigger_kind: kind,
        runnable_kind: "script",
        runnable_path: s,
      })),
    ],
  };
}

const sn = scriptNodeId;
const sorted = (it: Iterable<string>) => [...it].sort();

// a → x → b → y → c → z → d (linear chain through assets)
const chain = () =>
  graph({
    scripts: ["a", "b", "c", "d"],
    writes: [
      ["a", "x"],
      ["b", "y"],
      ["c", "z"],
    ],
    subs: [
      ["b", "x"],
      ["c", "y"],
      ["d", "z"],
    ],
  });

test("boundedSet stops at a single end node", () => {
  const res = boundedSet(buildLineageDag(chain()), sn("a"), [sn("c")]);
  expect(sorted(scriptsOf(res.nodes))).toEqual(["a", "b", "c"]);
  expect(res.nodes.has("datatable:z")).toBe(false);
});

test("boundedSet supports an asset as the end bound", () => {
  const res = boundedSet(buildLineageDag(chain()), sn("a"), ["datatable:y"]);
  expect(sorted(scriptsOf(res.nodes))).toEqual(["a", "b"]);
});

test("boundedSet drops ends not downstream of start", () => {
  const res = boundedSet(buildLineageDag(chain()), sn("c"), [sn("a")]);
  expect(res.droppedEnds).toEqual([sn("a")]);
  expect([...res.nodes]).toEqual([sn("c")]);
});

test("validStarts: schedule and manual roots, not events or subscribers", () => {
  const g = graph({
    scripts: ["a", "sub", "sched", "kfk"],
    writes: [["a", "x"]],
    subs: [["sub", "x"], ["sched", "x"]],
    native: [["schedule", "sched"], ["kafka", "kfk"]],
  });
  const starts = validStarts(g);
  expect(starts.has(sn("a"))).toBe(true); // manual root
  expect(starts.has(sn("sched"))).toBe(true); // schedule overrides subscriber
  expect(starts.has(sn("sub"))).toBe(false); // pure subscriber
  expect(starts.has(sn("kfk"))).toBe(false); // event-only
});

test("eventTriggerScripts: event handlers, incl. ones that also subscribe (descendants)", () => {
  const g = graph({
    scripts: ["a", "evt"],
    writes: [["a", "x"]],
    subs: [["evt", "x"]], // evt is a lineage descendant of a…
    native: [["kafka", "evt"]], // …and a kafka event handler
  });
  const ev = eventTriggerScripts(g);
  expect(ev.has(sn("evt"))).toBe(true); // excluded from a whole-pipeline run despite being a descendant
  expect(ev.has(sn("a"))).toBe(false);
});

test("assetUriToNodeId maps s3 → s3object, others verbatim", () => {
  expect(assetUriToNodeId("s3://b/k")).toBe("s3object:b/k");
  expect(assetUriToNodeId("datatable://main/users")).toBe("datatable:main/users");
  expect(assetUriToNodeId("nope")).toBe(undefined);
});

test("resolveToken: short name, full path, and asset URI", () => {
  const g = graph({ scripts: ["f/p/stage"], writes: [["f/p/stage", "main/staged"]] });
  g.assets = [{ kind: "datatable", path: "main/staged" }];
  expect(resolveToken(g, "stage")).toBe(sn("f/p/stage"));
  expect(resolveToken(g, "f/p/stage")).toBe(sn("f/p/stage"));
  expect(resolveToken(g, "datatable://main/staged")).toBe("datatable:main/staged");
  expect(resolveToken(g, "missing")).toBe(undefined);
});

test("topoOrder sorts the bounded scripts and flags cycles", () => {
  const { order, cyclic } = topoOrder(chain(), new Set(["a", "b", "c"]));
  expect(order).toEqual(["a", "b", "c"]);
  expect(cyclic).toEqual([]);
});

test("descendants/ancestors exclude the start even on a cycle", () => {
  // a → x → b → y → a (cycle): closures must not contain a.
  const g = graph({
    scripts: ["a", "b"],
    writes: [["a", "x"], ["b", "y"]],
    subs: [["b", "x"], ["a", "y"]],
  });
  const dag = buildLineageDag(g);
  expect(descendants(dag, sn("a")).has(sn("a"))).toBe(false);
  expect(ancestors(dag, sn("a")).has(sn("a"))).toBe(false);
});

test("topoOrder orders a pure reader after its producer", () => {
  // a writes x; c only *reads* x (no `// on x`). c must run after a.
  const g = graph({
    scripts: ["a", "c"],
    writes: [["a", "x"]],
    reads: [["c", "x"]],
  });
  const { order } = topoOrder(g, new Set(["a", "c"]));
  expect(order).toEqual(["a", "c"]);
});
