import { Codebase, SyncOptions } from "../core/conf.ts";
import { log } from "../../deps.ts";
import { digestDir } from "./utils.ts";

export type SyncCodebase = Codebase & { getDigest: () => Promise<string> };
export function listSyncCodebases(
  options: SyncOptions
): SyncCodebase[] {
  const res: SyncCodebase[] = [];
  const nb_codebase = options?.codebases?.length ?? 0;
  if (nb_codebase > 0) {
    log.info(`Found ${nb_codebase} codebases: ${options?.codebases?.map((c) => c.relative_path).join(", ")}`);
  }
  for (const codebase of options?.codebases ?? []) {
    let _digest: string | undefined = undefined;
    const getDigest: () => Promise<string> = async () => {
      if (_digest == undefined) {
        _digest = await digestDir(
          codebase.relative_path,
          JSON.stringify(codebase)
        );
        if (Array.isArray(codebase.assets) && codebase.assets.length > 0) {
          _digest += ".tar";
        }
        log.info(`Codebase ${codebase.relative_path}, digest: ${_digest}`);
      }
      return _digest;
    };
    res.push({ ...codebase, getDigest });
  }

  return res;
}
