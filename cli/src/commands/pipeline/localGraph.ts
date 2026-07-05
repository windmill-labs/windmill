// Build a pipeline asset-graph from LOCAL, not-yet-deployed script files.
//
// The deployed asset-graph endpoint (`GET /assets/graph`) only reflects scripts
// already pushed to the workspace. To get the same graph from the working tree
// (so `pipeline show --local` / `pipeline run --local` / `pipeline dev` work
// before deploy), we infer each script's assets + pipeline annotations with the
// SAME wasm parser the frontend uses (`windmill-parser-wasm-asset`,
// `frontend/src/lib/infer.ts:inferAssets`). That wasm returns the full serialized
// Rust `ParseAssetsOutput`: body-inferred `assets` (with r/w/rw access) AND the
// parsed pipeline annotations (`in_pipeline`, `triggers`, …) in one call — so we
// reproduce the endpoint payload without a backend round-trip or a TS re-port of
// the annotation parser.

import * as fs from "node:fs";
import * as path from "node:path";
import process from "node:process";
import { loadParser } from "../../utils/metadata.ts";
import * as log from "../../core/log.ts";
import { exts, removeExtensionToPath } from "../script/script.ts";
import { inferContentTypeFromFilePath } from "../../utils/script_common.ts";
import { getWmillYamlPath } from "../../core/conf.ts";

// Resolve the workspace root (the directory containing wmill.yaml) for reading
// local files, falling back to the current directory.
export function workspaceRoot(): string {
  const yaml = getWmillYamlPath();
  return yaml ? path.dirname(yaml) : process.cwd();
}

// Graph payload shape — structurally identical to the backend `/assets/graph`
// response and a superset of `boundedCascade.ts`'s `BCGraph`. Defined here (the
// graph module) and re-imported by pipeline.ts so there is one canonical type.
export type GraphRunnable = {
  path: string;
  usage_kind: "script" | "flow" | "job";
  in_pipeline?: boolean;
  // Annotation metadata the shared canvas renders as badges / lineage. Mirrors
  // the deployed graph's runnable node (frontend `AssetGraphRunnableNode`) so the
  // local-dev graph is the same surface as the deployed one for annotated scripts.
  partition_kind?: string;
  freshness?: string;
  tag?: string;
  retry?: { count: number; delay?: string };
  data_tests?: unknown[];
  column_lineage?: unknown[];
  // `// materialize <asset>` target + strategy — the script's declared output,
  // so the UI anchors column lineage / the materialize badge to it (the producer
  // write-edge is emitted separately). `scd2` also identifies the producer of a
  // `<dim>_current` view for the editor's schema-contract fallback.
  materialize_target?: { kind: string; path: string };
  materialize_strategy?: "replace" | "append" | "merge" | "scd2";
  // `on_schema_change=ignore` — producer's opt-out from downstream
  // schema-contract warnings; only present when set (default `warn` is absent),
  // mirroring the deployed graph node.
  materialize_on_schema_change?: string;
};
export type GraphEdge = {
  runnable_kind: string;
  runnable_path: string;
  asset_kind: string;
  asset_path: string;
  access_type?: "r" | "w" | "rw";
};
export type GraphTrigger =
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
export type AssetGraph = {
  runnables: GraphRunnable[];
  assets: { kind: string; path: string }[];
  edges: GraphEdge[];
  triggers: GraphTrigger[];
};

export type LocalScript = {
  path: string;
  content: string;
  language: string;
  // `// tag <worker-tag>` — routes the preview to that worker tag, matching the
  // deployed pipeline. Undefined → default worker.
  tag?: string;
};

