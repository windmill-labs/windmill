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
  nonAutorunTriggerScripts,
  reachableCutting,
  resolveToken,
  scriptNodeId,
  scriptPathOf,
  scriptsOf,
  topoOrder,
  validStarts,
} from "./boundedCascade.ts";
import {
  type AssetGraph,
  type GraphTrigger,
  type LocalScript,
  buildLocalPipelineGraph,
  workspaceRoot,
} from "./localGraph.ts";
import {
  mergeConfigWithConfigFile,
  type SyncOptions,
} from "../../core/conf.ts";
import { listSyncCodebases } from "../../utils/codebase.ts";
import { inferSchema } from "../../utils/metadata.ts";
import { readFile } from "node:fs/promises";
import pipelineDev from "./dev.ts";
import { generatePipelineDocs } from "./docs.ts";
import {
  type UploadBinding,
  devUploadKey,
  parseArgBinding,
  parseS3Uri,
  parseUploadBinding,
  s3Arg,
  s3ObjectParams,
} from "./pipelineUpload.ts";

// The graph payload types (AssetGraph / GraphRunnable / GraphEdge /
// GraphTrigger) are defined in ./localGraph.ts — the canonical pipeline graph
// module — and shared by the local (wasm-built) and deployed (apiGet) paths.
//
// Mirrors the asset-graph endpoint payload (backend/windmill-api-assets).
// TODO: the checked-in generated client (cli/gen, last regenerated 2025-04)
// predates these routes, so we raw-fetch and hand-roll the types. Once
// `cli/gen` is regenerated (run `cli/gen_wm_client.sh`, which is currently
// >700 openapi.yaml commits stale and would churn the whole client), replace
// `apiGet` + these types with the generated `wmill.getAssetsGraph(...)`
// (operationId getAssetsGraph) and `wmill.listPipelineFolders(...)`
// (operationId listPipelineFolders).

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
  opts: GlobalOptions & { json?: boolean; local?: boolean; defaultTs?: "bun" | "deno" },
  folder: string,
) {
  if (opts.json) log.setSilent(true);
  const f = folder.replace(/^f\//, "").replace(/\/$/, "");

  // Acquire the graph: `--local` builds it from working-tree files (wasm
  // inference, no deploy); otherwise fetch the deployed asset graph. The
  // deployed path also enriches roots with UI-only source markers (the local
  // path gets those from the wasm directly, so no enrichment).
  let graph: AssetGraph;
  let enrich: ((nativeByScript: Map<string, NativeMarker[]>, roots: string[]) => Promise<void>) | undefined;
  if (opts.local) {
    const merged = await mergeConfigWithConfigFile(opts);
    graph = (await buildLocalPipelineGraph({ root: workspaceRoot(), folder: f, defaultTs: merged.defaultTs })).graph;
  } else {
    const workspace = await resolveWorkspace(opts);
    await requireLogin(opts);
    graph = await apiGet<AssetGraph>(
      `/w/${workspace.workspaceId}/assets/graph?folder=${encodeURIComponent(f)}&asset_kinds=${ASSET_KINDS}`,
    );
    enrich = (nativeByScript, roots) => enrichRootMarkers(workspace.workspaceId, graph, nativeByScript, roots);
  }

  if (opts.json) {
    console.log(JSON.stringify(graph));
    return;
  }
  if (graph.runnables.length === 0) {
    log.info(
      opts.local
        ? `No \`// pipeline\` scripts found in f/${f} locally. Mark scripts with a \`// pipeline\` comment.`
        : `No pipeline scripts in f/${f}. Mark scripts with a \`// pipeline\` comment and push them.`,
    );
    return;
  }

  await renderGraph(graph, f, enrich);
}

type NativeMarker = { kind: string; path?: string; missing?: boolean };

// Deployed-only: UI-first markers (data_upload, webhook, email) have no trigger
// row — the graph endpoint's trigger enum can't surface them, so they only exist
// as `// on <kind>` annotations in the script body. Fetch just the root bodies
// and lift the marker kinds the canvas would show. (Local mode skips this: the
// wasm asset parser already returns these as triggers.)
//
// DRIFT RISK: this regex + MARKER_KINDS is a divergent, partial copy of the
// canonical annotation parser. The proper fix is to have the graph endpoint emit
// these UI-only markers as trigger rows (a backend change), after which this can
// be deleted. Until then, keep this list in sync with the canonical parser.
const MARKER_KINDS = new Set(["data_upload", "webhook", "email"]);

// Recover marker-only native triggers (`// on data_upload`/`webhook`/`email`) from
// a deployed script body — they have no trigger row for the graph endpoint to
// emit. Scans the LEADING comment header only (stops at the first non-comment
// line, blank lines allowed) like the canonical/fallback annotation parsers, so a
// body comment can't inject a phantom trigger.
export function recoverHeaderMarkers(content: string): string[] {
  const found: string[] = [];
  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    if (trimmed === "") continue;
    if (!trimmed.startsWith("//") && !trimmed.startsWith("--") && !trimmed.startsWith("#")) break;
    const m = line.match(/^\s*(?:\/\/|--|#)\s*on\s+(\w+)\s*$/);
    if (m && MARKER_KINDS.has(m[1]) && !found.includes(m[1])) found.push(m[1]);
  }
  return found;
}

async function enrichRootMarkers(
  workspaceId: string,
  graph: AssetGraph,
  nativeByScript: Map<string, NativeMarker[]>,
  roots: string[],
): Promise<void> {
  await Promise.all(
    roots.map(async (p) => {
      const r = graph.runnables.find((x) => x.path === p);
      if (r?.usage_kind !== "script") return;
      try {
        const script = await wmill.getScriptByPath({ workspace: workspaceId, path: p });
        const existing = nativeByScript.get(p) ?? [];
        for (const kind of recoverHeaderMarkers(script.content ?? "")) {
          if (!existing.some((t) => t.kind === kind)) existing.push({ kind });
        }
        if (existing.length > 0) nativeByScript.set(p, existing);
      } catch {
        // body fetch is best-effort enrichment only
      }
    }),
  );
}

// The deployed `/assets/graph` omits `data_upload`/`webhook`/`email` trigger rows
// (marker-only annotations with no trigger table), so on the deployed `run` path
// `validStarts`/`nonAutorunTriggerScripts` can't see them and would auto-run such
// an entrypoint with empty args. Recover them from the script bodies — the same
// body-scan the deployed `show` path uses (`enrichRootMarkers`) — and inject
// trigger rows so the run selection cuts them like the `--local` graph does.
async function enrichDeployedNonAutorunTriggers(
  workspaceId: string,
  graph: BCGraph,
): Promise<void> {
  const scripts = (graph.runnables ?? []).filter((r) => r.usage_kind === "script");
  await Promise.all(
    scripts.map(async (r) => {
      const known = new Set(
        (graph.triggers ?? [])
          .filter((t) => t.runnable_path === r.path && MARKER_KINDS.has(t.trigger_kind))
          .map((t) => t.trigger_kind),
      );
      let script;
      try {
        script = await wmill.getScriptByPath({ workspace: workspaceId, path: r.path });
      } catch (e: any) {
        // Fail CLOSED: if we can't read a script's body we can't rule out a
        // marker-only input trigger, so aborting is safer than auto-running it
        // with empty args. (The `show` enrichment stays best-effort — it only
        // affects the rendered tree, not what runs.)
        throw new Error(
          `Could not verify triggers for ${r.path} (${e?.body ?? e?.message ?? e}) — ` +
            `aborting so an input-only entrypoint isn't run with empty args; retry once resolved.`,
        );
      }
      for (const kind of recoverHeaderMarkers(script.content ?? "")) {
        if (known.has(kind)) continue;
        known.add(kind);
        graph.triggers.push({
          trigger_kind: kind,
          runnable_kind: "script",
          runnable_path: r.path,
        });
      }
    }),
  );
}

// Index a pipeline graph and render its DAG as an ASCII tree. Shared by the
// deployed (`show`) and local (`show --local`) paths; `enrich` lifts deployed
// UI-only root markers when provided.
async function renderGraph(
  graph: AssetGraph,
  f: string,
  enrich?: (nativeByScript: Map<string, NativeMarker[]>, roots: string[]) => Promise<void>,
) {
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
  const nativeByScript = new Map<string, NativeMarker[]>();
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

  // `// macros` libraries: badge the node with the macros it defines and list
  // its consumers as ƒ edges. Deployed graphs only — the local wasm parse does
  // not emit `macros`/`macro_edges`, so local mode renders them as plain nodes.
  const macrosByLib = new Map(
    graph.runnables
      .filter((r) => (r.macros?.length ?? 0) > 0)
      .map((r) => [r.path, r.macros!]),
  );
  const macroConsumersByLib = new Map<
    string,
    { consumer: string; names: string[] }[]
  >();
  for (const me of graph.macro_edges ?? []) {
    pushTo(macroConsumersByLib, me.lib_path, {
      consumer: me.consumer_path,
      names: me.macro_names,
    });
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
    const libMacros = macrosByLib.get(script);
    const macroBadge = libMacros
      ? " " +
        colors.magenta(`[ƒ macros: ${libMacros.map((m) => m.name).join(", ")}]`)
      : "";
    lines.push(`${prefix}${colors.bold(shortName(script))}${macroBadge}${triggerBadges(script)}${alsoOn}`);
    const childPrefix = prefix.replace(/├─ $/, "│  ").replace(/└─ $/, "   ");
    const macroConsumers = [...(macroConsumersByLib.get(script) ?? [])].sort(
      (a, b) => a.consumer.localeCompare(b.consumer),
    );
    macroConsumers.forEach(({ consumer, names }, i) => {
      const branch = i === macroConsumers.length - 1 ? "└─ƒ " : "├─ƒ ";
      lines.push(
        `${childPrefix}${branch}${shortName(consumer)} ${colors.dim(`(uses ${names.join(", ")})`)}`,
      );
    });
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

  // Deployed-only: lift UI-only source markers (data_upload/webhook/email) onto
  // the roots. No-op in local mode (the wasm already emitted them as triggers).
  if (enrich) await enrich(nativeByScript, roots);

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
async function waitJob(
  workspace: string,
  id: string,
): Promise<{ ok: boolean; result?: unknown }> {
  const MAX_RETRIES = 6000; // ~10min at 100ms
  for (let i = 0; i < MAX_RETRIES; i++) {
    try {
      const r = await wmill.getCompletedJobResultMaybe({
        workspace,
        id,
        getStarted: false,
      });
      // A completed job without an explicit `success: true` is a failure
      // (mirrors the frontend `waitJobTerminal`): the cascade only advances on
      // a confirmed success.
      if (r.completed) return { ok: r.success === true, result: r.result };
    } catch {
      // transient — retry
    }
    await new Promise((res) => setTimeout(res, 100));
  }
  throw new Error(`Timed out waiting for job ${id}`);
}

// Render a failed job's `{error: {name, message}}` result for the terminal, so
// the user sees WHY the cascade stopped without opening the UI. Result shapes
// vary (structured data-test payloads, plain strings), so fall back to raw
// JSON when the canonical error shape is absent.
function formatJobFailure(result: unknown): string | undefined {
  if (result == null) return undefined;
  const err = (result as any)?.error;
  if (err && typeof err === "object") {
    const name = typeof err.name === "string" ? err.name : undefined;
    const message = typeof err.message === "string" ? err.message : undefined;
    if (message) return name ? `${name}: ${message}` : message;
  }
  if (typeof result === "string") return result;
  try {
    return JSON.stringify(result, null, 2);
  } catch {
    return undefined;
  }
}

// Bounded-cascade run: start at a schedule / manual root, fan downstream, but
// stop at the `--to` end node(s). Scripts run in topological order; each is
// launched with `_wmill_skip_asset_dispatch` so the CLI owns the whole closure
// (the backend dispatcher never double-fires the deployed part). With no
// `--to`, runs the full read-aware downstream of `--from` (every descendant in
// the lineage DAG, pure readers included — broader than the canvas cascade,
// which dispatches subscribers only).
// Resolve one `--upload` binding to the run-arg it injects: pick the target
// S3Object parameter (explicit `:param`, else the script's sole S3Object arg)
// and turn the source into an object key — an `s3://` source is used as-is, a
// local path is uploaded to the workspace store under a deterministic dev key.
async function resolveUploadArgs(
  bindings: UploadBinding[],
  scriptPath: string,
  workspaceId: string,
  local: boolean | undefined,
  localScripts: Map<string, LocalScript> | undefined,
): Promise<Record<string, any>> {
  // Fetch the schema at most once, and only when a binding omits `:param`.
  let schema: any | undefined;
  const loadSchema = async (): Promise<any> => {
    if (schema) return schema;
    if (local) {
      const ls = localScripts?.get(scriptPath);
      if (!ls) {
        throw new Error(`No local file for ${scriptPath} to infer its S3Object parameter.`);
      }
      schema = (await inferSchema(ls.language as any, ls.content, {}, scriptPath)).schema;
    } else {
      schema = (await wmill.getScriptByPath({ workspace: workspaceId, path: scriptPath })).schema;
    }
    return schema;
  };

  const args: Record<string, any> = {};
  for (const binding of bindings) {
    let param = binding.param;
    if (!param) {
      const params = s3ObjectParams(await loadSchema());
      if (params.length === 1) param = params[0];
      else if (params.length === 0) {
        throw new Error(`${scriptPath} declares no S3Object parameter to bind --upload to.`);
      } else {
        throw new Error(
          `${scriptPath} has multiple S3Object parameters (${params.join(", ")}) — pick one with --upload ${binding.scriptTok}:<param>=${binding.source}`,
        );
      }
    }
    if (param in args) {
      throw new Error(`--upload binds ${scriptPath}:${param} more than once.`);
    }
    let obj: { s3: string; storage?: string };
    if (binding.source.startsWith("s3://")) {
      // Canonical `s3://<storage>/<key>`; keeps named storage (see parseS3Uri).
      obj = parseS3Uri(binding.source);
    } else {
      const buf = await readFile(binding.source);
      // Key scoped by script + param so distinct sources sharing a basename
      // (across scripts or params) don't clobber each other in the store.
      const key = devUploadKey(scriptPath, param, binding.source);
      await wmill.fileUpload({
        workspace: workspaceId,
        fileKey: key,
        requestBody: new Blob([buf]) as any,
      });
      log.info(colors.gray(`  ↑ ${binding.source} → s3://${key}`));
      obj = { s3: key };
    }
    Object.assign(args, s3Arg(param, obj));
  }
  return args;
}

// Default partition value for a `// partitioned <kind>` script, from the
// current UTC instant — mirrors the backend defaults (partition_ee.rs):
// hourly `%Y-%m-%dT%H`, weekly `%G-W%V` (ISO week-year), monthly `%Y-%m`,
// daily `%Y-%m-%d`. `dynamic` has no time default and returns undefined.
export function defaultPartitionValue(kind: string, now: Date = new Date()): string | undefined {
  const pad = (n: number) => String(n).padStart(2, "0");
  const date = `${now.getUTCFullYear()}-${pad(now.getUTCMonth() + 1)}-${pad(now.getUTCDate())}`;
  switch (kind) {
    case "hourly":
      return `${date}T${pad(now.getUTCHours())}`;
    case "weekly": {
      // ISO 8601 week (chrono's `%G-W%V`): the week containing this date's
      // nearest Thursday belongs to that Thursday's year.
      const d = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate()));
      const dow = d.getUTCDay() || 7; // Mon=1 … Sun=7
      d.setUTCDate(d.getUTCDate() + 4 - dow);
      const week = Math.ceil(((d.getTime() - Date.UTC(d.getUTCFullYear(), 0, 1)) / 86400000 + 1) / 7);
      return `${d.getUTCFullYear()}-W${pad(week)}`;
    }
    case "monthly":
      return date.slice(0, 7);
    case "daily":
      return date;
    default:
      return undefined;
  }
}

async function run(
  opts: GlobalOptions & SyncOptions & {
    from?: string;
    to?: string[];
    dryRun?: boolean;
    json?: boolean;
    local?: boolean;
    upload?: string[];
    arg?: string[];
    partition?: string;
    defaultTs?: "bun" | "deno";
  },
  folder: string,
) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const f = folder.replace(/^f\//, "").replace(/\/$/, "");

  // `--local` runs the working-tree scripts via preview (no deploy); the graph
  // is built from local files. Otherwise run deployed scripts by path off the
  // deployed asset graph. Both still execute against the remote backend, with
  // `_wmill_skip_asset_dispatch` so the CLI owns the whole closure.
  let graph: BCGraph;
  let localScripts: Map<string, LocalScript> | undefined;
  let tempScriptRefs: Record<string, string> | undefined;
  if (opts.local) {
    // Resolve config (defaultTs drives .ts → bun/deno language inference, so a
    // deno-default workspace previews under the right runtime).
    const merged = await mergeConfigWithConfigFile(opts);
    const built = await buildLocalPipelineGraph({
      root: workspaceRoot(),
      folder: f,
      defaultTs: merged.defaultTs,
    });
    graph = built.graph;
    localScripts = new Map(built.scripts.map((s) => [s.path, s]));
    // Resolve relative imports from local (not-yet-deployed) content so a script
    // that imports a sibling workspace lib previews against local edits. Same
    // trick as `wmill dev` / `wmill app dev`; degrades to undefined gracefully.
    // Skipped for `--dry-run`: it locks/uploads scripts as a side effect (it
    // regenerates `*.script.yaml` / `*.script.lock` / `wmill-lock.yaml`), and a
    // plan preview must stay READ-ONLY — the refs are only consumed when a
    // preview job actually runs below, which dry-run never reaches.
    if (!opts.dryRun) {
      try {
        const codebases = await listSyncCodebases(merged);
        const { buildPreviewTempScriptRefs } = await import(
          "../generate-metadata/generate-metadata.ts"
        );
        tempScriptRefs = await buildPreviewTempScriptRefs(
          workspace,
          merged,
          codebases,
          { kind: "all" },
        );
      } catch {
        // best-effort: relative imports fall back to deployed versions
      }
    }
  } else {
    graph = await apiGet<BCGraph>(
      `/w/${workspace.workspaceId}/assets/graph?folder=${encodeURIComponent(f)}&asset_kinds=${ASSET_KINDS}`,
    );
    // Recover marker-only `data_upload`/`webhook`/`email` triggers the graph
    // endpoint can't emit, so input-only entrypoints are cut here as they are
    // under `--local` (else they'd auto-run empty).
    await enrichDeployedNonAutorunTriggers(workspace.workspaceId, graph);
  }

  // Resolve a `--upload`/`--arg` script token to its graph node, with an
  // actionable error when the short name is ambiguous or matches nothing.
  const resolveScriptTokenOrThrow = (tok: string, flag: string): string => {
    const id = resolveToken(graph, tok);
    if (!id || !id.startsWith("script:")) {
      const matches = graph.runnables.filter(
        (r) => r.usage_kind === "script" && (r.path.split("/").pop() ?? r.path) === tok,
      );
      if (matches.length > 1) {
        throw new Error(
          `${flag} '${tok}' matches multiple scripts (${matches.map((r) => r.path).sort().join(", ")}) — use the full path.`,
        );
      }
      throw new Error(`${flag} '${tok}' matched no script in f/${f}.`);
    }
    return id;
  };

  // `--upload <script>[:<param>]=<file|s3://key>` binds an object to a
  // data_upload/webhook entry point so it becomes a runnable start (and its
  // downstream is no longer cut) — the arg is injected at execution. Repeatable
  // per script (accumulated) so a multi-S3Object entry can bind each param.
  const boundBindingsByPath = new Map<string, UploadBinding[]>();
  const boundNodeIds = new Set<string>();
  for (const spec of opts.upload ?? []) {
    const binding = parseUploadBinding(spec);
    const id = resolveScriptTokenOrThrow(binding.scriptTok, "--upload");
    const p = scriptPathOf(id);
    (boundBindingsByPath.get(p) ?? boundBindingsByPath.set(p, []).get(p)!).push(binding);
    boundNodeIds.add(id);
  }

  // `--arg <script>:<param>=<value>` overlays a plain run arg on a script in
  // the selection. Unlike `--upload` it does not make the script a runnable
  // start — it only supplies a value if the script runs.
  const plainArgsByPath = new Map<string, Record<string, unknown>>();
  for (const spec of opts.arg ?? []) {
    const b = parseArgBinding(spec);
    const p = scriptPathOf(resolveScriptTokenOrThrow(b.scriptTok, "--arg"));
    const merged = plainArgsByPath.get(p) ?? plainArgsByPath.set(p, {}).get(p)!;
    // Object.hasOwn, not `in`: a param named like a prototype member
    // (`toString`, `constructor`, …) must not trip the duplicate check.
    if (Object.hasOwn(merged, b.param)) {
      throw new Error(`--arg binds ${p}:${b.param} more than once.`);
    }
    merged[b.param] = b.value;
  }

  // `// macros` libraries are definition-only: their macros are injected into
  // consuming DuckDB scripts at run time, so "running" one is a no-op preview.
  // They are excluded from starts and from every selection below (they'd
  // otherwise read as manual roots, since nothing triggers them).
  const macroLibPaths = new Set(
    graph.runnables
      .filter((r) => r.usage_kind === "script" && (r.macros?.length ?? 0) > 0)
      .map((r) => r.path),
  );

  // Resolve the start: explicit --from (must be a valid start) or the folder's
  // sole valid start. `--upload`-bound scripts join the valid starts.
  const starts = new Set(
    [...validStarts(graph), ...boundNodeIds].filter(
      (id) => !macroLibPaths.has(scriptPathOf(id)),
    ),
  );
  // `runAll` = no `--from` on a multi-root pipeline (fan-in): run the whole
  // pipeline in topological order rather than forcing a single start, the way
  // `dbt run` (no `--select`) runs the entire project. `--to` still needs an
  // explicit `--from` since bounding is relative to one start.
  let runAll = false;
  let start: string | undefined;
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
    if (macroLibPaths.has(scriptPathOf(resolved))) {
      throw new Error(
        `--from '${opts.from}' is a \`// macros\` library — definition-only, injected into consuming scripts at run time, never a runnable step.`,
      );
    }
    if (!starts.has(resolved)) {
      throw new Error(
        `--from '${opts.from}' is not a valid bounded-run start. Starts must be schedule-triggered or manual roots; row-backed event triggers (kafka/mqtt/nats/postgres/sqs/gcp/email) fan out per-event and can't be bounded.`,
      );
    }
    start = resolved;
  } else if (starts.size === 1) {
    start = [...starts][0];
  } else if (starts.size === 0) {
    throw new Error(
      `No schedule or manual root in f/${f} to start a bounded run from.`,
    );
  } else if ((opts.to ?? []).length === 0) {
    // Multiple roots, no `--from`, no `--to` → run the whole pipeline.
    runAll = true;
  } else {
    throw new Error(
      `f/${f} has ${starts.size} possible starts — pass --from <script> when using --to. Candidates: ${scriptsOf(starts).sort().join(", ")}`,
    );
  }

  // Resolve --to end node(s): split on comma, resolve each. An unresolved or
  // ambiguous token is a hard error — silently ignoring it would run a
  // different subset than the user asked for.
  const toTokens = (opts.to ?? []).flatMap((t) => t.split(",")).map((t) => t.trim()).filter(
    Boolean,
  );
  const ends: string[] = [];
  const unresolved: string[] = [];
  for (const tok of toTokens) {
    const id = resolveToken(graph, tok);
    if (!id) unresolved.push(tok);
    else ends.push(id);
  }
  if (unresolved.length > 0) {
    const details = unresolved.map((tok) => {
      const matches = graph.runnables.filter(
        (r) => r.usage_kind === "script" && (r.path.split("/").pop() ?? r.path) === tok,
      );
      return matches.length > 1
        ? `'${tok}' (ambiguous: ${matches.map((r) => r.path).sort().join(", ")} — use the full path)`
        : `'${tok}' (no match in f/${f})`;
    });
    throw new Error(`--to could not be resolved: ${details.join("; ")}.`);
  }

  // Readable label for a node id: `script:<path>` → `<path>`; asset ids
  // (`<kind>:<path>`) are kept verbatim (slicing `script:` off them corrupts
  // the name).
  const idLabel = (id: string): string => (id.startsWith("script:") ? scriptPathOf(id) : id);

  const dag = buildLineageDag(graph);
  // CUT the selection at scripts whose trigger needs caller-supplied input or
  // per-event fanout (kafka/mqtt/nats/postgres/sqs/gcp/email/webhook/data_upload):
  // they can't run with empty args, so no run mode — whole-pipeline, single-root,
  // or bounded — may schedule them, nor a downstream consumer that only such a
  // handler feeds (it would get missing/stale inputs). Exclude `starts` from the
  // barriers: a valid start (a schedule/manual root, or an `--upload`-bound
  // handler) runs with its own input even if it ALSO carries a non-autorun
  // trigger — cutting it would drop a legitimately-scheduled root and its chain.
  const barriers = new Set(
    [...nonAutorunTriggerScripts(graph)].filter((id) => !starts.has(id)),
  );
  let selectedScripts: Set<string>;
  let reachableEnds: string[] = [];
  let droppedEnds: string[] = [];
  if (runAll) {
    // Whole pipeline = everything reachable from the valid starts (schedule/manual
    // roots), cutting at the barriers above. A node reachable via another,
    // non-barrier path still runs.
    selectedScripts = new Set(scriptsOf(reachableCutting(dag, starts, barriers)));
  } else if (ends.length === 0) {
    // No bound → full read-aware downstream of start (pure readers included),
    // still cut at barriers so an event/upload descendant isn't run empty.
    selectedScripts = new Set(scriptsOf(reachableCutting(dag, [start!], barriers)));
  } else {
    const res = boundedSet(dag, start!, ends);
    droppedEnds = res.droppedEnds;
    for (const d of droppedEnds) {
      log.warn(`end '${idLabel(d)}' is not downstream of the start — ignored.`);
    }
    // Intersect the bounded window with the barrier-cut closure from the start so
    // a barrier (or a node only reachable through one) inside the window is dropped.
    const reachable = reachableCutting(dag, [start!], barriers);
    selectedScripts = new Set(
      scriptsOf([...res.nodes].filter((n) => reachable.has(n))),
    );
    // An end that boundedSet reached but the barrier cut removed is NOT satisfied:
    // report it as dropped (not reachable) and warn, so `--json`/the plan don't
    // claim a bound was met when its only path ran through a skipped handler.
    for (const e of res.reachableEnds) {
      if (reachable.has(e)) {
        reachableEnds.push(e);
      } else {
        droppedEnds.push(e);
        log.warn(
          `end '${idLabel(e)}' is only reachable through a skipped input/event handler — not run (bind it with --upload to include it).`,
        );
      }
    }
  }

  // Selections can still pull a macro library in via graph reachability (e.g.
  // whole-pipeline mode collects every root's closure) — drop them before
  // ordering so they never run.
  for (const p of macroLibPaths) selectedScripts.delete(p);

  const { order, cyclic } = topoOrder(graph, selectedScripts);
  if (cyclic.length > 0) {
    log.warn(`Skipping ${cyclic.length} script(s) on a dependency cycle: ${cyclic.sort().join(", ")}`);
  }

  // `// partitioned` scripts need a resolved `partition` arg. Deployed runs get
  // it from backend run-start resolution, but previews (`--local`) never do —
  // so resolve it client-side there: `--partition` wins; otherwise time kinds
  // default to the current UTC period (mirroring the backend defaults; custom
  // `tz=`/`format=`/`start=` opts are a backend concern — pass --partition
  // explicitly to match them). `dynamic` has no default. For deployed runs the
  // arg is only injected when `--partition` is given (an explicit backfill).
  const partitionKindByPath = new Map(
    graph.runnables
      .filter((r) => r.usage_kind === "script" && r.partition_kind)
      .map((r) => [r.path, r.partition_kind!]),
  );
  const partitionValueFor = (nodePath: string): string | undefined => {
    const kind = partitionKindByPath.get(nodePath);
    if (!kind) return undefined;
    if (opts.partition) return opts.partition;
    if (!opts.local) return undefined; // deployed: backend resolves at run start
    return defaultPartitionValue(kind);
  };
  if (opts.local && !opts.partition) {
    const unresolvable = order.filter(
      (p) => partitionKindByPath.get(p) === "dynamic",
    );
    if (unresolvable.length > 0) {
      throw new Error(
        `dynamic \`// partitioned\` script(s) need an explicit partition for a local run: ${unresolvable.sort().join(", ")} — pass --partition <value>.`,
      );
    }
  }

  // A `--upload` whose script fell outside the selection is a no-op the user
  // should know about, rather than silently ignored.
  const orderSet = new Set(order);
  const boundInOrder = [...boundBindingsByPath.keys()].filter((p) => orderSet.has(p)).sort();
  for (const p of boundBindingsByPath.keys()) {
    if (!orderSet.has(p)) log.warn(`--upload for ${p} is unused — it isn't in the run selection.`);
  }
  for (const p of plainArgsByPath.keys()) {
    if (!orderSet.has(p)) log.warn(`--arg for ${p} is unused — it isn't in the run selection.`);
  }

  if (opts.json) {
    // Surface reachable/dropped ends so a machine-readable plan reflects the
    // same trimming the human-facing warning does — a resolved-but-unreachable
    // `--to` must not look like a clean plan that silently runs only the start.
    console.log(
      JSON.stringify({
        start: runAll ? null : scriptPathOf(start!),
        ends: ends.map(idLabel),
        reachableEnds: reachableEnds.map(idLabel),
        droppedEnds: droppedEnds.map(idLabel),
        order,
        cyclic,
        uploads: boundInOrder,
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
          colors.dim(runAll ? ` (whole pipeline)` : ` (from ${shortName(scriptPathOf(start!))})`),
      );
      order.forEach((p, i) => {
        const pv = partitionValueFor(p);
        const marks = [
          boundBindingsByPath.has(p) ? " (← --upload)" : "",
          plainArgsByPath.has(p) ? " (← --arg)" : "",
          pv
            ? ` (partition=${pv})`
            : partitionKindByPath.has(p)
              ? " (partition resolved by backend at run start)"
              : "",
        ].join("");
        log.info(`  ${i + 1}. ${p}${marks ? colors.dim(marks) : ""}`);
      });
    }
    return;
  }

  // Resolve `--upload` bindings to their injected args up front (uploading any
  // local sources) so a bad path / missing S3Object param fails before we start
  // launching jobs.
  const uploadArgs = new Map<string, Record<string, any>>();
  for (const nodePath of boundInOrder) {
    uploadArgs.set(
      nodePath,
      await resolveUploadArgs(
        boundBindingsByPath.get(nodePath)!,
        nodePath,
        workspace.workspaceId,
        opts.local,
        localScripts,
      ),
    );
  }

  // Execute in topological order, stopping on the first failure.
  for (const nodePath of order) {
    if (!opts.json) log.info(colors.gray(`▶ running ${nodePath}…`));
    // `--arg` spreads after `--upload` so an explicit value wins for the same
    // param; the internal dispatch guard spreads LAST so neither binding can
    // re-enable backend dispatch (the CLI owns the whole closure here).
    const partitionValue = partitionValueFor(nodePath);
    const nodeArgs = {
      ...(partitionValue !== undefined ? { partition: partitionValue } : {}),
      ...(uploadArgs.get(nodePath) ?? {}),
      ...(plainArgsByPath.get(nodePath) ?? {}),
      _wmill_skip_asset_dispatch: true,
    };
    let id: string;
    if (opts.local) {
      const ls = localScripts!.get(nodePath);
      if (!ls) {
        throw new Error(
          `No local file for ${nodePath} — is it a \`// pipeline\` script in f/${f}?`,
        );
      }
      id = await wmill.runScriptPreview({
        workspace: workspace.workspaceId,
        requestBody: {
          content: ls.content,
          language: ls.language as any,
          path: nodePath,
          args: nodeArgs,
          temp_script_refs: tempScriptRefs,
          // `// tag` routes to the same worker the deployed pipeline would.
          ...(ls.tag ? { tag: ls.tag } : {}),
        } as any,
      });
    } else {
      id = await wmill.runScriptByPath({
        workspace: workspace.workspaceId,
        path: nodePath,
        requestBody: nodeArgs,
      });
    }
    const { ok, result } = await waitJob(workspace.workspaceId, id);
    if (!ok) {
      const detail = formatJobFailure(result);
      const runUrl = `${workspace.remote}run/${id}?workspace=${workspace.workspaceId}`;
      throw new Error(
        `Bounded run failed at ${nodePath} (job ${id}).` +
          (detail ? `\n${detail}` : "") +
          `\nFull logs: ${runUrl}`,
      );
    }
    if (!opts.json) log.info(colors.green(`  ✓ ${nodePath}`));
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
  .option(
    "--local",
    "Build the graph from local working-tree files (// pipeline scripts) instead of the deployed workspace — no deploy needed.",
  )
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
  .option(
    "--local",
    "Run the local working-tree scripts via preview (no deploy) instead of the deployed versions; the graph is built from local files.",
  )
  .option(
    "--upload <binding:string>",
    "Bind an object to a data_upload/webhook entry point so it runs in the cascade, as SCRIPT[:PARAM]=SOURCE (SOURCE is a local file or an s3://key). Local files are uploaded to the workspace store; the S3Object param is inferred when the script has exactly one. Repeatable.",
    { collect: true },
  )
  .option(
    "--arg <binding:string>",
    "Pass a plain run arg to a script in the cascade, as SCRIPT:PARAM=VALUE (VALUE is parsed as JSON when possible, else taken as a string — e.g. daily_report:partition=2026-07-02). Repeatable.",
    { collect: true },
  )
  .option(
    "--partition <value:string>",
    "Partition value for `// partitioned` scripts in the run (e.g. 2026-06-30) — use it to backfill a past slice. With --local, time kinds (daily/hourly/weekly/monthly) default to the current UTC period when omitted; `dynamic` always needs it. Deployed runs without it defer to backend run-start resolution.",
  )
  .action(run as any)
  .command(
    "docs",
    "generate PIPELINE.md (+ AGENTS.md pointer) describing a folder's pipeline graph and datatable schemas, for an editor / agentic loop",
  )
  .arguments("<folder:string>")
  .option(
    "--local",
    "Build the graph from local working-tree files instead of the deployed workspace.",
  )
  .action(generatePipelineDocs as any)
  .command("dev", pipelineDev);

export default command;
