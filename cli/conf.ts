import { yamlParse } from "./deps.ts";

export interface SyncOptions {
  stateful?: boolean;
  raw?: boolean;
  yes?: boolean;
  skipPull?: boolean;
  failConflicts?: boolean;
  plainSecrets?: boolean;
  json?: boolean;
  skipVariables?: boolean;
  skipResources?: boolean;
  skipSecrets?: boolean;
  includeSchedules?: boolean;
  message?: string;
  includes?: string[];
  extraIncludes?: string[];
  excludes?: string[];
  defaultTs?: "bun" | "deno";
}

export async function readConfigFile(): Promise<SyncOptions> {
  try {
    const conf = yamlParse(
      await Deno.readTextFile("wmill.yaml")
    ) as SyncOptions;

    return typeof conf == "object" ? conf : ({} as SyncOptions);
  } catch (e) {
    return {};
  }
}

export async function mergeConfigWithConfigFile<T>(
  opts: T
): Promise<T & SyncOptions> {
  const configFile = await readConfigFile();
  return Object.assign(configFile, opts);
}
