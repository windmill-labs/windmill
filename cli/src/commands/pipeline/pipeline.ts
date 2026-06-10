import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";

import { OpenAPI } from "../../../gen/index.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";

// Mirrors the asset-graph endpoint payload (backend/windmill-api-assets).
// Raw-fetched because these routes are newer than the checked-in generated
// client; swap to the generated functions on the next client regen.
type GraphRunnable = {
  path: string;
  usage_kind: "script" | "flow" | "job";
  in_pipeline?: boolean;
};
type GraphEdge = {
  runnable_kind: string;
  runnable_path: string;
  asset_kind: string;
  asset_path: string;
  access_type?: "r" | "w" | "rw";
};
type GraphTrigger =
  | {
      trigger_kind: "asset";
      asset_kind: string;
      asset_path: string;
      runnable_kind: string;
      runnable_path: string;
    }
  | {
      trigger_kind: string;
      path?: string;
      runnable_kind: string;
      runnable_path: string;
      missing?: boolean;
    };
type AssetGraph = {
  runnables: GraphRunnable[];
  assets: { kind: string; path: string }[];
  edges: GraphEdge[];
  triggers: GraphTrigger[];
};

async function apiGet<T>(path: string): Promise<T> {
  const response = await fetch(`${OpenAPI.BASE}${path}`, {
    headers: { Authorization: `Bearer ${OpenAPI.TOKEN}` },
  });
  if (!response.ok) {
    const body = await response.text();
    throw new Error(`GET ${path} -> ${response.status}: ${body}`);
  }
  return (await response.json()) as T;
}

async function list(opts: GlobalOptions & { json?: boolean }) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const items = await apiGet<{ folder: string; script_count: number }[]>(
    `/w/${workspace.workspaceId}/assets/pipelines`,
  );
  if (opts.json) {
    console.log(JSON.stringify(items));
  } else if (items.length === 0) {
    log.info(
      "No pipelines in this workspace. Mark scripts with a `// pipeline` comment (plus `// on <spec>` triggers) and push them into a folder.",
    );
  } else {
    new Table()
      .header(["Folder", "Scripts"])
      .padding(2)
      .border(true)
      .body(items.map((p) => [`f/${p.folder}`, String(p.script_count)]))
      .render();
  }
}

const ASSET_KINDS = "s3object,ducklake,datatable,volume";

function assetUri(kind: string, path: string): string {
  const prefix = kind === "s3object" ? "s3" : kind;
  return `${prefix}://${path}`;
}

function shortName(scriptPath: string): string {
  return scriptPath.split("/").pop() ?? scriptPath;
}

