import { Codebase, SyncOptions } from "./conf.js";
import { log } from "./deps.js";
import { digestDir } from "./utils.js";

export type SyncCodebase = Codebase & { digest: string };
export async function listSyncCodebases(
  options: SyncOptions
): Promise<SyncCodebase[]> {
  const res: SyncCodebase[] = [];
  const nb_codebase = options?.codebases?.length ?? 0;
  if (nb_codebase > 0) {
    log.info(`Found ${nb_codebase} codebases:`);
  }
  for (const codebase of options?.codebases ?? []) {
    let digest = await digestDir(
      codebase.relative_path,
      JSON.stringify(codebase)
    );
    if (Array.isArray(codebase.assets) && codebase.assets.length > 0) {
      digest += ".tar";
    }
    log.info(`Codebase ${codebase.relative_path}, digest: ${digest}`);
    res.push({ ...codebase, digest });
  }

  return res;
}
