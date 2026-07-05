import { expect, test } from "bun:test";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { buildLocalPipelineGraph } from "../src/commands/pipeline/localGraph.ts";

// Build a throwaway workspace tree with `f/<folder>/<file>` scripts and a
// wmill.yaml at the root, then assert the graph the wasm-backed builder derives.
function withFolder(
  files: Record<string, string>,
  fn: (root: string, folder: string) => Promise<void> | void,
) {
  const root = mkdtempSync(join(tmpdir(), "wm-pl-"));
  writeFileSync(join(root, "wmill.yaml"), "defaultTs: bun\n");
  const folder = "mypipe";
  mkdirSync(join(root, "f", folder), { recursive: true });
  for (const [name, content] of Object.entries(files)) {
    writeFileSync(join(root, "f", folder, name), content);
  }
  return Promise.resolve(fn(root, folder)).finally(() =>
    rmSync(root, { recursive: true, force: true }),
  );
}

test("only `// pipeline` scripts become nodes; `// on` asset triggers wire edges", async () => {
  await withFolder(
    {
      // a source: pipeline member, subscribes to nothing, but is annotated.
      "raw.bun.ts":
        `// pipeline\nimport * as wmill from "windmill-client"\nexport async function main() {}\n`,
      // a transform: pipeline member, subscribes to raw.
      "staged.duckdb.sql":
        `-- pipeline\n-- on datatable://main/raw\nINSERT INTO main.staged SELECT 1;\n`,
      // not a pipeline member — must be excluded.
      "helper.bun.ts":
        `export async function main() {}\n`,
    },
    async (root, folder) => {
      const { graph, scripts } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });

      const paths = graph.runnables.map((r) => r.path).sort();
      expect(paths).toEqual(["f/mypipe/raw", "f/mypipe/staged"]);
      // helper is excluded
      expect(paths).not.toContain("f/mypipe/helper");
      expect(scripts.map((s) => s.path).sort()).toEqual(paths);

      // staged subscribes to datatable://main/raw via an asset trigger
      const assetTriggers = graph.triggers.filter((t) => t.trigger_kind === "asset");
      expect(assetTriggers).toHaveLength(1);
      const at = assetTriggers[0] as Extract<
        (typeof graph.triggers)[number],
        { trigger_kind: "asset" }
      >;
      expect(at.asset_kind).toBe("datatable");
      expect(at.asset_path).toBe("main/raw");
      expect(at.runnable_path).toBe("f/mypipe/staged");

      // the referenced asset exists in the asset set
      expect(graph.assets).toContainEqual({ kind: "datatable", path: "main/raw" });
    },
  );
});

test("native triggers surface as trigger rows (no deploy needed)", async () => {
  await withFolder(
    {
      "ingest.bun.ts":
        `// pipeline\n// on data_upload\nimport * as wmill from "windmill-client"\nexport async function main() {}\n`,
      "upload_trigger.duckdb.sql": `-- pipeline\n-- on data_upload\nSELECT 1;\n`,
    },
    async (root, folder) => {
      const { graph } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      const native = graph.triggers.filter((t) => t.trigger_kind !== "asset");
      expect(
        native
          .filter((t) => t.trigger_kind === "data_upload")
          .map((t) => t.runnable_path)
          .sort(),
      ).toEqual(["f/mypipe/ingest", "f/mypipe/upload_trigger"]);
    },
  );
});

test("retry delay metadata strips the optional `delay=` prefix", async () => {
  await withFolder(
    {
      "retry.duckdb.sql": `-- pipeline\n-- retry 2 delay=10s\nSELECT 1;\n`,
    },
    async (root, folder) => {
      const { graph } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      expect(graph.runnables.find((r) => r.path === "f/mypipe/retry")?.retry).toEqual({
        count: 2,
        delay: "10s",
      });
    },
  );
});

test("empty / no-pipeline folder yields an empty graph", async () => {
  await withFolder(
    { "plain.bun.ts": `export async function main() {}\n` },
    async (root, folder) => {
      const { graph } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      expect(graph.runnables).toHaveLength(0);
      expect(graph.assets).toHaveLength(0);
    },
  );
});