async function show(
  opts: GlobalOptions & { json?: boolean },
  folder: string,
) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const f = folder.replace(/^f\//, "").replace(/\/$/, "");
  const graph = await apiGet<AssetGraph>(
    `/w/${workspace.workspaceId}/assets/graph?folder=${encodeURIComponent(f)}&asset_kinds=${ASSET_KINDS}`,
  );
  if (opts.json) {
    console.log(JSON.stringify(graph));
    return;
  }
  if (graph.runnables.length === 0) {
    log.info(
      `No pipeline scripts in f/${f}. Mark scripts with a \`// pipeline\` comment and push them.`,
    );
    return;
  }

  // Index the graph: writes per script, subscribers per asset, native
  // trigger markers per script, asset subscriptions per script.
  const writesByScript = new Map<string, string[]>();
  for (const e of graph.edges) {
    if (e.access_type === "w" || e.access_type === "rw") {
      const uri = assetUri(e.asset_kind, e.asset_path);
      writesByScript.set(e.runnable_path, [
        ...(writesByScript.get(e.runnable_path) ?? []),
        uri,
      ]);
    }
  }
  const subsByAsset = new Map<string, string[]>();
  const subsByScript = new Map<string, string[]>();
  const nativeByScript = new Map<
    string,
    { kind: string; path?: string; missing?: boolean }[]
  >();
  for (const t of graph.triggers) {
    if (t.trigger_kind === "asset") {
      const at = t as Extract<GraphTrigger, { trigger_kind: "asset" }>;
      const uri = assetUri(at.asset_kind, at.asset_path);
      subsByAsset.set(uri, [...(subsByAsset.get(uri) ?? []), t.runnable_path]);
      subsByScript.set(t.runnable_path, [
        ...(subsByScript.get(t.runnable_path) ?? []),
        uri,
      ]);
    } else {
      const nt = t as Exclude<GraphTrigger, { trigger_kind: "asset" }>;
      nativeByScript.set(t.runnable_path, [
        ...(nativeByScript.get(t.runnable_path) ?? []),
        { kind: nt.trigger_kind, path: nt.path, missing: nt.missing },
      ]);
    }
  }

  function triggerBadges(script: string): string {
    const out: string[] = [];
    for (const t of nativeByScript.get(script) ?? []) {
      if (t.kind === "data_upload") {
        out.push(colors.magenta("[data upload]"));
      } else if (t.missing) {
        out.push(colors.red(`[${t.kind} ✗ missing]`));
      } else {
        out.push(colors.yellow(`[${t.kind}${t.path ? ` ${t.path}` : ""}]`));
      }
    }
    return out.length > 0 ? " " + out.join(" ") : "";
  }

  const printed = new Set<string>();
  const lines: string[] = [];

  function printScript(script: string, prefix: string, extraOn?: string[]) {
    const alsoOn =
      extraOn && extraOn.length > 0
        ? colors.dim(`  (also on: ${extraOn.join(", ")})`)
        : "";
    if (printed.has(script)) {
      lines.push(
        `${prefix}${colors.bold(shortName(script))}${colors.dim(" ↻ shown above")}`,
      );
      return;
    }
    printed.add(script);
    lines.push(`${prefix}${colors.bold(shortName(script))}${triggerBadges(script)}${alsoOn}`);
    const childPrefix = prefix.replace(/├─ $/, "│  ").replace(/└─ $/, "   ");
    const writes = [...(writesByScript.get(script) ?? [])].sort();
    writes.forEach((uri, i) => {
      const lastAsset = i === writes.length - 1;
      const assetBranch = lastAsset ? "└─▶ " : "├─▶ ";
      lines.push(`${childPrefix}${assetBranch}${colors.cyan(uri)}`);
      const assetChildPrefix = childPrefix + (lastAsset ? "    " : "│   ");
      const subs = [...(subsByAsset.get(uri) ?? [])].sort();
      subs.forEach((sub, j) => {
        const branch = j === subs.length - 1 ? "└─ " : "├─ ";
        const otherOn = (subsByScript.get(sub) ?? []).filter((u) => u !== uri);
        printScript(sub, assetChildPrefix + branch, otherOn);
      });
    });
  }

  // Roots: pipeline scripts that aren't subscribed to any asset — sources
  // (data upload, schedule, webhook) and manual entries.
  const roots = graph.runnables
    .map((r) => r.path)
    .filter((p) => !(subsByScript.get(p)?.length))
    .sort();

  // UI-first markers (data_upload, webhook) have no trigger row — the
  // graph endpoint can't surface them, they live as `// on <kind>`
  // annotations in the body. Roots are where sources matter, so fetch just
  // those bodies and lift the marker kinds the canvas would show.
  const MARKER_KINDS = ["data_upload", "webhook", "email"];
  await Promise.all(
    roots.map(async (p) => {
      const r = graph.runnables.find((x) => x.path === p);
      if (r?.usage_kind !== "script") return;
      try {
        const script = await wmill.getScriptByPath({
          workspace: workspace.workspaceId,
          path: p,
        });
        const existing = nativeByScript.get(p) ?? [];
        for (const line of (script.content ?? "").split("\n")) {
          const m = line.match(/^\s*(?:\/\/|--|#)\s*on\s+(\w+)\s*$/);
          if (!m) continue;
          const kind = m[1];
          if (!MARKER_KINDS.includes(kind)) continue;
          if (!existing.some((t) => t.kind === kind)) {
            existing.push({ kind });
          }
        }
        if (existing.length > 0) nativeByScript.set(p, existing);
      } catch {
        // body fetch is best-effort enrichment only
      }
    }),
  );

  const scriptCount = graph.runnables.length;
  const assetCount = graph.assets.length;
  log.info(
    colors.bold(`Pipeline f/${f}`) +
      colors.dim(` — ${scriptCount} script${scriptCount === 1 ? "" : "s"} · ${assetCount} asset${assetCount === 1 ? "" : "s"}`),
  );
  lines.push("");
  for (const root of roots) {
    printScript(root, "");
    lines.push("");
  }
  // Anything unreachable from the roots (e.g. cycles) still gets listed.
  for (const r of graph.runnables) {
    if (!printed.has(r.path)) {
      printScript(r.path, "");
      lines.push("");
    }
  }
  console.log(lines.join("\n"));
}

const command = new Command()
  .description(
    "inspect asset-driven pipelines (scripts marked `// pipeline`, wired by `// on <spec>` annotations)",
  )
  .command("list", "list pipeline folders in the workspace")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command(
    "show",
    "render a pipeline folder's DAG (sources, lineage, subscriptions) in the terminal",
  )
  .arguments("<folder:string>")
  .option("--json", "Output the raw asset graph as JSON")
  .action(show as any);

export default command;
