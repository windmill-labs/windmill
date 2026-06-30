// `wmill pipeline docs <folder>` — generate a PIPELINE.md (+ AGENTS.md / CLAUDE.md
// pointer) describing a folder's pipeline so an editor or agentic loop has the
// same context the UI surfaces: the asset DAG, per-script triggers/IO, and the
// schemas of the datatables the pipeline touches. Mirrors the app docs pattern
// (`app/generate_agents.ts:regenerateAgentDocs`) but scoped to a pipeline folder.

import { writeFile } from "node:fs/promises";
import * as path from "node:path";
import { OpenAPI } from "../../../gen/index.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as log from "../../core/log.ts";
import { colors } from "@cliffy/ansi/colors";
import { GlobalOptions } from "../../types.ts";
import {
  type AssetGraph,
  buildLocalPipelineGraph,
  workspaceRoot,
} from "./localGraph.ts";

const ASSET_KINDS = "s3object,ducklake,datatable,volume";

function assetUri(kind: string, p: string): string {
  const prefix = kind === "s3object" ? "s3" : kind;
  return `${prefix}://${p}`;
}

async function fetchDeployedGraph(
  workspaceId: string,
  folder: string,
): Promise<AssetGraph> {
  const res = await fetch(
    `${OpenAPI.BASE}/w/${workspaceId}/assets/graph?folder=${encodeURIComponent(folder)}&asset_kinds=${ASSET_KINDS}`,
    { headers: { Authorization: `Bearer ${OpenAPI.TOKEN}` } },
  );
  if (!res.ok) {
    throw new Error(`GET assets/graph -> ${res.status}: ${await res.text()}`);
  }
  return (await res.json()) as AssetGraph;
}

