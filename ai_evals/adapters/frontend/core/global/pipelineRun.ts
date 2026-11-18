// Runs the pipeline a case built, on the case's own workspace, once the chat is over: its nodes
// are deployed, the nodes nothing upstream feeds are run, and the server's asset triggers run
// the rest. What a case then asserts is the data the pipeline produced, not how its source reads.
import type { GlobalDraft } from "../../../../core/validators";
import type {
  PipelineRunQuery,
  PipelineRunResult,
  PipelineRunSpec,
} from "../../../../core/types";

export type Api = (method: string, path: string, body?: unknown) => Promise<any>;

const DEFAULT_TIMEOUT_SECONDS = 300;
/** Past `LISTEN_NEW_EVENTS_INTERVAL_SEC` (10 s by default). */
const PRODUCER_CACHE_EVICTION_MS = 12_000;
/** Deployments and the cascade are over once no job of the folder has been queued or running
 * for this long. */
const QUIET_MS = 5_000;

interface GraphTrigger {
  trigger_kind: string;
  asset_kind?: string;
  asset_path?: string;
  runnable_path: string;
}

interface GraphEdge {
  runnable_path: string;
  asset_kind: string;
  asset_path: string;
  access_type: "r" | "w" | "rw" | null;
}

export async function runPipeline(
  api: Api,
  spec: PipelineRunSpec,
  drafts: GlobalDraft[],
  /** Points every DuckLake at an empty catalog. The model's test runs fill the lake it worked
   * on, and a materialize that rewrites a table unchanged triggers nothing downstream. */
  emptyLakes: () => Promise<void>,
): Promise<PipelineRunResult> {
  const prefix = `f/${spec.folder}/`;
  const nodes = drafts.filter(
    (d) =>
      d.type === "script" &&
      d.path.startsWith(prefix) &&
      typeof d.value === "string" &&
      isPipelineNode(d.value),
  );
  const result: PipelineRunResult = { nodes: [], queries: [] };
  if (nodes.length === 0) {
    result.error = `no pipeline node was drafted under ${prefix}`;
    return result;
  }
  const deadline = Date.now() + (spec.timeoutSeconds ?? DEFAULT_TIMEOUT_SECONDS) * 1000;

  for (const node of nodes) {
    try {
      await api("POST", "scripts/create", {
        path: node.path,
        summary: node.summary ?? "",
        description: "",
        content: node.value,
        language: node.language ?? "duckdb",
      });
    } catch (error) {
      result.nodes.push({ path: node.path, deployError: String(error) });
    }
  }
  const deployed = nodes
    .map((n) => n.path)
    .filter((p) => !result.nodes.some((r) => r.path === p));
  // A failure past this point is the pipeline's outcome, not the harness's: it is recorded so
  // the case keeps its drafts and judge.
  try {
    await runFromRoots(api, spec, deployed, deadline, emptyLakes, result);
  } catch (error) {
    result.error = String(error);
  }
  for (const query of spec.queries ?? []) {
    result.queries.push(await runQuery(api, query));
  }
  return result;
}

async function runFromRoots(
  api: Api,
  spec: PipelineRunSpec,
  deployed: string[],
  deadline: number,
  emptyLakes: () => Promise<void>,
  result: PipelineRunResult,
): Promise<void> {
  const prefix = `f/${spec.folder}/`;
  // A node's asset triggers are recorded when its deployment job completes, not at create.
  await waitForQuietQueue(api, spec.folder, deadline);
  // Dispatch reads a per-process cache of which scripts produce what, which the model's test
  // runs have filled; a deploy evicts it only at each process's next notify-event poll.
  await new Promise((resolve) => setTimeout(resolve, PRODUCER_CACHE_EVICTION_MS));

  const graph = await api("GET", `assets/graph?folder=${encodeURIComponent(spec.folder)}`);
  const roots = rootNodes(
    deployed,
    (graph.triggers ?? []) as GraphTrigger[],
    (graph.edges ?? []) as GraphEdge[],
  );
  result.roots = roots;
  await emptyLakes();
  const startedAt = new Date(Date.now() - 1_000).toISOString();
  for (const path of roots) {
    await api("POST", `jobs/run/p/${path}`, {});
  }
  await waitForQuietQueue(api, spec.folder, deadline).catch((error) => {
    result.error = String(error);
  });

  const jobs: Array<{
    script_path?: string;
    success?: boolean;
    type?: string;
    created_by?: string;
    id: string;
  }> = await api(
    "GET",
    // Deployed runs of this run only: the model's own test runs are previews.
    `jobs/list?script_path_start=${encodeURIComponent(prefix)}&job_kinds=script` +
      `&created_after=${encodeURIComponent(startedAt)}&per_page=100`,
  );
  for (const path of deployed) {
    // Newest first: the last run of the node is what its table holds.
    const run = jobs.find((j) => j.script_path === path && j.type === "CompletedJob");
    const entry: PipelineRunResult["nodes"][number] = { path, ran: !!run };
    if (run) {
      entry.success = run.success === true;
      entry.triggeredBy = run.created_by;
      if (!entry.success) {
        const failed = await api("GET", `jobs_u/completed/get_result/${run.id}`).catch(
          (error) => ({ error: String(error) }),
        );
        entry.error = summarize(failed);
      }
    }
    result.nodes.push(entry);
  }
}