// Raw shape of the wasm `parse_assets_<lang>` JSON output (a serialized Rust
// `ParseAssetsOutput`). `triggers` is internally tagged on `kind`
// (`#[serde(tag = "kind", rename_all = "lowercase")]`): asset triggers carry
// `asset_kind` + `path`; native triggers are bare `{ kind }`.
type ParseAssetsRaw = {
  assets?: {
    kind: string;
    path: string;
    access_type?: "r" | "w" | "rw";
  }[];
  in_pipeline?: boolean;
  triggers?: (
    | { kind: "asset"; asset_kind: string; path: string; debounce?: string }
    | { kind: string }
  )[];
  // `// materialize [manual] <asset> …` — managed-materialize target (emitted by
  // the wasm asset parser). The body (a bare trailing SELECT) writes nothing
  // inferable, so the producer's output edge comes from here, not from `assets`.
  materialize?: {
    target_kind: string;
    target_path: string;
    manual?: boolean;
    append?: boolean;
    unique_key?: string;
    scd2?: boolean;
    // "ignore" when set; the default `warn` is skipped in serialization.
    on_schema_change?: string;
  };
  // `// tag <worker-tag>` — the worker tag the deployed pipeline routes to.
  tag?: string;
  // Other annotation metadata the deployed graph carries onto its runnable nodes.
  partition?: { kind: string };
  freshness?: { duration: string };
  retry?: { count: number; delay?: string };
  data_tests?: unknown[];
  column_lineage?: unknown[];
};

// Mirror `inferAssets` (frontend/src/lib/infer.ts): only ts / py / sql have a
// wasm asset parser. SQL dialects all route to `parse_assets_sql` — the comment-
// header annotation scan inside it is dialect-independent, so `in_pipeline` /
// `// on` come through even when body asset detection is duckdb-shaped. Languages
// with no entry (go, bash, …) fall back to a minimal annotation scan below.
function wasmFnForLanguage(language: string): "parse_assets_ts" | "parse_assets_py" | "parse_assets_sql" | undefined {
  switch (language) {
    case "bun":
    case "deno":
    case "nativets":
      return "parse_assets_ts";
    case "bunnative":
      return "parse_assets_ts";
    case "python3":
      return "parse_assets_py";
    case "duckdb":
    case "postgresql":
    case "mysql":
    case "bigquery":
    case "snowflake":
    case "mssql":
    case "oracledb":
      return "parse_assets_sql";
    default:
      return undefined;
  }
}

const ASSET_WASM_PKG = "windmill-parser-wasm-asset";

// Comment prefix per language, for the no-wasm annotation fallback.
function commentPrefix(language: string): string {
  if (
    language === "python3" ||
    language === "bash" ||
    language === "ansible" ||
    language === "ruby" ||
    language === "rlang" ||
    language === "nu" ||
    language === "powershell"
  )
    return "#";
  if (
    language === "postgresql" ||
    language === "mysql" ||
    language === "bigquery" ||
    language === "snowflake" ||
    language === "mssql" ||
    language === "oracledb" ||
    language === "duckdb"
  )
    return "--";
  return "//";
}

// Minimal annotation scan for languages without a wasm asset parser (go, bash).
// Deliberately a SUBSET of the canonical parsers (backend
// `parse_pipeline_annotations`, frontend `parsePipelineAnnotations.ts`): it
// recovers `// pipeline` membership, `// on <asset-uri | native-kind>`, and
// `// tag <worker-tag>` (routing affects execution) so the node, its trigger
// edges, and its worker tag still appear. Body-inferred I/O for these languages
// is out of scope (see plan); annotate explicitly to get edges.
const NATIVE_KINDS = new Set([
  "schedule",
  "webhook",
  "email",
  "kafka",
  "mqtt",
  "nats",
  "postgres",
  "sqs",
  "gcp",
  "data_upload",
]);
function fallbackParse(content: string, language: string): ParseAssetsRaw {
  const raw = commentPrefix(language);
  const p = raw.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const out: ParseAssetsRaw = { in_pipeline: false, triggers: [] };
  // Annotations live in the LEADING comment header (like the canonical parsers).
  // Stop at the first non-comment line so a body comment — e.g. `// on s3://…`
  // inside commented-out code or a heredoc — can't inject a phantom trigger.
  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    if (trimmed === "") continue;
    if (!trimmed.startsWith(raw)) break;
    if (line.match(new RegExp(`^\\s*${p}\\s*pipeline\\s*$`))) {
      out.in_pipeline = true;
      continue;
    }
    // `// tag <worker-tag>` — routes the preview; cheap to scan and it affects
    // execution, so recover it here too (the wasm path already carries it). A
    // worker tag is a single token — match `\S+` (not `.+`) so trailing prose
    // (`// tag gpu for heavy jobs`) can't smuggle a bogus multi-word tag.
    const tag = line.match(new RegExp(`^\\s*${p}\\s*tag\\s+(\\S+)\\s*$`));
    if (tag) {
      out.tag = tag[1];
      continue;
    }
    const on = line.match(new RegExp(`^\\s*${p}\\s*on\\s+(.+?)\\s*$`));
    if (!on) continue;
    // The asset/native kind is the FIRST token; trailing `key=value` options
    // (e.g. `debounce=5s`) are not part of the path.
    const rest = on[1].trim();
    const firstTok = rest.split(/\s+/)[0];
    const uri = firstTok.match(/^([a-z0-9_]+):\/\/(.+)$/i);
    if (uri) {
      const prefix = uri[1].toLowerCase();
      const kind = prefix === "s3" ? "s3object" : prefix;
      out.triggers!.push({ kind: "asset", asset_kind: kind, path: uri[2] });
    } else if (NATIVE_KINDS.has(firstTok) && rest === firstTok) {
      // A native marker (`// on data_upload`) must stand alone: the canonical
      // parser rejects a marker line with trailing content (`// on data_upload
      // f/foo`, `# on kafka topic`), so match that here to keep local/deployed
      // parity for the fallback surface.
      out.triggers!.push({ kind: firstTok });
    }
  }
  return out;
}

