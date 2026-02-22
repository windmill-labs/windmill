import { Codebase, SyncOptions } from "../core/conf.ts";
import * as log from "../core/log.ts";
import { digestDir } from "./utils.ts";

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