test("`// materialize <asset>` producer connects to its `// on` consumer", async () => {
  // The wasm asset parser doesn't surface `// materialize`; the CLI-side scan
  // must still emit the producer's write edge so it links to the consumer.
  await withFolder(
    {
      "load.duckdb.sql": `-- pipeline\n-- materialize ducklake://main/users\nSELECT 1 AS id;\n`,
      "consume.duckdb.sql":
        `-- pipeline\n-- on ducklake://main/users\nSELECT count(*) FROM users;\n`,
    },
    async (root, folder) => {
      const { graph } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });

      // the materialize target is a shared asset
      expect(graph.assets).toContainEqual({ kind: "ducklake", path: "main/users" });

      // the producer carries its materialize_target and a write edge
      const load = graph.runnables.find((r) => r.path === "f/mypipe/load");
      expect(load?.materialize_target).toEqual({ kind: "ducklake", path: "main/users" });
      expect(graph.edges).toContainEqual({
        runnable_kind: "script",
        runnable_path: "f/mypipe/load",
        asset_kind: "ducklake",
        asset_path: "main/users",
        access_type: "w",
      });

      // the consumer subscribes to the same asset (the connecting trigger)
      const at = graph.triggers.find(
        (t) => t.trigger_kind === "asset" && t.runnable_path === "f/mypipe/consume",
      ) as Extract<(typeof graph.triggers)[number], { trigger_kind: "asset" }> | undefined;
      expect(at?.asset_path).toBe("main/users");
    },
  );
});

// NOTE: the scd2 `<dim>_current` companion-view write edge (buildLocalPipelineGraph)
// is exercised end-to-end by the backend parser test
// (`materialize_scd2_write_targets_include_current_view`) and the frontend
// live-graph test (`resolveGraph.test.ts` — "scd2 materialize draft"). It has no
// unit test here because it depends on the `scd2` flag emitted by the bundled
// `windmill-parser-wasm-asset`, which lags the source (the vendored build
// predates the scd2 fields); the branch activates once a wasm carrying `scd2` is
// republished (cf. the wasm-publish plumbing in #9926).

test("a bare `.sql` (ambiguous dialect) is skipped, not a build-aborting crash", async () => {
  // `inferContentTypeFromFilePath` throws on a dialect-less `.sql`; one such file
  // must not abort the whole graph build (it also wedged `pipeline dev` at start).
  await withFolder(
    {
      "good.duckdb.sql": `-- pipeline\nSELECT 1;\n`,
      "ambiguous.sql": `-- pipeline\nSELECT 1;\n`,
    },
    async (root, folder) => {
      const { graph } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      const paths = graph.runnables.map((r) => r.path);
      expect(paths).toContain("f/mypipe/good");
      // the unclassifiable file is dropped — the build still succeeds
      expect(paths).not.toContain("f/mypipe/ambiguous");
    },
  );
});

test("defaultTs (from wmill.yaml) drives bare `.ts` runtime — deno, not always bun", async () => {
  await withFolder(
    { "etl.ts": `// pipeline\nexport async function main() {}\n` },
    async (root, folder) => {
      const bun = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      const deno = await buildLocalPipelineGraph({ root, folder, defaultTs: "deno" });
      expect(bun.scripts.find((s) => s.path === "f/mypipe/etl")?.language).toBe("bun");
      expect(deno.scripts.find((s) => s.path === "f/mypipe/etl")?.language).toBe("deno");
    },
  );
});

test("`// tag <worker>` is carried on the pushed script for preview routing", async () => {
  await withFolder(
    {
      "gpu_job.duckdb.sql": `-- pipeline\n-- tag gpu\nSELECT 1;\n`,
      "plain.duckdb.sql": `-- pipeline\nSELECT 2;\n`,
    },
    async (root, folder) => {
      const { scripts } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      expect(scripts.find((s) => s.path === "f/mypipe/gpu_job")?.tag).toBe("gpu");
      // a node without `// tag` carries no tag (→ default worker)
      expect(scripts.find((s) => s.path === "f/mypipe/plain")?.tag).toBeUndefined();
    },
  );
});

