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
    },
    async (root, folder) => {
      const { graph } = await buildLocalPipelineGraph({ root, folder, defaultTs: "bun" });
      const native = graph.triggers.filter((t) => t.trigger_kind !== "asset");
      expect(native.map((t) => t.trigger_kind)).toContain("data_upload");
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