// Render the pipeline graph as a markdown document.
function generatePipelineMarkdown(
  folder: string,
  graph: AssetGraph,
  datatableSchemas: any[],
  local: boolean,
): string {
  const writesByScript = new Map<string, string[]>();
  const readsByScript = new Map<string, string[]>();
  for (const e of graph.edges) {
    if (e.runnable_kind !== "script") continue;
    const uri = assetUri(e.asset_kind, e.asset_path);
    if (e.access_type === "w" || e.access_type === "rw") {
      (writesByScript.get(e.runnable_path) ?? writesByScript.set(e.runnable_path, []).get(e.runnable_path)!).push(uri);
    }
    if (e.access_type === "r" || e.access_type === "rw" || e.access_type === undefined) {
      (readsByScript.get(e.runnable_path) ?? readsByScript.set(e.runnable_path, []).get(e.runnable_path)!).push(uri);
    }
  }
  const onByScript = new Map<string, string[]>();
  const nativeByScript = new Map<string, string[]>();
  for (const t of graph.triggers) {
    if (t.runnable_kind !== "script") continue;
    if (t.trigger_kind === "asset") {
      const at = t as Extract<AssetGraph["triggers"][number], { trigger_kind: "asset" }>;
      (onByScript.get(at.runnable_path) ?? onByScript.set(at.runnable_path, []).get(at.runnable_path)!).push(assetUri(at.asset_kind, at.asset_path));
    } else {
      (nativeByScript.get(t.runnable_path) ?? nativeByScript.set(t.runnable_path, []).get(t.runnable_path)!).push(t.trigger_kind);
    }
  }

  const scripts = graph.runnables.filter((r) => r.usage_kind === "script").map((r) => r.path).sort();

  let md = `# Pipeline \`f/${folder}\`

${local ? "_Built from local working-tree files (`// pipeline` scripts)._" : "_Built from the deployed workspace asset graph._"}

A data pipeline is a folder of scripts marked \`// pipeline\` and wired by asset
annotations. A script subscribes to upstream data with \`// on <asset-uri>\` and
produces data by reading/writing assets in its body (\`datatable://\`,
\`ducklake://\`, \`s3://\`, \`volume://\`). The cascade runs a producer, then every
downstream subscriber, in topological order.

- **${scripts.length}** script${scripts.length === 1 ? "" : "s"} · **${graph.assets.length}** asset${graph.assets.length === 1 ? "" : "s"}

## Scripts

`;

  for (const s of scripts) {
    const on = onByScript.get(s) ?? [];
    const native = nativeByScript.get(s) ?? [];
    const writes = [...new Set(writesByScript.get(s) ?? [])].sort();
    const reads = [...new Set(readsByScript.get(s) ?? [])].sort();
    md += `### \`${s}\`\n\n`;
    if (native.length) md += `- **Triggers:** ${native.map((n) => `\`${n}\``).join(", ")}\n`;
    if (on.length) md += `- **On (subscribes to):** ${on.map((u) => `\`${u}\``).join(", ")}\n`;
    if (reads.length) md += `- **Reads:** ${reads.map((u) => `\`${u}\``).join(", ")}\n`;
    if (writes.length) md += `- **Writes:** ${writes.map((u) => `\`${u}\``).join(", ")}\n`;
    if (!native.length && !on.length && !reads.length && !writes.length) {
      md += `- _No declared triggers or asset IO._\n`;
    }
    md += `\n`;
  }

  // Datatable schemas, restricted to datatables this pipeline references.
  const referencedDatatables = new Set(
    graph.assets.filter((a) => a.kind === "datatable").map((a) => a.path.split("/")[0]),
  );
  const relevant = (datatableSchemas ?? []).filter((dt: any) => referencedDatatables.has(dt.datatable_name));
  if (relevant.length > 0) {
    md += `## Datatable schemas\n\n`;
    for (const dt of relevant) {
      md += `### Datatable \`${dt.datatable_name}\`\n\n`;
      if (dt.error) {
        md += `> ⚠️ ${dt.error}\n\n`;
        continue;
      }
      for (const [schemaName, tables] of Object.entries(dt.schemas ?? {})) {
        for (const [tableName, columns] of Object.entries(tables as Record<string, any>)) {
          const ref = schemaName === "public"
            ? `${dt.datatable_name}/${tableName}`
            : `${dt.datatable_name}/${schemaName}:${tableName}`;
          md += `- \`datatable://${ref}\`\n`;
          for (const [col, type] of Object.entries(columns as Record<string, any>)) {
            md += `  - \`${col}\`: ${type}\n`;
          }
        }
      }
      md += `\n`;
    }
  }

  md += `## Working with this pipeline

- **Inspect the graph:** \`wmill pipeline show ${folder} --local\`
- **Run the cascade locally (no deploy):** \`wmill pipeline run ${folder} --local\`
  (optionally \`--from <script> --to <script>\` to bound it, \`--dry-run\` to preview the plan)
- **Live visual preview while editing:** \`wmill pipeline dev ${folder}\`
- **Deploy:** \`wmill sync push\`

Mark a script as part of this pipeline with a bare \`// pipeline\` comment
(\`#\` for Python, \`--\` for SQL). Add \`// on <asset-uri>\` lines to subscribe it to
upstream assets.

---
*Generated by \`wmill pipeline docs\`.*
`;
  return md;
}

export async function generatePipelineDocs(
  opts: GlobalOptions & { local?: boolean; defaultTs?: "bun" | "deno" },
  folder: string,
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const f = folder.replace(/^f\//, "").replace(/\/$/, "");
  const root = workspaceRoot();

  const graph = opts.local
    ? (await buildLocalPipelineGraph({ root, folder: f, defaultTs: opts.defaultTs })).graph
    : await fetchDeployedGraph(workspace.workspaceId, f);

  if (graph.runnables.length === 0) {
    log.info(`No pipeline scripts in f/${f}.`);
    return;
  }

  let datatableSchemas: any[] = [];
  try {
    datatableSchemas = await wmill.listDataTableSchemas({ workspace: workspace.workspaceId });
  } catch (err: any) {
    log.warn(colors.yellow(`Could not fetch datatable schemas: ${err.message}`));
  }

  const md = generatePipelineMarkdown(f, graph, datatableSchemas, !!opts.local);
  const folderDir = path.join(root, "f", f);
  await writeFile(path.join(folderDir, "PIPELINE.md"), md, "utf-8");
  await writeFile(path.join(folderDir, "AGENTS.md"), `See @PIPELINE.md for this pipeline's graph, assets, and how to run it.\n`, "utf-8");
  await writeFile(path.join(folderDir, "CLAUDE.md"), `Instructions are in @PIPELINE.md\n`, "utf-8");
  log.info(colors.green(`✓ Wrote PIPELINE.md, AGENTS.md, CLAUDE.md to f/${f}`));
}
