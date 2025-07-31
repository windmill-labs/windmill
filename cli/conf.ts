import { log, yamlParseFile } from "./deps.ts";
import { getCurrentGitBranch, isGitRepository } from "./git.ts";

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
  branches?: { [branchName: string]: SyncOptions & { overrides?: Partial<SyncOptions>; promotionOverrides?: Partial<SyncOptions> } };
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

    // Check for obsolete overrides format - warn but don't error for now
    if (conf?.overrides && !conf.branches) {
      log.warn(
        "⚠️  The 'overrides' field is deprecated and will be removed in a future version.\n" +
        "   Please migrate to the new 'branches' format.\n" +
        "   See documentation for migration guide."
      );
    }

    if (conf?.defaultTs == undefined) {
      log.warn(
        "No defaultTs defined in your wmill.yaml. Using 'bun' as default."
      );
    }
    return typeof conf == "object" ? conf : ({} as SyncOptions);
  } catch (e) {
    if (e instanceof Error && e.message.includes("Obsolete configuration format")) {
      throw e; // Re-throw the specific obsolete format error
    }
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

// Get effective settings by merging top-level settings and overrides/branches
export function getEffectiveSettings(
  config: SyncOptions,
  baseUrl: string,
  workspaceId: string,
  repo: string
): SyncOptions {
  // Start with top-level settings from config (like old override system)
  const { overrides, branches, ...topLevelSettings } = config;
  let effective = { ...topLevelSettings };

  // Check if we're in a Git repository and have branch-based configuration
  if (config.branches && isGitRepository()) {
    const currentBranch = getCurrentGitBranch();

    if (currentBranch && config.branches[currentBranch]) {
      const branchConfig = config.branches[currentBranch];

      // Apply branch-level settings first
      Object.keys(branchConfig).forEach(key => {
        if (key !== 'overrides' && key !== 'promotionOverrides' && branchConfig[key as keyof SyncOptions] !== undefined) {
          (effective as any)[key] = branchConfig[key as keyof SyncOptions];
        }
      });

      // Apply branch overrides if they exist
      if (branchConfig.overrides) {
        Object.assign(effective, branchConfig.overrides);
      }

      // Apply promotion overrides if they exist
      if (branchConfig.promotionOverrides) {
        // Object.assign(effective, branchConfig.promotionOverrides);
      }

      log.info(`Applied settings for Git branch: ${currentBranch}`);
      return effective;
    } else if (currentBranch) {
      log.info(`No settings found for Git branch '${currentBranch}', using top-level settings`);
    }
  }

  // Fallback to existing override logic for backward compatibility
  if (!config.overrides) {
    if (repo) {
      log.info(`No overrides found in wmill.yaml, using top-level settings (repository flag ignored)`);
    }
    return effective;
  }

  // Construct override keys using the single format
  const workspaceKey = `${baseUrl}:${workspaceId}:*`;
  const repoKey = `${baseUrl}:${workspaceId}:${repo}`;

  let appliedOverrides: string[] = [];

  // Apply workspace-level overrides
  if (config.overrides[workspaceKey]) {
    Object.assign(effective, config.overrides[workspaceKey]);
    appliedOverrides.push("workspace-level");
  }

  // Apply repository-specific overrides (overrides workspace-level)
  if (config.overrides[repoKey]) {
    Object.assign(effective, config.overrides[repoKey]);
    appliedOverrides.push("repository-specific");
  } else if (repo) {
    // Repository was specified but no override found
    log.info(`Repository override not found for "${repo}", using ${appliedOverrides.length > 0 ? appliedOverrides.join(" + ") : "top-level"} settings`);
  }

  if (appliedOverrides.length > 0) {
    log.info(`Applied ${appliedOverrides.join(" + ")} overrides${repo ? ` for repository "${repo}"` : ""}`);
  }

  return effective;
}