test("go/bash fallback: leading-header `// on` only, options stripped, no body phantoms", async () => {
  await withFolder(
    {
      // header annotations (incl. `// tag`, one `// on` with a trailing option) +
      // a body comment that must NOT become a phantom trigger.
      "ingest.go":
        `// pipeline\n// tag heavy\n// on s3://demo/raw.csv debounce=5s\npackage inner\nfunc main() {\n\t// on s3://demo/PHANTOM.csv\n}\n`,
    },
    async (root, folder) => {
      const { graph, scripts } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      const ats = graph.triggers.filter((t) => t.trigger_kind === "asset") as Extract<
        (typeof graph.triggers)[number],
        { trigger_kind: "asset" }
      >[];
      // exactly one trigger: the body `// on …PHANTOM` is past the header → ignored
      expect(ats).toHaveLength(1);
      // the trailing `debounce=5s` option is stripped from the asset path
      expect(ats[0].asset_path).toBe("demo/raw.csv");
      expect(graph.assets.some((a) => a.path.includes("PHANTOM"))).toBe(false);
      // `// tag` is recovered by the fallback too (routes the preview)
      expect(scripts.find((s) => s.path === "f/mypipe/ingest")?.tag).toBe("heavy");
      expect(graph.runnables.find((r) => r.path === "f/mypipe/ingest")?.tag).toBe("heavy");
    },
  );
});

test("go/bash fallback: a native marker with trailing content is rejected (parity)", async () => {
  // The canonical parser rejects a marker line with trailing content, so the
  // fallback must too — else a `// on data_upload f/foo` / `# on kafka topic`
  // shows up locally as an upload/event trigger that deployed parsing drops.
  await withFolder(
    {
      "bare.go": `// pipeline\n// on data_upload\npackage inner\nfunc main() {}\n`,
      "trailing.go": `// pipeline\n// on data_upload f/foo\npackage inner\nfunc main() {}\n`,
      "kafka.rb": `# pipeline\n# on kafka topic\nputs "hi"\n`,
    },
    async (root, folder) => {
      const { graph } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      const nativeOf = (p: string) =>
        graph.triggers
          .filter((t) => t.trigger_kind !== "asset" && t.runnable_path === p)
          .map((t) => t.trigger_kind);
      // bare marker stands alone → recognized
      expect(nativeOf("f/mypipe/bare")).toEqual(["data_upload"]);
      // trailing content → rejected (no native trigger)
      expect(nativeOf("f/mypipe/trailing")).toEqual([]);
      expect(nativeOf("f/mypipe/kafka")).toEqual([]);
    },
  );
});

test("go/bash fallback: a multi-word `// tag` is rejected (single token only)", async () => {
  // A worker tag is one token; trailing prose must NOT smuggle a bogus tag that
  // would route the preview to a non-existent worker.
  await withFolder(
    {
      "single.go": `// pipeline\n// tag gpu\npackage inner\nfunc main() {}\n`,
      "multi.go": `// pipeline\n// tag gpu for heavy jobs\npackage inner\nfunc main() {}\n`,
    },
    async (root, folder) => {
      const { scripts } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      expect(scripts.find((s) => s.path === "f/mypipe/single")?.tag).toBe("gpu");
      // multi-word → no tag (default worker), not "gpu for heavy jobs"
      expect(scripts.find((s) => s.path === "f/mypipe/multi")?.tag).toBeUndefined();
    },
  );
});

test("`# volume:` producer connects to its `# on volume://` consumer", async () => {
  // Volume annotations are parsed separately from the wasm body parser (mirrors
  // the frontend/backend); without that pass the producer has no write edge.
  await withFolder(
    {
      "producer.py": `# pipeline\n# volume: cache /tmp/cache\ndef main():\n    pass\n`,
      "consumer.py": `# pipeline\n# on volume://cache\ndef main():\n    pass\n`,
    },
    async (root, folder) => {
      const { graph } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });

      expect(graph.assets).toContainEqual({ kind: "volume", path: "cache" });
      // producer carries the rw write edge to the volume
      const producerEdge = graph.edges.find(
        (e) =>
          e.runnable_path === "f/mypipe/producer" &&
          e.asset_kind === "volume" &&
          e.asset_path === "cache",
      );
      expect(producerEdge?.access_type).toBe("rw");
      // consumer subscribes to the same volume (the connecting trigger)
      const at = graph.triggers.find(
        (t) => t.trigger_kind === "asset" && t.runnable_path === "f/mypipe/consumer",
      ) as Extract<(typeof graph.triggers)[number], { trigger_kind: "asset" }> | undefined;
      expect(at?.asset_kind).toBe("volume");
      expect(at?.asset_path).toBe("cache");
    },
  );
});

