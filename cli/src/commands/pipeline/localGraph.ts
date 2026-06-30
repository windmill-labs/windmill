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
  // `// materialize <asset>` target — the script's declared output, mirrored
  // from the deployed graph so the UI anchors column lineage / the materialize
  // badge to it (the producer write-edge is emitted separately).
  materialize_target?: { kind: string; path: string };
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

export type LocalScript = { path: string; content: string; language: string };

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
  materialize?: { target_kind: string; target_path: string };
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
// `parse_pipeline_annotations`, frontend `parsePipelineAnnotations.ts`): it only
// recovers `// pipeline` membership and `// on <asset-uri | native-kind>` so the
// node and its trigger edges still appear in the graph. Body-inferred I/O for
// these languages is out of scope (see plan); annotate explicitly to get edges.
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
  const p = commentPrefix(language).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const out: ParseAssetsRaw = { in_pipeline: false, triggers: [] };
  for (const line of content.split("\n")) {
    const bare = line.match(new RegExp(`^\\s*${p}\\s*pipeline\\s*$`));
    if (bare) {
      out.in_pipeline = true;
      continue;
    }
    const on = line.match(new RegExp(`^\\s*${p}\\s*on\\s+(.+?)\\s*$`));
    if (!on) continue;
    const spec = on[1].trim();
    const uri = spec.match(/^([a-z0-9_]+):\/\/(.+)$/i);
    if (uri) {
      const prefix = uri[1].toLowerCase();
      const kind = prefix === "s3" ? "s3object" : prefix;
      out.triggers!.push({ kind: "asset", asset_kind: kind, path: uri[2] });
    } else {
      const word = spec.split(/\s+/)[0];
      if (NATIVE_KINDS.has(word)) out.triggers!.push({ kind: word });
    }
  }
  return out;
}

// Infer a single script's assets + pipeline annotations. Returns the raw
// `ParseAssetsOutput` (incl. `materialize`, which the wasm asset parser emits as
// of windmill-parser-wasm-asset 1.740.0 — kept in lockstep with the frontend's
// pin so the local graph mirrors the deployed one). wasm parse errors degrade to
// the annotation fallback so a syntactically-broken script still shows as a
// pipeline node if it's annotated.
export async function inferScriptAssets(
  content: string,
  language: string,
): Promise<ParseAssetsRaw> {
  const fn = wasmFnForLanguage(language);
  if (!fn) return fallbackParse(content, language);
  let raw: string;
  try {
    const mod = await loadParser(ASSET_WASM_PKG);
    raw = mod[fn](content) as string;
  } catch {
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
    pipelineScripts.push(s);
    const mat = out.materialize;
    runnables.push({
      path: s.path,
      usage_kind: "script",
      in_pipeline: true,
      ...(mat ? { materialize_target: { kind: mat.target_kind, path: mat.target_path } } : {}),
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
        triggers.push({
          trigger_kind: t.kind,
          runnable_kind: "script",
          runnable_path: s.path,
        });
      }
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
