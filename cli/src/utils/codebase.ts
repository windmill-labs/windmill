import { Codebase, SyncOptions } from "../core/conf.ts";
import * as log from "../core/log.ts";
import { digestDir, generateHash, generateHashFromBuffer } from "./utils.ts";
import { readFile, stat } from "node:fs/promises";
import path from "node:path";

async function digestPath(p: string): Promise<string> {
  let s;
  try {
    s = await stat(p);
  } catch {
    throw new Error(`Codebase extra_digest_paths entry not found: ${p}`);
  }
  return s.isDirectory()
    ? await digestDir(p, "")
    : await generateHashFromBuffer(await readFile(p));
}

/** Bundle inputs that no codebase digest covers, so edits to them would not trigger a re-push. */
export function uncoveredBundleInputs(
  codebase: Codebase,
  inputs: string[]
): string[] {
  const roots = [codebase.relative_path, ...(codebase.extra_digest_paths ?? [])]
    .map((r) => path.resolve(r));
  return inputs.filter((i) => {
    // non-file namespaces ("<define:x>", "ns:path") and installed packages
    if (/^<|^[a-z-]{2,}:/i.test(i) || i.includes("node_modules")) return false;
    const abs = path.resolve(i);
    return !roots.some((r) => abs == r || abs.startsWith(r + path.sep));
  });
}

export type SyncCodebase = Codebase & {
  getDigest: (forceTar?: boolean) => Promise<string>;
};
export function listSyncCodebases(options: SyncOptions): SyncCodebase[] {
  const res: SyncCodebase[] = [];
  const nb_codebase = options?.codebases?.length ?? 0;
  if (nb_codebase > 0) {
    log.info(
      `Found ${nb_codebase} codebases: ${options?.codebases
        ?.map((c) => c.relative_path)
        .join(", ")}`
    );
  }
  for (const codebase of options?.codebases ?? []) {
    let _digest: string | undefined = undefined;
    let alreadyPrinted = false;
    let hasAssets = false;
    const getDigest: (forceTar?: boolean) => Promise<string> = async (
      forceTar?: boolean
    ) => {
      if (_digest == undefined) {
        _digest = await digestDir(
          codebase.relative_path,
          JSON.stringify(codebase)
        );
        const extra = codebase.extra_digest_paths ?? [];
        if (extra.length > 0) {
          const hashes = await Promise.all(extra.map(digestPath));
          _digest = await generateHash(_digest + hashes.join(""));
        }
        if (codebase.format == "esm") {
          _digest += ".esm";
        }
        if (!alreadyPrinted) {
          alreadyPrinted = true;
          log.info(`Codebase ${codebase.relative_path}, digest: ${_digest}`);
        }
        hasAssets =
          Array.isArray(codebase.assets) && codebase.assets.length > 0;
      }
      if (forceTar || hasAssets) {
        return _digest + ".tar";
      } else {
        return _digest;
      }
    };
    res.push({ ...codebase, getDigest });
  }

  return res;
}
