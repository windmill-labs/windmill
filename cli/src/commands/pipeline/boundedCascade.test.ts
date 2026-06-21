// Mirror of
// frontend/src/lib/components/assets/AssetGraph/boundedCascade.test.ts — keep
// the two engines in sync.
import { assertEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import {
  type BCGraph,
  assetUriToNodeId,
  boundedSet,
  buildLineageDag,
  resolveToken,
  scriptNodeId,
  scriptsOf,
  topoOrder,
  validStarts,
} from "./boundedCascade.ts";

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

Deno.test("boundedSet stops at a single end node", () => {
  const res = boundedSet(buildLineageDag(chain()), sn("a"), [sn("c")]);
  assertEquals(sorted(scriptsOf(res.nodes)), ["a", "b", "c"]);
  assertEquals(res.nodes.has("datatable:z"), false);
});

Deno.test("boundedSet supports an asset as the end bound", () => {
  const res = boundedSet(buildLineageDag(chain()), sn("a"), ["datatable:y"]);
  assertEquals(sorted(scriptsOf(res.nodes)), ["a", "b"]);
});

Deno.test("boundedSet drops ends not downstream of start", () => {
  const res = boundedSet(buildLineageDag(chain()), sn("c"), [sn("a")]);
  assertEquals(res.droppedEnds, [sn("a")]);
  assertEquals([...res.nodes], [sn("c")]);
});

Deno.test("validStarts: schedule and manual roots, not events or subscribers", () => {
  const g = graph({
    scripts: ["a", "sub", "sched", "kfk"],
    writes: [["a", "x"]],
    subs: [["sub", "x"], ["sched", "x"]],
    native: [["schedule", "sched"], ["kafka", "kfk"]],
  });
  const starts = validStarts(g);
  assertEquals(starts.has(sn("a")), true); // manual root
  assertEquals(starts.has(sn("sched")), true); // schedule overrides subscriber
  assertEquals(starts.has(sn("sub")), false); // pure subscriber
  assertEquals(starts.has(sn("kfk")), false); // event-only
});

Deno.test("assetUriToNodeId maps s3 → s3object, others verbatim", () => {
  assertEquals(assetUriToNodeId("s3://b/k"), "s3object:b/k");
  assertEquals(assetUriToNodeId("datatable://main/users"), "datatable:main/users");
  assertEquals(assetUriToNodeId("nope"), undefined);
});

Deno.test("resolveToken: short name, full path, and asset URI", () => {
  const g = graph({ scripts: ["f/p/stage"], writes: [["f/p/stage", "main/staged"]] });
  // asset must exist in graph.assets for URI resolution
  g.assets = [{ kind: "datatable", path: "main/staged" }];
  assertEquals(resolveToken(g, "stage"), sn("f/p/stage"));
  assertEquals(resolveToken(g, "f/p/stage"), sn("f/p/stage"));
  assertEquals(resolveToken(g, "datatable://main/staged"), "datatable:main/staged");
  assertEquals(resolveToken(g, "missing"), undefined);
});

Deno.test("topoOrder sorts the bounded scripts and flags cycles", () => {
  const { order, cyclic } = topoOrder(chain(), new Set(["a", "b", "c"]));
  assertEquals(order, ["a", "b", "c"]);
  assertEquals(cyclic, []);
});

Deno.test("topoOrder orders a pure reader after its producer", () => {
  // a writes x; c only *reads* x (no `// on x`). c must run after a.
  const g = graph({
    scripts: ["a", "c"],
    writes: [["a", "x"]],
    reads: [["c", "x"]],
  });
  const { order } = topoOrder(g, new Set(["a", "c"]));
  assertEquals(order, ["a", "c"]);
});
