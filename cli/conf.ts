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

    // Error if obsolete overrides format is detected - no longer supported
    if (conf && 'overrides' in conf) {
      throw new Error(
        "❌ The 'overrides' field is no longer supported.\n" +
        "   The configuration system now uses Git branch-based configuration only.\n" +
        "   Please delete your wmill.yaml and run 'wmill init' to recreate it with the new format."
      );
    }

    if (conf?.defaultTs == undefined) {
      log.warn(
        "No defaultTs defined in your wmill.yaml. Using 'bun' as default."
      );
    }
    return typeof conf == "object" ? conf : ({} as SyncOptions);
  } catch (e) {
    if (e instanceof Error && (e.message.includes("overrides") || e.message.includes("Obsolete configuration format"))) {
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

// Get effective settings by merging top-level settings with branch-specific overrides
export function getEffectiveSettings(config: SyncOptions): SyncOptions {
  // Start with top-level settings from config
  const { branches, ...topLevelSettings } = config;
  let effective = { ...topLevelSettings };

  if (isGitRepository()) {
    const currentBranch = getCurrentGitBranch();

    if (currentBranch && branches && branches[currentBranch] && branches[currentBranch].overrides) {
      Object.assign(effective, branches[currentBranch].overrides);
      log.info(`Applied settings for Git branch: ${currentBranch}`);
    } else if (currentBranch) {
      log.debug(`No branch-specific overrides found for '${currentBranch}', using top-level settings`);
    }
  } else {
    log.debug("Not in a Git repository, using top-level settings");
  }

  return effective;
}
