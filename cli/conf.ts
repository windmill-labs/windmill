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
  includeWmillYaml?: boolean;
  settingsFromJson?: string;
  workspaces?: { [workspaceName: string]: WorkspaceProfile };
  defaultWorkspace?: string;
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
  skipSecrets: false,
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

export interface WorkspaceProfile {
  baseUrl: string;
  workspaceId: string;
  repositories?: { [repositoryPath: string]: RepositorySyncOptions };
  currentRepository?: string;
}

export interface RepositorySyncOptions {
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
  includes?: string[];
  extraIncludes?: string[];
  excludes?: string[];
  defaultTs?: "bun" | "deno";
  codebases?: Codebase[];
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
      // Check if this is a multi-workspace format with defaultTs in repositories
      let hasRepositoryDefaultTs = false;
      if (conf?.workspaces) {
        for (const workspace of Object.values(conf.workspaces)) {
          if (workspace.repositories) {
            for (const repo of Object.values(workspace.repositories)) {
              if (repo.defaultTs) {
                hasRepositoryDefaultTs = true;
                break;
              }
            }
          }
          if (hasRepositoryDefaultTs) break;
        }
      }

      // Only warn if neither top-level nor repository-level defaultTs is found
      if (!hasRepositoryDefaultTs) {
        log.warn(
          "No defaultTs defined in your wmill.yaml. Using 'bun' as default."
        );
      }
    }
    return typeof conf == "object" ? conf : ({} as SyncOptions);
  } catch (e) {
    log.warn(
      "No wmill.yaml found. Use 'wmill init' to bootstrap it. Using 'bun' as default typescript runtime."
    );
    return {};
  }
}

export function getWorkspaceProfile(
  config: SyncOptions,
  workspaceName: string
): WorkspaceProfile | undefined {
  return config.workspaces?.[workspaceName];
}

export function getWorkspaceRepositorySettings(
  config: SyncOptions,
  workspaceName: string,
  repositoryPath: string
): SyncOptions {
  const workspaceProfile = config.workspaces?.[workspaceName];
  if (!workspaceProfile) {
    // Fallback to legacy format - just return base config (no repository awareness)
    return config;
  }

  // Get only repository-specific settings (no workspace defaults)
  const repositorySettings = workspaceProfile.repositories?.[repositoryPath];
  if (!repositorySettings) {
    // No repository settings - return base config
    return config;
  }

  return {
    ...config,
    ...repositorySettings,
  };
}

export function listWorkspaces(config: SyncOptions): string[] {
  if (config.workspaces) {
    return Object.keys(config.workspaces);
  }
  return [];
}

export function listWorkspaceRepositories(
  config: SyncOptions,
  workspaceName: string
): string[] {
  const workspaceProfile = config.workspaces?.[workspaceName];
  if (workspaceProfile?.repositories) {
    return Object.keys(workspaceProfile.repositories);
  }
  return [];
}

export async function mergeConfigWithConfigFile<T>(
  opts: T
): Promise<T & SyncOptions> {
  const configFile = await readConfigFile();
  return Object.assign(configFile ?? {}, opts);
}