test("`// macros` library: node with signatures + call-detected consumer edge, counted", async () => {
  // The wasm asset parser drops `// macros`/`// use`; the CLI-side lexical scan
  // must surface the library node, its macro signatures, and the caller edge so
  // `--local` reaches parity with the deployed graph.
  await withFolder(
    {
      "macros_finance.duckdb.sql":
        `-- macros\nCREATE MACRO net_revenue(gross, refunds) AS gross - refunds;\nCREATE OR REPLACE MACRO safe_div(a, b) AS TABLE SELECT a / b;\n`,
      "fct.duckdb.sql":
        `-- pipeline\n-- on datatable://main/orders\nSELECT net_revenue(gross, refunds) AS rev FROM main.orders;\n`,
    },
    async (root, folder) => {
      const { graph } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });

      // the library is a node carrying its macro signatures
      const lib = graph.runnables.find((r) => r.path === "f/mypipe/macros_finance");
      expect(lib?.macros).toEqual([
        { name: "net_revenue", params: "gross, refunds", is_table: false },
        { name: "safe_div", params: "a, b", is_table: true },
      ]);

      // library counts toward the script total (2 scripts, not just the 1 pipeline member)
      expect(graph.runnables.map((r) => r.path).sort()).toEqual([
        "f/mypipe/fct",
        "f/mypipe/macros_finance",
      ]);

      // the caller edge names only the macro actually called (safe_div is not)
      expect(graph.macro_edges).toEqual([
        {
          lib_path: "f/mypipe/macros_finance",
          consumer_path: "f/mypipe/fct",
          macro_names: ["net_revenue"],
          via_use: false,
        },
      ]);
    },
  );
});

test("`// use <lib>` forces a whole-library edge even with no lexical call", async () => {
  // Dynamic-SQL callers annotate `// use`; the edge lists every macro and is
  // flagged via_use (mirrors the deployed graph).
  await withFolder(
    {
      "stats.duckdb.sql":
        `-- macros\nCREATE MACRO zscore(x, m, s) AS (x - m) / s;\n`,
      "report.duckdb.sql":
        `-- pipeline\n-- on datatable://main/metrics\n-- use f/mypipe/stats\nSELECT query('SELECT zscore(v, 0, 1) FROM main.metrics');\n`,
    },
    async (root, folder) => {
      const { graph } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      expect(graph.macro_edges).toEqual([
        {
          lib_path: "f/mypipe/stats",
          consumer_path: "f/mypipe/report",
          macro_names: ["zscore"],
          via_use: true,
        },
      ]);
      // the library still becomes a node
      expect(graph.runnables.some((r) => r.path === "f/mypipe/stats")).toBe(true);
    },
  );
});

test("an unused `// macros` library is not surfaced as a node", async () => {
  // Parity with the deployed graph: a library appears only when it is an edge
  // endpoint (a consumer actually uses it).
  await withFolder(
    {
      "unused.duckdb.sql": `-- macros\nCREATE MACRO helper(a) AS a + 1;\n`,
      "solo.duckdb.sql": `-- pipeline\nSELECT 1;\n`,
    },
    async (root, folder) => {
      const { graph } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      expect(graph.runnables.map((r) => r.path)).toEqual(["f/mypipe/solo"]);
      expect(graph.macro_edges).toBeUndefined();
    },
  );
});

