// `wmill pipeline docs <folder>` — generate a PIPELINE.md (+ AGENTS.md / CLAUDE.md
// pointer) describing a folder's pipeline so an editor or agentic loop has the
// same context the UI surfaces: the asset DAG, per-script triggers/IO, and the
// schemas of the datatables the pipeline touches. Mirrors the app docs pattern
// (`app/generate_agents.ts:regenerateAgentDocs`) but scoped to a pipeline folder.

import { writeFile } from "node:fs/promises";
import { existsSync, readFileSync } from "node:fs";
import * as path from "node:path";
import { OpenAPI } from "../../../gen/index.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { mergeConfigWithConfigFile } from "../../core/conf.ts";
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
export function generatePipelineMarkdown(
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

  // `// macros` libraries are definition-only nodes (not runnable pipeline
  // steps), so they get their own section below and are excluded from the
  // per-script listing + the script count.
  const macroLibs = graph.runnables
    .filter((r) => r.usage_kind === "script" && (r.macros?.length ?? 0) > 0)
    .sort((a, b) => a.path.localeCompare(b.path));
  const macroLibPaths = new Set(macroLibs.map((r) => r.path));
  const macroConsumersByLib = new Map<string, { consumer: string; names: string[]; viaUse?: boolean }[]>();
  for (const me of graph.macro_edges ?? []) {
    (macroConsumersByLib.get(me.lib_path) ?? macroConsumersByLib.set(me.lib_path, []).get(me.lib_path)!).push({
      consumer: me.consumer_path,
      names: me.macro_names,
      viaUse: me.via_use,
    });
  }

  const scripts = graph.runnables
    .filter((r) => r.usage_kind === "script" && !macroLibPaths.has(r.path))
    .map((r) => r.path)
    .sort();

  let md = `# Pipeline \`f/${folder}\`

${local ? "_Built from local working-tree files (`// pipeline` scripts)._" : "_Built from the deployed workspace asset graph._"}

A data pipeline is a folder of scripts marked \`// pipeline\` and wired by asset
annotations. A script subscribes to upstream data with \`// on <asset-uri>\` and
produces data by reading/writing assets in its body (\`datatable://\`,
\`ducklake://\`, \`s3://\`, \`volume://\`). The cascade runs a producer, then every
downstream subscriber, in topological order.

- **${scripts.length}** script${scripts.length === 1 ? "" : "s"} · **${graph.assets.length}** asset${graph.assets.length === 1 ? "" : "s"}${macroLibs.length > 0 ? ` · **${macroLibs.length}** macro librar${macroLibs.length === 1 ? "y" : "ies"}` : ""}

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

  // Macro libraries: `// macros` scripts whose macros are injected into consuming
  // DuckDB scripts at run time. List each library's signatures and its callers so
  // an agent discovers the reuse layer instead of re-inlining the logic.
  if (macroLibs.length > 0) {
    md += `## Macro libraries\n\n`;
    md += `\`// macros\` DuckDB libraries. Their \`CREATE MACRO\` definitions are injected as\nTEMP macros into consuming scripts at run time — call a macro by name, or force the\nwhole library in with \`// use <lib-path>\` (needed for macros only reached via dynamic SQL).\n\n`;
    for (const lib of macroLibs) {
      md += `### \`${lib.path}\`\n\n`;
      for (const m of lib.macros ?? []) {
        md += `- \`${m.name}(${m.params ?? ""})\`${m.is_table ? " → TABLE" : ""}\n`;
      }
      const consumers = [...(macroConsumersByLib.get(lib.path) ?? [])].sort((a, b) =>
        a.consumer.localeCompare(b.consumer),
      );
      if (consumers.length > 0) {
        md += `- **Used by:**\n`;
        for (const c of consumers) {
          md += `  - \`${c.consumer}\` (${c.viaUse ? "via \`// use\`" : `calls ${c.names.map((n) => `\`${n}\``).join(", ")}`})\n`;
        }
      }
      md += `\n`;
    }
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
  const f = folder.replace(/^f\//, "").replace(/\/$/, "");
  // `docs` WRITES PIPELINE.md / AGENTS.md / CLAUDE.md under `f/<folder>`; a `..`
  // segment would escape the folder and clobber files elsewhere in the tree.
  if (f.split("/").includes("..")) {
    log.error(colors.red(`Invalid folder '${folder}': '..' path segments are not allowed.`));
    return;
  }
  const root = workspaceRoot();
  // defaultTs (from wmill.yaml) drives .ts → bun/deno inference for the local graph.
  const merged = await mergeConfigWithConfigFile(opts);

  const workspace = opts.local ? undefined : await resolveWorkspace(opts);
  if (!opts.local) await requireLogin(opts);

  const graph = opts.local
    ? (await buildLocalPipelineGraph({ root, folder: f, defaultTs: merged.defaultTs })).graph
    : await fetchDeployedGraph(workspace!.workspaceId, f);

  if (graph.runnables.length === 0) {
    // The deployed graph is empty — but a user/agent in a working tree may have
    // local `// pipeline` scripts not yet deployed. Point them at --local rather
    // than leaving them thinking the folder is empty.
    if (!opts.local) {
      try {
        const localCount =
          (await buildLocalPipelineGraph({ root, folder: f, defaultTs: merged.defaultTs })).graph
            .runnables.length;
        if (localCount > 0) {
          log.info(
            `No deployed pipeline scripts in f/${f}, but ${localCount} \`// pipeline\` script(s) exist in the local working tree — run \`wmill pipeline docs ${f} --local\` to document those.`,
          );
          return;
        }
      } catch {
        // best-effort hint; fall through to the generic message
      }
    }
    log.info(`No pipeline scripts in f/${f}.`);
    return;
  }

  let datatableSchemas: any[] = [];
  const hasExplicitWorkspace =
    !!opts.workspace ||
    (!!opts.baseUrl && !!opts.token) ||
    (!!process.env["WM_WORKSPACE"] &&
      !!process.env["WM_TOKEN"] &&
      !!(process.env["BASE_INTERNAL_URL"] ?? process.env["BASE_URL"]));
  if (!opts.local || hasExplicitWorkspace) {
    try {
      const schemaWorkspace = workspace ?? await resolveWorkspace(opts);
      if (opts.local) await requireLogin(opts);
      datatableSchemas = await wmill.listDataTableSchemas({ workspace: schemaWorkspace.workspaceId });
    } catch (err: any) {
      log.warn(colors.yellow(`Could not fetch datatable schemas: ${err.message}`));
    }
  }

  const md = generatePipelineMarkdown(f, graph, datatableSchemas, !!opts.local);
  const folderDir = path.join(root, "f", f);
  await writeFile(path.join(folderDir, "PIPELINE.md"), md, "utf-8");

  // PIPELINE.md is ours to own. AGENTS.md / CLAUDE.md are commonly user-authored,
  // so write the pointer only when the file is ABSENT or is byte-for-byte the
  // exact pointer we generate — never clobber hand-written instructions, even a
  // file that merely references `@PIPELINE.md` alongside its own content.
  const written = ["PIPELINE.md"];
  const pointers: Array<[string, string]> = [
    ["AGENTS.md", `See @PIPELINE.md for this pipeline's graph, assets, and how to run it.\n`],
    ["CLAUDE.md", `Instructions are in @PIPELINE.md\n`],
  ];
  for (const [name, content] of pointers) {
    const p = path.join(folderDir, name);
    if (existsSync(p)) {
      let existing: string;
      try {
        existing = readFileSync(p, "utf-8");
      } catch {
        continue;
      }
      // Anything other than our exact generated pointer is treated as
      // hand-written and preserved (writing when identical is a no-op anyway).
      if (existing.trim() !== content.trim()) {
        log.warn(
          colors.yellow(`Kept existing f/${f}/${name} (hand-written — not the generated pointer)`),
        );
        continue;
      }
    }
    await writeFile(p, content, "utf-8");
    written.push(name);
  }
  log.info(colors.green(`✓ Wrote ${written.join(", ")} to f/${f}`));
}
