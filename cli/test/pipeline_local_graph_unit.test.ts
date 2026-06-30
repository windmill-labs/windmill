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
