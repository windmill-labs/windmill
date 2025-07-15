import { log, yamlParseFile } from "./deps.ts";

export interface SyncOptions {
  stateful?: boolean;
  raw?: boolean;
  yes?: boolean;
  dryRun?: boolean;
  skipPull?: boolean;
  failConflicts?: boolean;
  plainSecrets?: boolean;
  json?: boolean;
  skipVariables?: boolean;
  skipResources?: boolean;
  skipResourceTypes?: boolean;
  skipSecrets?: boolean;
  skipScripts?: boolean;
  skipFlows?: boolean;
  skipApps?: boolean;
  skipFolders?: boolean;
  includeSchedules?: boolean;
  includeTriggers?: boolean;
  includeUsers?: boolean;
  includeGroups?: boolean;
  includeSettings?: boolean;
  includeKey?: boolean;
  message?: string;
  includes?: string[];
  extraIncludes?: string[];
  excludes?: string[];
  defaultTs?: "bun" | "deno";
  codebases?: Codebase[];
  parallel?: number;
  jsonOutput?: boolean;
  overrides?: { [key: string]: Partial<SyncOptions> };
}

export interface Codebase {
  relative_path: string;
  includes?: string[];
  excludes?: string[];
  assets?: {
    from: string;
    to: string;
  }[];
  customBundler?: string;
  external?: string[];
  define?: { [key: string]: string };
  inject?: string[];
}

export async function readConfigFile(): Promise<SyncOptions> {
  try {
    const conf = (await yamlParseFile("wmill.yaml")) as SyncOptions;
    if (conf?.defaultTs == undefined) {
      log.warn(
        "No defaultTs defined in your wmill.yaml. Using 'bun' as default."
      );
    }
    return typeof conf == "object" ? conf : ({} as SyncOptions);
  } catch (e) {
    log.warn(
      "No wmill.yaml found. Use 'wmill init' to bootstrap it. Using 'bun' as default typescript runtime."
    );
    return {};
  }
}

// Default sync options - shared across the codebase to prevent duplication
export const DEFAULT_SYNC_OPTIONS: Readonly<Required<Pick<SyncOptions,
  'defaultTs' | 'includes' | 'excludes' | 'codebases' | 'skipVariables' | 'skipResources' |
  'skipResourceTypes' | 'skipSecrets' | 'includeSchedules' | 'includeTriggers' |
  'skipScripts' | 'skipFlows' | 'skipApps' | 'skipFolders' |
  'includeUsers' | 'includeGroups' | 'includeSettings' | 'includeKey'
>>> = {
  defaultTs: 'bun',
  includes: ['f/**'],
  excludes: [],
  codebases: [],
  skipVariables: false,
  skipResources: false,
  skipResourceTypes: false,
  skipSecrets: true,
  skipScripts: false,
  skipFlows: false,
  skipApps: false,
  skipFolders: false,
  includeSchedules: false,
  includeTriggers: false,
  includeUsers: false,
  includeGroups: false,
  includeSettings: false,
  includeKey: false
} as const;

export async function mergeConfigWithConfigFile<T>(
  opts: T
): Promise<T & SyncOptions> {
  const configFile = await readConfigFile();
  return Object.assign(configFile ?? {}, opts);
}

// Get effective settings by merging top-level settings and overrides
export function getEffectiveSettings(
  config: SyncOptions,
  baseUrl: string,
  workspaceId: string,
  repo: string
): SyncOptions {
  // Start with empty object - no defaults
  let effective = {} as SyncOptions;
  
  // Merge top-level settings from config (which contains user's chosen defaults)
  Object.keys(config).forEach(key => {
    if (key !== 'overrides' && config[key as keyof SyncOptions] !== undefined) {
      (effective as any)[key] = config[key as keyof SyncOptions];
    }
  });
  
  if (!config.overrides) {
    return effective;
  }
  
  // Construct override keys using the single format
  const workspaceKey = `${baseUrl}:${workspaceId}:*`;
  const repoKey = `${baseUrl}:${workspaceId}:${repo}`;
  
  // Apply workspace-level overrides
  if (config.overrides[workspaceKey]) {
    Object.assign(effective, config.overrides[workspaceKey]);
  }
  
  // Apply repository-specific overrides (overrides workspace-level)
  if (config.overrides[repoKey]) {
    Object.assign(effective, config.overrides[repoKey]);
  }
  
  return effective;
}