test("a shared macro library OUTSIDE the pipeline folder is resolved (workspace-wide)", async () => {
  // The deployed graph loads the macro registry workspace-wide and only
  // folder-scopes consumers, so a pipeline in `f/mypipe` can use a shared library
  // in `f/shared`. Local discovery must walk the whole `f/` tree, not just the
  // folder, or the edge/node/docs are missing (parity gap).
  const root = mkdtempSync(join(tmpdir(), "wm-pl-"));
  writeFileSync(join(root, "wmill.yaml"), "defaultTs: bun\n");
  mkdirSync(join(root, "f", "shared"), { recursive: true });
  mkdirSync(join(root, "f", "mypipe"), { recursive: true });
  writeFileSync(
    join(root, "f", "shared", "stats.duckdb.sql"),
    `-- macros\nCREATE MACRO zscore(x, m, s) AS (x - m) / s;\n`,
  );
  // one consumer calls the shared macro lexically, another pulls it via `// use`
  writeFileSync(
    join(root, "f", "mypipe", "fct.duckdb.sql"),
    `-- pipeline\n-- on datatable://main/metrics\nSELECT zscore(v, 0, 1) FROM main.metrics;\n`,
  );
  writeFileSync(
    join(root, "f", "mypipe", "dyn.duckdb.sql"),
    `-- pipeline\n-- on datatable://main/metrics\n-- use f/shared/stats\nSELECT query('SELECT zscore(v, 0, 1)');\n`,
  );
  try {
    const { graph } = await buildLocalPipelineGraph({ root, folder: "mypipe", defaultTs: "bun" });
    expect(graph.macro_edges).toEqual([
      {
        lib_path: "f/shared/stats",
        consumer_path: "f/mypipe/dyn",
        macro_names: ["zscore"],
        via_use: true,
      },
      {
        lib_path: "f/shared/stats",
        consumer_path: "f/mypipe/fct",
        macro_names: ["zscore"],
        via_use: false,
      },
    ]);
    // the out-of-folder library is surfaced as a node with its signatures
    expect(graph.runnables.find((r) => r.path === "f/shared/stats")?.macros).toEqual([
      { name: "zscore", params: "x, m, s", is_table: false },
    ]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("macro library that CONSUMES another library gets a lib→lib edge (both nodes surface)", async () => {
  // The deploy path records macro_usage for any DuckDB script, including a macro
  // library calling another library's macros. A `base → derived` edge must exist
  // (and `base` must not disappear) even though `derived` is not a `// pipeline`.
  await withFolder(
    {
      "base.duckdb.sql": `-- macros\nCREATE MACRO base_add(a, b) AS a + b;\n`,
      // derived is a `// macros` library (NOT `// pipeline`) whose body calls base_add
      "derived.duckdb.sql":
        `-- macros\nCREATE MACRO derived_sum(a, b) AS base_add(a, b) * 2;\n`,
      // the pipeline consumer calls derived_sum
      "report.duckdb.sql":
        `-- pipeline\n-- on datatable://main/t\nSELECT derived_sum(x, y) FROM main.t;\n`,
    },
    async (root, folder) => {
      const { graph } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      expect(graph.macro_edges).toEqual([
        // base → derived: the intermediate library is itself a consumer
        {
          lib_path: "f/mypipe/base",
          consumer_path: "f/mypipe/derived",
          macro_names: ["base_add"],
          via_use: false,
        },
        // derived → report: the pipeline consumer
        {
          lib_path: "f/mypipe/derived",
          consumer_path: "f/mypipe/report",
          macro_names: ["derived_sum"],
          via_use: false,
        },
      ]);
      // both libraries surface as nodes with their signatures (base does NOT
      // disappear just because it is only reached transitively)
      expect(graph.runnables.find((r) => r.path === "f/mypipe/base")?.macros).toEqual([
        { name: "base_add", params: "a, b", is_table: false },
      ]);
      expect(graph.runnables.find((r) => r.path === "f/mypipe/derived")?.macros).toEqual([
        { name: "derived_sum", params: "a, b", is_table: false },
      ]);
    },
  );
});

test("a `// macros` (slash-prefix) DuckDB library is detected (backend prefix parity)", async () => {
  // A `.duckdb.sql` library may head its annotation with `// macros` (the backend
  // accepts `//`/`--`/`#` for any language); the local scan must too.
  await withFolder(
    {
      "lib.duckdb.sql": `// macros\nCREATE MACRO dbl(a) AS a * 2;\n`,
      "use_it.duckdb.sql": `-- pipeline\n-- on datatable://main/t\nSELECT dbl(x) FROM main.t;\n`,
    },
    async (root, folder) => {
      const { graph } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      expect(graph.macro_edges).toEqual([
        {
          lib_path: "f/mypipe/lib",
          consumer_path: "f/mypipe/use_it",
          macro_names: ["dbl"],
          via_use: false,
        },
      ]);
      expect(graph.runnables.find((r) => r.path === "f/mypipe/lib")?.macros).toEqual([
        { name: "dbl", params: "a", is_table: false },
      ]);
    },
  );
});

test("a non-pipeline DuckDB macro consumer is a display-only node, never a run step", async () => {
  // A `.duckdb.sql` helper that calls a macro but isn't `// pipeline` surfaces as
  // a graph node (lineage parity) but is NOT a runnable pipeline script: it must
  // be absent from `scripts` (the previewable set `run --local` selects from), so
  // it can never be scheduled or fail a preview for lack of local content.
  await withFolder(
    {
      "lib.duckdb.sql": `-- macros\nCREATE MACRO dbl(a) AS a * 2;\n`,
      "helper.duckdb.sql": `-- on ducklake://main/src\nSELECT dbl(x) FROM main.src;\n`,
      "root.duckdb.sql": `-- pipeline\n-- materialize ducklake://main/out\nSELECT dbl(1) AS v;\n`,
    },
    async (root, folder) => {
      const { graph, scripts } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });

      // the helper IS a node (macro consumer, for lineage display) …
      const helper = graph.runnables.find((r) => r.path === "f/mypipe/helper");
      expect(helper).toBeDefined();
      // … but not a pipeline member (no local file to preview) and carries no macros
      expect(helper?.in_pipeline).toBeFalsy();
      expect(helper?.macros).toBeUndefined();
      // the previewable set (what `run --local` can schedule) is ONLY the member
      expect(scripts.map((s) => s.path)).toEqual(["f/mypipe/root"]);
      // and the macro edge still connects lib → helper for the lineage view
      expect(graph.macro_edges).toContainEqual({
        lib_path: "f/mypipe/lib",
        consumer_path: "f/mypipe/helper",
        macro_names: ["dbl"],
        via_use: false,
      });
    },
  );
});

test("`#`-comment languages (ruby) use the `#` annotation fallback (no wasm parser)", async () => {
  await withFolder(
    { "ingest.rb": `# pipeline\n# on s3://demo/raw.csv\nputs "hi"\n` },
    async (root, folder) => {
      const { graph } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      expect(graph.runnables.map((r) => r.path)).toContain("f/mypipe/ingest");
      const at = graph.triggers.find(
        (t) => t.trigger_kind === "asset" && t.runnable_path === "f/mypipe/ingest",
      ) as Extract<(typeof graph.triggers)[number], { trigger_kind: "asset" }> | undefined;
      expect(at?.asset_kind).toBe("s3object");
      expect(at?.asset_path).toBe("demo/raw.csv");
    },
  );
});

test("pipeline docs renders a `Macro libraries` section (call + `// use`)", async () => {
  const { generatePipelineMarkdown } = await import(
    "../src/commands/pipeline/docs.ts"
  );
  await withFolder(
    {
      "macros_finance.duckdb.sql":
        `-- macros\nCREATE MACRO net_revenue(gross, refunds) AS gross - refunds;\nCREATE MACRO safe_div(a, b) AS TABLE SELECT a / b;\n`,
      "fct.duckdb.sql":
        `-- pipeline\n-- on datatable://main/orders\nSELECT net_revenue(gross, refunds) FROM main.orders;\n`,
      "report.duckdb.sql":
        `-- pipeline\n-- on datatable://main/orders\n-- use f/mypipe/macros_finance\nSELECT query('SELECT safe_div(1, 2)');\n`,
    },
    async (root, folder) => {
      const { graph } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      const md = generatePipelineMarkdown(folder, graph, [], true);

      // count line mentions the library; two pipeline scripts, one library
      expect(md).toContain("**2** scripts · **1** asset · **1** macro library");
      // dedicated section with signatures (TABLE marker) and callers
      expect(md).toContain("## Macro libraries");
      expect(md).toContain("### `f/mypipe/macros_finance`");
      expect(md).toContain("- `net_revenue(gross, refunds)`");
      expect(md).toContain("- `safe_div(a, b)` → TABLE");
      expect(md).toContain("`f/mypipe/fct` (calls `net_revenue`)");
      expect(md).toContain("`f/mypipe/report` (via `// use`)");
      // the library is NOT double-listed under `## Scripts`
      expect(md).not.toContain("### `f/mypipe/macros_finance`\n\n- **");
    },
  );
});
