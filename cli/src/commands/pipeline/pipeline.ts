import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";

import { OpenAPI } from "../../../gen/index.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";
import {
  type BCGraph,
  boundedSet,
  buildLineageDag,
  descendants,
  resolveToken,
  scriptNodeId,
  scriptPathOf,
  scriptsOf,
  topoOrder,
  validStarts,
} from "./boundedCascade.ts";

// Mirrors the asset-graph endpoint payload (backend/windmill-api-assets).
// TODO: the checked-in generated client (cli/gen, last regenerated 2025-04)
// predates these routes, so we raw-fetch and hand-roll the types. Once
// `cli/gen` is regenerated (run `cli/gen_wm_client.sh`, which is currently
// >700 openapi.yaml commits stale and would churn the whole client), replace
// `apiGet` + these types with the generated `wmill.getAssetsGraph(...)`
// (operationId getAssetsGraph) and `wmill.listPipelineFolders(...)`
// (operationId listPipelineFolders).
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

// Append to a multimap value, creating the bucket on first use. Avoids the
// O(n^2) spread-rebuild pattern (`map.set(k, [...(map.get(k) ?? []), v])`).
function pushTo<K, V>(map: Map<K, V[]>, key: K, val: V): void {
  (map.get(key) ?? map.set(key, []).get(key)!).push(val);
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
      pushTo(writesByScript, e.runnable_path, uri);
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
      pushTo(subsByAsset, uri, t.runnable_path);
      pushTo(subsByScript, t.runnable_path, uri);
    } else {
      const nt = t as Exclude<GraphTrigger, { trigger_kind: "asset" }>;
      pushTo(nativeByScript, t.runnable_path, {
        kind: nt.trigger_kind,
        path: nt.path,
        missing: nt.missing,
      });
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
  // graph endpoint's trigger enum (schedule/email/kafka/mqtt/nats/postgres/
  // sqs/gcp) can't surface them, so they only exist as `// on <kind>`
  // annotations in the script body. Roots are where sources matter, so fetch
  // just those bodies and lift the marker kinds the canvas would show.
  //
  // DRIFT RISK: this regex + MARKER_KINDS is a divergent, partial copy of the
  // canonical annotation parser. The proper fix is to have the graph endpoint
  // emit these UI-only markers as trigger rows (a backend change), after which
  // this whole Promise.all body-fetch can be deleted and read straight from
  // the response. Until then, keep this list in sync with the canonical parser.
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

// Poll a launched job to a terminal state. Modest fixed cadence; capped so a
// wedged job can't hang the CLI forever.
async function waitJob(workspace: string, id: string): Promise<boolean> {
  const MAX_RETRIES = 6000; // ~10min at 100ms
  for (let i = 0; i < MAX_RETRIES; i++) {
    try {
      const r = await wmill.getCompletedJobResultMaybe({
        workspace,
        id,
        getStarted: false,
      });
      if (r.completed) return r.success !== false;
    } catch {
      // transient — retry
    }
    await new Promise((res) => setTimeout(res, 100));
  }
  throw new Error(`Timed out waiting for job ${id}`);
}

// Bounded-cascade run: start at a schedule / manual root, fan downstream, but
// stop at the `--to` end node(s). Scripts run in topological order; each is
// launched with `_wmill_skip_asset_dispatch` so the CLI owns the whole closure
// (the backend dispatcher never double-fires the deployed part). With no
// `--to`, runs the full downstream of `--from` (parity with the canvas cascade).
async function run(
  opts: GlobalOptions & {
    from?: string;
    to?: string[];
    dryRun?: boolean;
    json?: boolean;
  },
  folder: string,
) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const f = folder.replace(/^f\//, "").replace(/\/$/, "");
  const graph = await apiGet<BCGraph>(
    `/w/${workspace.workspaceId}/assets/graph?folder=${encodeURIComponent(f)}&asset_kinds=${ASSET_KINDS}`,
  );

  // Resolve the start: explicit --from (must be a valid start) or the folder's
  // sole valid start.
  const starts = validStarts(graph);
  let start: string;
  if (opts.from) {
    const resolved = resolveToken(graph, opts.from);
    if (!resolved) {
      // Distinguish "no match" from "ambiguous short name" (resolveToken
      // returns undefined for both) so the hint is actionable.
      const matches = graph.runnables.filter(
        (r) => r.usage_kind === "script" && (r.path.split("/").pop() ?? r.path) === opts.from,
      );
      if (matches.length > 1) {
        throw new Error(
          `--from '${opts.from}' matches multiple scripts (${matches.map((r) => r.path).sort().join(", ")}) — pass the full path.`,
        );
      }
      throw new Error(`--from '${opts.from}' matched no script in f/${f}.`);
    }
    if (!starts.has(resolved)) {
      throw new Error(
        `--from '${opts.from}' is not a valid bounded-run start. Starts must be schedule-triggered or manual roots; event-triggered scripts (kafka/webhook/…) fan out per-event and can't be bounded.`,
      );
    }
    start = resolved;
  } else if (starts.size === 1) {
    start = [...starts][0];
  } else if (starts.size === 0) {
    throw new Error(
      `No schedule or manual root in f/${f} to start a bounded run from.`,
    );
  } else {
    throw new Error(
      `f/${f} has ${starts.size} possible starts — pass --from <script>. Candidates: ${scriptsOf(starts).sort().join(", ")}`,
    );
  }

  // Resolve --to end node(s): split on comma, resolve each, warn on misses.
  const toTokens = (opts.to ?? []).flatMap((t) => t.split(",")).map((t) => t.trim()).filter(
    Boolean,
  );
  const ends: string[] = [];
  for (const tok of toTokens) {
    const id = resolveToken(graph, tok);
    if (!id) {
      log.warn(`--to '${tok}' matched no node in f/${f} — ignored.`);
      continue;
    }
    ends.push(id);
  }
  // A bound was requested but nothing resolved — refuse rather than silently
  // running the whole pipeline.
  if (toTokens.length > 0 && ends.length === 0) {
    throw new Error(`None of the --to end node(s) matched a node in f/${f}.`);
  }

  const dag = buildLineageDag(graph);
  let selectedScripts: Set<string>;
  if (ends.length === 0) {
    // No bound → full downstream of start (the unbounded cascade).
    const all = new Set(descendants(dag, start));
    all.add(start);
    selectedScripts = new Set(scriptsOf(all));
  } else {
    const res = boundedSet(dag, start, ends);
    for (const d of res.droppedEnds) {
      log.warn(`end '${scriptPathOf(d) || d}' is not downstream of the start — ignored.`);
    }
    selectedScripts = new Set(scriptsOf(res.nodes));
  }

  const { order, cyclic } = topoOrder(graph, selectedScripts);
  if (cyclic.length > 0) {
    log.warn(`Skipping ${cyclic.length} script(s) on a dependency cycle: ${cyclic.sort().join(", ")}`);
  }

  if (opts.json) {
    console.log(
      JSON.stringify({
        start: scriptPathOf(start),
        ends: ends.map((e) => (e.startsWith("script:") ? scriptPathOf(e) : e)),
        order,
        cyclic,
      }),
    );
  }

  if (order.length === 0) {
    if (!opts.json) log.info("Nothing to run.");
    return;
  }

  if (opts.dryRun) {
    if (!opts.json) {
      log.info(
        colors.bold(`Bounded run plan — ${order.length} script${order.length === 1 ? "" : "s"}`) +
          colors.dim(` (from ${shortName(scriptPathOf(start))})`),
      );
      order.forEach((p, i) => log.info(`  ${i + 1}. ${p}`));
    }
    return;
  }

  // Execute in topological order, stopping on the first failure.
  for (const path of order) {
    if (!opts.json) log.info(colors.gray(`▶ running ${path}…`));
    const id = await wmill.runScriptByPath({
      workspace: workspace.workspaceId,
      path,
      requestBody: { _wmill_skip_asset_dispatch: true },
    });
    const ok = await waitJob(workspace.workspaceId, id);
    if (!ok) {
      throw new Error(`Bounded run failed at ${path} (job ${id}).`);
    }
    if (!opts.json) log.info(colors.green(`  ✓ ${path}`));
  }
  if (!opts.json) {
    log.info(colors.green.bold(`Bounded run complete — ${order.length} script(s) succeeded.`));
  }
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
  .action(show as any)
  .command(
    "run",
    "run a bounded cascade: from a schedule/manual root, fan downstream up to the --to end node(s)",
  )
  .arguments("<folder:string>")
  .option(
    "--from <script:string>",
    "Start script (short name or path). Defaults to the folder's sole schedule/manual root.",
  )
  .option(
    "--to <node:string>",
    "End node(s) to stop at — script names/paths or asset URIs (e.g. datatable://main/staged). Repeatable or comma-separated. Omit to run the full downstream.",
    { collect: true },
  )
  .option("--dry-run", "Print the topological run plan without executing.")
  .option("--json", "Output the plan as JSON (for piping to jq).")
  .action(run as any);

export default command;