function recoverHeaderNativeTriggers(content: string, language: string): string[] {
  const raw = commentPrefix(language);
  const p = raw.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const out: string[] = [];
  const seen = new Set<string>();
  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    if (trimmed === "") continue;
    if (!trimmed.startsWith(raw)) break;
    const marker = line.match(new RegExp(`^\\s*${p}\\s*on\\s+(\\S+)\\s*$`));
    if (!marker) continue;
    const kind = marker[1];
    if (!NATIVE_KINDS.has(kind) || seen.has(kind)) continue;
    seen.add(kind);
    out.push(kind);
  }
  return out;
}

function normalizeRetry(retry: ParseAssetsRaw["retry"]): ParseAssetsRaw["retry"] {
  if (!retry?.delay) return retry;
  return { ...retry, delay: retry.delay.replace(/^delay=/, "") };
}

// Comment prefix for `volume:` annotations. Deliberately NOT `commentPrefix`
// above (which returns `--` for SQL): volume annotations are only recognized for
// the languages the backend/frontend recognize them for — mirrors
// `asset_inference.rs:comment_prefix` and `infer.ts:getCommentPrefix` (SQL → none).
function volumeCommentPrefix(language: string): string | undefined {
  switch (language) {
    case "python3":
    case "bash":
    case "powershell":
    case "ansible":
    case "ruby":
    case "rlang":
      return "#";
    case "deno":
    case "bun":
    case "bunnative":
    case "nativets":
    case "go":
      return "//";
    default:
      return undefined;
  }
}

// `<prefix> volume: <path>` lines in the LEADING comment block, each an `rw`
// volume asset. Scanning stops at the first non-comment line (blank lines
// skipped). Exact mirror of frontend `parseVolumeAnnotations` (infer.ts) /
// backend `parse_volume_annotations` (asset_inference.rs) — the wasm body parser
// does NOT emit these, so the local graph must supplement them or a `volume:`
// producer shows disconnected from its `// on volume://…` consumers.
function parseVolumeAnnotations(
  content: string,
  prefix: string,
): { kind: string; path: string; access_type: "rw" }[] {
  const out: { kind: string; path: string; access_type: "rw" }[] = [];
  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    if (trimmed === "") continue;
    if (!trimmed.startsWith(prefix)) break;
    const after = trimmed.slice(prefix.length).trim();
    const m = after.match(/^volume:\s*(\S+)/);
    if (m) out.push({ kind: "volume", path: m[1], access_type: "rw" });
  }
  return out;
}

// Infer a single script's assets + pipeline annotations. Returns the raw
// `ParseAssetsOutput` (incl. `materialize`, which the wasm asset parser emits as
// of windmill-parser-wasm-asset 1.740.0 — kept in lockstep with the frontend's
// pin so the local graph mirrors the deployed one), plus `volume:` annotation
// assets the wasm doesn't surface (merged in below, like the frontend/backend).
// wasm parse errors degrade to the annotation fallback so a syntactically-broken
// script still shows as a pipeline node if it's annotated.
export async function inferScriptAssets(
  content: string,
  language: string,
): Promise<ParseAssetsRaw> {
  const out = await inferScriptAssetsBody(content, language);
  const vp = volumeCommentPrefix(language);
  if (vp) {
    const vols = parseVolumeAnnotations(content, vp);
    if (vols.length > 0) out.assets = [...(out.assets ?? []), ...vols];
  }
  return out;
}

