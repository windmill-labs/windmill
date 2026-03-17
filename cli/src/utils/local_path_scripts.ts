import { execSync } from "node:child_process";
import { readFile, stat } from "node:fs/promises";
import type { SyncCodebase } from "./codebase.ts";
import { parseMetadataFileIfExists } from "./metadata.ts";
import { inferContentTypeFromFilePath } from "./script_common.ts";
import { findCodebase } from "../commands/sync/sync.ts";
import type { LocalScriptInfo } from "../../windmill-utils-internal/src/inline-scripts/replacer.ts";
import type { RawScript } from "../../../gen/types.gen.ts";

export class UnsupportedLocalPathScriptPreviewError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "UnsupportedLocalPathScriptPreviewError";
  }
}

async function readOptionalLock(scriptPath: string): Promise<string | undefined> {
  try {
    return await readFile(scriptPath + ".script.lock", "utf-8");
  } catch {
    return undefined;
  }
}

function normalizeOptionalLock(lock: string | undefined): string | undefined {
  return typeof lock === "string" && lock.trim() === "" ? undefined : lock;
}

async function bundleSingleFileCodebaseScript(
  filePath: string,
  codebase: SyncCodebase
): Promise<string> {
  if (codebase.customBundler) {
    return execSync(codebase.customBundler + " " + filePath, {
      maxBuffer: 1024 * 1024 * 50,
    }).toString();
  }

  const esbuild = await import("esbuild");
  const out = await esbuild.build({
    entryPoints: [filePath],
    // Inline rawscripts are executed through the standard module wrapper,
    // so the bundle must expose `main` as an ESM export.
    format: "esm",
    bundle: true,
    write: false,
    external: codebase.external,
    inject: codebase.inject,
    define: codebase.define,
    loader: codebase.loader ?? { ".node": "file" },
    outdir: "/",
    platform: "node",
    packages: "bundle",
    target: "esnext",
    banner: codebase.banner,
  });

  if (out.outputFiles.length === 0) {
    throw new Error(`No output files found for ${filePath}`);
  }
  if (out.outputFiles.length > 1) {
    throw new UnsupportedLocalPathScriptPreviewError(
      `Local PathScript ${filePath} requires a multi-file bundle, which flow preview/dev cannot inline yet`
    );
  }
  if (Array.isArray(codebase.assets) && codebase.assets.length > 0) {
    throw new UnsupportedLocalPathScriptPreviewError(
      `Local PathScript ${filePath} requires codebase assets, which flow preview/dev cannot inline yet`
    );
  }

  return out.outputFiles[0].text;
}

export function createPreviewLocalScriptReader(opts: {
  exts: string[];
  defaultTs?: "bun" | "deno";
  codebases: SyncCodebase[];
}): (scriptPath: string) => Promise<LocalScriptInfo | undefined> {
  return async (scriptPath) => {
    const localScript = await resolvePreviewLocalScriptState(scriptPath, opts);
    if (!localScript) {
      return undefined;
    }

    const content = localScript.codebase
      ? await bundleSingleFileCodebaseScript(localScript.filePath, localScript.codebase)
      : localScript.content;

    return {
      content,
      language: localScript.language,
      lock: localScript.lock,
      tag: localScript.tag,
    };
  };
}

export type PreviewLocalScriptState = {
  filePath: string;
  content: string;
  language: RawScript["language"];
  lock?: string;
  tag?: string;
  codebase?: SyncCodebase;
  codebaseDigest?: string;
};

export async function resolvePreviewLocalScriptState(
  scriptPath: string,
  opts: {
    exts: string[];
    defaultTs?: "bun" | "deno";
    codebases: SyncCodebase[];
  }
): Promise<PreviewLocalScriptState | undefined> {
  for (const ext of opts.exts) {
    const filePath = scriptPath + ext;
    let fileStat;
    try {
      fileStat = await stat(filePath);
    } catch {
      continue;
    }
    if (!fileStat.isFile()) continue;

    const language = inferContentTypeFromFilePath(filePath, opts.defaultTs);
    const metadata = await parseMetadataFileIfExists(scriptPath);
    const rawLock = metadata?.payload?.lock ?? (await readOptionalLock(scriptPath));
    const codebase =
      language === "bun" ? findCodebase(filePath, opts.codebases) : undefined;

    return {
      filePath,
      content: await readFile(filePath, "utf-8"),
      language,
      lock: normalizeOptionalLock(rawLock),
      tag: metadata?.payload?.tag,
      codebase,
      codebaseDigest: codebase
        ? await codebase.getDigest(
            Array.isArray(codebase.assets) && codebase.assets.length > 0
          )
        : undefined,
    };
  }

  return undefined;
}
