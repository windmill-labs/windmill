import { createHash } from "node:crypto";
import { stat } from "node:fs/promises";
import { sep as SEP } from "node:path";

import { Command } from "@cliffy/command";
import * as log from "../../core/log.ts";
import { mergeConfigWithConfigFile } from "../../core/conf.ts";
import { GlobalOptions } from "../../types.ts";
import { readLocalFlow } from "../flow/flow.ts";
import {
  findContentFile,
  readModulesFromDisk,
} from "../script/script.ts";
import { replaceLock } from "../../utils/metadata.ts";
import { listSyncCodebases, SyncCodebase } from "../../utils/codebase.ts";
import { findCodebase } from "../sync/sync.ts";
import {
  extractFolderPath,
  getModuleFolderSuffix,
  isFlowPath,
  isMissingDbtDescriptor,
  isModuleEntryPoint,
  scriptPathToRemotePath,
} from "../../utils/resource_folders.ts";
import { inferContentTypeFromFilePath } from "../../utils/script_common.ts";
import { readTextFile } from "../../utils/utils.ts";
import { yamlParseFile } from "../../utils/yaml.ts";

// The server computes the same digests for the `digest` and `root_digest` claims of job
// OIDC tokens (backend/windmill-common/src/item_digest.rs): both must hash the same bytes.

/** RFC 8785 canonical JSON, with null and undefined object members left out. */
export function canonicalJson(value: unknown): string {
  if (value === null || value === undefined) return "null";
  if (Array.isArray(value)) {
    return "[" + value.map(canonicalJson).join(",") + "]";
  }
  if (typeof value === "object") {
    // Sorting strings in JS compares UTF-16 code units, as RFC 8785 requires.
    const members = Object.entries(value as Record<string, unknown>)
      .filter(([, v]) => v !== null && v !== undefined)
      .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
    return (
      "{" +
      members
        .map(([k, v]) => JSON.stringify(k) + ":" + canonicalJson(v))
        .join(",") +
      "}"
    );
  }
  return JSON.stringify(value);
}

export function digestOf(value: unknown): string {
  return createHash("sha256").update(canonicalJson(value), "utf8").digest("hex");
}

/** The inline scripts of a flow value, by step id. */
function inlineSteps(value: unknown, steps: Record<string, string>) {
  if (Array.isArray(value)) {
    value.forEach((v) => inlineSteps(v, steps));
  } else if (value && typeof value === "object") {
    const o = value as Record<string, any>;
    if (typeof o.id === "string" && o.value?.type === "rawscript") {
      steps[o.id] = digestOf({ content: o.value.content, lock: o.value.lock });
    }
    Object.values(o).forEach((v) => inlineSteps(v, steps));
  }
}

export interface ItemDigest {
  path: string;
  kind: "script" | "flow";
  digest: string;
  /** For a flow, the digest of each step's inline script, by step id. */
  steps?: Record<string, string>;
}

export async function flowDigest(folder: string): Promise<ItemDigest> {
  const { flow, missingFiles } = await readLocalFlow(folder);
  if (missingFiles.length > 0) {
    throw new Error(`Missing inline script file(s): ${missingFiles.join(", ")}`);
  }
  const steps: Record<string, string> = {};
  inlineSteps(flow.value, steps);
  const path = folder
    .replaceAll(SEP, "/")
    .replace(/\/$/, "")
    .replace(/(\.flow|__flow)$/, "");
  return { path, kind: "flow", digest: digestOf(flow.value), steps };
}

export async function scriptDigest(
  contentFile: string,
  defaultTs?: "bun" | "deno",
  codebases: SyncCodebase[] = []
): Promise<ItemDigest> {
  const moduleEntry = isModuleEntryPoint(contentFile);
  const remotePath = scriptPathToRemotePath(contentFile);
  const base = remotePath.replaceAll("/", SEP);
  const language = inferContentTypeFromFilePath(contentFile, defaultTs);
  const metaBase = moduleEntry ? base + getModuleFolderSuffix() + SEP + "script" : base + ".script";
  let meta: any = undefined;
  for (const ext of [".yaml", ".json"]) {
    if (await stat(metaBase + ext).catch(() => undefined)) {
      meta = await yamlParseFile(metaBase + ext);
      break;
    }
  }
  replaceLock(meta);
  let content = "";
  try {
    content = await readTextFile(contentFile);
  } catch (e) {
    if (!isMissingDbtDescriptor(contentFile, e)) throw e;
  }
  // What `wmill sync push` sends: the codebase is derived from wmill.yaml, never read
  // from the metadata file. Only bundling tells whether push adds `.tar`, so a pulled
  // value that is this digest plus `.tar` is taken as is.
  const codebase = language == "bun" ? findCodebase(contentFile, codebases) : undefined;
  let codebaseDigest = codebase ? await codebase.getDigest() : undefined;
  if (codebaseDigest && meta?.codebase === codebaseDigest + ".tar") {
    codebaseDigest = meta.codebase;
  }
  const isDbt = language === "dbt";
  const modules = await readModulesFromDisk(
    base + getModuleFolderSuffix(language),
    defaultTs,
    moduleEntry,
    isDbt
  );
  const digest = digestOf({
    content,
    lock: meta?.lock,
    codebase: codebaseDigest,
    modules: modules
      ? Object.fromEntries(
          Object.entries(modules).map(([p, m]) => [p, { content: m.content, lock: m.lock }])
        )
      : undefined,
  });
  return { path: remotePath, kind: "script", digest };
}

/** The item at a local path of a sync checkout: a flow folder, a script file, or a script's
 * metadata file. Paths resolve against the current directory, which must be the
 * checkout's root, as for `wmill sync push`. */
export async function itemDigest(
  localPath: string,
  defaultTs?: "bun" | "deno",
  codebases: SyncCodebase[] = []
): Promise<ItemDigest> {
  const folderPath = localPath.endsWith(SEP) ? localPath : localPath + SEP;
  if (isFlowPath(folderPath)) {
    return flowDigest(extractFolderPath(folderPath, "flow")!);
  }
  if (/\.script\.(yaml|json)$/.test(localPath) || /script\.(yaml|json)$/.test(localPath)) {
    return scriptDigest(await findContentFile(localPath), defaultTs, codebases);
  }
  return scriptDigest(localPath, defaultTs, codebases);
}

async function digest(
  opts: GlobalOptions & { json?: boolean },
  ...paths: string[]
) {
  if (opts.json) log.setSilent(true);
  const merged = await mergeConfigWithConfigFile(opts);
  const codebases = listSyncCodebases(merged);
  const results: ItemDigest[] = [];
  for (const p of paths) {
    results.push(await itemDigest(p, merged.defaultTs, codebases));
  }
  if (opts.json) {
    console.log(JSON.stringify(results, null, 2));
    return;
  }
  for (const r of results) {
    console.log(`${r.digest}  ${r.path}`);
    for (const [id, d] of Object.entries(r.steps ?? {})) {
      console.log(`${d}  ${r.path}/${id}`);
    }
  }
}

const command = new Command()
  .description(
    "Compute the content digest of local scripts and flows, as the `digest` and `root_digest` claims of job OIDC tokens carry it. Run from the root of a sync checkout, pulled after the deployment's dependency jobs completed: they write the lockfiles the digest covers. A flow also lists the digest of each inline step, as `<flow path>/<step id>`."
  )
  .arguments("<paths...:string>")
  .option("--json", "Output the digests as JSON")
  .action(digest as any);

export default command;