function isPipelineNode(content: string): boolean {
  return /^\s*(--|\/\/|#)\s*pipeline\b/m.test(content);
}

/**
 * Nodes no other node of the pipeline feeds: none of the assets they read or trigger on is
 * written by another node. Reads count too, so a node that reads an upstream table but
 * triggers on the wrong asset is never run directly, and fails as never having run.
 */
export function rootNodes(
  paths: string[],
  triggers: GraphTrigger[],
  edges: GraphEdge[],
): string[] {
  const inPipeline = new Set(paths);
  const writers = new Map<string, Set<string>>();
  for (const e of edges) {
    if (inPipeline.has(e.runnable_path) && (e.access_type === "w" || e.access_type === "rw")) {
      const key = `${e.asset_kind}:${e.asset_path}`;
      writers.set(key, (writers.get(key) ?? new Set()).add(e.runnable_path));
    }
  }
  const fedByAnother = (path: string, kind: string | undefined, assetPath: string | undefined) =>
    [...(writers.get(`${kind}:${assetPath}`) ?? [])].some((writer) => writer !== path);
  return paths.filter(
    (path) =>
      !triggers.some(
        (t) =>
          t.trigger_kind === "asset" &&
          t.runnable_path === path &&
          fedByAnother(path, t.asset_kind, t.asset_path),
      ) &&
      !edges.some(
        (e) =>
          e.runnable_path === path &&
          (e.access_type === "r" || e.access_type === "rw") &&
          fedByAnother(path, e.asset_kind, e.asset_path),
      ),
  );
}

async function waitForQuietQueue(api: Api, folder: string, deadline: number): Promise<void> {
  let quietSince: number | undefined;
  await waitUntil(deadline, `the jobs of f/${folder} to finish`, async () => {
    const active: unknown[] = await api(
      "GET",
      `jobs/queue/list?script_path_start=${encodeURIComponent(`f/${folder}/`)}&per_page=10`,
    );
    if (active.length > 0) {
      quietSince = undefined;
      return false;
    }
    quietSince ??= Date.now();
    return Date.now() - quietSince >= QUIET_MS;
  });
}

async function runQuery(api: Api, query: PipelineRunQuery) {
  const lake = query.ducklake ?? "main";
  const content = `ATTACH 'ducklake://${lake}' AS dl;\n${query.sql}`;
  const result = await api("POST", "jobs/run_wait_result/preview", {
    content,
    language: "duckdb",
    args: {},
  }).catch((error) => ({ error: String(error) }));
  if (result && typeof result === "object" && !Array.isArray(result) && "error" in result) {
    return { name: query.name, error: summarize(result) };
  }
  return { name: query.name, rows: Array.isArray(result) ? result : [result] };
}

function summarize(value: unknown): string {
  const error =
    value && typeof value === "object" && "error" in value
      ? (value as { error: unknown }).error
      : value;
  const text =
    error && typeof error === "object" && "message" in error
      ? String((error as { message: unknown }).message)
      : typeof error === "string"
        ? error
        : JSON.stringify(error);
  return text.length > 600 ? `${text.slice(0, 600)}…` : text;
}

async function waitUntil(
  deadline: number,
  what: string,
  done: () => Promise<boolean>,
): Promise<void> {
  while (!(await done())) {
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await new Promise((resolve) => setTimeout(resolve, 1_000));
  }
}
