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
