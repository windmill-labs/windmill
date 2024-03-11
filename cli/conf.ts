import { log, yamlParse } from "./deps.ts";

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
  includeUsers?: boolean;
  includeGroups?: boolean;
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
    if (conf?.defaultTs == undefined) {
      log.warning(
        'No defaultTs defined in your wmill.yaml, using deno as default typescript language. Set defaultTs in wmill.yaml to "bun" to switch (https://www.windmill.dev/docs/advanced/cli/sync#wmillyaml)'
      );
    }
    return typeof conf == "object" ? conf : ({} as SyncOptions);
  } catch (e) {
    log.warning(
      'No wmill.yaml found, using deno as default typescript language. Create a wmill.yaml with a defaultTs set to "bun" to switch (https://www.windmill.dev/docs/advanced/cli/sync#wmillyaml)'
    );
    return {};
  }
}

export async function mergeConfigWithConfigFile<T>(
  opts: T
): Promise<T & SyncOptions> {
  const configFile = await readConfigFile();
  return Object.assign(configFile, opts);
}