// A missing/broken wasm asset parser degrades every script to the
// annotation-only fallback: the graph keeps `// on` edges but silently loses
// all inferred writes (materialize targets, COPY TO, writeS3File), which reads
// as "my pipeline has no lineage". Warn once so a packaging or install problem
// is visible instead of masquerading as a parsing limitation.
let warnedAssetParserUnavailable = false;

async function inferScriptAssetsBody(
  content: string,
  language: string,
): Promise<ParseAssetsRaw> {
  const fn = wasmFnForLanguage(language);
  if (!fn) return fallbackParse(content, language);
  let raw: string;
  try {
    const mod = await loadParser(ASSET_WASM_PKG);
    raw = mod[fn](content) as string;
  } catch (e) {
    if (!warnedAssetParserUnavailable) {
      warnedAssetParserUnavailable = true;
      log.warnStderr(
        `warning: ${ASSET_WASM_PKG} failed to load (${
          e instanceof Error ? e.message : e
        }) — falling back to annotation-only parsing; inferred read/write edges (materialize targets, COPY TO, writeS3File) will be missing from the local graph`,
      );
    }
    return fallbackParse(content, language);
  }
  if (raw.startsWith("err:")) return fallbackParse(content, language);
  try {
    return JSON.parse(raw) as ParseAssetsRaw;
  } catch {
    return fallbackParse(content, language);
  }
}

// Recursively collect every local script file under `folderDir`, returning each
// one's windmill path (relative to `root`, extension stripped), content, and
// language. Skips lock files and metadata YAMLs.
export async function collectScripts(
  folderDir: string,
  root: string,
  defaultTs: "bun" | "deno" | undefined,
): Promise<LocalScript[]> {
  const out: LocalScript[] = [];
  function walk(dir: string) {
    let entries: fs.Dirent[];
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of entries) {
      if (e.name.startsWith(".") || e.name === "node_modules") continue;
      const abs = path.join(dir, e.name);
      if (e.isDirectory()) {
        walk(abs);
        continue;
      }
      if (!e.isFile()) continue;
      const ext = exts.find((x) => e.name.endsWith(x));
      if (!ext) continue;
      const relFromRoot = path.relative(root, abs).replaceAll("\\", "/");
      // A bare `.sql` (no dialect) makes inferContentTypeFromFilePath throw —
      // skip the file rather than letting one unclassifiable file abort the
      // whole graph build (which would also wedge `pipeline dev` at startup).
      let language: string;
      try {
        language = inferContentTypeFromFilePath(abs, defaultTs);
      } catch {
        continue;
      }
      out.push({
        path: removeExtensionToPath(relFromRoot),
        content: fs.readFileSync(abs, "utf-8"),
        language,
      });
    }
  }
  walk(folderDir);
  out.sort((a, b) => a.path.localeCompare(b.path));
  return out;
}

