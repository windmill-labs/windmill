import { Codebase, SyncOptions } from "./conf.ts";
import { log } from "./deps.ts";
import { digestDir } from "./utils.ts";

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
    const digest = await digestDir(codebase.relative_path);
    log.info(`Codebase ${codebase.relative_path}, digest: ${digest}`);
    res.push({ ...codebase, digest });
  }

  return res;
}
