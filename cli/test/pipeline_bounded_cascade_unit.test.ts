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
  validFromStarts,
  nonAutorunTriggerScripts,
  reachableCutting,
  isScriptNode,
  scriptPathOf,
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

test("validStarts: schedule and manual roots, not events/uploads/webhooks or subscribers", () => {
  const g = graph({
    scripts: ["a", "sub", "sched", "kfk", "upl", "hook"],
    writes: [["a", "x"]],
    subs: [["sub", "x"], ["sched", "x"]],
    native: [
      ["schedule", "sched"],
      ["kafka", "kfk"],
      ["data_upload", "upl"],
      ["webhook", "hook"],
    ],
  });
  const starts = validStarts(g);
  expect(starts.has(sn("a"))).toBe(true); // manual root
  expect(starts.has(sn("sched"))).toBe(true); // schedule overrides subscriber
  expect(starts.has(sn("sub"))).toBe(false); // pure subscriber
  expect(starts.has(sn("kfk"))).toBe(false); // event-only
  // data_upload / webhook need caller-supplied input → not auto-run roots
  expect(starts.has(sn("upl"))).toBe(false);
  expect(starts.has(sn("hook"))).toBe(false);
});

test("a scheduled root with a secondary data_upload trigger stays a start and isn't cut", () => {
  // Regression: the barrier set must exclude valid starts, else a script with
  // both `// on schedule` and `// on data_upload` resolves as the start yet is
  // also a barrier, so reachableCutting skips it → empty run plan.
  const g = graph({
    scripts: ["sched_upload", "consumer"],
    writes: [["sched_upload", "x"]],
    subs: [["consumer", "x"]],
    native: [
      ["schedule", "sched_upload"],
      ["data_upload", "sched_upload"],
    ],
  });
  const starts = validStarts(g);
  expect(starts.has(sn("sched_upload"))).toBe(true); // schedule wins over the upload trigger
  // Mirror run()'s barrier set: nonAutorun handlers minus the valid starts.
  const barriers = new Set([...nonAutorunTriggerScripts(g)].filter((id) => !starts.has(id)));
  expect(barriers.has(sn("sched_upload"))).toBe(false); // the scheduled root is protected
  const sel = new Set(
    [...reachableCutting(buildLineageDag(g), starts, barriers)]
      .filter(isScriptNode)
      .map(scriptPathOf),
  );
  expect(sel.has("sched_upload")).toBe(true); // runs on its schedule path…
  expect(sel.has("consumer")).toBe(true); // …and its downstream isn't cut off
});

test("validFromStarts: mid-DAG models are eligible starts, event/upload/webhook are not", () => {
  // a(root) → x → sub → y → reader ; k=kafka, upl=data_upload, hook=webhook.
  const g = graph({
    scripts: ["a", "sub", "reader", "k", "upl", "hook"],
    writes: [["a", "x"], ["sub", "y"]],
    reads: [["reader", "y"]],
    subs: [["sub", "x"]],
    native: [["kafka", "k"], ["data_upload", "upl"], ["webhook", "hook"]],
  });
  const from = validFromStarts(g);
  expect(from.has(sn("a"))).toBe(true); // root
  expect(from.has(sn("sub"))).toBe(true); // mid-DAG subscriber — NOT a validStart
  expect(from.has(sn("reader"))).toBe(true); // pure reader
  expect(validStarts(g).has(sn("sub"))).toBe(false); // old root-only gate rejected it
  // Non-autorun handlers stay out (they need caller input / fan out per event).
  expect(from.has(sn("k"))).toBe(false);
  expect(from.has(sn("upl"))).toBe(false);
  expect(from.has(sn("hook"))).toBe(false);
});

test("validFromStarts keeps a scheduled root that also carries a non-autorun trigger", () => {
  // Regression: `--from sched_upload` must be accepted just like the implicit
  // start. `sched_upload` is a scheduled root AND a data_upload handler — schedule
  // wins in validStarts, so it stays --from-eligible despite being a nonAutorun
  // handler (else explicit --from throws where the implicit start succeeds).
  const g = graph({
    scripts: ["sched_upload", "consumer"],
    writes: [["sched_upload", "x"]],
    subs: [["consumer", "x"]],
    native: [["schedule", "sched_upload"], ["data_upload", "sched_upload"]],
  });
  expect(validStarts(g).has(sn("sched_upload"))).toBe(true);
  expect(nonAutorunTriggerScripts(g).has(sn("sched_upload"))).toBe(true);
  expect(validFromStarts(g).has(sn("sched_upload"))).toBe(true); // union with roots
});

test("a mid-DAG start runs itself + downstream, never upstream", () => {
  // Starting at `sub`, the unbounded downstream is {sub, reader}; `a`/`x` upstream
  // are never pulled in (dbt `--select sub+`).
  const g = graph({
    scripts: ["a", "sub", "reader"],
    writes: [["a", "x"], ["sub", "y"]],
    reads: [["reader", "y"]],
    subs: [["sub", "x"]],
  });
  const dag = buildLineageDag(g);
  const downstream = new Set([sn("sub"), ...descendants(dag, sn("sub"))]);
  expect(sorted(scriptsOf(downstream))).toEqual(["reader", "sub"]);
  expect(downstream.has(sn("a"))).toBe(false);
  expect(downstream.has("datatable:x")).toBe(false);
});

test("nonAutorunTriggerScripts: event + upload/webhook handlers, incl. subscribers (descendants)", () => {
  const g = graph({
    scripts: ["a", "evt", "upl"],
    writes: [["a", "x"]],
    subs: [["evt", "x"], ["upl", "x"]], // both are lineage descendants of a…
    native: [["kafka", "evt"], ["data_upload", "upl"]], // …and input-requiring handlers
  });
  const ev = nonAutorunTriggerScripts(g);
  // excluded from a whole-pipeline run despite being descendants
  expect(ev.has(sn("evt"))).toBe(true);
  expect(ev.has(sn("upl"))).toBe(true);
  expect(ev.has(sn("a"))).toBe(false);
});

test("reachableCutting: drops barrier + its exclusive downstream, keeps alt-path nodes", () => {
  // root_a → x → handler(kafka) → y → consumer ; root_b → z ; (variant: consumer also ← z)
  const g = graph({
    scripts: ["root_a", "root_b", "handler", "consumer"],
    writes: [["root_a", "x"], ["root_b", "z"], ["handler", "y"]],
    subs: [["handler", "x"], ["consumer", "y"]],
    native: [["kafka", "handler"]],
  });
  const dag = buildLineageDag(g);
  const scripts = (s: Set<string>) =>
    new Set([...s].filter(isScriptNode).map(scriptPathOf));

  // consumer only reachable via the event handler → both excluded
  expect(scripts(reachableCutting(dag, validStarts(g), nonAutorunTriggerScripts(g)))).toEqual(
    new Set(["root_a", "root_b"]),
  );

  // now consumer also reads z (a non-event path via root_b) → it stays
  const g2 = graph({
    scripts: ["root_a", "root_b", "handler", "consumer"],
    writes: [["root_a", "x"], ["root_b", "z"], ["handler", "y"]],
    subs: [["handler", "x"], ["consumer", "y"], ["consumer", "z"]],
    native: [["kafka", "handler"]],
  });
  expect(
    scripts(reachableCutting(buildLineageDag(g2), validStarts(g2), nonAutorunTriggerScripts(g2))),
  ).toEqual(new Set(["root_a", "root_b", "consumer"]));
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