// Build the full pipeline asset-graph from local files in `f/<folder>/`.
// Only `// pipeline` scripts become graph nodes (pipeline membership), mirroring
// the deployed graph endpoint. Returns the graph plus the in-pipeline scripts'
// content (the dev watcher pushes the latter; the runner re-reads it for preview).
export async function buildLocalPipelineGraph(args: {
  root: string;
  folder: string;
  defaultTs?: "bun" | "deno";
}): Promise<{ graph: AssetGraph; scripts: LocalScript[] }> {
  const folderClean = args.folder.replace(/^f\//, "").replace(/\/$/, "");
  const folderDir = path.join(args.root, "f", folderClean);
  const all = await collectScripts(folderDir, args.root, args.defaultTs);

  const runnables: GraphRunnable[] = [];
  const edges: GraphEdge[] = [];
  const triggers: GraphTrigger[] = [];
  const assetSet = new Map<string, { kind: string; path: string }>();
  const pipelineScripts: LocalScript[] = [];

  for (const s of all) {
    const out = await inferScriptAssets(s.content, s.language);
    if (!out.in_pipeline) continue; // not a pipeline member
    const retry = normalizeRetry(out.retry);
    const nativeTriggers = recoverHeaderNativeTriggers(s.content, s.language);
    // Carry the parsed `// tag` so previews route to the same worker the
    // deployed pipeline would (both `pipeline run --local` and `/pipeline_dev`).
    pipelineScripts.push(out.tag ? { ...s, tag: out.tag } : s);
    const mat = out.materialize;
    // Managed-materialize write strategy, derived like the deployed graph —
    // precedence mirrors the runtime: scd2 (`history`) > append > merge
    // (key=<col>) > replace. Manual mode has no managed strategy.
    const materialize_strategy =
      mat && !mat.manual
        ? mat.scd2
          ? "scd2"
          : mat.append
            ? "append"
            : mat.unique_key
              ? "merge"
              : "replace"
        : undefined;
    runnables.push({
      path: s.path,
      usage_kind: "script",
      in_pipeline: true,
      ...(out.partition ? { partition_kind: out.partition.kind } : {}),
      ...(out.freshness ? { freshness: out.freshness.duration } : {}),
      ...(out.tag ? { tag: out.tag } : {}),
      ...(retry ? { retry } : {}),
      ...(out.data_tests && out.data_tests.length > 0 ? { data_tests: out.data_tests } : {}),
      ...(out.column_lineage && out.column_lineage.length > 0
        ? { column_lineage: out.column_lineage }
        : {}),
      ...(mat ? { materialize_target: { kind: mat.target_kind, path: mat.target_path } } : {}),
      ...(materialize_strategy ? { materialize_strategy } : {}),
      ...(mat?.on_schema_change === "ignore"
        ? { materialize_on_schema_change: "ignore" }
        : {}),
    });

    for (const a of out.assets ?? []) {
      assetSet.set(`${a.kind}:${a.path}`, { kind: a.kind, path: a.path });
      edges.push({
        runnable_kind: "script",
        runnable_path: s.path,
        asset_kind: a.kind,
        asset_path: a.path,
        access_type: a.access_type,
      });
    }
    // `// materialize <asset>` declares a write output via annotation, not the
    // SQL body, so body-inference misses it. Translate the parsed materialize
    // target into the producer's write edge here (mirrors frontend
    // resolveGraph.ts) so the materialized asset connects to its `// on`
    // consumers; dedup against any body write.
    if (mat) {
      assetSet.set(`${mat.target_kind}:${mat.target_path}`, {
        kind: mat.target_kind,
        path: mat.target_path,
      });
      const hasWrite = edges.some(
        (e) =>
          e.runnable_path === s.path &&
          e.asset_kind === mat.target_kind &&
          e.asset_path === mat.target_path &&
          (e.access_type === "w" || e.access_type === "rw"),
      );
      if (!hasWrite) {
        edges.push({
          runnable_kind: "script",
          runnable_path: s.path,
          asset_kind: mat.target_kind,
          asset_path: mat.target_path,
          access_type: "w",
        });
      }
    }
    const existingNativeTriggers = new Set<string>();
    for (const t of out.triggers ?? []) {
      if (t.kind === "asset") {
        const at = t as { kind: "asset"; asset_kind: string; path: string };
        assetSet.set(`${at.asset_kind}:${at.path}`, {
          kind: at.asset_kind,
          path: at.path,
        });
        triggers.push({
          trigger_kind: "asset",
          asset_kind: at.asset_kind,
          asset_path: at.path,
          runnable_kind: "script",
          runnable_path: s.path,
        });
      } else {
        existingNativeTriggers.add(t.kind);
        triggers.push({
          trigger_kind: t.kind,
          runnable_kind: "script",
          runnable_path: s.path,
        });
      }
    }
    for (const kind of nativeTriggers) {
      if (existingNativeTriggers.has(kind)) continue;
      triggers.push({
        trigger_kind: kind,
        runnable_kind: "script",
        runnable_path: s.path,
      });
    }
  }

  return {
    graph: {
      runnables,
      assets: [...assetSet.values()],
      edges,
      triggers,
    },
    scripts: pipelineScripts,
  };
}
